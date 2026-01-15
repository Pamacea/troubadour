//! Troubadour GUI - Tauri Backend
//!
//! This module provides Tauri commands to interface with the Troubadour audio mixer.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use troubadour_core::domain::audio::{AudioDevice, AudioEnumerator};
use troubadour_core::domain::config::{PresetManager, TroubadourConfig};
use troubadour_core::domain::mixer::{ChannelId, MixerChannel, MixerEngine};
use troubadour_infra::audio::CpalAudioEnumerator;

/// Application state shared across Tauri commands
pub struct AppState {
    mixer: Arc<Mutex<MixerEngine>>,
    preset_manager: Arc<Mutex<PresetManager>>,
    rt: tokio::runtime::Runtime,
    enumerator: Arc<CpalAudioEnumerator>,
}

impl AppState {
    pub fn new() -> Self {
        // Initialize mixer with default configuration
        let mixer = Arc::new(Mutex::new(MixerEngine::new()));

        // Add 3 default channels for immediate usability
        {
            let mut mix = mixer.lock().unwrap();
            mix.add_channel(MixerChannel::new(
                ChannelId::new("input-1".to_string()),
                "Input 1".to_string(),
            ));
            mix.add_channel(MixerChannel::new(
                ChannelId::new("input-2".to_string()),
                "Input 2".to_string(),
            ));
            mix.add_channel(MixerChannel::new(
                ChannelId::new("input-3".to_string()),
                "Input 3".to_string(),
            ));

            // Add a master output channel
            mix.add_channel(MixerChannel::new(
                ChannelId::new("master".to_string()),
                "Master".to_string(),
            ));

            // Set up default routing: all inputs → master
            let input1 = ChannelId::new("input-1".to_string());
            let input2 = ChannelId::new("input-2".to_string());
            let input3 = ChannelId::new("input-3".to_string());
            let master = ChannelId::new("master".to_string());

            mix.routing_mut().set_route(&input1, &master, true);
            mix.routing_mut().set_route(&input2, &master, true);
            mix.routing_mut().set_route(&input3, &master, true);
        }

        // Initialize preset manager
        let preset_manager = Arc::new(Mutex::new(
            PresetManager::new(PathBuf::from("presets"))
        ));

        // Create Tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

        // Initialize audio enumerator
        let enumerator = Arc::new(CpalAudioEnumerator::new());

        Self {
            mixer,
            preset_manager,
            rt,
            enumerator,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// List all available audio devices
#[tauri::command]
fn list_audio_devices(state: tauri::State<AppState>) -> Result<Vec<DeviceInfo>, String> {
    state
        .enumerator
        .input_devices()
        .map(|devices| {
            devices
                .into_iter()
                .map(|d| DeviceInfo {
                    id: d.id().as_str().to_string(),
                    name: d.name().clone(),
                    device_type: "Input",
                    max_channels: d.max_channels().map(|c| c.count()).unwrap_or(2),
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get all mixer channels with their current state
#[tauri::command]
fn get_channels(state: tauri::State<AppState>) -> Result<Vec<ChannelInfo>, String> {
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    Ok(mixer
        .channels()
        .map(|ch| ChannelInfo {
            id: ch.id.as_str().to_string(),
            name: ch.name.clone(),
            volume_db: ch.volume.db(),
            muted: ch.muted,
            solo: ch.solo,
            level_db: ch.level.current_db,
            peak_db: ch.level.peak_db,
        })
        .collect())
}

/// Set volume for a channel (in decibels)
#[tauri::command]
fn set_volume(
    state: tauri::State<AppState>,
    channel_id: String,
    volume_db: f32,
) -> Result<(), String> {
    let id = ChannelId::new(channel_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let channel = mixer
        .channel_mut(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?;

    channel.set_volume(volume_db);
    Ok(())
}

/// Toggle mute for a channel
#[tauri::command]
fn toggle_mute(state: tauri::State<AppState>, channel_id: String) -> Result<bool, String> {
    let id = ChannelId::new(channel_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let channel = mixer
        .channel_mut(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?;

    Ok(channel.toggle_mute())
}

/// Toggle solo for a channel
#[tauri::command]
fn toggle_solo(state: tauri::State<AppState>, channel_id: String) -> Result<bool, String> {
    let id = ChannelId::new(channel_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let channel = mixer
        .channel_mut(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?;

    Ok(channel.toggle_solo())
}

/// Add a new channel
#[tauri::command]
fn add_channel(
    state: tauri::State<AppState>,
    channel_id: String,
    name: String,
) -> Result<(), String> {
    let id = ChannelId::new(channel_id);
    let channel = MixerChannel::new(id.clone(), name);
    state.mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .add_channel(channel);
    Ok(())
}

/// Remove a channel
#[tauri::command]
fn remove_channel(state: tauri::State<AppState>, channel_id: String) -> Result<(), String> {
    let id = ChannelId::new(channel_id);
    state.mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .remove_channel(&id)
        .map_err(|e| e.to_string())
}

/// Get routing matrix
#[tauri::command]
fn get_routing(state: tauri::State<AppState>) -> Result<Vec<RouteInfo>, String> {
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;
    let routing = mixer.routing();

    // Get all channels first
    let channel_ids: Vec<_> = mixer.channels().map(|ch| ch.id.clone()).collect();

    let mut routes = Vec::new();
    // Iterate through all possible routes
    for from in &channel_ids {
        let outputs = routing.get_outputs(from);
        for to in outputs {
            routes.push(RouteInfo {
                from: from.as_str().to_string(),
                to: to.as_str().to_string(),
                enabled: true,
            });
        }
    }

    Ok(routes)
}

/// Set a route (enable/disable connection between channels)
#[tauri::command]
fn set_route(
    state: tauri::State<AppState>,
    from: String,
    to: String,
    enabled: bool,
) -> Result<(), String> {
    let from_id = ChannelId::new(from);
    let to_id = ChannelId::new(to);

    state.mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .routing_mut()
        .set_route(&from_id, &to_id, enabled);

    Ok(())
}

/// List all available presets
#[tauri::command]
fn list_presets(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .rt
        .block_on(async {
            state
                .preset_manager
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?
                .list_presets()
                .await
                .map_err(|e| e.to_string())
        })
}

/// Load a preset by name
#[tauri::command]
fn load_preset(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<PresetInfo, String> {
    state
        .rt
        .block_on(async {
            let config = state
                .preset_manager
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?
                .load_preset(&name)
                .await
                .map_err(|e| e.to_string())?;

            // Apply configuration to mixer
            let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

            // Clear existing channels
            // (In real implementation, we'd be more careful here)
            for channel_config in &config.mixer.channels {
                let id = ChannelId::new(channel_config.id.clone());
                let mut channel = MixerChannel::new(id.clone(), channel_config.name.clone());
                channel.set_volume(channel_config.volume_db);
                if channel_config.muted {
                    channel.toggle_mute();
                }
                if channel_config.solo {
                    channel.toggle_solo();
                }
                mixer.add_channel(channel);
            }

            // Apply routing
            for route_config in &config.mixer.routing.routes {
                let from = ChannelId::new(route_config.from.clone());
                let to = ChannelId::new(route_config.to.clone());
                mixer.routing_mut().set_route(&from, &to, route_config.enabled);
            }

            Ok(PresetInfo {
                name,
                channel_count: config.mixer.channels.len(),
                route_count: config.mixer.routing.routes.len(),
            })
        })
}

/// Save current mixer state as a preset
#[tauri::command]
fn save_preset(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    state
        .rt
        .block_on(async {
            let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

            // Build configuration from current mixer state
            let channels: Vec<_> = mixer
                .channels()
                .map(|ch| troubadour_core::domain::config::ChannelConfig {
                    id: ch.id.as_str().to_string(),
                    name: ch.name.clone(),
                    volume_db: ch.volume.db(),
                    muted: ch.muted,
                    solo: ch.solo,
                })
                .collect();

            let routing = mixer.routing();
            let channel_ids: Vec<_> = mixer.channels().map(|ch| ch.id.clone()).collect();

            let routes: Vec<_> = channel_ids
                .iter()
                .flat_map(|from| {
                    routing.get_outputs(from)
                        .into_iter()
                        .map(move |to| troubadour_core::domain::config::RouteConfig {
                            from: from.as_str().to_string(),
                            to: to.as_str().to_string(),
                            enabled: true,
                        })
                })
                .collect();

            let mixer_config = troubadour_core::domain::config::MixerConfig {
                channels,
                routing: troubadour_core::domain::config::RoutingConfig { routes },
            };

            let config = TroubadourConfig {
                app: troubadour_core::domain::config::AppConfig::default(),
                audio: troubadour_core::domain::config::AudioDeviceConfig::default(),
                mixer: mixer_config,
            };

            state
                .preset_manager
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?
                .save_preset(&name, &config)
                .await
                .map_err(|e| e.to_string())
        })
}

/// Delete a preset
#[tauri::command]
fn delete_preset(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    state
        .rt
        .block_on(async {
            state
                .preset_manager
                .lock()
                .map_err(|e| format!("Lock error: {}", e))?
                .delete_preset(&name)
                .await
                .map_err(|e| e.to_string())
        })
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(serde::Serialize)]
struct DeviceInfo {
    id: String,
    name: String,
    device_type: &'static str,
    max_channels: u16,
}

#[derive(serde::Serialize)]
struct ChannelInfo {
    id: String,
    name: String,
    volume_db: f32,
    muted: bool,
    solo: bool,
    level_db: f32,
    peak_db: f32,
}

#[derive(serde::Serialize)]
struct RouteInfo {
    from: String,
    to: String,
    enabled: bool,
}

#[derive(serde::Serialize)]
struct PresetInfo {
    name: String,
    channel_count: usize,
    route_count: usize,
}

// ============================================================================
// Tauri Builder
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("troubadour=debug,info"))
        )
        .init();

    // Create app state
    let state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_audio_devices,
            get_channels,
            set_volume,
            toggle_mute,
            toggle_solo,
            add_channel,
            remove_channel,
            get_routing,
            set_route,
            list_presets,
            load_preset,
            save_preset,
            delete_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
