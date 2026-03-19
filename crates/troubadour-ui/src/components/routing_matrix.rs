use dioxus::prelude::*;

use troubadour_shared::audio::ChannelId;

#[derive(Props, Clone, PartialEq)]
pub struct RoutingMatrixProps {
    pub inputs: Vec<(ChannelId, String)>,
    pub outputs: Vec<(ChannelId, String)>,
    pub routes: Vec<(ChannelId, ChannelId)>,
    pub on_toggle_route: EventHandler<(ChannelId, ChannelId)>,
}

/// Matrice de routage — grille entrées × sorties.
///
/// Chaque cellule est un toggle : clique = connecte/déconnecte.
/// Les cellules actives sont vertes, les inactives sont grises.
#[component]
pub fn RoutingMatrix(props: RoutingMatrixProps) -> Element {
    rsx! {
        div { class: "bg-zinc-900 rounded-lg border border-zinc-800 p-4",
            h3 { class: "text-sm font-semibold text-zinc-400 mb-3", "Routing Matrix" }

            div { class: "overflow-x-auto",
                table { class: "border-collapse",
                    // Header : noms des sorties
                    thead {
                        tr {
                            th { class: "p-1" }
                            for (_id, name) in &props.outputs {
                                th { class: "p-1 text-[10px] font-mono text-zinc-500 text-center w-10",
                                    "{name}"
                                }
                            }
                        }
                    }
                    // Corps : une ligne par entrée
                    tbody {
                        for (input_id, input_name) in &props.inputs {
                            tr {
                                td { class: "p-1 text-[10px] font-mono text-zinc-500 text-right pr-2",
                                    "{input_name}"
                                }
                                for (output_id, _output_name) in &props.outputs {
                                    {
                                        let is_active = props.routes.contains(&(*input_id, *output_id));
                                        let in_id = *input_id;
                                        let out_id = *output_id;
                                        let cell_class = if is_active {
                                            "w-8 h-8 rounded border border-emerald-500 bg-emerald-600 cursor-pointer flex items-center justify-center"
                                        } else {
                                            "w-8 h-8 rounded border border-zinc-700 bg-zinc-800 cursor-pointer flex items-center justify-center hover:bg-zinc-700"
                                        };
                                        rsx! {
                                            td { class: "p-1",
                                                div {
                                                    class: "{cell_class}",
                                                    onclick: move |_| props.on_toggle_route.call((in_id, out_id)),
                                                    if is_active {
                                                        span { class: "text-[10px] text-white font-bold", "x" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
