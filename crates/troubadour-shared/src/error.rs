/// Erreurs partagées de Troubadour.
///
/// # `thiserror` vs erreurs manuelles
/// En Rust, les erreurs sont des types normaux qui implémentent le trait `Error`.
/// On pourrait le faire à la main :
///
/// ```ignore
/// impl std::fmt::Display for TroubadourError { ... }
/// impl std::error::Error for TroubadourError { ... }
/// ```
///
/// C'est verbeux et répétitif. `thiserror` génère tout ça automatiquement
/// avec des attributs `#[error("...")]`. Zéro coût au runtime (c'est une macro).
///
/// # Pourquoi pas `anyhow` ?
/// `anyhow::Error` est un type d'erreur "fourre-tout" → pratique pour les apps.
/// `thiserror` crée des types d'erreur précis → mieux pour les bibliothèques.
/// Comme `troubadour-shared` est une lib utilisée par d'autres crates,
/// on veut des erreurs typées que le code appelant peut matcher.
#[derive(Debug, thiserror::Error)]
pub enum TroubadourError {
    #[error("Audio device not found: {0}")]
    DeviceNotFound(String),

    #[error("Audio stream error: {0}")]
    StreamError(String),

    #[error("Unsupported sample rate: {0} Hz")]
    UnsupportedSampleRate(u32),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Channel {0} not found")]
    ChannelNotFound(usize),
}

/// Type alias pour simplifier les signatures.
///
/// Au lieu d'écrire `Result<T, TroubadourError>` partout,
/// on écrit `TroubadourResult<T>`. C'est une convention Rust :
/// chaque crate définit son propre `Result` type.
///
/// Exemple :
/// ```ignore
/// fn do_thing() -> TroubadourResult<()> { ... }
/// // équivalent à :
/// fn do_thing() -> Result<(), TroubadourError> { ... }
/// ```
pub type TroubadourResult<T> = Result<T, TroubadourError>;
