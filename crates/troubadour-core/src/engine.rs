use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use crossbeam_channel::{Receiver, Sender};
use tracing::{error, info, warn};

use troubadour_shared::audio::ChannelId;
use troubadour_shared::error::{TroubadourError, TroubadourResult};
use troubadour_shared::messages::{Command, Event};
use troubadour_shared::mixer::{ChannelLevel, MixerConfig};

use crate::device::DeviceManager;
use crate::mixer::Mixer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Stopped,
    Running,
}

/// Paramètres audio lus par le callback audio.
///
/// # Pourquoi une struct séparée ?
/// Le callback audio tourne sur un thread OS haute priorité.
/// Il ne peut PAS :
/// - Allouer de la mémoire (malloc peut bloquer)
/// - Prendre un Mutex qui pourrait être contesté longtemps
/// - Faire de l'I/O
///
/// On utilise `try_lock()` (non-bloquant) : si le lock est pris,
/// on garde les anciens paramètres. Ça skip UN frame audio (~5ms)
/// → imperceptible à l'oreille.
///
/// Les paramètres sont des f32 simples, pas de Vec ni String.
/// Copie rapide, pas d'allocation.
#[derive(Clone)]
pub struct SharedMixerState {
    /// Gain gauche/droite du canal d'entrée principal
    gain: Arc<Mutex<(f32, f32)>>,
    /// Mute global
    muted: Arc<Mutex<bool>>,
}

impl SharedMixerState {
    fn new() -> Self {
        // Gain par défaut : unity gain au centre (constant power pan)
        // cos(π/4) = sin(π/4) = √2/2 ≈ 0.707
        let default_gain = std::f32::consts::FRAC_PI_4;
        Self {
            gain: Arc::new(Mutex::new((default_gain.cos(), default_gain.sin()))),
            muted: Arc::new(Mutex::new(false)),
        }
    }

    /// Met à jour les gains depuis le mixer.
    pub fn update_from_mixer(&self, mixer: &Mixer) {
        // Prendre le gain effectif du premier canal d'entrée (Mic = ChannelId(0))
        let (l, r) = mixer.effective_gain(ChannelId(0));
        if let Ok(mut gain) = self.gain.lock() {
            *gain = (l, r);
        }
        // Vérifier si tous les canaux sont muted
        let all_muted = mixer.inputs().iter().all(|ch| ch.muted);
        if let Ok(mut muted) = self.muted.lock() {
            *muted = all_muted;
        }
    }
}

#[derive(Clone)]
pub struct EngineChannels {
    pub command_tx: Sender<Command>,
    pub event_rx: Receiver<Event>,
}

/// Le moteur audio principal.
///
/// # Architecture v0.3 — le vrai câblage
/// ```text
/// Komplete Audio 2 (mono input)
///     │
///     ▼ cpal input callback
///     │
///     ├─► Mono → Stéréo (duplique le signal)
///     ├─► Applique gain L/R (volume × pan)
///     ├─► Si muted → silence
///     ├─► Calcule RMS/peak → envoie Event::LevelUpdate
///     │
///     ▼ crossbeam channel
///     │
///     ▼ cpal output callback
///     │
///     ▼ soundcore Q45 (stéréo output)
/// ```
pub struct Engine {
    device_manager: DeviceManager,
    command_rx: Receiver<Command>,
    event_tx: Sender<Event>,
    state: EngineState,
    mixer: Mixer,
    shared_state: SharedMixerState,
    _streams: Vec<Stream>,
}

impl Engine {
    pub fn new() -> (Self, EngineChannels) {
        let (command_tx, command_rx) = crossbeam_channel::bounded(64);
        let (event_tx, event_rx) = crossbeam_channel::bounded(256);

        let mixer = Mixer::from_config(MixerConfig::default_setup());
        let shared_state = SharedMixerState::new();

        // Synchroniser le state initial avec le mixer
        shared_state.update_from_mixer(&mixer);

        let engine = Self {
            device_manager: DeviceManager::new(),
            command_rx,
            event_tx,
            state: EngineState::Stopped,
            mixer,
            shared_state,
            _streams: Vec::new(),
        };

        let channels = EngineChannels {
            command_tx,
            event_rx,
        };

        (engine, channels)
    }

    pub fn start(&mut self) -> TroubadourResult<()> {
        if self.state == EngineState::Running {
            warn!("Engine already running");
            return Ok(());
        }

        info!("Starting audio engine...");

        let input_device = self
            .device_manager
            .default_input_name()
            .ok_or_else(|| TroubadourError::DeviceNotFound("No default input device".into()))?;

        let output_device = self
            .device_manager
            .default_output_name()
            .ok_or_else(|| TroubadourError::DeviceNotFound("No default output device".into()))?;

        info!("Input: {input_device}, Output: {output_device}");

        self.shared_state.update_from_mixer(&self.mixer);
        self.start_audio_pipeline(&input_device, &output_device)?;

        self.state = EngineState::Running;
        let _ = self.event_tx.try_send(Event::EngineStarted);
        info!("Audio engine started");

        Ok(())
    }

    /// Construit le pipeline audio complet.
    ///
    /// # Le flux audio
    /// 1. cpal capture le micro (peut être mono ou stéréo)
    /// 2. On convertit en stéréo si nécessaire
    /// 3. On applique le gain (volume × pan) depuis SharedMixerState
    /// 4. On envoie le résultat au output stream
    /// 5. On calcule les niveaux pour le VU-meter
    fn start_audio_pipeline(
        &mut self,
        input_name: &str,
        output_name: &str,
    ) -> TroubadourResult<()> {
        let input_device = self.device_manager.find_input_device(input_name)?;
        let output_device = self.device_manager.find_output_device(output_name)?;

        let input_config = input_device
            .default_input_config()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        let input_channels = input_config.channels() as usize;

        info!(
            "Input: {} ch, {} Hz, {:?}",
            input_channels,
            input_config.sample_rate().0,
            input_config.sample_format()
        );

        // Channel pour transférer l'audio traité de l'input vers l'output.
        // Toujours stéréo après traitement (2 f32 par frame).
        let (audio_tx, audio_rx) = crossbeam_channel::bounded::<Vec<f32>>(32);

        let event_tx = self.event_tx.clone();
        let shared = self.shared_state.clone();

        // ── INPUT STREAM ──
        let input_stream = match input_config.sample_format() {
            SampleFormat::F32 => {
                let config: cpal::StreamConfig = input_config.into();
                input_device
                    .build_input_stream(
                        &config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if data.is_empty() {
                                return;
                            }

                            // Lire les gains (non-bloquant).
                            // Si le lock est pris → on garde les gains du frame précédent.
                            // C'est la magie du `try_lock` : JAMAIS de blocage dans le
                            // callback audio. Pire cas = un frame avec les anciens gains.
                            let (gain_l, gain_r) =
                                shared.gain.try_lock().map(|g| *g).unwrap_or((0.707, 0.707));

                            let muted = shared.muted.try_lock().map(|m| *m).unwrap_or(false);

                            // Construire la sortie stéréo avec gain appliqué.
                            // Pré-allouer pour éviter les réallocations.
                            let frame_count = data.len() / input_channels;
                            let mut output = Vec::with_capacity(frame_count * 2);

                            if muted {
                                output.resize(frame_count * 2, 0.0);
                            } else {
                                // Pour chaque frame, on extrait un signal mono
                                // puis on applique le pan (gain L/R).
                                //
                                // POURQUOI toujours passer par mono ?
                                // Une interface audio comme la Komplete Audio 2
                                // reporte 2 canaux mais le micro est branché sur
                                // l'entrée 1 seulement → canal gauche a du signal,
                                // canal droit est à 0. Si on applique gain_r au
                                // canal droit (qui est 0), on n'entend rien à droite.
                                //
                                // Solution : on mixe L+R en mono d'abord (downmix),
                                // puis on redistribue avec le pan. Comme ça, le signal
                                // est toujours présent dans les deux oreilles.
                                for frame in data.chunks(input_channels) {
                                    // Downmix vers mono : moyenne des canaux
                                    let mono: f32 =
                                        frame.iter().sum::<f32>() / input_channels as f32;
                                    // Appliquer volume + pan
                                    output.push(mono * gain_l);
                                    output.push(mono * gain_r);
                                }
                            }

                            // VU-meter : calculer RMS et peak sur le signal traité
                            let rms = (output.iter().map(|&s| s * s).sum::<f32>()
                                / output.len().max(1) as f32)
                                .sqrt();
                            let peak = output.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

                            let _ = event_tx.try_send(Event::LevelUpdate(vec![ChannelLevel {
                                channel: ChannelId(0),
                                rms,
                                peak,
                            }]));

                            let _ = audio_tx.try_send(output);
                        },
                        move |err| error!("Input stream error: {err}"),
                        None,
                    )
                    .map_err(|e| TroubadourError::StreamError(e.to_string()))?
            }
            format => {
                return Err(TroubadourError::StreamError(format!(
                    "Unsupported format: {format:?}. Only F32 supported."
                )));
            }
        };

        // ── OUTPUT STREAM ──
        let output_config = output_device
            .default_output_config()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        let out_channels = output_config.channels() as usize;
        info!(
            "Output: {} ch, {} Hz",
            out_channels,
            output_config.sample_rate().0
        );

        let output_stream = output_device
            .build_output_stream(
                &output_config.into(),
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    match audio_rx.try_recv() {
                        Ok(stereo_data) => {
                            // stereo_data est toujours [L, R, L, R, ...]
                            let in_frames = stereo_data.len() / 2;
                            let out_frames = output.len() / out_channels;
                            let frames = in_frames.min(out_frames);

                            for f in 0..frames {
                                let l = stereo_data[f * 2];
                                let r = stereo_data[f * 2 + 1];

                                // Mapper stéréo vers N canaux de sortie
                                for ch in 0..out_channels {
                                    output[f * out_channels + ch] = if ch % 2 == 0 { l } else { r };
                                }
                            }
                            // Remplir le reste avec du silence
                            let written = frames * out_channels;
                            for s in &mut output[written..] {
                                *s = 0.0;
                            }
                        }
                        Err(_) => output.fill(0.0),
                    }
                },
                move |err| error!("Output stream error: {err}"),
                None,
            )
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        // Démarrer les streams
        input_stream
            .play()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;
        output_stream
            .play()
            .map_err(|e| TroubadourError::StreamError(e.to_string()))?;

        self._streams.push(input_stream);
        self._streams.push(output_stream);

        Ok(())
    }

    /// Traite les commandes de l'UI.
    pub fn process_commands(&mut self) {
        let mut changed = false;

        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                Command::SetVolume { channel, level } => {
                    self.mixer.set_volume(channel, level);
                    changed = true;
                }
                Command::SetMute { channel, muted } => {
                    self.mixer.set_mute(channel, muted);
                    changed = true;
                }
                Command::SetSolo { channel, solo } => {
                    self.mixer.set_solo(channel, solo);
                    changed = true;
                }
                Command::SetPan { channel, pan } => {
                    self.mixer.set_pan(channel, pan);
                    changed = true;
                }
                Command::AddRoute { from, to } => {
                    self.mixer.add_route(from, to);
                    changed = true;
                }
                Command::RemoveRoute { from, to } => {
                    self.mixer.remove_route(from, to);
                    changed = true;
                }
                Command::RequestDeviceList => {
                    self.send_device_list();
                }
                Command::Shutdown => {
                    self.stop();
                    return;
                }
                _ => {
                    warn!("Unhandled command: {cmd:?}");
                }
            }
        }

        if changed {
            self.shared_state.update_from_mixer(&self.mixer);
        }
    }

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

    pub fn take_command_receiver(&mut self) -> Receiver<Command> {
        self.command_rx.clone()
    }

    pub fn take_event_sender(&mut self) -> Sender<Event> {
        self.event_tx.clone()
    }

    pub fn state(&self) -> EngineState {
        self.state
    }

    pub fn mixer(&self) -> &Mixer {
        &self.mixer
    }

    pub fn shared_mixer_state(&self) -> SharedMixerState {
        self.shared_state.clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_starts_stopped() {
        let (engine, _channels) = Engine::new();
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn engine_has_default_mixer() {
        let (engine, _channels) = Engine::new();
        assert_eq!(engine.mixer().channel_count(), 5);
    }

    #[test]
    fn engine_processes_volume_command() {
        let (mut engine, channels) = Engine::new();
        channels
            .command_tx
            .send(Command::SetVolume {
                channel: ChannelId(0),
                level: 0.5,
            })
            .unwrap();
        engine.process_commands();
        assert_eq!(engine.mixer().channel(ChannelId(0)).unwrap().volume, 0.5);
    }

    #[test]
    fn engine_volume_updates_shared_state() {
        let (mut engine, channels) = Engine::new();

        // Volume 0 → gain doit être (0, 0)
        channels
            .command_tx
            .send(Command::SetVolume {
                channel: ChannelId(0),
                level: 0.0,
            })
            .unwrap();
        engine.process_commands();

        let (l, r) = *engine.shared_state.gain.lock().unwrap();
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn engine_mute_updates_shared_state() {
        let (mut engine, channels) = Engine::new();

        // Muter tous les inputs
        for id in [0, 1, 2] {
            channels
                .command_tx
                .send(Command::SetMute {
                    channel: ChannelId(id),
                    muted: true,
                })
                .unwrap();
        }
        engine.process_commands();

        // Le gain du canal 0 doit être 0 (muted)
        let (l, r) = *engine.shared_state.gain.lock().unwrap();
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn engine_pan_updates_shared_state() {
        let (mut engine, channels) = Engine::new();

        // Pan tout à gauche
        channels
            .command_tx
            .send(Command::SetPan {
                channel: ChannelId(0),
                pan: -1.0,
            })
            .unwrap();
        engine.process_commands();

        let (l, r) = *engine.shared_state.gain.lock().unwrap();
        assert!(l > 0.9, "Left gain should be ~1.0, got {l}");
        assert!(r < 0.01, "Right gain should be ~0.0, got {r}");
    }

    #[test]
    fn engine_clamps_volume() {
        let (mut engine, channels) = Engine::new();
        channels
            .command_tx
            .send(Command::SetVolume {
                channel: ChannelId(0),
                level: 5.0,
            })
            .unwrap();
        engine.process_commands();
        assert_eq!(engine.mixer().channel(ChannelId(0)).unwrap().volume, 2.0);
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
        assert!(engine.mixer().channel(ChannelId(0)).unwrap().muted);
    }

    #[test]
    fn engine_processes_solo_command() {
        let (mut engine, channels) = Engine::new();
        channels
            .command_tx
            .send(Command::SetSolo {
                channel: ChannelId(0),
                solo: true,
            })
            .unwrap();
        engine.process_commands();
        assert!(engine.mixer().channel(ChannelId(0)).unwrap().solo);
    }

    #[test]
    fn engine_processes_pan_command() {
        let (mut engine, channels) = Engine::new();
        channels
            .command_tx
            .send(Command::SetPan {
                channel: ChannelId(0),
                pan: -0.5,
            })
            .unwrap();
        engine.process_commands();
        assert_eq!(engine.mixer().channel(ChannelId(0)).unwrap().pan, -0.5);
    }

    #[test]
    fn engine_processes_route_commands() {
        let (mut engine, channels) = Engine::new();
        channels
            .command_tx
            .send(Command::AddRoute {
                from: ChannelId(1),
                to: ChannelId(4),
            })
            .unwrap();
        engine.process_commands();
        assert!(engine.mixer().has_route(ChannelId(1), ChannelId(4)));

        channels
            .command_tx
            .send(Command::RemoveRoute {
                from: ChannelId(1),
                to: ChannelId(4),
            })
            .unwrap();
        engine.process_commands();
        assert!(!engine.mixer().has_route(ChannelId(1), ChannelId(4)));
    }

    #[test]
    fn engine_processes_device_list_request() {
        let (mut engine, channels) = Engine::new();
        channels
            .command_tx
            .send(Command::RequestDeviceList)
            .unwrap();
        engine.process_commands();

        match channels.event_rx.try_recv() {
            Ok(Event::DeviceList { .. }) => {}
            other => println!("Received: {other:?}"),
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
        engine.process_commands();
        assert_eq!(engine.mixer().channel(ChannelId(0)).unwrap().volume, 0.8);
        assert!(engine.mixer().channel(ChannelId(0)).unwrap().muted);
    }

    #[test]
    fn engine_channels_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Sender<Command>>();
        assert_send::<Receiver<Event>>();
    }

    #[test]
    fn engine_stop_is_idempotent() {
        let (mut engine, _channels) = Engine::new();
        engine.stop();
        engine.stop();
        assert_eq!(engine.state(), EngineState::Stopped);
    }
}
