use std::collections::HashMap;

use troubadour_shared::audio::ChannelId;
use troubadour_shared::mixer::{ChannelConfig, ChannelKind, ChannelLevel, MixerConfig, Route};

/// État runtime d'un canal (données qui changent chaque frame audio).
///
/// # Séparation config vs runtime
/// `ChannelConfig` (dans shared) = ce que l'utilisateur configure (volume, mute...).
/// `ChannelState` (ici) = ce que le moteur calcule en temps réel (niveaux).
///
/// Pourquoi séparer ? Parce que :
/// 1. `ChannelConfig` est sérialisé en TOML → pas besoin des niveaux
/// 2. `ChannelState` change 48000x/seconde → pas besoin de sérialisation
/// 3. Ownership clair : le core possède le state, l'UI possède la config
#[derive(Debug, Clone)]
struct ChannelState {
    /// Niveau RMS actuel (0.0 → 1.0+)
    rms: f32,
    /// Niveau peak actuel avec decay lent
    peak: f32,
    /// Peak hold : le peak max récent, décroît lentement
    /// pour l'affichage du marqueur "peak hold" sur le VU-meter.
    peak_hold: f32,
    /// Compteur de frames pour le decay du peak hold
    peak_hold_timer: u32,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            rms: 0.0,
            peak: 0.0,
            peak_hold: 0.0,
            peak_hold_timer: 0,
        }
    }
}

/// Le mixer audio principal.
///
/// # `HashMap` vs `Vec` pour les canaux
/// On utilise `HashMap<ChannelId, ...>` au lieu de `Vec<...>` parce que :
/// - Les ChannelId ne sont pas forcément contigus (0, 1, 5, 12...)
/// - L'accès par ID est O(1) dans les deux cas (index vs hash)
/// - Supprimer un canal au milieu d'un Vec déplace tous les suivants
/// - HashMap est plus naturel pour un "dictionnaire" de canaux
///
/// Pour un mixer audio avec < 100 canaux, la performance est identique.
/// Sur des milliers de canaux, Vec serait plus cache-friendly, mais
/// on n'aura jamais des milliers de canaux dans un mixer desktop.
pub struct Mixer {
    channels: HashMap<ChannelId, ChannelConfig>,
    states: HashMap<ChannelId, ChannelState>,
    routes: Vec<Route>,
}

impl Mixer {
    /// Crée un mixer vide.
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            states: HashMap::new(),
            routes: Vec::new(),
        }
    }

    /// Crée un mixer à partir d'une configuration.
    ///
    /// # `impl Into<MixerConfig>` — flexibilité
    /// Accepte `MixerConfig` directement ou tout type convertible.
    /// En pratique, on passe toujours un `MixerConfig`, mais cette
    /// signature est idiomatique en Rust pour les constructeurs.
    pub fn from_config(config: MixerConfig) -> Self {
        let mut mixer = Self::new();

        for channel in config.channels {
            mixer.states.insert(channel.id, ChannelState::default());
            mixer.channels.insert(channel.id, channel);
        }

        mixer.routes = config.routes;
        mixer
    }

    /// Ajoute un canal au mixer.
    pub fn add_channel(&mut self, config: ChannelConfig) {
        self.states.insert(config.id, ChannelState::default());
        self.channels.insert(config.id, config);
    }

    /// Supprime un canal et toutes ses routes.
    pub fn remove_channel(&mut self, id: ChannelId) {
        self.channels.remove(&id);
        self.states.remove(&id);
        // Supprimer toutes les routes qui référencent ce canal
        self.routes.retain(|r| r.from != id && r.to != id);
    }

    /// Retourne la config d'un canal.
    pub fn channel(&self, id: ChannelId) -> Option<&ChannelConfig> {
        self.channels.get(&id)
    }

    /// Retourne la config mutable d'un canal.
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut ChannelConfig> {
        self.channels.get_mut(&id)
    }

    /// Change le volume d'un canal (clampé entre 0.0 et 2.0).
    pub fn set_volume(&mut self, id: ChannelId, volume: f32) {
        if let Some(ch) = self.channels.get_mut(&id) {
            ch.volume = volume.clamp(0.0, 2.0);
        }
    }

    /// Mute/unmute un canal.
    pub fn set_mute(&mut self, id: ChannelId, muted: bool) {
        if let Some(ch) = self.channels.get_mut(&id) {
            ch.muted = muted;
        }
    }

    /// Active/désactive le solo sur un canal.
    pub fn set_solo(&mut self, id: ChannelId, solo: bool) {
        if let Some(ch) = self.channels.get_mut(&id) {
            ch.solo = solo;
        }
    }

    /// Change le pan stéréo d'un canal (clampé entre -1.0 et 1.0).
    pub fn set_pan(&mut self, id: ChannelId, pan: f32) {
        if let Some(ch) = self.channels.get_mut(&id) {
            ch.pan = pan.clamp(-1.0, 1.0);
        }
    }

    /// Ajoute une route (si elle n'existe pas déjà).
    pub fn add_route(&mut self, from: ChannelId, to: ChannelId) -> bool {
        let route = Route::new(from, to);
        if self.routes.contains(&route) {
            return false;
        }
        // Vérifier que les canaux existent
        if !self.channels.contains_key(&from) || !self.channels.contains_key(&to) {
            return false;
        }
        self.routes.push(route);
        true
    }

    /// Supprime une route.
    pub fn remove_route(&mut self, from: ChannelId, to: ChannelId) {
        self.routes.retain(|r| !(r.from == from && r.to == to));
    }

    /// Vérifie si une route existe.
    pub fn has_route(&self, from: ChannelId, to: ChannelId) -> bool {
        self.routes.contains(&Route::new(from, to))
    }

    /// Retourne toutes les routes.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Calcule le gain effectif d'un canal, en tenant compte de mute et solo.
    ///
    /// # La logique Solo
    /// - Si AUCUN canal n'est solo → tous sont audibles (sauf les muted)
    /// - Si AU MOINS UN canal est solo → seuls les canaux solo passent
    ///
    /// C'est le comportement standard des consoles de mixage.
    ///
    /// # Pan → gain stéréo
    /// Le pan utilise la loi "constant power" (égale puissance) :
    /// - Pan centre (0.0) : L = 0.707, R = 0.707 (√2/2)
    /// - Pan gauche (-1.0) : L = 1.0, R = 0.0
    /// - Pan droite (1.0) : L = 0.0, R = 1.0
    ///
    /// Pourquoi √2/2 au centre et pas 1.0 ?
    /// Parce que L+R au centre donnerait 2.0 = trop fort.
    /// Avec √2/2, la puissance perçue reste constante quel que soit le pan.
    pub fn effective_gain(&self, id: ChannelId) -> (f32, f32) {
        let ch = match self.channels.get(&id) {
            Some(ch) => ch,
            None => return (0.0, 0.0),
        };

        // Mute = silence
        if ch.muted {
            return (0.0, 0.0);
        }

        // Solo logic
        let any_solo = self.channels.values().any(|c| c.solo);
        if any_solo && !ch.solo {
            return (0.0, 0.0);
        }

        // Constant power pan law
        // Angle de 0 (gauche) à π/2 (droite)
        let angle = (ch.pan + 1.0) * 0.5 * std::f32::consts::FRAC_PI_2;
        let gain_left = ch.volume * angle.cos();
        let gain_right = ch.volume * angle.sin();

        (gain_left, gain_right)
    }

    /// Met à jour les niveaux audio d'un canal à partir de samples.
    ///
    /// # Algorithme VU-meter
    /// 1. Calcul du RMS sur le buffer (énergie moyenne)
    /// 2. Peak = max absolu du buffer
    /// 3. Smoothing : le RMS et peak descendent lentement (attack rapide, release lent)
    ///    → le meter ne "saute" pas brutalement, c'est plus agréable visuellement
    /// 4. Peak hold : le marqueur peak reste en haut pendant ~500ms puis descend
    pub fn update_levels(&mut self, id: ChannelId, samples: &[f32]) {
        let state = match self.states.get_mut(&id) {
            Some(s) => s,
            None => return,
        };

        if samples.is_empty() {
            return;
        }

        // RMS = √(mean(sample²))
        let rms = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        // Peak = max(|sample|)
        let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

        // Smoothing avec constantes attack/release
        // Attack rapide (0.3) = monte vite quand le son arrive
        // Release lent (0.05) = descend doucement quand le son s'arrête
        const ATTACK: f32 = 0.3;
        const RELEASE: f32 = 0.05;

        // RMS smoothing
        state.rms = if rms > state.rms {
            state.rms + (rms - state.rms) * ATTACK
        } else {
            state.rms + (rms - state.rms) * RELEASE
        };

        // Peak smoothing
        state.peak = if peak > state.peak {
            state.peak + (peak - state.peak) * ATTACK
        } else {
            state.peak + (peak - state.peak) * RELEASE
        };

        // Peak hold : garde le max pendant ~500ms (environ 25 frames à 60fps)
        if peak > state.peak_hold {
            state.peak_hold = peak;
            state.peak_hold_timer = 25;
        } else if state.peak_hold_timer > 0 {
            state.peak_hold_timer -= 1;
        } else {
            // Decay lent du peak hold
            state.peak_hold *= 0.95;
        }
    }

    /// Retourne les niveaux actuels de tous les canaux (pour l'UI).
    pub fn get_levels(&self) -> Vec<ChannelLevel> {
        self.states
            .iter()
            .map(|(&id, state)| ChannelLevel {
                channel: id,
                rms: state.rms,
                peak: state.peak,
            })
            .collect()
    }

    /// Retourne les canaux d'entrée.
    pub fn inputs(&self) -> Vec<&ChannelConfig> {
        self.channels
            .values()
            .filter(|c| c.kind == ChannelKind::Input)
            .collect()
    }

    /// Retourne les canaux de sortie.
    pub fn outputs(&self) -> Vec<&ChannelConfig> {
        self.channels
            .values()
            .filter(|c| c.kind == ChannelKind::Output)
            .collect()
    }

    /// Nombre total de canaux.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Exporte la config actuelle (pour sauvegarde).
    pub fn to_config(&self) -> MixerConfig {
        MixerConfig {
            channels: self.channels.values().cloned().collect(),
            routes: self.routes.clone(),
        }
    }
}

impl Default for Mixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_mixer() -> Mixer {
        Mixer::from_config(MixerConfig::default_setup())
    }

    #[test]
    fn mixer_from_config() {
        let mixer = setup_mixer();
        assert_eq!(mixer.channel_count(), 5);
        assert_eq!(mixer.inputs().len(), 3);
        assert_eq!(mixer.outputs().len(), 2);
    }

    #[test]
    fn set_volume() {
        let mut mixer = setup_mixer();
        mixer.set_volume(ChannelId(0), 0.5);
        assert_eq!(mixer.channel(ChannelId(0)).unwrap().volume, 0.5);
    }

    #[test]
    fn volume_clamped() {
        let mut mixer = setup_mixer();
        mixer.set_volume(ChannelId(0), 5.0);
        assert_eq!(mixer.channel(ChannelId(0)).unwrap().volume, 2.0);

        mixer.set_volume(ChannelId(0), -1.0);
        assert_eq!(mixer.channel(ChannelId(0)).unwrap().volume, 0.0);
    }

    #[test]
    fn mute_channel() {
        let mut mixer = setup_mixer();
        mixer.set_mute(ChannelId(0), true);
        assert!(mixer.channel(ChannelId(0)).unwrap().muted);

        let (l, r) = mixer.effective_gain(ChannelId(0));
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn solo_logic_no_solo() {
        let mixer = setup_mixer();
        // Aucun solo → tous audibles
        let (l, r) = mixer.effective_gain(ChannelId(0));
        assert!(l > 0.0);
        assert!(r > 0.0);
    }

    #[test]
    fn solo_logic_one_solo() {
        let mut mixer = setup_mixer();
        mixer.set_solo(ChannelId(0), true);

        // Channel 0 (solo) → audible
        let (l, r) = mixer.effective_gain(ChannelId(0));
        assert!(l > 0.0 || r > 0.0);

        // Channel 1 (pas solo) → silence
        let (l, r) = mixer.effective_gain(ChannelId(1));
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn solo_multiple() {
        let mut mixer = setup_mixer();
        mixer.set_solo(ChannelId(0), true);
        mixer.set_solo(ChannelId(1), true);

        // Les deux solos sont audibles
        let (l0, _) = mixer.effective_gain(ChannelId(0));
        let (l1, _) = mixer.effective_gain(ChannelId(1));
        assert!(l0 > 0.0);
        assert!(l1 > 0.0);

        // Channel 2 (pas solo) → silence
        let (l2, _) = mixer.effective_gain(ChannelId(2));
        assert_eq!(l2, 0.0);
    }

    #[test]
    fn pan_center() {
        let mixer = setup_mixer();
        // Pan 0.0 (centre) → gain identique L et R
        let (l, r) = mixer.effective_gain(ChannelId(0));
        assert!((l - r).abs() < 0.01);
    }

    #[test]
    fn pan_left() {
        let mut mixer = setup_mixer();
        mixer.set_pan(ChannelId(0), -1.0);
        let (l, r) = mixer.effective_gain(ChannelId(0));
        assert!(l > 0.9); // presque tout à gauche
        assert!(r < 0.01); // presque rien à droite
    }

    #[test]
    fn pan_right() {
        let mut mixer = setup_mixer();
        mixer.set_pan(ChannelId(0), 1.0);
        let (l, r) = mixer.effective_gain(ChannelId(0));
        assert!(l < 0.01);
        assert!(r > 0.9);
    }

    #[test]
    fn pan_clamped() {
        let mut mixer = setup_mixer();
        mixer.set_pan(ChannelId(0), -5.0);
        assert_eq!(mixer.channel(ChannelId(0)).unwrap().pan, -1.0);
    }

    #[test]
    fn add_route() {
        let mut mixer = setup_mixer();
        // Route qui n'existe pas encore
        let added = mixer.add_route(ChannelId(1), ChannelId(4));
        assert!(added);
        assert!(mixer.has_route(ChannelId(1), ChannelId(4)));
    }

    #[test]
    fn add_duplicate_route() {
        let mut mixer = setup_mixer();
        // Cette route existe déjà dans default_setup
        let added = mixer.add_route(ChannelId(0), ChannelId(3));
        assert!(!added);
    }

    #[test]
    fn add_route_nonexistent_channel() {
        let mut mixer = setup_mixer();
        let added = mixer.add_route(ChannelId(99), ChannelId(3));
        assert!(!added);
    }

    #[test]
    fn remove_route() {
        let mut mixer = setup_mixer();
        assert!(mixer.has_route(ChannelId(0), ChannelId(3)));
        mixer.remove_route(ChannelId(0), ChannelId(3));
        assert!(!mixer.has_route(ChannelId(0), ChannelId(3)));
    }

    #[test]
    fn remove_channel_removes_routes() {
        let mut mixer = setup_mixer();
        assert!(mixer.has_route(ChannelId(0), ChannelId(3)));
        mixer.remove_channel(ChannelId(0));
        assert!(!mixer.has_route(ChannelId(0), ChannelId(3)));
        assert!(mixer.channel(ChannelId(0)).is_none());
    }

    #[test]
    fn update_levels_rms() {
        let mut mixer = setup_mixer();

        // Envoyer un signal constant de 0.5
        let samples = vec![0.5_f32; 256];
        mixer.update_levels(ChannelId(0), &samples);

        let levels = mixer.get_levels();
        let level = levels.iter().find(|l| l.channel == ChannelId(0)).unwrap();

        // Le RMS d'un signal constant = la valeur elle-même
        // Mais avec le smoothing, le premier update ne sera pas exact
        assert!(level.rms > 0.0);
        assert!(level.peak > 0.0);
    }

    #[test]
    fn update_levels_silence() {
        let mut mixer = setup_mixer();

        // Silence
        let samples = vec![0.0_f32; 256];
        mixer.update_levels(ChannelId(0), &samples);

        let levels = mixer.get_levels();
        let level = levels.iter().find(|l| l.channel == ChannelId(0)).unwrap();
        assert_eq!(level.rms, 0.0);
        assert_eq!(level.peak, 0.0);
    }

    #[test]
    fn levels_converge_after_multiple_updates() {
        let mut mixer = setup_mixer();

        // Envoyer le même signal plusieurs fois → le RMS doit converger
        let samples = vec![0.5_f32; 256];
        for _ in 0..50 {
            mixer.update_levels(ChannelId(0), &samples);
        }

        let levels = mixer.get_levels();
        let level = levels.iter().find(|l| l.channel == ChannelId(0)).unwrap();

        // Après 50 updates, le RMS doit être très proche de 0.5
        assert!(
            (level.rms - 0.5).abs() < 0.05,
            "RMS should converge to ~0.5, got {}",
            level.rms
        );
    }

    #[test]
    fn to_config_roundtrip() {
        let mut mixer = setup_mixer();
        mixer.set_volume(ChannelId(0), 0.7);
        mixer.add_route(ChannelId(1), ChannelId(4));

        let config = mixer.to_config();
        let mixer2 = Mixer::from_config(config);

        assert_eq!(mixer2.channel_count(), mixer.channel_count());
        assert_eq!(mixer2.channel(ChannelId(0)).unwrap().volume, 0.7);
        assert!(mixer2.has_route(ChannelId(1), ChannelId(4)));
    }

    #[test]
    fn effective_gain_nonexistent_channel() {
        let mixer = setup_mixer();
        let (l, r) = mixer.effective_gain(ChannelId(99));
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }
}
