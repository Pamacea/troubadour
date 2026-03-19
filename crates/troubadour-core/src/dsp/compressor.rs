use super::Processor;

/// Compresseur dynamique — réduit la plage dynamique du signal.
///
/// # Comment ça marche ?
/// Le compresseur rend les sons forts plus doux et (avec le makeup gain)
/// les sons doux plus forts. Résultat : un volume plus constant.
///
/// Imagine que tu cries dans le micro → le compresseur baisse le volume.
/// Tu chuchotes → le volume reste élevé grâce au makeup gain.
///
/// # Paramètres
/// - `threshold` : au-dessus de ce seuil, le compresseur s'active (0.0-1.0)
/// - `ratio` : facteur de compression (2.0 = pour 2dB au-dessus du seuil,
///   seulement 1dB passe. 10.0 = quasi-limiter)
/// - `attack` : vitesse de réaction quand le signal monte (lent = laisse passer
///   les transitoires/attaques, rapide = compresse tout)
/// - `release` : vitesse de relâchement quand le signal descend
/// - `makeup_gain` : gain ajouté après compression pour compenser la perte
///
/// # Le gain reduction
/// Le compresseur calcule un "gain reduction" (combien il baisse le volume).
/// C'est affiché dans l'UI comme un indicateur (barre rouge qui descend).
pub struct Compressor {
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    makeup_gain: f32,
    envelope: f32,
    /// Le gain reduction actuel (0.0 = pas de compression, négatif = compression)
    gain_reduction: f32,
    bypassed: bool,
}

impl Compressor {
    pub fn new() -> Self {
        Self {
            threshold: 0.4,   // Seuil plus haut - comprime seulement les vrais pics
            ratio: 3.0,       // 3:1 = compression douce
            attack: 0.005,    // Tres rapide
            release: 0.02,    // Release doux
            makeup_gain: 1.2, // Makeup leger pour ne pas amplifier le bruit
            envelope: 0.0,
            gain_reduction: 0.0,
            bypassed: false,
        }
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.01, 1.0);
    }

    /// Ratio de compression.
    /// 1.0 = pas de compression, 2.0 = 2:1, 10.0 = quasi-limiter.
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(1.0, 20.0);
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.clamp(0.001, 0.5);
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release.clamp(0.001, 0.5);
    }

    /// Makeup gain : compense la perte de volume due à la compression.
    /// 1.0 = pas de gain, 2.0 = double le volume.
    pub fn set_makeup_gain(&mut self, gain: f32) {
        self.makeup_gain = gain.clamp(0.0, 4.0);
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn ratio(&self) -> f32 {
        self.ratio
    }

    pub fn attack(&self) -> f32 {
        self.attack
    }

    pub fn release(&self) -> f32 {
        self.release
    }

    pub fn makeup_gain(&self) -> f32 {
        self.makeup_gain
    }

    /// Retourne le gain reduction actuel (pour l'UI).
    /// Valeur entre 0.0 (pas de compression) et 1.0 (compression max).
    pub fn current_gain_reduction(&self) -> f32 {
        self.gain_reduction
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Compressor {
    fn process_sample(&mut self, sample: f32) -> f32 {
        if self.bypassed {
            return sample;
        }

        // 1. Envelope follower (comme le noise gate)
        let abs_sample = sample.abs();
        let coeff = if abs_sample > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope += coeff * (abs_sample - self.envelope);

        // 2. Calculer le gain
        let gain = if self.envelope > self.threshold {
            // Au-dessus du seuil : comprimer
            //
            // Formule : gain = threshold + (envelope - threshold) / ratio
            // Normalisé : gain = résultat / envelope
            //
            // Exemple avec ratio 4:1, threshold 0.3, envelope 0.7 :
            //   target = 0.3 + (0.7 - 0.3) / 4 = 0.3 + 0.1 = 0.4
            //   gain = 0.4 / 0.7 ≈ 0.57
            //   → le signal est réduit à 57% de sa valeur originale
            let target = self.threshold + (self.envelope - self.threshold) / self.ratio;
            target / self.envelope.max(0.0001) // .max pour éviter division par 0
        } else {
            // Sous le seuil : pas de compression
            1.0
        };

        // Stocker le gain reduction pour l'UI
        self.gain_reduction = 1.0 - gain;

        // 3. Appliquer le gain + makeup
        sample * gain * self.makeup_gain
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain_reduction = 0.0;
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
    fn compressor_no_compression_below_threshold() {
        let mut comp = Compressor::new();
        comp.set_threshold(0.5);
        comp.set_makeup_gain(1.0); // Pas de makeup pour simplifier le test

        // Signal faible (0.1) sous le threshold (0.5) → pas de compression
        for _ in 0..200 {
            comp.process_sample(0.1);
        }

        let out = comp.process_sample(0.1);
        assert!(
            (out - 0.1).abs() < 0.05,
            "Should not compress below threshold, got {out}"
        );
    }

    #[test]
    fn compressor_reduces_loud_signal() {
        let mut comp = Compressor::new();
        comp.set_threshold(0.2);
        comp.set_ratio(4.0);
        comp.set_makeup_gain(1.0);

        // Signal fort (0.8) bien au-dessus du threshold (0.2)
        for _ in 0..200 {
            comp.process_sample(0.8);
        }

        let out = comp.process_sample(0.8);
        // Avec ratio 4:1, le signal doit être réduit significativement
        assert!(out < 0.5, "Should compress loud signal, got {out}");
        assert!(out > 0.1, "Should not kill the signal, got {out}");
    }

    #[test]
    fn compressor_gain_reduction_indicator() {
        let mut comp = Compressor::new();
        comp.set_threshold(0.2);
        comp.set_ratio(4.0);

        // Signal fort → gain reduction doit être > 0
        for _ in 0..200 {
            comp.process_sample(0.8);
        }

        assert!(
            comp.current_gain_reduction() > 0.1,
            "Should have gain reduction, got {}",
            comp.current_gain_reduction()
        );
    }

    #[test]
    fn compressor_makeup_gain() {
        let mut comp = Compressor::new();
        comp.set_threshold(0.5);
        comp.set_ratio(2.0);
        comp.set_makeup_gain(2.0);

        // Signal sous le threshold → pas de compression mais makeup s'applique
        for _ in 0..200 {
            comp.process_sample(0.1);
        }

        let out = comp.process_sample(0.1);
        // 0.1 * 1.0 (no compression) * 2.0 (makeup) = 0.2
        assert!(out > 0.15, "Makeup gain should amplify, got {out}");
    }

    #[test]
    fn compressor_bypass() {
        let mut comp = Compressor::new();
        comp.set_bypass(true);
        assert_eq!(comp.process_sample(0.8), 0.8);
    }

    #[test]
    fn compressor_reset() {
        let mut comp = Compressor::new();
        for _ in 0..200 {
            comp.process_sample(0.8);
        }
        assert!(comp.current_gain_reduction() > 0.0);

        comp.reset();
        assert_eq!(comp.current_gain_reduction(), 0.0);
    }

    #[test]
    fn compressor_ratio_clamping() {
        let mut comp = Compressor::new();
        comp.set_ratio(0.5);
        assert_eq!(comp.ratio(), 1.0);
        comp.set_ratio(100.0);
        assert_eq!(comp.ratio(), 20.0);
    }
}
