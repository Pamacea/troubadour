use super::Processor;

/// Limiter — empêche le signal de dépasser un plafond.
///
/// # Différence avec le compresseur
/// - Compresseur : réduit PROGRESSIVEMENT le signal au-dessus du seuil
/// - Limiter : BLOQUE le signal à un niveau maximum (ratio infini)
///
/// Le limiter est toujours le DERNIER dans la chaîne.
/// C'est le filet de sécurité qui empêche le clipping (distorsion).
///
/// # Clipping, c'est quoi ?
/// L'audio numérique est stocké en f32 entre -1.0 et 1.0.
/// Si un sample dépasse 1.0 (ou descend sous -1.0), le DAC (convertisseur
/// numérique→analogique) le "coupe" à 1.0. Ça crée une distorsion
/// très désagréable (un "crack" dans les enceintes/casque).
///
/// Le limiter prévient ça en réduisant le gain AVANT que le signal dépasse.
pub struct Limiter {
    /// Le plafond : le signal ne dépassera jamais cette valeur.
    /// 0.95 par défaut (un peu de marge avant le vrai 1.0)
    ceiling: f32,
    release: f32,
    /// Le gain appliqué (descend quand le signal approche le ceiling)
    gain: f32,
    bypassed: bool,
}

impl Limiter {
    pub fn new() -> Self {
        Self {
            ceiling: 0.95,
            release: 0.01,
            gain: 1.0,
            bypassed: false,
        }
    }

    /// Configure le plafond (0.1 → 1.0).
    pub fn set_ceiling(&mut self, ceiling: f32) {
        self.ceiling = ceiling.clamp(0.1, 1.0);
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release.clamp(0.001, 0.5);
    }

    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }

    pub fn release(&self) -> f32 {
        self.release
    }

    /// Retourne le gain actuel (pour l'UI).
    /// 1.0 = pas de limiting, < 1.0 = le limiter travaille.
    pub fn current_gain(&self) -> f32 {
        self.gain
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for Limiter {
    fn process_sample(&mut self, sample: f32) -> f32 {
        if self.bypassed {
            return sample;
        }

        let abs_sample = sample.abs();

        // Si le sample dépasse le ceiling → réduire le gain immédiatement
        if abs_sample * self.gain > self.ceiling {
            // Calculer le gain nécessaire pour rester sous le ceiling.
            // gain = ceiling / |sample|
            self.gain = self.ceiling / abs_sample.max(0.0001);
        } else {
            // Le signal est sous le ceiling → relâcher le gain doucement
            // vers 1.0 (pas de limiting).
            self.gain += self.release * (1.0 - self.gain);
        }

        sample * self.gain
    }

    fn reset(&mut self) {
        self.gain = 1.0;
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
    fn limiter_passes_normal_signal() {
        let mut lim = Limiter::new();

        // Signal normal (0.5) sous le ceiling (0.95) → passe tel quel
        for _ in 0..100 {
            lim.process_sample(0.5);
        }

        let out = lim.process_sample(0.5);
        assert!(
            (out - 0.5).abs() < 0.05,
            "Normal signal should pass, got {out}"
        );
    }

    #[test]
    fn limiter_clamps_loud_signal() {
        let mut lim = Limiter::new();
        lim.set_ceiling(0.5);

        // Signal fort (0.9) → doit être réduit sous le ceiling (0.5)
        let out = lim.process_sample(0.9);
        assert!(
            out <= 0.5 + 0.01,
            "Signal should be limited to ~0.5, got {out}"
        );
    }

    #[test]
    fn limiter_never_exceeds_ceiling() {
        let mut lim = Limiter::new();
        lim.set_ceiling(0.8);

        // Envoyer des signaux de plus en plus forts
        for level in [0.5, 0.8, 1.0, 1.5, 2.0, 5.0] {
            let out = lim.process_sample(level);
            assert!(
                out <= 0.8 + 0.01,
                "Output {out} exceeds ceiling 0.8 for input {level}"
            );
        }
    }

    #[test]
    fn limiter_recovers_after_peak() {
        let mut lim = Limiter::new();

        // Pic fort
        lim.process_sample(2.0);
        assert!(lim.current_gain() < 1.0);

        // Le gain doit remonter progressivement vers 1.0
        for _ in 0..500 {
            lim.process_sample(0.1);
        }
        assert!(
            lim.current_gain() > 0.8,
            "Gain should recover, got {}",
            lim.current_gain()
        );
    }

    #[test]
    fn limiter_bypass() {
        let mut lim = Limiter::new();
        lim.set_bypass(true);
        // En bypass, même un signal > 1.0 passe
        assert_eq!(lim.process_sample(2.0), 2.0);
    }

    #[test]
    fn limiter_ceiling_clamping() {
        let mut lim = Limiter::new();
        lim.set_ceiling(0.05);
        assert_eq!(lim.ceiling(), 0.1); // Min 0.1
        lim.set_ceiling(5.0);
        assert_eq!(lim.ceiling(), 1.0); // Max 1.0
    }

    #[test]
    fn limiter_reset() {
        let mut lim = Limiter::new();
        lim.process_sample(5.0);
        assert!(lim.current_gain() < 1.0);

        lim.reset();
        assert_eq!(lim.current_gain(), 1.0);
    }
}
