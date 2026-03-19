use serde::{Deserialize, Serialize};

use crate::audio::ChannelId;

/// Type de canal dans le mixer.
///
/// # Enum vs booléen
/// On pourrait utiliser `is_input: bool`, mais un enum est plus expressif.
/// `ChannelKind::Input` se lit mieux que `true` dans le code.
/// Et si on ajoute un 3ème type plus tard (Bus, Monitor...),
/// un enum s'étend naturellement, un bool non.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    Input,
    Output,
}

/// Configuration d'un canal du mixer.
///
/// Représente un canal nommé (ex: "Mic", "Desktop", "Discord")
/// avec ses contrôles : volume, mute, solo, pan.
///
/// # Séparation config vs état runtime
/// `ChannelConfig` est la configuration persistante (sauvegardée en TOML).
/// L'état runtime (niveau audio actuel, peak hold) vit dans le core
/// et n'est PAS sérialisé — il change 60x par seconde.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: ChannelId,
    pub name: String,
    pub kind: ChannelKind,

    /// Volume linéaire : 0.0 = silence, 1.0 = unity gain, >1.0 = boost
    /// On stocke en linéaire (pas en dB) car le DSP travaille en linéaire.
    /// La conversion dB ↔ linéaire se fait uniquement dans l'UI.
    pub volume: f32,

    /// Mute coupe le son sans changer le volume.
    /// Quand on unmute, le volume revient à sa valeur précédente.
    pub muted: bool,

    /// Solo = n'écouter QUE ce canal (et les autres canaux solo).
    /// Si aucun canal n'est solo, tous sont audibles.
    /// Si au moins un canal est solo, seuls les canaux solo passent.
    pub solo: bool,

    /// Pan stéréo : -1.0 = tout à gauche, 0.0 = centre, 1.0 = tout à droite.
    ///
    /// # Pourquoi pas un enum Left/Center/Right ?
    /// Parce que le pan est continu. L'utilisateur peut mettre "légèrement à gauche"
    /// (-0.3). Un enum ne permettrait que des positions discrètes.
    pub pan: f32,

    /// Nom du device audio physique associé (si applicable).
    /// `None` = pas encore assigné.
    pub device_name: Option<String>,
}

impl ChannelConfig {
    /// Crée un nouveau canal avec des valeurs par défaut.
    pub fn new(id: ChannelId, name: impl Into<String>, kind: ChannelKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            volume: 1.0,
            muted: false,
            solo: false,
            pan: 0.0,
            device_name: None,
        }
    }

    /// Crée un canal d'entrée.
    pub fn input(id: usize, name: impl Into<String>) -> Self {
        Self::new(ChannelId(id), name, ChannelKind::Input)
    }

    /// Crée un canal de sortie.
    pub fn output(id: usize, name: impl Into<String>) -> Self {
        Self::new(ChannelId(id), name, ChannelKind::Output)
    }
}

/// Une route audio : connecte une entrée à une sortie.
///
/// # Le pattern "newtype" pour la clarté
/// On pourrait juste utiliser `(ChannelId, ChannelId)`, mais une struct
/// nommée avec `from` et `to` est beaucoup plus claire à l'usage.
/// `Route { from: ChannelId(0), to: ChannelId(2) }` vs `(0, 2)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Route {
    pub from: ChannelId,
    pub to: ChannelId,
}

impl Route {
    pub fn new(from: ChannelId, to: ChannelId) -> Self {
        Self { from, to }
    }
}

/// Niveau audio mesuré sur un canal (pour les VU-meters).
///
/// # Peak vs RMS
/// - **RMS** (Root Mean Square) : mesure l'énergie "moyenne" perçue.
///   C'est ce que l'oreille entend. Utilisé pour le corps du VU-meter.
/// - **Peak** : la valeur absolue maximale. Détecte les crêtes qui
///   pourraient causer du clipping (distorsion). Affiché comme un
///   petit marqueur au-dessus de la barre RMS.
///
/// Les deux sont en valeur linéaire (0.0 → 1.0+).
/// Conversion en dB : `20.0 * level.log10()`
#[derive(Debug, Clone, Copy)]
pub struct ChannelLevel {
    pub channel: ChannelId,
    pub rms: f32,
    pub peak: f32,
}

/// État complet du mixer, sérialisable pour la config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MixerConfig {
    pub channels: Vec<ChannelConfig>,
    pub routes: Vec<Route>,
}

impl MixerConfig {
    /// Crée une config mixer par défaut avec des canaux typiques.
    pub fn default_setup() -> Self {
        Self {
            channels: vec![
                ChannelConfig::input(0, "Mic"),
                ChannelConfig::input(1, "Desktop"),
                ChannelConfig::input(2, "Browser"),
                ChannelConfig::output(3, "Headphones"),
                ChannelConfig::output(4, "Speakers"),
            ],
            routes: vec![
                // Par défaut : tout va dans les écouteurs
                Route::new(ChannelId(0), ChannelId(3)), // Mic → Headphones
                Route::new(ChannelId(1), ChannelId(3)), // Desktop → Headphones
                Route::new(ChannelId(2), ChannelId(3)), // Browser → Headphones
            ],
        }
    }

    /// Retourne les canaux d'entrée.
    pub fn inputs(&self) -> Vec<&ChannelConfig> {
        self.channels
            .iter()
            .filter(|c| c.kind == ChannelKind::Input)
            .collect()
    }

    /// Retourne les canaux de sortie.
    pub fn outputs(&self) -> Vec<&ChannelConfig> {
        self.channels
            .iter()
            .filter(|c| c.kind == ChannelKind::Output)
            .collect()
    }

    /// Vérifie si une route existe.
    pub fn has_route(&self, from: ChannelId, to: ChannelId) -> bool {
        self.routes.contains(&Route::new(from, to))
    }

    /// Ajoute une route (si elle n'existe pas déjà).
    pub fn add_route(&mut self, from: ChannelId, to: ChannelId) {
        let route = Route::new(from, to);
        if !self.routes.contains(&route) {
            self.routes.push(route);
        }
    }

    /// Supprime une route.
    pub fn remove_route(&mut self, from: ChannelId, to: ChannelId) {
        // `retain` garde les éléments qui satisfont le prédicat.
        // C'est l'équivalent de `.filter()` mais en place (modifie le Vec).
        self.routes.retain(|r| !(r.from == from && r.to == to));
    }

    /// Trouve un canal par son ID.
    ///
    /// # `Option<&ChannelConfig>` vs panic
    /// On retourne `Option` au lieu de paniquer si l'ID n'existe pas.
    /// L'appelant décide quoi faire : `.unwrap()` si c'est un bug,
    /// ou `.ok_or(err)?` pour propager l'erreur proprement.
    pub fn channel(&self, id: ChannelId) -> Option<&ChannelConfig> {
        self.channels.iter().find(|c| c.id == id)
    }

    /// Trouve un canal mutable par son ID.
    pub fn channel_mut(&mut self, id: ChannelId) -> Option<&mut ChannelConfig> {
        self.channels.iter_mut().find(|c| c.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_config_defaults() {
        let ch = ChannelConfig::input(0, "Mic");
        assert_eq!(ch.volume, 1.0);
        assert!(!ch.muted);
        assert!(!ch.solo);
        assert_eq!(ch.pan, 0.0);
        assert_eq!(ch.kind, ChannelKind::Input);
    }

    #[test]
    fn channel_config_output() {
        let ch = ChannelConfig::output(5, "Speakers");
        assert_eq!(ch.kind, ChannelKind::Output);
        assert_eq!(ch.name, "Speakers");
        assert_eq!(ch.id, ChannelId(5));
    }

    #[test]
    fn default_mixer_setup() {
        let config = MixerConfig::default_setup();
        assert_eq!(config.channels.len(), 5);
        assert_eq!(config.inputs().len(), 3);
        assert_eq!(config.outputs().len(), 2);
        assert_eq!(config.routes.len(), 3);
    }

    #[test]
    fn add_route() {
        let mut config = MixerConfig::default();
        config.add_route(ChannelId(0), ChannelId(3));
        assert!(config.has_route(ChannelId(0), ChannelId(3)));
        assert!(!config.has_route(ChannelId(0), ChannelId(4)));
    }

    #[test]
    fn add_duplicate_route_is_idempotent() {
        let mut config = MixerConfig::default();
        config.add_route(ChannelId(0), ChannelId(3));
        config.add_route(ChannelId(0), ChannelId(3));
        assert_eq!(config.routes.len(), 1);
    }

    #[test]
    fn remove_route() {
        let mut config = MixerConfig::default_setup();
        assert!(config.has_route(ChannelId(0), ChannelId(3)));
        config.remove_route(ChannelId(0), ChannelId(3));
        assert!(!config.has_route(ChannelId(0), ChannelId(3)));
    }

    #[test]
    fn find_channel_by_id() {
        let config = MixerConfig::default_setup();
        let ch = config.channel(ChannelId(0)).unwrap();
        assert_eq!(ch.name, "Mic");
    }

    #[test]
    fn find_nonexistent_channel() {
        let config = MixerConfig::default_setup();
        assert!(config.channel(ChannelId(99)).is_none());
    }

    #[test]
    fn modify_channel_volume() {
        let mut config = MixerConfig::default_setup();
        let ch = config.channel_mut(ChannelId(0)).unwrap();
        ch.volume = 0.5;
        assert_eq!(config.channel(ChannelId(0)).unwrap().volume, 0.5);
    }

    #[test]
    fn solo_logic() {
        // Quand solo est activé sur un canal, les autres non-solo sont coupés.
        // Ce test vérifie la structure, la logique sera dans le core.
        let mut config = MixerConfig::default_setup();
        config.channel_mut(ChannelId(0)).unwrap().solo = true;

        let soloed: Vec<_> = config.channels.iter().filter(|c| c.solo).collect();
        assert_eq!(soloed.len(), 1);
        assert_eq!(soloed[0].name, "Mic");
    }

    #[test]
    fn mixer_config_serialization() {
        let config = MixerConfig::default_setup();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: MixerConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.channels.len(), 5);
        assert_eq!(parsed.routes.len(), 3);
        assert_eq!(parsed.channel(ChannelId(0)).unwrap().name, "Mic");
    }

    #[test]
    fn route_equality() {
        let r1 = Route::new(ChannelId(0), ChannelId(3));
        let r2 = Route::new(ChannelId(0), ChannelId(3));
        let r3 = Route::new(ChannelId(1), ChannelId(3));
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }
}
