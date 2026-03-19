use dioxus::prelude::*;

mod components;

/// Le CSS Tailwind compilé, inclus dans le binaire.
///
/// # `include_str!` — inclusion au compile-time
/// Cette macro lit le fichier au moment de la compilation et l'insère
/// comme une `&'static str`. Le binaire est autonome : pas besoin
/// de distribuer le fichier CSS séparément.
const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("troubadour=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting Troubadour...");

    // Créer l'engine et démarrer l'audio
    let (mut engine, channels) = troubadour_core::engine::Engine::new();
    match engine.start() {
        Ok(()) => tracing::info!("Audio engine started"),
        Err(e) => tracing::error!("Failed to start audio engine: {e}"),
    }

    // Thread de commandes
    let command_rx = engine.take_command_receiver();
    let _event_tx = engine.take_event_sender();

    std::thread::spawn(move || {
        loop {
            match command_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(troubadour_shared::messages::Command::Shutdown) => break,
                Ok(cmd) => tracing::debug!("Command: {cmd:?}"),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    let _ = channels
        .command_tx
        .try_send(troubadour_shared::messages::Command::RequestDeviceList);

    // Configuration desktop avec custom head pour le CSS.
    //
    // # `LaunchBuilder` — la méthode Dioxus 0.6 pour configurer le desktop
    // Au lieu de `dioxus::launch(app)`, on utilise le builder pour :
    // - Injecter du CSS dans le <head> via `with_custom_head`
    // - Configurer la taille de fenêtre, le titre, etc.
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_custom_head(format!("<style>{TAILWIND_CSS}</style>"))
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("Troubadour")
                        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0)),
                ),
        )
        .launch(app);
}

/// Composant racine.
fn app() -> Element {
    rsx! {
        components::mixer_view::MixerView {}
    }
}
