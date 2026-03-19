use serde::{Deserialize, Serialize};

use crate::audio::{BufferSize, SampleRate};

/// Configuration persistante de Troubadour.
///
/// # `#[serde(default)]`
/// Si un champ est absent du fichier TOML, serde utilise
/// `Default::default()` au lieu de planter. Essentiel pour
/// la rétrocompatibilité : on peut ajouter des champs sans
/// casser les configs existantes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub audio: AudioConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default)]
    pub sample_rate: SampleRate,

    #[serde(default)]
    pub buffer_size: BufferSize,

    /// Nom du device d'entrée préféré.
    ///
    /// # `Option<String>` — le "null" de Rust
    /// En Rust, il n'y a pas de `null`. À la place, `Option<T>` est soit :
    /// - `Some(value)` → il y a une valeur
    /// - `None` → il n'y a pas de valeur
    ///
    /// Le compilateur FORCE à gérer les deux cas. Impossible d'avoir
    /// un NullPointerException. C'est une des grandes forces de Rust.
    #[serde(default)]
    pub input_device: Option<String>,

    #[serde(default)]
    pub output_device: Option<String>,
}

/// `Default` pour `AudioConfig` — valeurs par défaut sensées.
///
/// On implémente `Default` manuellement plutôt que `#[derive(Default)]`
/// car `derive` mettrait `input_device: None` et `output_device: None`
/// (ce qui est correct ici), mais si on voulait un défaut custom
/// pour un champ, on ne pourrait pas. Ici c'est equivalent, mais
/// c'est une bonne habitude pour les configs.
impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: SampleRate::default(),
            buffer_size: BufferSize::default(),
            input_device: None,
            output_device: None,
        }
    }
}

impl AppConfig {
    /// Charge la config depuis un fichier TOML.
    ///
    /// # `Result` et l'opérateur `?`
    /// `?` est du sucre syntaxique. Si le Result est `Err`, la fonction
    /// retourne immédiatement cette erreur. Si c'est `Ok`, on récupère la valeur.
    ///
    /// Sans `?` :
    /// ```ignore
    /// let content = match std::fs::read_to_string(path) {
    ///     Ok(c) => c,
    ///     Err(e) => return Err(e.into()),
    /// };
    /// ```
    /// Avec `?` : `let content = std::fs::read_to_string(path)?;`
    ///
    /// Beaucoup plus lisible, et le compilateur vérifie que les types d'erreur
    /// sont compatibles (grâce au trait `From`).
    pub fn load(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Sauvegarde la config dans un fichier TOML.
    pub fn save(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        // `if let` est un match simplifié quand on ne s'intéresse qu'à un cas.
        // Ici on crée le dossier parent s'il n'existe pas.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = AppConfig::default();
        assert_eq!(config.audio.sample_rate, SampleRate::Hz48000);
        assert_eq!(config.audio.buffer_size, BufferSize::Samples256);
        assert!(config.audio.input_device.is_none());
        assert!(config.audio.output_device.is_none());
    }

    #[test]
    fn config_serialization_roundtrip() {
        // Test que serialize → deserialize donne le même résultat.
        // C'est un pattern de test classique pour la sérialisation.
        let config = AppConfig {
            audio: AudioConfig {
                sample_rate: SampleRate::Hz96000,
                buffer_size: BufferSize::Samples128,
                input_device: Some("Blue Yeti".to_string()),
                output_device: Some("HD 600".to_string()),
            },
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.audio.sample_rate, SampleRate::Hz96000);
        assert_eq!(parsed.audio.buffer_size, BufferSize::Samples128);
        assert_eq!(parsed.audio.input_device.as_deref(), Some("Blue Yeti"));
        assert_eq!(parsed.audio.output_device.as_deref(), Some("HD 600"));
    }

    #[test]
    fn config_from_partial_toml() {
        // Seul sample_rate est défini → le reste prend les valeurs par défaut.
        // C'est exactement pourquoi on a #[serde(default)].
        let toml_str = r#"
            [audio]
            sample_rate = "48000"
        "#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.audio.sample_rate, SampleRate::Hz48000);
        assert_eq!(config.audio.buffer_size, BufferSize::Samples256); // défaut
        assert!(config.audio.input_device.is_none()); // défaut
    }

    #[test]
    fn config_from_empty_toml() {
        // Un fichier TOML complètement vide doit fonctionner.
        let config: AppConfig = toml::from_str("").unwrap();
        assert_eq!(config.audio.sample_rate, SampleRate::Hz48000);
    }

    #[test]
    fn config_save_and_load() {
        // Test d'intégration : écrire sur disque puis relire.
        //
        // `tempfile` serait mieux, mais pour garder les deps minimales,
        // on utilise un chemin temporaire avec le PID du process.
        let dir = std::env::temp_dir().join(format!("troubadour-test-{}", std::process::id()));
        let path = dir.join("config.toml");

        let config = AppConfig {
            audio: AudioConfig {
                sample_rate: SampleRate::Hz44100,
                buffer_size: BufferSize::Samples64,
                input_device: Some("Test Mic".to_string()),
                output_device: None,
            },
        };

        config.save(&path).unwrap();
        let loaded = AppConfig::load(&path).unwrap();

        assert_eq!(loaded.audio.sample_rate, SampleRate::Hz44100);
        assert_eq!(loaded.audio.buffer_size, BufferSize::Samples64);
        assert_eq!(loaded.audio.input_device.as_deref(), Some("Test Mic"));

        // Nettoyage
        let _ = std::fs::remove_dir_all(&dir);
    }
}
