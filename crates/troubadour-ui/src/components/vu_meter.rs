use dioxus::prelude::*;

/// Props pour le composant VU-meter.
///
/// # `#[derive(Props)]` — le système de props de Dioxus
/// Similaire aux props React. Chaque champ devient une prop du composant.
/// `Clone + PartialEq` sont requis par Dioxus pour détecter les changements
/// et éviter les re-renders inutiles (comme React.memo).
#[derive(Props, Clone, PartialEq)]
pub struct VuMeterProps {
    /// Niveau RMS (0.0 → 1.0+)
    pub level: f32,
    /// Afficher horizontalement (true) ou verticalement (false)
    #[props(default = false)]
    pub horizontal: bool,
}

/// Composant VU-meter — affiche le niveau audio d'un canal.
///
/// # Zones de couleur
/// - Vert (0-60%) : niveau normal
/// - Jaune (60-80%) : attention
/// - Rouge (80-100%+) : danger de clipping
///
/// # `#[component]` — macro composant Dioxus
/// Transforme une fonction en composant. La macro :
/// 1. Génère le type Props automatiquement si on utilise des paramètres
/// 2. Gère le lifecycle (mount, update, unmount)
/// 3. Optimise les re-renders
#[component]
pub fn VuMeter(props: VuMeterProps) -> Element {
    let level_pct = (props.level * 100.0).min(100.0);

    // Couleur basée sur le niveau — même logique que v0.1 mais en Tailwind
    let color_class = if level_pct > 80.0 {
        "bg-red-500"
    } else if level_pct > 60.0 {
        "bg-yellow-500"
    } else {
        "bg-emerald-500"
    };

    if props.horizontal {
        // VU-meter horizontal (pour le header ou les résumés)
        rsx! {
            div { class: "w-full h-2 bg-zinc-800 rounded-full overflow-hidden",
                div {
                    class: "h-full rounded-full transition-all duration-75 {color_class}",
                    style: "width: {level_pct}%",
                }
            }
        }
    } else {
        // VU-meter vertical (pour les channel strips)
        rsx! {
            div { class: "relative w-3 h-48 bg-zinc-800 rounded-full overflow-hidden",
                div {
                    class: "absolute bottom-0 w-full rounded-full transition-all duration-75 {color_class}",
                    style: "height: {level_pct}%",
                }
            }
        }
    }
}
