use super::Processor;

/// Type de filtre EQ.
///
/// # Les 3 types classiques d'un EQ paramétrique
/// - **LowShelf** : booste/coupe les fréquences SOUS une fréquence donnée
///   (ex: +3dB sous 200Hz = plus de basses)
/// - **Peaking** : booste/coupe autour d'une fréquence précise
///   (ex: -5dB à 3kHz = réduit la zone nasale de la voix)
/// - **HighShelf** : booste/coupe les fréquences AU-DESSUS d'une fréquence
///   (ex: +2dB au-dessus de 8kHz = plus d'air/brillance)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
}

/// Une bande d'EQ paramétrique.
///
/// # Biquad filter — le filtre numérique universel
/// Tous les filtres audio (low pass, high pass, shelf, peaking, notch...)
/// peuvent être implémentés comme un filtre "biquad" (bi-quadratique).
///
/// L'équation : y[n] = (b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]) / a0
///
/// Les coefficients (b0, b1, b2, a0, a1, a2) déterminent le type de filtre.
/// On les recalcule quand les paramètres changent (fréquence, gain, Q).
///
/// # Pourquoi "biquad" ?
/// "Bi" = deux pôles et deux zéros dans le plan complexe (z-transform).
/// C'est la forme la plus simple de filtre IIR (Infinite Impulse Response)
/// qui peut modéliser tous les filtres audio classiques.
#[derive(Debug, Clone)]
pub struct EqBand {
    pub filter_type: FilterType,
    /// Fréquence centrale en Hz (20 → 20000)
    pub frequency: f32,
    /// Gain en dB (-12.0 → +12.0). Négatif = coupe, positif = booste.
    pub gain_db: f32,
    /// Q factor (largeur de la bande). Plus Q est grand, plus la bande est étroite.
    /// 0.5 = très large, 1.0 = standard, 4.0 = chirurgical
    pub q: f32,
    /// Coefficients du filtre biquad
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    /// État du filtre (mémoire des 2 samples précédents)
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    /// Active/désactivée
    pub enabled: bool,
}

impl EqBand {
    /// Crée une nouvelle bande EQ.
    pub fn new(filter_type: FilterType, frequency: f32, gain_db: f32, q: f32) -> Self {
        let mut band = Self {
            filter_type,
            frequency: frequency.clamp(20.0, 20000.0),
            gain_db: gain_db.clamp(-12.0, 12.0),
            q: q.clamp(0.1, 10.0),
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            enabled: true,
        };
        band.compute_coefficients(48000.0);
        band
    }

    /// Recalcule les coefficients biquad.
    ///
    /// # La formule de Robert Bristow-Johnson
    /// C'est LA référence pour les coefficients de filtres audio biquad.
    /// "Audio EQ Cookbook" — utilisé par tous les DAW et plugins.
    ///
    /// Les formules dépendent du type de filtre mais partagent des
    /// variables intermédiaires : omega, sin, cos, alpha, A.
    pub fn compute_coefficients(&mut self, sample_rate: f32) {
        let a = 10.0_f32.powf(self.gain_db / 40.0); // Amplitude from dB
        let omega = 2.0 * std::f32::consts::PI * self.frequency / sample_rate;
        let sin_w = omega.sin();
        let cos_w = omega.cos();
        let alpha = sin_w / (2.0 * self.q);

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Normaliser par a0
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Traite un sample avec le filtre biquad.
    ///
    /// # Direct Form I
    /// y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
    ///
    /// On garde en mémoire les 2 derniers samples d'entrée (x1, x2)
    /// et les 2 derniers samples de sortie (y1, y2).
    pub fn process(&mut self, sample: f32) -> f32 {
        if !self.enabled {
            return sample;
        }

        let out = self.b0 * sample + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        // Mettre à jour l'état
        self.x2 = self.x1;
        self.x1 = sample;
        self.y2 = self.y1;
        self.y1 = out;

        out
    }

    /// Réinitialise l'état du filtre.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// EQ paramétrique complet avec N bandes.
///
/// Un EQ paramétrique standard a 3-5 bandes :
/// - Bande 1 : Low Shelf (basses)
/// - Bande 2 : Peaking (bas-médiums)
/// - Bande 3 : Peaking (médiums)
/// - Bande 4 : Peaking (hauts-médiums)
/// - Bande 5 : High Shelf (aigus)
pub struct ParametricEq {
    bands: Vec<EqBand>,
    bypassed: bool,
}

impl ParametricEq {
    pub fn new() -> Self {
        Self {
            bands: Vec::new(),
            bypassed: false,
        }
    }

    /// Crée un EQ 3 bandes par défaut (flat — 0dB partout).
    pub fn default_3band() -> Self {
        Self {
            bands: vec![
                EqBand::new(FilterType::LowShelf, 200.0, 0.0, 0.7),
                EqBand::new(FilterType::Peaking, 1000.0, 0.0, 1.0),
                EqBand::new(FilterType::HighShelf, 8000.0, 0.0, 0.7),
            ],
            bypassed: false,
        }
    }

    /// Nombre de bandes.
    pub fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// Accède à une bande par index.
    pub fn band(&self, index: usize) -> Option<&EqBand> {
        self.bands.get(index)
    }

    /// Modifie une bande par index.
    pub fn band_mut(&mut self, index: usize) -> Option<&mut EqBand> {
        self.bands.get_mut(index)
    }

    /// Met à jour les paramètres d'une bande et recalcule les coefficients.
    pub fn set_band(
        &mut self,
        index: usize,
        frequency: f32,
        gain_db: f32,
        q: f32,
        sample_rate: f32,
    ) {
        if let Some(band) = self.bands.get_mut(index) {
            band.frequency = frequency.clamp(20.0, 20000.0);
            band.gain_db = gain_db.clamp(-12.0, 12.0);
            band.q = q.clamp(0.1, 10.0);
            band.compute_coefficients(sample_rate);
        }
    }

    /// Réinitialise toutes les bandes.
    pub fn reset_all(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }
}

impl Default for ParametricEq {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for ParametricEq {
    fn process_sample(&mut self, sample: f32) -> f32 {
        if self.bypassed {
            return sample;
        }

        let mut s = sample;
        for band in &mut self.bands {
            s = band.process(s);
        }
        s
    }

    fn reset(&mut self) {
        self.reset_all();
    }

    fn set_bypass(&mut self, bypass: bool) {
        self.bypassed = bypass;
    }

    fn is_bypassed(&self) -> bool {
        self.bypassed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_eq_is_passthrough() {
        // Un EQ avec toutes les bandes à 0dB ne doit pas modifier le signal.
        let mut eq = ParametricEq::default_3band();

        // Envoyer une sinusoïde et vérifier que le RMS est quasi identique
        let input: Vec<f32> = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.5)
            .collect();

        let output: Vec<f32> = input.iter().map(|&s| eq.process_sample(s)).collect();

        let in_rms = (input.iter().map(|s| s * s).sum::<f32>() / input.len() as f32).sqrt();
        let out_rms = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();

        let ratio = out_rms / in_rms;
        assert!(
            (0.95..=1.05).contains(&ratio),
            "Flat EQ should be passthrough, ratio = {ratio}"
        );
    }

    #[test]
    fn eq_boost_increases_energy() {
        let mut eq = ParametricEq::default_3band();
        // Boost les médiums de +6dB
        eq.set_band(1, 1000.0, 6.0, 1.0, 48000.0);

        let input: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin() * 0.3)
            .collect();

        let output: Vec<f32> = input.iter().map(|&s| eq.process_sample(s)).collect();

        // Ignorer les premiers samples (le filtre a besoin de "se stabiliser")
        let out_rms =
            (output[512..].iter().map(|s| s * s).sum::<f32>() / (output.len() - 512) as f32).sqrt();
        let in_rms =
            (input[512..].iter().map(|s| s * s).sum::<f32>() / (input.len() - 512) as f32).sqrt();

        assert!(
            out_rms > in_rms * 1.3,
            "Boost should increase energy: in={in_rms}, out={out_rms}"
        );
    }

    #[test]
    fn eq_cut_decreases_energy() {
        let mut eq = ParametricEq::default_3band();
        // Cut les médiums de -6dB
        eq.set_band(1, 1000.0, -6.0, 1.0, 48000.0);

        let input: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin() * 0.5)
            .collect();

        let output: Vec<f32> = input.iter().map(|&s| eq.process_sample(s)).collect();

        let out_rms =
            (output[512..].iter().map(|s| s * s).sum::<f32>() / (output.len() - 512) as f32).sqrt();
        let in_rms =
            (input[512..].iter().map(|s| s * s).sum::<f32>() / (input.len() - 512) as f32).sqrt();

        assert!(
            out_rms < in_rms * 0.8,
            "Cut should decrease energy: in={in_rms}, out={out_rms}"
        );
    }

    #[test]
    fn eq_bypass() {
        let mut eq = ParametricEq::default_3band();
        eq.set_band(1, 1000.0, 12.0, 1.0, 48000.0);
        eq.set_bypass(true);
        assert_eq!(eq.process_sample(0.5), 0.5);
    }

    #[test]
    fn eq_band_count() {
        let eq = ParametricEq::default_3band();
        assert_eq!(eq.band_count(), 3);
    }

    #[test]
    fn eq_band_access() {
        let eq = ParametricEq::default_3band();
        let band = eq.band(0).unwrap();
        assert_eq!(band.filter_type, FilterType::LowShelf);
        assert_eq!(band.frequency, 200.0);
    }

    #[test]
    fn eq_reset() {
        let mut eq = ParametricEq::default_3band();
        // Process some samples
        for i in 0..100 {
            eq.process_sample((i as f32 * 0.1).sin());
        }
        // Reset
        eq.reset();
        // After reset, internal state should be zero
        let band = eq.band(0).unwrap();
        assert_eq!(band.x1, 0.0);
        assert_eq!(band.y1, 0.0);
    }
}
