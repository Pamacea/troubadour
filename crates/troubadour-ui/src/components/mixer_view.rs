use dioxus::prelude::*;

use troubadour_shared::audio::ChannelId;
use troubadour_shared::mixer::{ChannelKind, MixerConfig};

use super::channel_strip::ChannelStrip;
use super::routing_matrix::RoutingMatrix;

/// Vue principale du mixer — assemble les channel strips et la matrice de routage.
///
/// # State management avec `use_signal`
/// Chaque `use_signal` crée un état réactif. Quand on appelle `.set()`,
/// Dioxus re-rend le composant automatiquement (comme useState en React).
///
/// Différence avec React : les signals sont Copy et peuvent être passés
/// librement dans les closures sans se soucier des captures.
#[component]
pub fn MixerView() -> Element {
    // Initialiser le mixer avec la config par défaut
    let mut mixer_config = use_signal(MixerConfig::default_setup);

    // Simuler des niveaux audio (sera connecté au vrai engine en v0.3)
    let levels = use_signal(|| {
        vec![
            (ChannelId(0), 0.0_f32),
            (ChannelId(1), 0.0_f32),
            (ChannelId(2), 0.0_f32),
            (ChannelId(3), 0.0_f32),
            (ChannelId(4), 0.0_f32),
        ]
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

    // Extraire les données pour la routing matrix
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

    // Clone les données des channels pour le rendu
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
                        p { class: "text-xs text-zinc-500", "Virtual Audio Mixer — v0.2.0" }
                    }
                    div { class: "flex items-center gap-3",
                        // Indicateur engine
                        div { class: "flex items-center gap-2",
                            div { class: "w-2 h-2 rounded-full bg-emerald-500" }
                            span { class: "text-xs text-zinc-500", "Engine Running" }
                        }
                        // Compteur devices
                        span { class: "text-xs text-zinc-600",
                            "{devices.0.len()} in / {devices.1.len()} out"
                        }
                    }
                }
            }

            // Mixer principal
            div { class: "p-6",
                // Section : Inputs
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
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.volume = vol; }
                                        },
                                        on_mute_toggle: move |_| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.muted = !c.muted; }
                                        },
                                        on_solo_toggle: move |_| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.solo = !c.solo; }
                                        },
                                        on_pan_change: move |pan: f32| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.pan = pan; }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                // Section : Outputs
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
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.volume = vol; }
                                        },
                                        on_mute_toggle: move |_| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.muted = !c.muted; }
                                        },
                                        on_solo_toggle: move |_| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.solo = !c.solo; }
                                        },
                                        on_pan_change: move |pan: f32| {
                                            if let Some(c) = mixer_config.write().channel_mut(ch_id) { c.pan = pan; }
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
                        } else {
                            config.add_route(from, to);
                        }
                    },
                }
            }

            // Footer
            footer { class: "border-t border-zinc-800 px-6 py-3",
                p { class: "text-[10px] text-zinc-600",
                    "Troubadour v0.2.0 — Mixing Core"
                }
            }
        }
    }
}
