use serde::{Deserialize, Serialize};

use crate::dsp::EffectsPreset;
use crate::mixer::MixerConfig;

/// Profil complet de Troubadour.
///
/// # Profil = tout l'état sauvegardé
/// Un profil capture TOUT ce que l'utilisateur a configuré :
/// - Le mixer (canaux, volumes, routes)
/// - Les effets DSP (gate, EQ, compressor, limiter)
/// - Le device sélectionné
///
/// L'utilisateur peut switcher entre profils en un clic :
/// "Gaming" → volumes différents, gate activé
/// "Streaming" → compression plus forte, EQ voice
/// "Music" → pas de DSP, volume neutre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub mixer: MixerConfig,
    pub effects: EffectsPreset,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
}

impl Profile {
    /// Crée un profil par défaut.
    pub fn default_profile() -> Self {
        Self {
            name: "Default".to_string(),
            mixer: MixerConfig::default_setup(),
            effects: EffectsPreset::default_preset(),
            input_device: None,
            output_device: None,
        }
    }

    /// Profil Gaming : gate actif, compression forte.
    pub fn gaming() -> Self {
        Self {
            name: "Gaming".to_string(),
            mixer: MixerConfig::default_setup(),
            effects: EffectsPreset::streaming(), // Bonne config pour gaming aussi
            input_device: None,
            output_device: None,
        }
    }

    /// Profil Streaming : EQ voice, compression, gate.
    pub fn streaming() -> Self {
        Self {
            name: "Streaming".to_string(),
            mixer: MixerConfig::default_setup(),
            effects: EffectsPreset::streaming(),
            input_device: None,
            output_device: None,
        }
    }

    /// Profil Music : DSP minimal.
    pub fn music() -> Self {
        Self {
            name: "Music".to_string(),
            mixer: MixerConfig::default_setup(),
            effects: EffectsPreset::clean(),
            input_device: None,
            output_device: None,
        }
    }

    /// Profil Meeting : gate + compression légère.
    pub fn meeting() -> Self {
        Self {
            name: "Meeting".to_string(),
            mixer: MixerConfig::default_setup(),
            effects: EffectsPreset::default_preset(),
            input_device: None,
            output_device: None,
        }
    }

    /// Tous les profils intégrés.
    pub fn builtin_profiles() -> Vec<Self> {
        vec![
            Self::default_profile(),
            Self::gaming(),
            Self::streaming(),
            Self::music(),
            Self::meeting(),
        ]
    }

    /// Sauvegarde le profil dans un fichier TOML.
    pub fn save(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Charge un profil depuis un fichier TOML.
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let profile: Self = toml::from_str(&content)?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile() {
        let profile = Profile::default_profile();
        assert_eq!(profile.name, "Default");
        assert!(profile.input_device.is_none());
    }

    #[test]
    fn builtin_profiles_count() {
        assert_eq!(Profile::builtin_profiles().len(), 5);
    }

    #[test]
    fn profile_serialization_roundtrip() {
        let profile = Profile::streaming();
        let toml_str = toml::to_string_pretty(&profile).unwrap();
        let parsed: Profile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "Streaming");
        assert!(parsed.effects.noise_gate.enabled);
    }

    #[test]
    fn profile_save_and_load() {
        let dir = std::env::temp_dir().join(format!("troubadour-profile-{}", std::process::id()));
        let path = dir.join("test.toml");

        let profile = Profile::gaming();
        profile.save(&path).unwrap();

        let loaded = Profile::load(&path).unwrap();
        assert_eq!(loaded.name, "Gaming");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
