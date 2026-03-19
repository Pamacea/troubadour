use serde::{Deserialize, Serialize};

/// Sample rates supportés par Troubadour.
///
/// # Pourquoi un enum et pas un u32 ?
/// Un `u32` accepterait n'importe quelle valeur (genre 12345 Hz).
/// Un enum restreint aux valeurs valides → impossible de construire
/// un état invalide. C'est le pattern "Make illegal states unrepresentable".
///
/// # Les derives
/// - `Debug` : permet `println!("{:?}", rate)` pour le debug
/// - `Clone, Copy` : types simples copiables (comme un i32)
///   → `Copy` = copie implicite (pas besoin de `.clone()`)
///   → `Clone` est requis par `Copy` (c'est un supertrait)
/// - `PartialEq, Eq` : comparaison avec `==`
/// - `Serialize, Deserialize` : conversion vers/depuis TOML/JSON via serde
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleRate {
    #[serde(rename = "44100")]
    Hz44100,
    #[serde(rename = "48000")]
    Hz48000,
    #[serde(rename = "96000")]
    Hz96000,
    #[serde(rename = "192000")]
    Hz192000,
}

impl SampleRate {
    /// Convertit l'enum en valeur numérique.
    ///
    /// # Pourquoi `self` et pas `&self` ?
    /// Parce que `SampleRate` implémente `Copy`. Le passer par valeur
    /// est aussi efficace qu'une référence (c'est juste un entier en mémoire).
    /// Règle : si le type est `Copy`, préfère `self` à `&self`.
    pub const fn as_hz(self) -> u32 {
        match self {
            Self::Hz44100 => 44_100,
            Self::Hz48000 => 48_000,
            Self::Hz96000 => 96_000,
            Self::Hz192000 => 192_000,
        }
    }
}

/// `Default` permet d'écrire `SampleRate::default()` → Hz48000.
/// C'est un trait standard de Rust. Beaucoup de fonctions/structs
/// l'utilisent : `Option::unwrap_or_default()`, `#[serde(default)]`, etc.
impl Default for SampleRate {
    fn default() -> Self {
        Self::Hz48000
    }
}

/// Tailles de buffer audio supportées.
///
/// Plus petit = moins de latence mais plus de charge CPU.
/// Plus grand = plus stable mais plus de latence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BufferSize {
    #[serde(rename = "64")]
    Samples64,
    #[serde(rename = "128")]
    Samples128,
    #[serde(rename = "256")]
    #[default]
    Samples256,
    #[serde(rename = "512")]
    Samples512,
}

impl BufferSize {
    pub const fn as_frames(self) -> u32 {
        match self {
            Self::Samples64 => 64,
            Self::Samples128 => 128,
            Self::Samples256 => 256,
            Self::Samples512 => 512,
        }
    }

    /// Calcule la latence en millisecondes pour un sample rate donné.
    ///
    /// Formule : latence = (buffer_size / sample_rate) × 1000
    /// Exemple : 256 samples @ 48kHz = 5.33ms
    pub fn latency_ms(self, sample_rate: SampleRate) -> f64 {
        f64::from(self.as_frames()) / f64::from(sample_rate.as_hz()) * 1000.0
    }
}

/// Identifie un périphérique audio du système.
///
/// # `String` vs `&str`
/// On utilise `String` (owned) et pas `&str` (borrowed) car cette struct
/// doit vivre indépendamment — elle est envoyée entre threads via channels.
/// `&str` est une référence → il faudrait une lifetime → complique tout.
/// Règle : dans les structs qui voyagent, utilise `String`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Nom affiché par le système ("Realtek HD Audio", "Blue Yeti", etc.)
    pub name: String,
    /// `true` = entrée (micro), `false` = sortie (casque/enceintes)
    pub is_input: bool,
    /// Nombre de canaux supportés (1 = mono, 2 = stéréo)
    pub channels: u16,
    /// Sample rates supportés par ce device
    pub supported_sample_rates: Vec<SampleRate>,
}

/// Identifiant unique d'un canal dans le mixer.
///
/// # Pourquoi un newtype ?
/// `ChannelId(usize)` au lieu de juste `usize` empêche de confondre
/// un channel ID avec un index de tableau ou un autre nombre.
/// Le compilateur refuse : `fn get_channel(id: ChannelId)` ne peut pas
/// recevoir un `usize` brut. C'est du typage fort gratuit.
///
/// Le `(pub usize)` rend le champ interne accessible.
/// On pourrait le rendre privé et forcer un constructeur, mais
/// pour un ID simple, `pub` suffit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rate_as_hz() {
        assert_eq!(SampleRate::Hz44100.as_hz(), 44_100);
        assert_eq!(SampleRate::Hz48000.as_hz(), 48_000);
        assert_eq!(SampleRate::Hz96000.as_hz(), 96_000);
        assert_eq!(SampleRate::Hz192000.as_hz(), 192_000);
    }

    #[test]
    fn sample_rate_default_is_48k() {
        assert_eq!(SampleRate::default(), SampleRate::Hz48000);
    }

    #[test]
    fn buffer_size_as_frames() {
        assert_eq!(BufferSize::Samples64.as_frames(), 64);
        assert_eq!(BufferSize::Samples256.as_frames(), 256);
    }

    #[test]
    fn buffer_latency_calculation() {
        let latency = BufferSize::Samples256.latency_ms(SampleRate::Hz48000);
        // 256 / 48000 * 1000 = 5.333...
        assert!((latency - 5.333).abs() < 0.01);
    }

    #[test]
    fn channel_id_equality() {
        // Deux ChannelId avec la même valeur sont égaux
        assert_eq!(ChannelId(0), ChannelId(0));
        // Deux ChannelId différents ne le sont pas
        assert_ne!(ChannelId(0), ChannelId(1));
    }

    #[test]
    fn device_info_clone() {
        let device = DeviceInfo {
            name: String::from("Test Mic"),
            is_input: true,
            channels: 1,
            supported_sample_rates: vec![SampleRate::Hz48000],
        };
        // Clone crée une copie profonde indépendante
        let cloned = device.clone();
        assert_eq!(cloned.name, "Test Mic");
        assert_eq!(cloned.channels, 1);
    }
}
