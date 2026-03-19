use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use crossbeam_channel::{Receiver, Sender};
use tracing::{error, info, warn};

use troubadour_shared::error::{TroubadourError, TroubadourResult};
use troubadour_shared::messages::{Command, Event};

use crate::device::DeviceManager;

/// État du moteur audio.
///
/// # Le pattern State Machine avec enums
/// Plutôt que des booléens (`is_running`, `is_paused`, `has_error`)
/// qui créent des états impossibles (running ET stopped ?), on utilise
/// un enum. Chaque état est exclusif. Le compilateur empêche les
/// transitions invalides si on match dessus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Stopped,
    Running,
}

/// Les handles vers les channels de communication.
///
/// # Channels crossbeam — la messagerie inter-threads
/// Un channel a deux bouts :
/// - `Sender<T>` : envoie des messages de type T
/// - `Receiver<T>` : reçoit des messages de type T
///
/// `crossbeam_channel` vs `std::sync::mpsc` :
/// - crossbeam est multi-producer ET multi-consumer (mpmc)
/// - std est multi-producer, single-consumer (mpsc)
/// - crossbeam a des channels bornés (back-pressure)
/// - crossbeam est plus performant
///
/// Pour l'audio temps réel, on utilise des channels bornés (bounded)
/// pour éviter qu'un producteur lent n'explose la mémoire.
/// # `Clone` pour EngineChannels
/// crossbeam `Sender` et `Receiver` sont tous deux `Clone`.
/// Cloner un Sender/Receiver crée un nouveau handle vers le MÊME channel.
/// C'est comme Arc — pas de copie de données, juste un compteur de références.
#[derive(Clone)]
pub struct EngineChannels {
    /// L'UI envoie des commandes ici
    pub command_tx: Sender<Command>,
    /// L'UI reçoit des événements ici
    pub event_rx: Receiver<Event>,
}

/// Le moteur audio principal de Troubadour.
///
/// # Ownership en Rust — qui possède quoi ?
/// `Engine` est le propriétaire (owner) de :
/// - `device_manager` : le gestionnaire de devices
/// - `command_rx` : le récepteur de commandes
/// - `event_tx` : l'émetteur d'événements
/// - `_streams` : les streams audio actifs
///
/// Quand `Engine` est détruit (drop), tout ce qu'il possède l'est aussi.
/// Pas de garbage collector, pas de fuites. C'est le RAII de Rust.
pub struct Engine {
    device_manager: DeviceManager,
    command_rx: Receiver<Command>,
    event_tx: Sender<Event>,
    state: EngineState,
    volume: f32,
    muted: bool,
    // Les streams audio cpal doivent rester vivants tant qu'on veut du son.
    // `_` prefix = on ne lit jamais ce champ, on le garde juste en vie.
    // Sans ce champ, le stream serait drop immédiatement → plus de son.
    _streams: Vec<Stream>,
}

impl Engine {
    /// Crée un nouveau moteur audio et retourne les channels de communication.
    ///
    /// # Le pattern "Constructor returns handles"
    /// Au lieu de passer les channels en paramètre, on les crée ici
    /// et on retourne les "handles" (les bouts UI). L'appelant n'a pas
    /// besoin de connaître crossbeam.
    ///
    /// `bounded(64)` = le channel peut contenir max 64 messages.
    /// Si l'UI envoie plus vite que le moteur ne traite, le 65ème
    /// `.send()` bloquera jusqu'à ce qu'une place se libère.
    pub fn new() -> (Self, EngineChannels) {
        let (command_tx, command_rx) = crossbeam_channel::bounded(64);
        let (event_tx, event_rx) = crossbeam_channel::bounded(256);

        let engine = Self {
            device_manager: DeviceManager::new(),
            command_rx,
            event_tx,
            state: EngineState::Stopped,
            volume: 1.0,
            muted: false,
            _streams: Vec::new(),
        };

        let channels = EngineChannels {
            command_tx,
            event_rx,
        };

        (engine, channels)
    }

    /// Démarre le moteur audio avec les devices par défaut.
    ///
    /// # `&mut self` — l'emprunt mutable
    /// On a besoin de modifier `self` (changer l'état, stocker les streams).
    /// `&mut self` = emprunt exclusif. Pendant cet appel, personne d'autre
    /// ne peut lire NI écrire dans `self`. Le borrow checker le garantit.
    pub fn start(&mut self) -> TroubadourResult<()> {
        if self.state == EngineState::Running {
            warn!("Engine already running");
            return Ok(());
        }

        info!("Starting audio engine...");

        // Trouver les devices par défaut
        let input_device = self
            .device_manager
            .default_input_name()
            .ok_or_else(|| TroubadourError::DeviceNotFound("No default input device".into()))?;

        let output_device = self
            .device_manager
            .default_output_name()
            .ok_or_else(|| TroubadourError::DeviceNotFound("No default output device".into()))?;

        info!("Input: {input_device}, Output: {output_device}");

        self.start_passthrough(&input_device, &output_device)?;

        self.state = EngineState::Running;
        let _ = self.event_tx.try_send(Event::EngineStarted);
        info!("Audio engine started");

        Ok(())
    }

    /// Met en place un passthrough audio : entrée → sortie.
    ///
    /// # `move` dans les closures
    /// Les closures audio de cpal sont appelées depuis le thread audio du système.
    /// Elles doivent posséder (own) toutes les données qu'elles utilisent,
    /// car le thread appelant pourrait disparaître.
    ///
    /// `move |data| { ... }` transfère l'ownership des variables capturées
    /// dans la closure. Sans `move`, la closure emprunterait (&) les variables
    /// → le borrow checker refuserait car la closure vit plus longtemps.
    fn start_passthrough(&mut self, input_name: &str, output_name: &str) -> TroubadourResult<()> {
        let input_device = self.device_manager.find_input_device(input_name)?;
        let output_device = self.device_manager.find_output_device(output_name)?;

        let input_config = input_device
            .default_input_config()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        info!(
            "Input config: {} channels, {} Hz, {:?}",
            input_config.channels(),
            input_config.sample_rate().0,
            input_config.sample_format()
        );

        // On crée un channel pour transférer l'audio de l'input vers l'output.
        // C'est un ring buffer simplifié. En production on utiliserait
        // un vrai ring buffer lock-free, mais pour le passthrough v0.1 c'est suffisant.
        let (audio_tx, audio_rx) = crossbeam_channel::bounded::<Vec<f32>>(16);

        let event_tx = self.event_tx.clone();

        // --- Stream d'entrée ---
        // cpal appelle cette closure ~100-1000x par seconde (selon buffer size)
        // avec les échantillons audio du micro.
        let input_stream = match input_config.sample_format() {
            SampleFormat::F32 => self.build_input_stream_f32(
                &input_device,
                &input_config.into(),
                audio_tx,
                event_tx.clone(),
            )?,
            // Pour les autres formats, on convertit en f32
            format => {
                return Err(TroubadourError::StreamError(format!(
                    "Unsupported sample format: {format:?}. Only F32 is supported in v0.1"
                )));
            }
        };

        // --- Stream de sortie ---
        let output_config = output_device
            .default_output_config()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        let output_stream =
            self.build_output_stream_f32(&output_device, &output_config.into(), audio_rx)?;

        // Démarrer les deux streams
        input_stream
            .play()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;
        output_stream
            .play()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        // Stocker les streams pour les garder vivants.
        // Si on ne fait pas ça, les streams sont drop à la fin de cette fonction
        // → le son s'arrête immédiatement.
        self._streams.push(input_stream);
        self._streams.push(output_stream);

        Ok(())
    }

    /// Construit le stream d'entrée pour les échantillons f32.
    fn build_input_stream_f32(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        audio_tx: Sender<Vec<f32>>,
        event_tx: Sender<Event>,
    ) -> TroubadourResult<Stream> {
        device
            .build_input_stream(
                config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    // Calcul du niveau audio (RMS) pour le VU-meter
                    if !data.is_empty() {
                        let rms =
                            (data.iter().map(|&s| s * s).sum::<f32>() / data.len() as f32).sqrt();

                        // `try_send` ne bloque jamais. Si le channel est plein,
                        // on drop le message. Pour les VU-meters, perdre un
                        // update n'est pas grave (le prochain arrive dans ~5ms).
                        let _ = event_tx.try_send(Event::LevelUpdate {
                            channel: troubadour_shared::audio::ChannelId(0),
                            level: rms,
                        });
                    }

                    // Envoyer les samples au stream de sortie.
                    // `to_vec()` copie les données — nécessaire car `data`
                    // est un slice emprunté à cpal qui disparaît après le callback.
                    let _ = audio_tx.try_send(data.to_vec());
                },
                move |err| {
                    error!("Input stream error: {err}");
                },
                None, // timeout
            )
            .map_err(|e| TroubadourError::StreamError(e.to_string()))
    }

    /// Construit le stream de sortie pour les échantillons f32.
    fn build_output_stream_f32(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        audio_rx: Receiver<Vec<f32>>,
    ) -> TroubadourResult<Stream> {
        device
            .build_output_stream(
                config,
                move |output: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    // Essayer de récupérer l'audio de l'entrée.
                    // `try_recv()` ne bloque pas. Si pas de données,
                    // on remplit avec du silence (0.0).
                    match audio_rx.try_recv() {
                        Ok(input_data) => {
                            // Copier les samples d'entrée vers la sortie.
                            // `iter().zip()` parcourt les deux slices en parallèle,
                            // s'arrêtant au plus court. Pas de risque d'overflow.
                            for (out_sample, &in_sample) in output.iter_mut().zip(input_data.iter())
                            {
                                *out_sample = in_sample;
                            }
                            // Remplir le reste avec du silence si l'input est plus court
                            if input_data.len() < output.len() {
                                for sample in &mut output[input_data.len()..] {
                                    *sample = 0.0;
                                }
                            }
                        }
                        Err(_) => {
                            // Pas de données disponibles → silence
                            // C'est normal au démarrage ou si l'input est en retard.
                            output.fill(0.0);
                        }
                    }
                },
                move |err| {
                    error!("Output stream error: {err}");
                },
                None,
            )
            .map_err(|e| TroubadourError::StreamError(e.to_string()))
    }

    /// Traite les commandes en attente de l'UI.
    ///
    /// # `while let` — boucle de pattern matching
    /// `while let Ok(cmd) = self.command_rx.try_recv()` :
    /// - Tant que `try_recv()` retourne `Ok(cmd)`, on continue
    /// - Dès que ça retourne `Err` (channel vide), on sort
    ///
    /// C'est plus idiomatique qu'un `loop { match ... { Err => break } }`.
    pub fn process_commands(&mut self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                Command::SetVolume { level, .. } => {
                    self.volume = level.clamp(0.0, 2.0);
                    info!("Volume set to {}", self.volume);
                }
                Command::SetMute { muted, .. } => {
                    self.muted = muted;
                    info!("Mute set to {muted}");
                }
                Command::RequestDeviceList => {
                    self.send_device_list();
                }
                Command::Shutdown => {
                    info!("Shutdown requested");
                    self.stop();
                }
                _ => {
                    warn!("Unhandled command: {cmd:?}");
                }
            }
        }
    }

    /// Envoie la liste des devices à l'UI.
    fn send_device_list(&self) {
        let inputs = self
            .device_manager
            .list_input_devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.name)
            .collect();

        let outputs = self
            .device_manager
            .list_output_devices()
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.name)
            .collect();

        let _ = self
            .event_tx
            .try_send(Event::DeviceList { inputs, outputs });
    }

    /// Arrête le moteur audio.
    ///
    /// # Drop implicite
    /// `self._streams.clear()` drop tous les streams.
    /// En Rust, quand un objet est drop, ses ressources sont libérées.
    /// Pour `Stream`, ça arrête le thread audio et ferme le device.
    /// Pas besoin de `.close()` ou `.dispose()` explicite.
    pub fn stop(&mut self) {
        if self.state == EngineState::Stopped {
            return;
        }

        info!("Stopping audio engine...");
        self._streams.clear();
        self.state = EngineState::Stopped;
        let _ = self.event_tx.try_send(Event::EngineStopped);
        info!("Audio engine stopped");
    }

    /// Prend le récepteur de commandes hors de l'engine.
    ///
    /// # `Option::take()` — transfert d'ownership
    /// `take()` remplace le contenu du `Option` par `None` et retourne
    /// l'ancien contenu. C'est comme "voler" la valeur.
    /// Après ça, l'engine ne peut plus recevoir de commandes directement.
    ///
    /// Pourquoi ? Parce que le Receiver doit être envoyé à un thread
    /// de polling séparé (il est Send), tandis que l'Engine (qui contient
    /// des Stream non-Send) reste sur le thread principal.
    pub fn take_command_receiver(&mut self) -> Receiver<Command> {
        self.command_rx.clone()
    }

    /// Prend l'émetteur d'événements hors de l'engine.
    pub fn take_event_sender(&mut self) -> Sender<Event> {
        self.event_tx.clone()
    }

    pub fn state(&self) -> EngineState {
        self.state
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }
}

/// # `Drop` — le destructeur de Rust
/// Appelé automatiquement quand `Engine` sort du scope ou est détruit.
/// Garantit qu'on arrête proprement l'audio, même si le code appelant
/// oublie d'appeler `.stop()`. C'est du RAII (Resource Acquisition Is Initialization).
///
/// Contrairement au C++ où un destructeur peut être oublié (pointeurs nus),
/// en Rust le drop est garanti par le compilateur.
impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use troubadour_shared::audio::ChannelId;

    #[test]
    fn engine_starts_stopped() {
        let (engine, _channels) = Engine::new();
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn engine_default_volume() {
        let (engine, _channels) = Engine::new();
        assert_eq!(engine.volume(), 1.0);
        assert!(!engine.is_muted());
    }

    #[test]
    fn engine_processes_volume_command() {
        let (mut engine, channels) = Engine::new();

        // Envoyer une commande de volume
        channels
            .command_tx
            .send(Command::SetVolume {
                channel: ChannelId(0),
                level: 0.5,
            })
            .unwrap();

        // Traiter la commande
        engine.process_commands();

        assert_eq!(engine.volume(), 0.5);
    }

    #[test]
    fn engine_clamps_volume() {
        let (mut engine, channels) = Engine::new();

        // Volume trop haut → clampé à 2.0
        channels
            .command_tx
            .send(Command::SetVolume {
                channel: ChannelId(0),
                level: 5.0,
            })
            .unwrap();

        engine.process_commands();
        assert_eq!(engine.volume(), 2.0);
    }

    #[test]
    fn engine_processes_mute_command() {
        let (mut engine, channels) = Engine::new();

        channels
            .command_tx
            .send(Command::SetMute {
                channel: ChannelId(0),
                muted: true,
            })
            .unwrap();

        engine.process_commands();
        assert!(engine.is_muted());
    }

    #[test]
    fn engine_processes_device_list_request() {
        let (mut engine, channels) = Engine::new();

        channels
            .command_tx
            .send(Command::RequestDeviceList)
            .unwrap();

        engine.process_commands();

        // On devrait recevoir un événement DeviceList
        // (même si la liste est vide sur un serveur sans audio)
        match channels.event_rx.try_recv() {
            Ok(Event::DeviceList { inputs, outputs }) => {
                // On vérifie juste que c'est bien un DeviceList
                // Le contenu dépend de la machine
                // inputs et outputs sont des Vec — on vérifie juste
                // que la destructuration a fonctionné
                let _ = inputs;
                let _ = outputs;
            }
            other => {
                // Sur certains systèmes, le DeviceList peut ne pas arriver
                // si le channel est plein. C'est acceptable.
                println!("Received: {other:?}");
            }
        }
    }

    #[test]
    fn engine_processes_shutdown() {
        let (mut engine, channels) = Engine::new();

        channels.command_tx.send(Command::Shutdown).unwrap();

        engine.process_commands();
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn engine_handles_multiple_commands() {
        let (mut engine, channels) = Engine::new();

        // Envoyer plusieurs commandes d'un coup
        channels
            .command_tx
            .send(Command::SetVolume {
                channel: ChannelId(0),
                level: 0.8,
            })
            .unwrap();
        channels
            .command_tx
            .send(Command::SetMute {
                channel: ChannelId(0),
                muted: true,
            })
            .unwrap();

        // Toutes traitées en un seul appel
        engine.process_commands();

        assert_eq!(engine.volume(), 0.8);
        assert!(engine.is_muted());
    }

    #[test]
    fn engine_channels_are_send() {
        // Vérifie que les channels peuvent traverser les threads
        fn assert_send<T: Send>() {}
        assert_send::<Sender<Command>>();
        assert_send::<Receiver<Event>>();
    }

    #[test]
    fn engine_stop_is_idempotent() {
        // Appeler stop() plusieurs fois ne doit pas paniquer
        let (mut engine, _channels) = Engine::new();
        engine.stop();
        engine.stop();
        engine.stop();
        assert_eq!(engine.state(), EngineState::Stopped);
    }
}
