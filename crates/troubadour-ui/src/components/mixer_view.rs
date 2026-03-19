use dioxus::prelude::*;

use troubadour_shared::audio::ChannelId;
use troubadour_shared::messages::{Command, Event};
use troubadour_shared::mixer::{ChannelKind, MixerConfig};

use super::channel_strip::ChannelStrip;
use super::routing_matrix::RoutingMatrix;

/// Vue principale du mixer.
///
/// # Câblage v0.3
/// Chaque action UI (fader, mute, solo...) envoie une `Command` au moteur audio
/// via `crate::send_command()`. Le moteur met à jour son mixer interne,
/// recalcule les gains, et le callback audio applique les changements.
///
/// Les niveaux audio remontent via `Event::LevelUpdate` et sont affichés
/// dans les VU-meters en temps réel.
#[component]
pub fn MixerView() -> Element {
    let mut mixer_config = use_signal(MixerConfig::default_setup);

    // Niveaux audio reçus du moteur
    let mut levels = use_signal(|| {
        vec![
            (ChannelId(0), 0.0_f32),
            (ChannelId(1), 0.0_f32),
            (ChannelId(2), 0.0_f32),
            (ChannelId(3), 0.0_f32),
            (ChannelId(4), 0.0_f32),
        ]
    });

    // Polling des événements du moteur audio (~60fps)
    use_future(move || async move {
        loop {
            // Drainer tous les événements en attente
            while let Some(event) = crate::try_recv_event() {
                match event {
                    Event::LevelUpdate(channel_levels) => {
                        let mut lvls = levels.write();
                        for cl in &channel_levels {
                            if let Some(entry) = lvls.iter_mut().find(|(id, _)| *id == cl.channel) {
                                entry.1 = cl.rms;
                            }
                        }
                    }
                    Event::DeviceList { .. } => {}
                    Event::EngineStarted => {}
                    Event::EngineStopped => {}
                    Event::DeviceChanged => {}
                    Event::Error(msg) => {
                        tracing::error!("Engine error: {msg}");
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        }
    });

    // Lister les devices
    let devices = use_hook(|| {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let inputs: Vec<String> = host
            .input_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        let outputs: Vec<String> = host
            .output_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        (inputs, outputs)
    });

    let config = mixer_config.read();
    let inputs_for_matrix: Vec<(ChannelId, String)> = config
        .channels
        .iter()
        .filter(|c| c.kind == ChannelKind::Input)
        .map(|c| (c.id, c.name.clone()))
        .collect();
    let outputs_for_matrix: Vec<(ChannelId, String)> = config
        .channels
        .iter()
        .filter(|c| c.kind == ChannelKind::Output)
        .map(|c| (c.id, c.name.clone()))
        .collect();
    let routes_for_matrix: Vec<(ChannelId, ChannelId)> =
        config.routes.iter().map(|r| (r.from, r.to)).collect();
    let channels_data: Vec<_> = config.channels.clone();
    let levels_data = levels.read().clone();
    drop(config);

    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-200 font-sans",
            // Header
            header { class: "border-b border-zinc-800 px-6 py-4",
                div { class: "flex items-center justify-between",
                    div {
                        h1 { class: "text-xl font-bold text-white", "Troubadour" }
                        p { class: "text-xs text-zinc-500", "Virtual Audio Mixer — v0.3.0" }
                    }
                    div { class: "flex items-center gap-3",
                        div { class: "flex items-center gap-2",
                            div { class: "w-2 h-2 rounded-full bg-emerald-500 animate-pulse" }
                            span { class: "text-xs text-zinc-500", "Live Audio" }
                        }
                        span { class: "text-xs text-zinc-600",
                            "{devices.0.len()} in / {devices.1.len()} out"
                        }
                    }
                }
            }

            div { class: "p-6",
                // Inputs
                div { class: "mb-8",
                    h2 { class: "text-sm font-semibold text-zinc-400 mb-3 uppercase tracking-wider",
                        "Input Channels"
                    }
                    div { class: "flex gap-3 overflow-x-auto pb-2",
                        for ch in channels_data.iter().filter(|c| c.kind == ChannelKind::Input) {
                            {
                                let ch_id = ch.id;
                                let level = levels_data.iter()
                                    .find(|(id, _)| *id == ch_id)
                                    .map(|(_, l)| *l)
                                    .unwrap_or(0.0);

                                rsx! {
                                    ChannelStrip {
                                        key: "{ch_id:?}",
                                        name: ch.name.clone(),
                                        volume: ch.volume,
                                        muted: ch.muted,
                                        solo: ch.solo,
                                        pan: ch.pan,
                                        level: level,
                                        is_input: true,
                                        on_volume_change: move |vol: f32| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) {
                                                c.volume = vol;
                                            }
                                            crate::send_command(Command::SetVolume {
                                                channel: ch_id,
                                                level: vol,
                                            });
                                        },
                                        on_mute_toggle: move |_| {
                                            let new_muted;
                                            {
                                                let mut cfg = mixer_config.write();
                                                if let Some(c) = cfg.channel_mut(ch_id) {
                                                    c.muted = !c.muted;
                                                    new_muted = c.muted;
                                                } else {
                                                    return;
                                                }
                                            }
                                            crate::send_command(Command::SetMute {
                                                channel: ch_id,
                                                muted: new_muted,
                                            });
                                        },
                                        on_solo_toggle: move |_| {
                                            let new_solo;
                                            {
                                                let mut cfg = mixer_config.write();
                                                if let Some(c) = cfg.channel_mut(ch_id) {
                                                    c.solo = !c.solo;
                                                    new_solo = c.solo;
                                                } else {
                                                    return;
                                                }
                                            }
                                            crate::send_command(Command::SetSolo {
                                                channel: ch_id,
                                                solo: new_solo,
                                            });
                                        },
                                        on_pan_change: move |pan: f32| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) {
                                                c.pan = pan;
                                            }
                                            crate::send_command(Command::SetPan {
                                                channel: ch_id,
                                                pan,
                                            });
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                // Outputs
                div { class: "mb-8",
                    h2 { class: "text-sm font-semibold text-zinc-400 mb-3 uppercase tracking-wider",
                        "Output Channels"
                    }
                    div { class: "flex gap-3 overflow-x-auto pb-2",
                        for ch in channels_data.iter().filter(|c| c.kind == ChannelKind::Output) {
                            {
                                let ch_id = ch.id;
                                let level = levels_data.iter()
                                    .find(|(id, _)| *id == ch_id)
                                    .map(|(_, l)| *l)
                                    .unwrap_or(0.0);

                                rsx! {
                                    ChannelStrip {
                                        key: "{ch_id:?}",
                                        name: ch.name.clone(),
                                        volume: ch.volume,
                                        muted: ch.muted,
                                        solo: ch.solo,
                                        pan: ch.pan,
                                        level: level,
                                        is_input: false,
                                        on_volume_change: move |vol: f32| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) {
                                                c.volume = vol;
                                            }
                                            crate::send_command(Command::SetVolume {
                                                channel: ch_id,
                                                level: vol,
                                            });
                                        },
                                        on_mute_toggle: move |_| {
                                            let new_muted;
                                            {
                                                let mut cfg = mixer_config.write();
                                                if let Some(c) = cfg.channel_mut(ch_id) {
                                                    c.muted = !c.muted;
                                                    new_muted = c.muted;
                                                } else {
                                                    return;
                                                }
                                            }
                                            crate::send_command(Command::SetMute {
                                                channel: ch_id,
                                                muted: new_muted,
                                            });
                                        },
                                        on_solo_toggle: move |_| {
                                            let new_solo;
                                            {
                                                let mut cfg = mixer_config.write();
                                                if let Some(c) = cfg.channel_mut(ch_id) {
                                                    c.solo = !c.solo;
                                                    new_solo = c.solo;
                                                } else {
                                                    return;
                                                }
                                            }
                                            crate::send_command(Command::SetSolo {
                                                channel: ch_id,
                                                solo: new_solo,
                                            });
                                        },
                                        on_pan_change: move |pan: f32| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) {
                                                c.pan = pan;
                                            }
                                            crate::send_command(Command::SetPan {
                                                channel: ch_id,
                                                pan,
                                            });
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                // Routing Matrix
                RoutingMatrix {
                    inputs: inputs_for_matrix,
                    outputs: outputs_for_matrix,
                    routes: routes_for_matrix,
                    on_toggle_route: move |(from, to): (ChannelId, ChannelId)| {
                        let mut config = mixer_config.write();
                        if config.has_route(from, to) {
                            config.remove_route(from, to);
                            crate::send_command(Command::RemoveRoute { from, to });
                        } else {
                            config.add_route(from, to);
                            crate::send_command(Command::AddRoute { from, to });
                        }
                    },
                }
            }

            footer { class: "border-t border-zinc-800 px-6 py-3",
                p { class: "text-[10px] text-zinc-600",
                    "Troubadour v0.3.0 — Live Audio"
                }
            }
        }
    }
}
