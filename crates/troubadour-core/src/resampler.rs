use rubato::{FftFixedInOut, Resampler as _};
use troubadour_shared::error::{TroubadourError, TroubadourResult};

/// Wrapper autour de rubato pour la conversion de sample rate.
///
/// # Pourquoi un wrapper ?
/// `rubato::FftFixedInOut` est un type générique complexe avec beaucoup
/// de paramètres. Ce wrapper :
/// 1. Simplifie l'API pour notre cas d'usage (audio interleaved f32)
/// 2. Gère la conversion interleaved ↔ planar (voir plus bas)
/// 3. Cache les détails d'implémentation de rubato
///
/// # Interleaved vs Planar
/// L'audio du système (cpal) arrive en format **interleaved** :
///   [L0, R0, L1, R1, L2, R2, ...]
///
/// rubato travaille en format **planar** (un Vec par canal) :
///   canal 0: [L0, L1, L2, ...]
///   canal 1: [R0, R1, R2, ...]
///
/// On doit convertir dans les deux sens. C'est un coût CPU, mais
/// c'est nécessaire car les deux libs ont des conventions différentes.
pub struct AudioResampler {
    resampler: FftFixedInOut<f32>,
    channels: usize,
    /// Nombre de frames en entrée attendu par rubato à chaque appel.
    /// Une "frame" = 1 sample par canal (ex: 1 frame stéréo = 2 samples).
    input_frames: usize,
}

impl AudioResampler {
    /// Crée un nouveau resampler.
    ///
    /// # Paramètres
    /// - `from_rate` : sample rate source (ex: 44100)
    /// - `to_rate` : sample rate destination (ex: 48000)
    /// - `channels` : nombre de canaux (1 = mono, 2 = stéréo)
    /// - `chunk_size` : nombre de frames par chunk (ex: 256)
    ///
    /// # `FftFixedInOut` — pourquoi FFT ?
    /// rubato propose plusieurs algorithmes de resampling :
    /// - `SincFixedIn` : filtre sinc, taille d'entrée fixe → plus précis
    /// - `FftFixedInOut` : basé sur FFT, tailles fixe in ET out → plus prévisible
    ///
    /// On choisit `FftFixedInOut` car dans le contexte audio temps réel,
    /// on a besoin de savoir exactement combien de samples on produit
    /// à chaque appel. Pas de surprise = pas de glitch audio.
    pub fn new(
        from_rate: u32,
        to_rate: u32,
        channels: usize,
        chunk_size: usize,
    ) -> TroubadourResult<Self> {
        // Si les rates sont identiques, on crée quand même le resampler
        // mais il sera un "passthrough" (ratio = 1.0).
        let resampler =
            FftFixedInOut::new(from_rate as usize, to_rate as usize, chunk_size, channels)
                .map_err(|e| TroubadourError::StreamError(format!("Resampler init failed: {e}")))?;

        let input_frames = resampler.input_frames_max();

        Ok(Self {
            resampler,
            channels,
            input_frames,
        })
    }

    /// Nombre de frames d'entrée attendu par appel.
    pub fn input_frames_required(&self) -> usize {
        self.input_frames
    }

    /// Nombre de frames de sortie produit par appel.
    pub fn output_frames(&self) -> usize {
        self.resampler.output_frames_max()
    }

    /// Convertit un buffer interleaved d'un sample rate à un autre.
    ///
    /// # Le flux de données
    /// ```text
    /// interleaved input    →  deinterleave  →  rubato  →  interleave  →  output
    /// [L0,R0,L1,R1,...]   →  [[L0,L1,...],  →  resamp  →  [L0,R0,...]
    ///                          [R0,R1,...]]
    /// ```
    ///
    /// # `&mut self` — pourquoi mutable ?
    /// rubato maintient un état interne (filtres FFT, buffers).
    /// Chaque appel modifie cet état. D'où le `&mut`.
    pub fn process(&mut self, interleaved_input: &[f32]) -> TroubadourResult<Vec<f32>> {
        let frames = interleaved_input.len() / self.channels;

        // Étape 1 : Deinterleave (interleaved → planar)
        let planar_input = Self::deinterleave(interleaved_input, self.channels, frames);

        // Étape 2 : Resampling
        // `process()` retourne un Vec<Vec<f32>> (un Vec par canal)
        let planar_output = self
            .resampler
            .process(&planar_input, None)
            .map_err(|e| TroubadourError::StreamError(format!("Resampling failed: {e}")))?;

        // Étape 3 : Interleave (planar → interleaved)
        Ok(Self::interleave(&planar_output))
    }

    /// Vérifie si le resampling est nécessaire (rates différents).
    pub fn is_passthrough(from_rate: u32, to_rate: u32) -> bool {
        from_rate == to_rate
    }

    /// Convertit un buffer interleaved en format planar.
    ///
    /// # Algorithme
    /// ```text
    /// Input:  [L0, R0, L1, R1, L2, R2]
    /// Output: [[L0, L1, L2], [R0, R1, R2]]
    /// ```
    ///
    /// On itère sur chaque frame et on distribue les samples
    /// dans le bon canal. `chunks_exact` est plus efficace que
    /// `chunks` car le compilateur sait que chaque chunk a
    /// exactement `channels` éléments → il peut optimiser les
    /// bounds checks.
    fn deinterleave(interleaved: &[f32], channels: usize, frames: usize) -> Vec<Vec<f32>> {
        let mut planar = vec![Vec::with_capacity(frames); channels];

        for frame in interleaved.chunks_exact(channels) {
            for (ch, &sample) in frame.iter().enumerate() {
                planar[ch].push(sample);
            }
        }

        planar
    }

    /// Convertit un buffer planar en format interleaved.
    ///
    /// # L'inverse de deinterleave
    /// ```text
    /// Input:  [[L0, L1, L2], [R0, R1, R2]]
    /// Output: [L0, R0, L1, R1, L2, R2]
    /// ```
    fn interleave(planar: &[Vec<f32>]) -> Vec<f32> {
        if planar.is_empty() {
            return Vec::new();
        }

        let frames = planar[0].len();
        let channels = planar.len();
        let mut interleaved = Vec::with_capacity(frames * channels);

        for frame_idx in 0..frames {
            for channel in planar {
                interleaved.push(channel[frame_idx]);
            }
        }

        interleaved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deinterleave_stereo() {
        // [L0, R0, L1, R1] → [[L0, L1], [R0, R1]]
        let interleaved = vec![1.0, 2.0, 3.0, 4.0];
        let planar = AudioResampler::deinterleave(&interleaved, 2, 2);

        assert_eq!(planar.len(), 2); // 2 canaux
        assert_eq!(planar[0], vec![1.0, 3.0]); // canal gauche
        assert_eq!(planar[1], vec![2.0, 4.0]); // canal droit
    }

    #[test]
    fn deinterleave_mono() {
        let interleaved = vec![1.0, 2.0, 3.0];
        let planar = AudioResampler::deinterleave(&interleaved, 1, 3);

        assert_eq!(planar.len(), 1);
        assert_eq!(planar[0], vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn interleave_stereo() {
        let planar = vec![vec![1.0, 3.0], vec![2.0, 4.0]];
        let interleaved = AudioResampler::interleave(&planar);

        assert_eq!(interleaved, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn interleave_empty() {
        let planar: Vec<Vec<f32>> = vec![];
        let interleaved = AudioResampler::interleave(&planar);
        assert!(interleaved.is_empty());
    }

    #[test]
    fn roundtrip_deinterleave_interleave() {
        // Deinterleave puis re-interleave doit donner l'original.
        // C'est un test de "roundtrip" — très utile pour valider
        // que deux opérations inverses sont correctes.
        let original = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let planar = AudioResampler::deinterleave(&original, 2, 3);
        let result = AudioResampler::interleave(&planar);

        assert_eq!(original.len(), result.len());
        for (a, b) in original.iter().zip(result.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn passthrough_detection() {
        assert!(AudioResampler::is_passthrough(48000, 48000));
        assert!(!AudioResampler::is_passthrough(44100, 48000));
    }

    #[test]
    fn create_resampler_same_rate() {
        // Même sample rate → le resampler fonctionne comme passthrough
        let resampler = AudioResampler::new(48000, 48000, 2, 256);
        assert!(resampler.is_ok());
    }

    #[test]
    fn create_resampler_44100_to_48000() {
        let resampler = AudioResampler::new(44100, 48000, 2, 1024);
        assert!(resampler.is_ok());

        let r = resampler.unwrap();
        assert!(r.input_frames_required() > 0);
        assert!(r.output_frames() > 0);
    }

    #[test]
    fn resample_silence() {
        // Resampler du silence doit produire du silence (ou très proche de 0).
        // C'est un test important : le resampler ne doit pas introduire
        // de bruit sur un signal nul.
        let mut resampler = AudioResampler::new(44100, 48000, 2, 1024).unwrap();
        let input_frames = resampler.input_frames_required();
        let silence = vec![0.0_f32; input_frames * 2]; // stéréo

        let output = resampler.process(&silence).unwrap();

        // Chaque sample de sortie doit être très proche de 0
        for &sample in &output {
            assert!(sample.abs() < 0.001, "Expected silence, got {sample}");
        }
    }

    #[test]
    fn resample_preserves_energy() {
        // Un signal sinusoïdal resampleé doit conserver approximativement
        // la même énergie (RMS). C'est un test de qualité du resampling.
        let mut resampler = AudioResampler::new(44100, 48000, 1, 1024).unwrap();
        let input_frames = resampler.input_frames_required();

        // Générer une sinusoïde à 440Hz (La) à 44.1kHz
        let input: Vec<f32> = (0..input_frames)
            .map(|i| {
                let t = i as f32 / 44100.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        // RMS de l'entrée
        let input_rms = (input.iter().map(|&s| s * s).sum::<f32>() / input.len() as f32).sqrt();

        let output = resampler.process(&input).unwrap();

        // RMS de la sortie
        let output_rms = (output.iter().map(|&s| s * s).sum::<f32>() / output.len() as f32).sqrt();

        // L'énergie doit être conservée approximativement.
        // Le FFT resampler introduit des effets de bord sur les petits chunks
        // (début/fin du buffer), ce qui fait baisser légèrement le RMS.
        // Sur un flux audio continu, l'énergie serait beaucoup plus proche.
        // Tolérance large pour les tests unitaires sur chunks isolés.
        let ratio = output_rms / input_rms;
        assert!(
            (0.5..=1.5).contains(&ratio),
            "Energy ratio {ratio} is outside acceptable range (0.5-1.5)"
        );
    }

    #[test]
    fn resample_96k_to_48k_downsampling() {
        // Test de downsampling : 96kHz → 48kHz (divise par 2)
        let resampler = AudioResampler::new(96000, 48000, 2, 1024).unwrap();
        let input_frames = resampler.input_frames_required();
        let output_frames = resampler.output_frames();

        // Le nombre de frames en sortie doit être ~moitié de l'entrée
        let ratio = output_frames as f64 / input_frames as f64;
        assert!(
            (0.4..=0.6).contains(&ratio),
            "Expected ~0.5 ratio for 96k→48k, got {ratio}"
        );
    }
}
