use crate::audio::{BufferSize, ChannelId, SampleRate};

/// Commandes envoyées de l'UI vers le moteur audio.
///
/// # Enum comme message — le pattern "Message Passing"
/// En Rust, on évite le state partagé entre threads (Mutex, Arc<Mutex>).
/// À la place, on envoie des messages typés via des channels.
/// Chaque variante de l'enum est un type de commande différent.
///
/// L'avantage vs un trait object (`Box<dyn Command>`) :
/// - Pas d'allocation heap (l'enum est sur la stack)
/// - Le compilateur vérifie qu'on gère tous les cas (`match` exhaustif)
/// - Sérializable facilement
///
/// # Pourquoi pas de `Serialize` ici ?
/// Ces messages voyagent entre threads du même process (via crossbeam),
/// pas sur le réseau. Pas besoin de sérialisation.
#[derive(Debug, Clone)]
pub enum Command {
    /// Change le volume d'un canal (0.0 = silence, 1.0 = nominal, >1.0 = boost)
    SetVolume { channel: ChannelId, level: f32 },

    /// Mute ou unmute un canal
    SetMute { channel: ChannelId, muted: bool },

    /// Sélectionne le device d'entrée actif
    SetInputDevice { name: String },

    /// Sélectionne le device de sortie actif
    SetOutputDevice { name: String },

    /// Change le buffer size (affecte la latence)
    SetBufferSize(BufferSize),

    /// Change le sample rate
    SetSampleRate(SampleRate),

    /// Demande la liste des devices disponibles
    RequestDeviceList,

    /// Arrête le moteur audio proprement
    Shutdown,
}

/// Événements envoyés du moteur audio vers l'UI.
///
/// C'est le chemin inverse : le moteur informe l'UI de ce qui se passe.
#[derive(Debug, Clone)]
pub enum Event {
    /// Niveau audio actuel d'un canal (pour les VU-meters)
    /// `level` est en valeur linéaire (0.0 → 1.0+)
    LevelUpdate { channel: ChannelId, level: f32 },

    /// Liste des devices audio disponibles sur le système
    DeviceList {
        inputs: Vec<String>,
        outputs: Vec<String>,
    },

    /// Le moteur audio a démarré
    EngineStarted,

    /// Le moteur audio s'est arrêté
    EngineStopped,

    /// Une erreur s'est produite dans le moteur
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_send_and_sync() {
        /// # `Send` et `Sync` — la sécurité des threads en Rust
        ///
        /// `Send` = ce type peut être envoyé à un autre thread.
        /// `Sync` = ce type peut être partagé entre threads (via &T).
        ///
        /// Le compilateur les implémente automatiquement si tous les
        /// champs sont eux-mêmes Send/Sync. Si un champ ne l'est pas
        /// (comme `Rc<T>`), le type entier perd Send/Sync.
        ///
        /// Cette fonction ne fait rien au runtime — elle vérifie juste
        /// que le type satisfait les traits au moment de la compilation.
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Command>();
        assert_sync::<Command>();
        assert_send::<Event>();
        assert_sync::<Event>();
    }

    #[test]
    fn command_debug_format() {
        // `Debug` permet d'afficher le contenu pour le debugging.
        // `{:?}` = format debug, `{:#?}` = format debug pretty-printed.
        let cmd = Command::SetVolume {
            channel: ChannelId(0),
            level: 0.75,
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("SetVolume"));
        assert!(debug_str.contains("0.75"));
    }
}
