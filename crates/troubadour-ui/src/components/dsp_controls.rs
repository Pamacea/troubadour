use dioxus::prelude::*;

use troubadour_shared::dsp::EffectsPreset;

#[derive(Props, Clone, PartialEq)]
pub struct DspControlsProps {
    pub gate_enabled: bool,
    pub gate_threshold: f32,
    pub comp_enabled: bool,
    pub comp_threshold: f32,
    pub comp_ratio: f32,
    pub comp_makeup: f32,
    pub eq_enabled: bool,
    pub eq_low_db: f32,
    pub eq_mid_db: f32,
    pub eq_high_db: f32,
    pub limiter_ceiling: f32,
    pub current_preset: String,
    pub on_gate_toggle: EventHandler<bool>,
    pub on_gate_threshold: EventHandler<f32>,
    pub on_comp_toggle: EventHandler<bool>,
    pub on_comp_threshold: EventHandler<f32>,
    pub on_comp_ratio: EventHandler<f32>,
    pub on_comp_makeup: EventHandler<f32>,
    pub on_eq_toggle: EventHandler<bool>,
    pub on_eq_low: EventHandler<f32>,
    pub on_eq_mid: EventHandler<f32>,
    pub on_eq_high: EventHandler<f32>,
    pub on_limiter_ceiling: EventHandler<f32>,
    pub on_preset_select: EventHandler<String>,
}

/// Panneau de contrôle DSP — gate, EQ, compressor, limiter.
#[component]
pub fn DspControls(props: DspControlsProps) -> Element {
    let presets = EffectsPreset::builtin_presets();

    let gate_btn = if props.gate_enabled {
        "px-3 py-1 text-xs font-bold rounded bg-emerald-600 text-white"
    } else {
        "px-3 py-1 text-xs font-bold rounded bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
    };

    let comp_btn = if props.comp_enabled {
        "px-3 py-1 text-xs font-bold rounded bg-emerald-600 text-white"
    } else {
        "px-3 py-1 text-xs font-bold rounded bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
    };

    let eq_btn = if props.eq_enabled {
        "px-3 py-1 text-xs font-bold rounded bg-emerald-600 text-white"
    } else {
        "px-3 py-1 text-xs font-bold rounded bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
    };

    let gate_thresh_pct = (props.gate_threshold * 1000.0) as i32;
    let comp_thresh_pct = (props.comp_threshold * 100.0) as i32;
    let comp_ratio_x10 = (props.comp_ratio * 10.0) as i32;
    let comp_makeup_pct = (props.comp_makeup * 100.0) as i32;
    let eq_low_i = (props.eq_low_db * 10.0) as i32;
    let eq_mid_i = (props.eq_mid_db * 10.0) as i32;
    let eq_high_i = (props.eq_high_db * 10.0) as i32;
    let limiter_ceil_pct = (props.limiter_ceiling * 100.0) as i32;

    rsx! {
        div { class: "bg-zinc-900 rounded-lg border border-zinc-800 p-4",
            // Header + Presets
            div { class: "flex items-center justify-between mb-4",
                h3 { class: "text-sm font-semibold text-zinc-400", "DSP Effects" }
                div { class: "flex gap-2",
                    for preset in &presets {
                        {
                            let name = preset.name.clone();
                            let is_active = props.current_preset == name;
                            let btn_class = if is_active {
                                "px-2 py-0.5 text-[10px] rounded bg-sky-600 text-white"
                            } else {
                                "px-2 py-0.5 text-[10px] rounded bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
                            };
                            rsx! {
                                button {
                                    class: "{btn_class}",
                                    onclick: move |_| props.on_preset_select.call(name.clone()),
                                    "{preset.name}"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-2 gap-4",
                // Noise Gate
                div { class: "space-y-2",
                    div { class: "flex items-center gap-2",
                        button {
                            class: "{gate_btn}",
                            onclick: move |_| props.on_gate_toggle.call(!props.gate_enabled),
                            "Gate"
                        }
                        span { class: "text-[10px] text-zinc-600",
                            if props.gate_enabled { "ON" } else { "OFF" }
                        }
                    }
                    if props.gate_enabled {
                        div { class: "flex items-center gap-2",
                            span { class: "text-[10px] text-zinc-500 w-12", "Thresh" }
                            input {
                                r#type: "range",
                                min: "1",
                                max: "50",
                                value: "{gate_thresh_pct}",
                                class: "flex-1 h-0.5 accent-emerald-500",
                                oninput: move |evt| {
                                    if let Ok(v) = evt.value().parse::<f32>() {
                                        props.on_gate_threshold.call(v / 1000.0);
                                    }
                                },
                            }
                        }
                    }
                }

                // EQ
                div { class: "space-y-2",
                    div { class: "flex items-center gap-2",
                        button {
                            class: "{eq_btn}",
                            onclick: move |_| props.on_eq_toggle.call(!props.eq_enabled),
                            "EQ"
                        }
                        span { class: "text-[10px] text-zinc-600",
                            if props.eq_enabled { "ON" } else { "OFF" }
                        }
                    }
                    if props.eq_enabled {
                        div { class: "space-y-1",
                            // Low
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-zinc-500 w-12", "Low" }
                                input {
                                    r#type: "range",
                                    min: "-120",
                                    max: "120",
                                    value: "{eq_low_i}",
                                    class: "flex-1 h-0.5 accent-amber-500",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            props.on_eq_low.call(v / 10.0);
                                        }
                                    },
                                }
                                span { class: "text-[9px] text-zinc-600 w-10 text-right",
                                    "{props.eq_low_db:.1}dB"
                                }
                            }
                            // Mid
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-zinc-500 w-12", "Mid" }
                                input {
                                    r#type: "range",
                                    min: "-120",
                                    max: "120",
                                    value: "{eq_mid_i}",
                                    class: "flex-1 h-0.5 accent-amber-500",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            props.on_eq_mid.call(v / 10.0);
                                        }
                                    },
                                }
                                span { class: "text-[9px] text-zinc-600 w-10 text-right",
                                    "{props.eq_mid_db:.1}dB"
                                }
                            }
                            // High
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-zinc-500 w-12", "High" }
                                input {
                                    r#type: "range",
                                    min: "-120",
                                    max: "120",
                                    value: "{eq_high_i}",
                                    class: "flex-1 h-0.5 accent-amber-500",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            props.on_eq_high.call(v / 10.0);
                                        }
                                    },
                                }
                                span { class: "text-[9px] text-zinc-600 w-10 text-right",
                                    "{props.eq_high_db:.1}dB"
                                }
                            }
                        }
                    }
                }

                // Compressor
                div { class: "space-y-2",
                    div { class: "flex items-center gap-2",
                        button {
                            class: "{comp_btn}",
                            onclick: move |_| props.on_comp_toggle.call(!props.comp_enabled),
                            "Comp"
                        }
                        span { class: "text-[10px] text-zinc-600",
                            if props.comp_enabled { "ON" } else { "OFF" }
                        }
                    }
                    if props.comp_enabled {
                        div { class: "space-y-1",
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-zinc-500 w-12", "Thresh" }
                                input {
                                    r#type: "range",
                                    min: "5",
                                    max: "80",
                                    value: "{comp_thresh_pct}",
                                    class: "flex-1 h-0.5 accent-red-500",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            props.on_comp_threshold.call(v / 100.0);
                                        }
                                    },
                                }
                            }
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-zinc-500 w-12", "Ratio" }
                                input {
                                    r#type: "range",
                                    min: "10",
                                    max: "200",
                                    value: "{comp_ratio_x10}",
                                    class: "flex-1 h-0.5 accent-red-500",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            props.on_comp_ratio.call(v / 10.0);
                                        }
                                    },
                                }
                                span { class: "text-[9px] text-zinc-600 w-8 text-right",
                                    "{props.comp_ratio:.1}:1"
                                }
                            }
                            div { class: "flex items-center gap-2",
                                span { class: "text-[10px] text-zinc-500 w-12", "Makeup" }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "400",
                                    value: "{comp_makeup_pct}",
                                    class: "flex-1 h-0.5 accent-red-500",
                                    oninput: move |evt| {
                                        if let Ok(v) = evt.value().parse::<f32>() {
                                            props.on_comp_makeup.call(v / 100.0);
                                        }
                                    },
                                }
                            }
                        }
                    }
                }

                // Limiter
                div { class: "space-y-2",
                    div { class: "flex items-center gap-2",
                        span { class: "px-3 py-1 text-xs font-bold rounded bg-emerald-600 text-white",
                            "Limiter"
                        }
                        span { class: "text-[10px] text-zinc-600", "Always ON" }
                    }
                    div { class: "flex items-center gap-2",
                        span { class: "text-[10px] text-zinc-500 w-12", "Ceiling" }
                        input {
                            r#type: "range",
                            min: "10",
                            max: "100",
                            value: "{limiter_ceil_pct}",
                            class: "flex-1 h-0.5 accent-orange-500",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse::<f32>() {
                                    props.on_limiter_ceiling.call(v / 100.0);
                                }
                            },
                        }
                        span { class: "text-[9px] text-zinc-600 w-8 text-right",
                            "{props.limiter_ceiling:.0}%"
                        }
                    }
                }
            }
        }
    }
}
