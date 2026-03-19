use dioxus::prelude::*;

mod components;

const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("troubadour=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting Troubadour...");

    let (mut engine, channels) = troubadour_core::engine::Engine::new();

    match engine.start() {
        Ok(()) => tracing::info!("Audio engine started"),
        Err(e) => tracing::error!("Failed to start audio engine: {e}"),
    }

    // UN SEUL thread traite les commandes.
    // Pas de clonage du receiver — sinon crossbeam distribue les messages
    // et certaines commandes sont "volées" par le mauvais thread.
    //
    // Ce thread possède un Mixer local qui synchronise vers le SharedMixerState.
    // Le SharedMixerState est lu par le callback audio (try_lock).
    let shared_mixer = engine.shared_mixer_state();
    // Créer un channel dédié pour les commandes du thread de traitement.
    // L'UI envoie sur `cmd_tx`, le thread lit sur `cmd_rx`.
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<troubadour_shared::messages::Command>(64);

    std::thread::spawn(move || {
        let mut mixer = troubadour_core::mixer::Mixer::from_config(
            troubadour_shared::mixer::MixerConfig::default_setup(),
        );

        loop {
            match cmd_rx.recv_timeout(std::time::Duration::from_millis(5)) {
                Ok(cmd) => {
                    use troubadour_shared::messages::Command;
                    match cmd {
                        Command::SetVolume { channel, level } => {
                            mixer.set_volume(channel, level);
                            tracing::info!("Volume: {level:.2} on {channel:?}");
                        }
                        Command::SetMute { channel, muted } => {
                            mixer.set_mute(channel, muted);
                            tracing::info!("Mute: {muted} on {channel:?}");
                        }
                        Command::SetSolo { channel, solo } => {
                            mixer.set_solo(channel, solo);
                            tracing::info!("Solo: {solo} on {channel:?}");
                        }
                        Command::SetPan { channel, pan } => {
                            mixer.set_pan(channel, pan);
                            tracing::info!("Pan: {pan:.2} on {channel:?}");
                        }
                        Command::Shutdown => break,
                        _ => {}
                    }
                    shared_mixer.update_from_mixer(&mixer);
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    // Stocker les handles pour l'UI
    CMD_TX.write().unwrap().replace(cmd_tx);
    EVENT_RX.write().unwrap().replace(channels.event_rx);
    DSP_CHAIN
        .write()
        .unwrap()
        .replace(engine.shared_dsp_chain());

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

// Sender dédié pour les commandes UI → thread de traitement
static CMD_TX: std::sync::RwLock<
    Option<crossbeam_channel::Sender<troubadour_shared::messages::Command>>,
> = std::sync::RwLock::new(None);

// Receiver pour les événements engine → UI
static EVENT_RX: std::sync::RwLock<
    Option<crossbeam_channel::Receiver<troubadour_shared::messages::Event>>,
> = std::sync::RwLock::new(None);

pub fn send_command(cmd: troubadour_shared::messages::Command) {
    if let Ok(guard) = CMD_TX.read()
        && let Some(tx) = guard.as_ref()
    {
        let _ = tx.try_send(cmd);
    }
}

// Handle vers la chaîne DSP partagée avec le callback audio
static DSP_CHAIN: std::sync::RwLock<
    Option<std::sync::Arc<std::sync::Mutex<troubadour_core::dsp::EffectsChain>>>,
> = std::sync::RwLock::new(None);

/// Reconstruit la chaîne DSP depuis un preset.
/// Appelé à chaque changement dans l'UI DSP.
pub fn update_dsp(preset: &troubadour_shared::dsp::EffectsPreset) {
    if let Ok(guard) = DSP_CHAIN.read()
        && let Some(dsp_arc) = guard.as_ref()
    {
        let new_chain = troubadour_core::dsp::EffectsChain::from_preset(preset);
        if let Ok(mut chain) = dsp_arc.lock() {
            *chain = new_chain;
        }
    }
}

pub fn try_recv_event() -> Option<troubadour_shared::messages::Event> {
    if let Ok(guard) = EVENT_RX.read()
        && let Some(rx) = guard.as_ref()
    {
        return rx.try_recv().ok();
    }
    None
}

fn app() -> Element {
    rsx! {
        components::mixer_view::MixerView {}
    }
}
