use crate::audio::{BufferSize, ChannelId, SampleRate};
use crate::mixer::ChannelLevel;

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
#[derive(Debug, Clone)]
pub enum Command {
    // === Contrôles par canal ===
    /// Change le volume d'un canal (0.0 = silence, 1.0 = nominal, >1.0 = boost)
    SetVolume { channel: ChannelId, level: f32 },

    /// Mute ou unmute un canal
    SetMute { channel: ChannelId, muted: bool },

    /// Active/désactive le solo sur un canal
    SetSolo { channel: ChannelId, solo: bool },

    /// Change le pan stéréo d'un canal (-1.0 gauche, 0.0 centre, 1.0 droite)
    SetPan { channel: ChannelId, pan: f32 },

    // === Routing ===
    /// Connecte une entrée à une sortie
    AddRoute { from: ChannelId, to: ChannelId },

    /// Déconnecte une route
    RemoveRoute { from: ChannelId, to: ChannelId },

    // === Devices ===
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
#[derive(Debug, Clone)]
pub enum Event {
    /// Niveaux audio de TOUS les canaux (envoyé ~30-60x/seconde).
    ///
    /// # Pourquoi un Vec et pas un event par canal ?
    /// Envoyer 10 events séparés pour 10 canaux = 10 allocations dans le channel.
    /// Un seul Vec = 1 allocation. Pour du temps réel à 60fps, ça compte.
    LevelUpdate(Vec<ChannelLevel>),

    /// Liste des devices audio disponibles sur le système
    DeviceList {
        inputs: Vec<String>,
        outputs: Vec<String>,
    },

    /// Un device a été branché ou débranché
    DeviceChanged,

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
        /// `Send` = ce type peut être envoyé à un autre thread.
        /// `Sync` = ce type peut être partagé entre threads (via &T).
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Command>();
        assert_sync::<Command>();
        assert_send::<Event>();
        assert_sync::<Event>();
    }

    #[test]
    fn command_debug_format() {
        let cmd = Command::SetVolume {
            channel: ChannelId(0),
            level: 0.75,
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("SetVolume"));
        assert!(debug_str.contains("0.75"));
    }

    #[test]
    fn new_commands_exist() {
        // Vérifie que les nouvelles commandes compilent
        let _ = Command::SetSolo {
            channel: ChannelId(0),
            solo: true,
        };
        let _ = Command::SetPan {
            channel: ChannelId(0),
            pan: -0.5,
        };
        let _ = Command::AddRoute {
            from: ChannelId(0),
            to: ChannelId(3),
        };
        let _ = Command::RemoveRoute {
            from: ChannelId(0),
            to: ChannelId(3),
        };
    }
}
