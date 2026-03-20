use dioxus::prelude::*;

use troubadour_shared::profile::Profile;

#[derive(Props, Clone, PartialEq)]
pub struct ProfileBarProps {
    pub current_profile: String,
    pub on_select_profile: EventHandler<String>,
}

/// Barre de profils rapides — switch en un clic.
#[component]
pub fn ProfileBar(props: ProfileBarProps) -> Element {
    let profiles = Profile::builtin_profiles();

    rsx! {
        div { class: "flex items-center gap-2",
            span { class: "text-[10px] text-zinc-600 uppercase tracking-wider mr-1", "Profile" }
            for profile in &profiles {
                {
                    let name = profile.name.clone();
                    let is_active = props.current_profile == name;
                    let btn_class = if is_active {
                        "px-3 py-1 text-[11px] font-medium rounded-full bg-sky-600 text-white"
                    } else {
                        "px-3 py-1 text-[11px] font-medium rounded-full bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-300 transition-colors"
                    };
                    rsx! {
                        button {
                            class: "{btn_class}",
                            onclick: move |_| props.on_select_profile.call(name.clone()),
                            "{profile.name}"
                        }
                    }
                }
            }
        }
    }
}
