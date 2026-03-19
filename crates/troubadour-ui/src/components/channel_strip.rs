use dioxus::prelude::*;

use super::vu_meter::VuMeter;

#[derive(Props, Clone, PartialEq)]
pub struct ChannelStripProps {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub solo: bool,
    pub pan: f32,
    pub level: f32,
    pub is_input: bool,
    pub on_volume_change: EventHandler<f32>,
    pub on_mute_toggle: EventHandler<()>,
    pub on_solo_toggle: EventHandler<()>,
    pub on_pan_change: EventHandler<f32>,
}

/// Channel strip — une tranche de console pour un canal audio.
#[component]
pub fn ChannelStrip(props: ChannelStripProps) -> Element {
    let volume_pct = (props.volume * 100.0) as i32;
    let pan_pct = (props.pan * 100.0) as i32;
    let pan_display = if props.pan < -0.05 {
        format!("L{:.0}", props.pan.abs() * 100.0)
    } else if props.pan > 0.05 {
        format!("R{:.0}", props.pan * 100.0)
    } else {
        "C".to_string()
    };

    let strip_border = if props.is_input {
        "border-sky-900/50"
    } else {
        "border-violet-900/50"
    };

    let kind_label = if props.is_input { "IN" } else { "OUT" };
    let kind_class = if props.is_input {
        "bg-sky-900/50 text-sky-400"
    } else {
        "bg-violet-900/50 text-violet-400"
    };

    let mute_class = if props.muted {
        "px-2 py-0.5 text-[10px] font-bold rounded bg-red-600 text-white"
    } else {
        "px-2 py-0.5 text-[10px] font-bold rounded bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
    };

    let solo_class = if props.solo {
        "px-2 py-0.5 text-[10px] font-bold rounded bg-amber-500 text-black"
    } else {
        "px-2 py-0.5 text-[10px] font-bold rounded bg-zinc-800 text-zinc-500 hover:bg-zinc-700"
    };

    rsx! {
        div { class: "flex flex-col items-center gap-2 p-3 bg-zinc-900 rounded-lg border {strip_border} min-w-20",

            // Badge IN/OUT
            span { class: "text-[10px] font-mono px-1.5 py-0.5 rounded {kind_class}",
                "{kind_label}"
            }

            // Nom du canal
            p { class: "text-xs font-medium text-zinc-300 truncate max-w-[70px] text-center",
                "{props.name}"
            }

            // VU-meter vertical
            VuMeter { level: props.level }

            // Valeur du volume
            p { class: "text-[11px] font-mono text-zinc-500",
                "{volume_pct}%"
            }

            // Fader (input range)
            input {
                r#type: "range",
                min: "0",
                max: "200",
                value: "{volume_pct}",
                class: "w-16 h-1 accent-emerald-500",
                oninput: move |evt| {
                    if let Ok(val) = evt.value().parse::<f32>() {
                        props.on_volume_change.call(val / 100.0);
                    }
                },
            }

            // Pan
            div { class: "flex items-center gap-1",
                span { class: "text-[9px] text-zinc-600", "L" }
                input {
                    r#type: "range",
                    min: "-100",
                    max: "100",
                    value: "{pan_pct}",
                    class: "w-12 h-0.5 accent-zinc-500",
                    oninput: move |evt| {
                        if let Ok(val) = evt.value().parse::<f32>() {
                            props.on_pan_change.call(val / 100.0);
                        }
                    },
                }
                span { class: "text-[9px] text-zinc-600", "R" }
            }
            span { class: "text-[9px] font-mono text-zinc-600", "{pan_display}" }

            // Boutons Mute / Solo
            div { class: "flex gap-1",
                button {
                    class: "{mute_class}",
                    onclick: move |_| props.on_mute_toggle.call(()),
                    "M"
                }
                button {
                    class: "{solo_class}",
                    onclick: move |_| props.on_solo_toggle.call(()),
                    "S"
                }
            }
        }
    }
}
