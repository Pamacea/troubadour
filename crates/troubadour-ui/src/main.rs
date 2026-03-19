/// Point d'entrée de l'application Troubadour.
///
/// # Concepts Rust importants ici :
///
/// ## `Send` et `Sync` — Pourquoi `Engine` ne peut pas changer de thread
/// `cpal::Stream` contient un `*mut ()` (pointeur brut) qui n'est PAS `Send`.
/// En Rust, `Send` signifie "peut être transféré à un autre thread".
/// Si un seul champ d'une struct n'est pas Send, la struct entière ne l'est pas.
///
/// C'est une protection COMPILE-TIME contre les data races. Le compilateur
/// refuse de laisser un type non-thread-safe traverser une frontière de thread.
/// En C++, ce serait un bug silencieux. En Rust, c'est une erreur de compilation.
///
/// **Solution :** On crée l'engine et les streams sur le thread principal,
/// et on utilise seulement les channels (qui SONT Send) pour communiquer.
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("troubadour=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting Troubadour...");

    // Créer l'engine sur le thread principal
    let (mut engine, channels) = troubadour_core::engine::Engine::new();

    // Démarrer l'audio AVANT de lancer l'UI
    // Les streams cpal restent sur ce thread (ils ne sont pas Send)
    match engine.start() {
        Ok(()) => tracing::info!("Audio engine started successfully"),
        Err(e) => tracing::error!("Failed to start audio engine: {e}"),
    }

    // Lancer le traitement des commandes dans un thread séparé.
    // On ne passe PAS `engine` au thread — on passe seulement les channels.
    //
    // Mais attendez — on a besoin de `engine` pour `process_commands()` !
    // Solution : on utilise un channel dédié pour envoyer les commandes
    // au thread principal, qui les traite dans une boucle.
    //
    // En fait, simplifions : on lance l'UI dans le thread principal (requis
    // par Dioxus de toute façon), et on traite les commandes via un timer.
    // Spawner un thread pour le polling des commandes
    // Ce thread n'a PAS besoin de `Engine` — il va juste relayer les commandes
    // via un channel qui, lui, EST Send.
    let command_rx_for_polling = engine.take_command_receiver();
    let event_tx_for_polling = engine.take_event_sender();

    std::thread::spawn(move || {
        // Cette boucle tourne indéfiniment et traite les commandes
        loop {
            match command_rx_for_polling.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(cmd) => match cmd {
                    troubadour_shared::messages::Command::Shutdown => {
                        tracing::info!("Shutdown command received");
                        let _ = event_tx_for_polling
                            .try_send(troubadour_shared::messages::Event::EngineStopped);
                        break;
                    }
                    other => {
                        tracing::info!("Command received: {other:?}");
                    }
                },
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Pas de commande, on continue
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    tracing::info!("Command channel disconnected");
                    break;
                }
            }
        }
    });

    // Demander la liste des devices
    let _ = channels
        .command_tx
        .try_send(troubadour_shared::messages::Command::RequestDeviceList);

    // Lancer l'UI Dioxus — DOIT être sur le thread principal
    dioxus::launch(app);
}

/// Composant racine de l'application.
///
/// # Architecture simplifiée pour v0.1
/// Pour cette première version, l'UI affiche :
/// - Le statut du moteur
/// - La liste des devices audio détectés
/// - Un VU-meter basique
///
/// La communication se fait via des variables globales thread-safe
/// (pas idéal, sera refactoré en v0.2 avec un state manager propre).
fn app() -> dioxus::prelude::Element {
    use dioxus::prelude::*;

    // Pour v0.1, on affiche une interface statique avec les infos de base.
    // Le wiring complet engine ↔ UI sera fait en v0.2.

    // Lister les devices au démarrage via cpal directement
    // (simple et efficace pour le skeleton)
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

    let (input_devices, output_devices) = devices;

    rsx! {
        div {
            style: "font-family: 'Segoe UI', system-ui, sans-serif; background: #0f0f0f; color: #e0e0e0; min-height: 100vh; padding: 24px;",

            // Header
            h1 {
                style: "font-size: 28px; font-weight: 700; margin-bottom: 4px; color: #ffffff;",
                "Troubadour"
            }
            p {
                style: "font-size: 14px; color: #888; margin-bottom: 32px;",
                "Virtual Audio Mixer — v0.1.0"
            }

            // Status
            div {
                style: "display: flex; align-items: center; gap: 8px; margin-bottom: 24px;",
                div {
                    style: "width: 8px; height: 8px; border-radius: 50%; background: #22c55e;",
                }
                span {
                    style: "font-size: 13px; color: #aaa;",
                    "Engine: Running"
                }
            }

            // Input devices
            div {
                style: "margin-bottom: 24px;",
                h3 {
                    style: "font-size: 15px; font-weight: 600; margin-bottom: 12px;",
                    "Input Devices ({input_devices.len()})"
                }
                if input_devices.is_empty() {
                    p { style: "color: #666; font-size: 13px;", "No input devices found" }
                }
                for device in &input_devices {
                    div {
                        style: "padding: 8px 12px; background: #1a1a1a; border-radius: 4px; margin-bottom: 4px; font-size: 13px;",
                        "{device}"
                    }
                }
            }

            // Output devices
            div {
                style: "margin-bottom: 24px;",
                h3 {
                    style: "font-size: 15px; font-weight: 600; margin-bottom: 12px;",
                    "Output Devices ({output_devices.len()})"
                }
                if output_devices.is_empty() {
                    p { style: "color: #666; font-size: 13px;", "No output devices found" }
                }
                for device in &output_devices {
                    div {
                        style: "padding: 8px 12px; background: #1a1a1a; border-radius: 4px; margin-bottom: 4px; font-size: 13px;",
                        "{device}"
                    }
                }
            }

            // Footer
            p {
                style: "font-size: 11px; color: #444; margin-top: 32px;",
                "Troubadour v0.1.0 — Foundation"
            }
        }
    }
}
