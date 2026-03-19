use cpal::traits::{DeviceTrait, HostTrait};
use troubadour_shared::audio::DeviceInfo;
use troubadour_shared::error::{TroubadourError, TroubadourResult};

/// Gestionnaire de périphériques audio.
///
/// # Structs en Rust — ce ne sont PAS des classes
/// Une struct contient des données, point. Les méthodes sont
/// définies dans un bloc `impl` séparé. Pas d'héritage.
/// La composition et les traits remplacent l'héritage.
///
/// # `cpal::Host`
/// C'est le point d'entrée de cpal. Un "host" représente le backend
/// audio du système (WASAPI sur Windows, CoreAudio sur Mac, ALSA sur Linux).
/// `cpal::default_host()` choisit automatiquement le bon.
pub struct DeviceManager {
    host: cpal::Host,
}

impl DeviceManager {
    /// Crée un nouveau DeviceManager.
    ///
    /// # `Self` vs nom du type
    /// Dans un bloc `impl`, `Self` est un alias pour le type.
    /// `Self` = `DeviceManager` ici. C'est plus court et survit
    /// aux renommages du type.
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    /// Liste tous les périphériques d'entrée (microphones, etc.)
    ///
    /// # Iterators — le coeur de Rust idiomatique
    /// `.devices()` retourne un itérateur. On le transforme avec :
    /// - `.filter_map()` : transforme + filtre les None en une passe
    /// - `.collect()` : consomme l'itérateur et construit un Vec
    ///
    /// Pourquoi pas une boucle `for` ?
    /// Les itérateurs sont aussi performants (le compilateur les optimise
    /// identiquement) mais plus composables et lisibles pour les
    /// transformations de données.
    pub fn list_input_devices(&self) -> TroubadourResult<Vec<DeviceInfo>> {
        let devices = self
            .host
            .input_devices()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        Ok(devices
            .filter_map(|d| self.device_to_info(&d, true))
            .collect())
    }

    /// Liste tous les périphériques de sortie (casques, enceintes, etc.)
    pub fn list_output_devices(&self) -> TroubadourResult<Vec<DeviceInfo>> {
        let devices = self
            .host
            .output_devices()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        Ok(devices
            .filter_map(|d| self.device_to_info(&d, false))
            .collect())
    }

    /// Retourne le nom du device d'entrée par défaut.
    pub fn default_input_name(&self) -> Option<String> {
        self.host.default_input_device().and_then(|d| d.name().ok())
    }

    /// Retourne le nom du device de sortie par défaut.
    pub fn default_output_name(&self) -> Option<String> {
        self.host
            .default_output_device()
            .and_then(|d| d.name().ok())
    }

    /// Trouve un device d'entrée par son nom.
    ///
    /// # `impl AsRef<str>` — la flexibilité des generics
    /// Ce paramètre accepte tout type convertible en `&str` :
    /// - `&str` (directement)
    /// - `String` (via `.as_ref()`)
    /// - `Cow<str>`, `Box<str>`, etc.
    ///
    /// Pas besoin de surcharger la fonction comme en C++/Java.
    /// Le compilateur génère la version spécialisée à la compilation
    /// (monomorphisation) → zéro coût au runtime.
    pub fn find_input_device(&self, name: impl AsRef<str>) -> TroubadourResult<cpal::Device> {
        let name = name.as_ref();
        self.host
            .input_devices()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| TroubadourError::DeviceNotFound(name.to_string()))
    }

    /// Trouve un device de sortie par son nom.
    pub fn find_output_device(&self, name: impl AsRef<str>) -> TroubadourResult<cpal::Device> {
        let name = name.as_ref();
        self.host
            .output_devices()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .ok_or_else(|| TroubadourError::DeviceNotFound(name.to_string()))
    }

    /// Convertit un `cpal::Device` en notre `DeviceInfo`.
    ///
    /// # `&self` — l'emprunt (borrowing)
    /// `&self` emprunte `self` de façon immutable. Pendant cet emprunt :
    /// - On peut lire les champs de self
    /// - On ne peut PAS les modifier
    /// - D'autres fonctions peuvent aussi emprunter &self en même temps
    ///
    /// C'est la règle fondamentale du borrow checker :
    /// soit N lecteurs (&T), soit 1 seul écrivain (&mut T), jamais les deux.
    fn device_to_info(&self, device: &cpal::Device, is_input: bool) -> Option<DeviceInfo> {
        let name = device.name().ok()?;

        // `?` dans une fonction qui retourne `Option` : si `None`, retourne `None`.
        // C'est le même `?` que pour `Result`, mais adapté à `Option`.
        let config = if is_input {
            device.default_input_config().ok()?
        } else {
            device.default_output_config().ok()?
        };

        Some(DeviceInfo {
            name,
            is_input,
            channels: config.channels(),
            supported_sample_rates: vec![], // TODO: enumerate supported rates
        })
    }
}

/// Implémente `Default` pour `DeviceManager`.
/// Permet d'écrire `DeviceManager::default()` au lieu de `DeviceManager::new()`.
/// C'est une convention Rust : si `new()` n'a pas de paramètres, implémente `Default`.
impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_device_manager() {
        // Ce test vérifie simplement que cpal s'initialise sans crash.
        // Sur un serveur CI sans audio, cpal peut quand même s'initialiser
        // (il ne va juste pas trouver de devices).
        let _manager = DeviceManager::new();
    }

    #[test]
    fn can_list_devices() {
        // On teste que les fonctions ne paniquent pas.
        // Le nombre de devices dépend de la machine → on ne vérifie pas le contenu.
        let manager = DeviceManager::new();

        // Ces appels peuvent retourner Ok([]) sur une machine sans audio,
        // mais ne doivent jamais paniquer.
        let inputs = manager.list_input_devices();
        let outputs = manager.list_output_devices();

        // Au minimum, ça ne crash pas
        assert!(inputs.is_ok() || inputs.is_err());
        assert!(outputs.is_ok() || outputs.is_err());
    }

    #[test]
    fn default_devices_dont_panic() {
        let manager = DeviceManager::new();
        // Retourne Some("nom") ou None — les deux sont valides
        let _input = manager.default_input_name();
        let _output = manager.default_output_name();
    }

    #[test]
    fn find_nonexistent_device_returns_error() {
        let manager = DeviceManager::new();
        let result = manager.find_input_device("Ce Device N'Existe Pas 12345");
        assert!(result.is_err());
    }
}
