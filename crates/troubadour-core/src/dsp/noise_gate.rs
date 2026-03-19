use super::Processor;

/// Noise Gate — coupe le son en dessous d'un seuil.
///
/// # Comment ça marche ?
/// Un noise gate est comme une porte automatique :
/// - Quand le signal est au-dessus du seuil → la porte s'ouvre (son passe)
/// - Quand le signal descend sous le seuil → la porte se ferme (silence)
///
/// # Pourquoi c'est essentiel pour les micros ?
/// Un micro capte toujours du bruit de fond (ventilateur, rue, etc.).
/// Le noise gate coupe ce bruit quand tu ne parles pas.
/// Sans gate, les autres entendent un "shhhhh" constant.
///
/// # Paramètres
/// - `threshold` : le seuil en valeur linéaire (ex: 0.01 = très sensible)
/// - `attack` : vitesse d'ouverture (0.0-1.0, rapide → le début du mot n'est pas coupé)
/// - `release` : vitesse de fermeture (0.0-1.0, lent → pas de coupure brutale entre les mots)
///
/// # L'envelope follower
/// On ne compare pas directement chaque sample au seuil (ça causerait
/// du "chattering" — ouverture/fermeture rapide sur un signal oscillant).
/// Au lieu de ça, on suit l'enveloppe du signal (sa "forme" lissée).
pub struct NoiseGate {
    threshold: f32,
    attack: f32,
    release: f32,
    /// L'enveloppe lissée du signal (0.0 → 1.0+)
    envelope: f32,
    /// Le gain appliqué (0.0 = fermé, 1.0 = ouvert)
    gain: f32,
    bypassed: bool,
}

impl NoiseGate {
    pub fn new() -> Self {
        Self {
            threshold: 0.005,
            attack: 0.3,
            release: 0.002,
            envelope: 0.0,
            gain: 0.0,
            bypassed: true, // OFF par defaut — l'utilisateur l'active quand il veut
        }
    }

    /// Configure le seuil du gate.
    /// Plus le seuil est bas, plus le gate est sensible.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Configure la vitesse d'ouverture (0.001 lent → 0.5 rapide).
    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.clamp(0.001, 0.5);
    }

    /// Configure la vitesse de fermeture (0.001 lent → 0.5 rapide).
    pub fn set_release(&mut self, release: f32) {
        self.release = release.clamp(0.001, 0.5);
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn attack(&self) -> f32 {
        self.attack
    }

    pub fn release(&self) -> f32 {
        self.release
    }

    /// Retourne le gain actuel du gate (0.0 fermé → 1.0 ouvert).
    /// Utile pour l'UI (indicateur d'état du gate).
    pub fn current_gain(&self) -> f32 {
        self.gain
    }
}

impl Default for NoiseGate {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for NoiseGate {
    fn process_sample(&mut self, sample: f32) -> f32 {
        if self.bypassed {
            return sample;
        }

        // 1. Suivre l'enveloppe du signal
        //    L'enveloppe est un lissage exponentiel de la valeur absolue.
        //    C'est comme un VU-meter très rapide.
        let abs_sample = sample.abs();
        let coeff = if abs_sample > self.envelope {
            self.attack // Monte vite
        } else {
            self.release // Descend lentement
        };
        self.envelope += coeff * (abs_sample - self.envelope);

        // 2. Décider si la porte est ouverte ou fermée
        //    Au lieu d'un switch binaire (0 ou 1), on fait une transition
        //    douce pour éviter les clics audibles.
        let target_gain = if self.envelope > self.threshold {
            1.0
        } else {
            0.0
        };

        // Smoothing du gain pour éviter les clics
        // Plus rapide que l'envelope car on veut une transition clean
        self.gain += 0.05 * (target_gain - self.gain);

        sample * self.gain
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.gain = 0.0;
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
    fn gate_blocks_silence() {
        let mut gate = NoiseGate::new();
        gate.set_bypass(false);
        // Un signal très faible doit être coupé
        for _ in 0..100 {
            let out = gate.process_sample(0.001);
            // Le gain devrait tendre vers 0
            assert!(out.abs() < 0.01);
        }
    }

    #[test]
    fn gate_passes_loud_signal() {
        let mut gate = NoiseGate::new();
        gate.set_bypass(false);
        gate.set_threshold(0.01);

        // Signal fort → le gate s'ouvre progressivement
        for _ in 0..200 {
            gate.process_sample(0.5);
        }

        // Après convergence, le signal doit passer presque intact
        let out = gate.process_sample(0.5);
        assert!(out > 0.3, "Gate should be open for loud signal, got {out}");
    }

    #[test]
    fn gate_closes_after_signal_drops() {
        let mut gate = NoiseGate::new();
        gate.set_bypass(false);

        // Ouvrir le gate avec un signal fort
        for _ in 0..200 {
            gate.process_sample(0.5);
        }
        assert!(gate.current_gain() > 0.5);

        // Le signal disparaît → le gate doit se fermer
        // Avec release=0.002, il faut beaucoup d'itérations pour que
        // l'enveloppe descende sous le seuil (0.005).
        for _ in 0..5000 {
            gate.process_sample(0.0);
        }
        assert!(
            gate.current_gain() < 0.3,
            "Gate should close, gain = {}",
            gate.current_gain()
        );
    }

    #[test]
    fn gate_bypass() {
        let mut gate = NoiseGate::new();
        gate.set_bypass(true);

        // En bypass, le signal passe tel quel
        assert_eq!(gate.process_sample(0.001), 0.001);
        assert_eq!(gate.process_sample(0.5), 0.5);
    }

    #[test]
    fn gate_threshold_config() {
        let mut gate = NoiseGate::new();
        gate.set_threshold(0.5);
        assert_eq!(gate.threshold(), 0.5);

        // Clamping
        gate.set_threshold(2.0);
        assert_eq!(gate.threshold(), 1.0);
    }

    #[test]
    fn gate_reset() {
        let mut gate = NoiseGate::new();
        gate.set_bypass(false);

        // Ouvrir le gate
        for _ in 0..200 {
            gate.process_sample(0.5);
        }
        assert!(gate.current_gain() > 0.5);

        // Reset → le gate repart de zéro
        gate.reset();
        assert_eq!(gate.current_gain(), 0.0);
    }
}
