//! DSP (Digital Signal Processing) module for Troubadour.
//!
//! # Architecture
//! Chaque processeur DSP implémente le trait `Processor`.
//! Les processeurs sont chaînés dans un `EffectsChain` :
//!
//! ```text
//! audio in → [NoiseGate] → [Compressor] → [EQ] → [Limiter] → audio out
//! ```
//!
//! # Traitement sample-par-sample vs buffer
//! On traite sample par sample (pas par buffer) car :
//! - Plus simple à implémenter correctement
//! - Le noise gate et compressor ont besoin de l'historique sample par sample
//! - La performance est suffisante (un Ryzen/i7 traite des millions de samples/sec)
//!
//! En production audio pro, on traiterait par blocs SIMD pour gagner 4-8x,
//! mais pour un mixer avec < 10 canaux, c'est overkill.

pub mod compressor;
pub mod eq;
pub mod limiter;
pub mod noise_gate;

/// Trait commun à tous les processeurs DSP.
///
/// # Traits en Rust — l'équivalent des interfaces
/// Un trait définit un ensemble de méthodes qu'un type DOIT implémenter.
/// C'est comme une interface en Java/TypeScript, mais en plus puissant :
/// - Les traits peuvent avoir des méthodes par défaut (comme `is_active`)
/// - Les traits supportent les generics et les associated types
/// - On peut implémenter un trait pour n'importe quel type (même String !)
///
/// Ici, tout processeur DSP doit pouvoir :
/// 1. Traiter un sample audio (`process_sample`)
/// 2. Être réinitialisé (`reset`)
/// 3. Être bypassé (`set_bypass` / `is_bypassed`)
pub trait Processor: Send {
    /// Traite un seul sample audio et retourne le sample traité.
    ///
    /// # Pourquoi `&mut self` ?
    /// Les processeurs DSP ont un état interne (envelope follower,
    /// filtres IIR, etc.) qui change à chaque sample. D'où le `&mut`.
    fn process_sample(&mut self, sample: f32) -> f32;

    /// Réinitialise l'état interne du processeur.
    /// Appelé quand on change de source audio ou au démarrage.
    fn reset(&mut self);

    /// Active ou désactive le bypass.
    /// Quand bypassé, `process_sample` retourne le sample inchangé.
    fn set_bypass(&mut self, bypass: bool);

    /// Retourne `true` si le processeur est bypassé.
    fn is_bypassed(&self) -> bool;
}

/// Chaîne d'effets — applique une série de processeurs en séquence.
///
/// # `Vec<Box<dyn Processor>>` — le polymorphisme en Rust
/// On veut stocker différents types de processeurs dans un même Vec.
/// En Rust, on ne peut pas faire `Vec<Processor>` car le compilateur
/// doit connaître la taille de chaque élément à la compilation.
///
/// `Box<dyn Processor>` résout ça :
/// - `Box` : allocation sur le heap (taille fixe = un pointeur)
/// - `dyn Processor` : "n'importe quel type qui implémente Processor"
///
/// C'est du "dynamic dispatch" : l'appel à `process_sample` passe
/// par une vtable (table de pointeurs de fonctions) au runtime.
/// Coût : ~1ns par appel (un pointeur indirect). Négligeable pour l'audio.
///
/// L'alternative serait les enums (static dispatch, 0 coût), mais
/// ça oblige à lister tous les processeurs dans l'enum. Moins flexible.
pub struct EffectsChain {
    processors: Vec<Box<dyn Processor>>,
}

impl EffectsChain {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Crée une chaîne avec les effets par défaut pour un micro.
    ///
    /// L'ordre est important ! Gate → Compressor → Limiter.
    /// - Gate d'abord : coupe le bruit AVANT qu'il soit amplifié
    /// - Compressor ensuite : régularise les niveaux
    /// - Limiter en dernier : protection finale contre le clipping
    pub fn default_mic_chain() -> Self {
        let mut chain = Self::new();
        chain.add(Box::new(noise_gate::NoiseGate::new()));
        chain.add(Box::new(eq::ParametricEq::default_3band()));
        chain.add(Box::new(compressor::Compressor::new()));
        chain.add(Box::new(limiter::Limiter::new()));
        chain
    }

    /// Ajoute un processeur à la fin de la chaîne.
    pub fn add(&mut self, processor: Box<dyn Processor>) {
        self.processors.push(processor);
    }

    /// Traite un sample à travers toute la chaîne.
    ///
    /// Chaque processeur reçoit le résultat du précédent.
    /// Les processeurs bypassés sont skippés.
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        let mut s = sample;
        for proc in &mut self.processors {
            s = proc.process_sample(s);
        }
        s
    }

    /// Réinitialise tous les processeurs.
    pub fn reset(&mut self) {
        for proc in &mut self.processors {
            proc.reset();
        }
    }

    /// Reconstruit la chaîne depuis un preset sérialisé.
    ///
    /// # Pourquoi reconstruire au lieu de modifier ?
    /// Les `Box<dyn Processor>` ne permettent pas de downcast facilement.
    /// On pourrait utiliser `Any` + downcasting mais c'est fragile.
    /// Reconstruire la chaîne est simple, rapide (~1us), et sans risque.
    /// Le callback audio verra la nouvelle chaîne au prochain `try_lock`.
    pub fn from_preset(preset: &troubadour_shared::dsp::EffectsPreset) -> Self {
        let mut chain = Self::new();

        // Gate
        let mut gate = noise_gate::NoiseGate::new();
        gate.set_threshold(preset.noise_gate.threshold);
        gate.set_attack(preset.noise_gate.attack);
        gate.set_release(preset.noise_gate.release);
        gate.set_bypass(!preset.noise_gate.enabled);
        chain.add(Box::new(gate));

        // EQ
        let mut eq = eq::ParametricEq::default_3band();
        if preset.eq.bands.len() >= 3 {
            eq.set_band(
                0,
                preset.eq.bands[0].frequency,
                preset.eq.bands[0].gain_db,
                preset.eq.bands[0].q,
                48000.0,
            );
            eq.set_band(
                1,
                preset.eq.bands[1].frequency,
                preset.eq.bands[1].gain_db,
                preset.eq.bands[1].q,
                48000.0,
            );
            eq.set_band(
                2,
                preset.eq.bands[2].frequency,
                preset.eq.bands[2].gain_db,
                preset.eq.bands[2].q,
                48000.0,
            );
        }
        eq.set_bypass(!preset.eq.enabled);
        chain.add(Box::new(eq));

        // Compressor
        let mut comp = compressor::Compressor::new();
        comp.set_threshold(preset.compressor.threshold);
        comp.set_ratio(preset.compressor.ratio);
        comp.set_attack(preset.compressor.attack);
        comp.set_release(preset.compressor.release);
        comp.set_makeup_gain(preset.compressor.makeup_gain);
        comp.set_bypass(!preset.compressor.enabled);
        chain.add(Box::new(comp));

        // Limiter
        let mut lim = limiter::Limiter::new();
        lim.set_ceiling(preset.limiter.ceiling);
        lim.set_release(preset.limiter.release);
        lim.set_bypass(!preset.limiter.enabled);
        chain.add(Box::new(lim));

        chain
    }

    /// Nombre de processeurs dans la chaîne.
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Vérifie si la chaîne est vide.
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}

impl Default for EffectsChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Processeur de test qui multiplie par un facteur.
    struct Gain {
        factor: f32,
        bypassed: bool,
    }

    impl Gain {
        fn new(factor: f32) -> Self {
            Self {
                factor,
                bypassed: false,
            }
        }
    }

    impl Processor for Gain {
        fn process_sample(&mut self, sample: f32) -> f32 {
            if self.bypassed {
                return sample;
            }
            sample * self.factor
        }

        fn reset(&mut self) {}

        fn set_bypass(&mut self, bypass: bool) {
            self.bypassed = bypass;
        }

        fn is_bypassed(&self) -> bool {
            self.bypassed
        }
    }

    #[test]
    fn empty_chain_passthrough() {
        let mut chain = EffectsChain::new();
        assert_eq!(chain.process_sample(0.5), 0.5);
    }

    #[test]
    fn chain_applies_processors_in_order() {
        let mut chain = EffectsChain::new();
        chain.add(Box::new(Gain::new(2.0))); // x2
        chain.add(Box::new(Gain::new(0.5))); // x0.5
        // 0.5 * 2.0 * 0.5 = 0.5
        assert_eq!(chain.process_sample(0.5), 0.5);
    }

    #[test]
    fn chain_bypass_skips_processor() {
        let mut chain = EffectsChain::new();
        let mut gain = Gain::new(2.0);
        gain.set_bypass(true);
        chain.add(Box::new(gain));
        // Bypassed → passthrough
        assert_eq!(chain.process_sample(0.5), 0.5);
    }

    #[test]
    fn default_mic_chain_has_four_processors() {
        let chain = EffectsChain::default_mic_chain();
        assert_eq!(chain.len(), 4); // gate + eq + compressor + limiter
    }
}
