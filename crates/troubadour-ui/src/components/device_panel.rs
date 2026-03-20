use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct DevicePanelProps {
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub selected_input: String,
    pub selected_output: String,
    pub on_select_input: EventHandler<String>,
    pub on_select_output: EventHandler<String>,
}

/// Panneau de sélection des périphériques audio.
#[component]
pub fn DevicePanel(props: DevicePanelProps) -> Element {
    rsx! {
        div { class: "bg-zinc-900 rounded-lg border border-zinc-800 p-4",
            h3 { class: "text-sm font-semibold text-zinc-400 mb-3", "Audio Devices" }

            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                // Input device
                div {
                    label { class: "block text-[11px] text-zinc-500 mb-1 uppercase tracking-wider",
                        "Input Device"
                    }
                    select {
                        class: "w-full bg-zinc-800 text-zinc-200 text-xs rounded px-3 py-2 border border-zinc-700 focus:border-sky-500 outline-none",
                        value: "{props.selected_input}",
                        onchange: move |evt| {
                            props.on_select_input.call(evt.value());
                        },
                        for device in &props.input_devices {
                            option {
                                value: "{device}",
                                selected: *device == props.selected_input,
                                "{device}"
                            }
                        }
                    }
                }

                // Output device
                div {
                    label { class: "block text-[11px] text-zinc-500 mb-1 uppercase tracking-wider",
                        "Output Device"
                    }
                    select {
                        class: "w-full bg-zinc-800 text-zinc-200 text-xs rounded px-3 py-2 border border-zinc-700 focus:border-violet-500 outline-none",
                        value: "{props.selected_output}",
                        onchange: move |evt| {
                            props.on_select_output.call(evt.value());
                        },
                        for device in &props.output_devices {
                            option {
                                value: "{device}",
                                selected: *device == props.selected_output,
                                "{device}"
                            }
                        }
                    }
                }
            }
        }
    }
}
