use dioxus::prelude::*;

use troubadour_shared::audio::ChannelId;
use troubadour_shared::dsp::EffectsPreset;
use troubadour_shared::messages::{Command, Event};
use troubadour_shared::mixer::{ChannelKind, MixerConfig};
use troubadour_shared::profile::Profile;

use super::channel_strip::ChannelStrip;
use super::device_panel::DevicePanel;
use super::dsp_controls::DspControls;
use super::profile_bar::ProfileBar;
use super::routing_matrix::RoutingMatrix;

/// Onglet actif dans l'interface.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Mixer,
    Effects,
    Devices,
}

#[component]
pub fn MixerView() -> Element {
    let mut mixer_config = use_signal(MixerConfig::default_setup);
    let mut dsp_preset = use_signal(EffectsPreset::default_preset);
    let mut current_profile = use_signal(|| "Default".to_string());
    let mut active_tab = use_signal(|| Tab::Mixer);
    let mut selected_input = use_signal(String::new);
    let mut selected_output = use_signal(String::new);

    let mut levels = use_signal(|| {
        vec![
            (ChannelId(0), 0.0_f32),
            (ChannelId(1), 0.0_f32),
            (ChannelId(2), 0.0_f32),
            (ChannelId(3), 0.0_f32),
            (ChannelId(4), 0.0_f32),
        ]
    });

    // Polling events
    use_future(move || async move {
        loop {
            while let Some(event) = crate::try_recv_event() {
                if let Event::LevelUpdate(channel_levels) = event {
                    let mut lvls = levels.write();
                    for cl in &channel_levels {
                        if let Some(entry) = lvls.iter_mut().find(|(id, _)| *id == cl.channel) {
                            entry.1 = cl.rms;
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        }
    });

    // Device list
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

        // Set defaults
        if let Some(default_in) = host.default_input_device().and_then(|d| d.name().ok()) {
            selected_input.set(default_in);
        }
        if let Some(default_out) = host.default_output_device().and_then(|d| d.name().ok()) {
            selected_output.set(default_out);
        }

        (inputs, outputs)
    });

    // Prepare data
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

    let current_tab = *active_tab.read();

    // Tab button helper
    let tab_class = |tab: Tab| -> &'static str {
        if current_tab == tab {
            "px-4 py-2 text-sm font-medium text-white border-b-2 border-sky-500"
        } else {
            "px-4 py-2 text-sm font-medium text-zinc-500 hover:text-zinc-300 border-b-2 border-transparent"
        }
    };

    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-200 font-sans flex flex-col",
            // ── HEADER ──
            header { class: "border-b border-zinc-800",
                // Top bar
                div { class: "px-6 py-3 flex items-center justify-between",
                    div { class: "flex items-center gap-4",
                        h1 { class: "text-lg font-bold text-white", "Troubadour" }
                        div { class: "flex items-center gap-2",
                            div { class: "w-1.5 h-1.5 rounded-full bg-emerald-500" }
                            span { class: "text-[10px] text-zinc-500", "Live" }
                        }
                    }
                    // Profile bar
                    ProfileBar {
                        current_profile: current_profile.read().clone(),
                        on_select_profile: move |name: String| {
                            let profile = match name.as_str() {
                                "Gaming" => Profile::gaming(),
                                "Streaming" => Profile::streaming(),
                                "Music" => Profile::music(),
                                "Meeting" => Profile::meeting(),
                                _ => Profile::default_profile(),
                            };
                            dsp_preset.set(profile.effects.clone());
                            crate::update_dsp(&profile.effects);
                            current_profile.set(name);
                        },
                    }
                    span { class: "text-[10px] text-zinc-600",
                        "{devices.0.len()} in / {devices.1.len()} out"
                    }
                }
                // Tab bar
                div { class: "px-6 flex gap-1",
                    button {
                        class: tab_class(Tab::Mixer),
                        onclick: move |_| active_tab.set(Tab::Mixer),
                        "Mixer"
                    }
                    button {
                        class: tab_class(Tab::Effects),
                        onclick: move |_| active_tab.set(Tab::Effects),
                        "Effects"
                    }
                    button {
                        class: tab_class(Tab::Devices),
                        onclick: move |_| active_tab.set(Tab::Devices),
                        "Devices"
                    }
                }
            }

            // ── CONTENT ──
            div { class: "flex-1 p-6 overflow-y-auto",
                match current_tab {
                    Tab::Mixer => rsx! {
                        // Inputs
                        div { class: "mb-6",
                            h2 { class: "text-xs font-semibold text-zinc-500 mb-3 uppercase tracking-wider",
                                "Input Channels"
                            }
                            div { class: "flex gap-3 overflow-x-auto pb-2",
                                for ch in channels_data.iter().filter(|c| c.kind == ChannelKind::Input) {
                                    { render_channel_strip(ch, &levels_data, true, mixer_config) }
                                }
                            }
                        }
                        // Outputs
                        div { class: "mb-6",
                            h2 { class: "text-xs font-semibold text-zinc-500 mb-3 uppercase tracking-wider",
                                "Output Channels"
                            }
                            div { class: "flex gap-3 overflow-x-auto pb-2",
                                for ch in channels_data.iter().filter(|c| c.kind == ChannelKind::Output) {
                                    { render_channel_strip(ch, &levels_data, false, mixer_config) }
                                }
                            }
                        }
                        // Routing
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
                    },
                    Tab::Effects => rsx! {
                        {
                            let preset = dsp_preset.read();
                            rsx! {
                                DspControls {
                                    gate_enabled: preset.noise_gate.enabled,
                                    gate_threshold: preset.noise_gate.threshold,
                                    comp_enabled: preset.compressor.enabled,
                                    comp_threshold: preset.compressor.threshold,
                                    comp_ratio: preset.compressor.ratio,
                                    comp_makeup: preset.compressor.makeup_gain,
                                    eq_enabled: preset.eq.enabled,
                                    eq_low_db: preset.eq.bands.first().map(|b| b.gain_db).unwrap_or(0.0),
                                    eq_mid_db: preset.eq.bands.get(1).map(|b| b.gain_db).unwrap_or(0.0),
                                    eq_high_db: preset.eq.bands.get(2).map(|b| b.gain_db).unwrap_or(0.0),
                                    limiter_ceiling: preset.limiter.ceiling,
                                    current_preset: preset.name.clone(),
                                    on_gate_toggle: move |enabled: bool| {
                                        dsp_preset.write().noise_gate.enabled = enabled;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_gate_threshold: move |v: f32| {
                                        dsp_preset.write().noise_gate.threshold = v;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_comp_toggle: move |enabled: bool| {
                                        dsp_preset.write().compressor.enabled = enabled;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_comp_threshold: move |v: f32| {
                                        dsp_preset.write().compressor.threshold = v;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_comp_ratio: move |v: f32| {
                                        dsp_preset.write().compressor.ratio = v;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_comp_makeup: move |v: f32| {
                                        dsp_preset.write().compressor.makeup_gain = v;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_eq_toggle: move |enabled: bool| {
                                        dsp_preset.write().eq.enabled = enabled;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_eq_low: move |v: f32| {
                                        if let Some(b) = dsp_preset.write().eq.bands.first_mut() {
                                            b.gain_db = v;
                                        }
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_eq_mid: move |v: f32| {
                                        if let Some(b) = dsp_preset.write().eq.bands.get_mut(1) {
                                            b.gain_db = v;
                                        }
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_eq_high: move |v: f32| {
                                        if let Some(b) = dsp_preset.write().eq.bands.get_mut(2) {
                                            b.gain_db = v;
                                        }
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_limiter_ceiling: move |v: f32| {
                                        dsp_preset.write().limiter.ceiling = v;
                                        crate::update_dsp(&dsp_preset.read());
                                    },
                                    on_preset_select: move |name: String| {
                                        let preset = match name.as_str() {
                                            "Streaming" => EffectsPreset::streaming(),
                                            "Clean" => EffectsPreset::clean(),
                                            _ => EffectsPreset::default_preset(),
                                        };
                                        dsp_preset.set(preset.clone());
                                        crate::update_dsp(&preset);
                                    },
                                }
                            }
                        }
                    },
                    Tab::Devices => rsx! {
                        DevicePanel {
                            input_devices: devices.0.clone(),
                            output_devices: devices.1.clone(),
                            selected_input: selected_input.read().clone(),
                            selected_output: selected_output.read().clone(),
                            on_select_input: move |name: String| {
                                selected_input.set(name.clone());
                                crate::send_command(Command::SetInputDevice { name });
                            },
                            on_select_output: move |name: String| {
                                selected_output.set(name.clone());
                                crate::send_command(Command::SetOutputDevice { name });
                            },
                        }
                    },
                }
            }

            // ── FOOTER ──
            footer { class: "border-t border-zinc-800 px-6 py-2 flex items-center justify-between",
                p { class: "text-[10px] text-zinc-600", "Troubadour v0.4.0" }
                div { class: "flex items-center gap-3 text-[10px] text-zinc-600",
                    span { "Input: {selected_input}" }
                    span { "Output: {selected_output}" }
                }
            }
        }
    }
}

/// Helper pour rendre un channel strip avec ses callbacks.
fn render_channel_strip(
    ch: &troubadour_shared::mixer::ChannelConfig,
    levels_data: &[(ChannelId, f32)],
    is_input: bool,
    mut mixer_config: Signal<MixerConfig>,
) -> Element {
    let ch_id = ch.id;
    let level = levels_data
        .iter()
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
            is_input: is_input,
            on_volume_change: move |vol: f32| {
                if let Some(c) = mixer_config.write().channel_mut(ch_id) {
                    c.volume = vol;
                }
                crate::send_command(Command::SetVolume { channel: ch_id, level: vol });
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
                crate::send_command(Command::SetMute { channel: ch_id, muted: new_muted });
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
                crate::send_command(Command::SetSolo { channel: ch_id, solo: new_solo });
            },
            on_pan_change: move |pan: f32| {
                if let Some(c) = mixer_config.write().channel_mut(ch_id) {
                    c.pan = pan;
                }
                crate::send_command(Command::SetPan { channel: ch_id, pan });
            },
        }
    }
}
