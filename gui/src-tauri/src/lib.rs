//! Troubadour GUI - Tauri Backend
//!
//! This module provides Tauri commands to interface with the Troubadour audio mixer.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::info;
use troubadour_core::domain::audio::AudioEnumerator;
use troubadour_core::domain::config::{ConfigManager, PresetManager, TroubadourConfig, MixerConfig};
use troubadour_core::domain::mixer::{ChannelId, BusId, MixerChannel, MixerEngine};
use troubadour_infra::audio::{CpalEnumerator, AudioEngine};

/// Validate and sanitize a channel ID
/// Only allows alphanumeric characters, hyphens, and underscores
fn validate_channel_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Channel ID cannot be empty".to_string());
    }
    if id.len() > 100 {
        return Err("Channel ID too long (max 100 characters)".to_string());
    }
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Channel ID contains invalid characters".to_string());
    }
    Ok(())
}

/// Validate and sanitize a channel name
/// Only allows alphanumeric, spaces, and common punctuation
fn validate_channel_name(name: &str) -> Result<(), String> {
    if name.len() > 200 {
        return Err("Channel name too long (max 200 characters)".to_string());
    }
    // Allow alphanumeric, spaces, and common punctuation
    if !name.chars().all(|c| {
        c.is_alphanumeric()
        || c.is_whitespace()
        || "()-_.,'/".contains(c)
    }) {
        return Err("Channel name contains invalid characters".to_string());
    }
    Ok(())
}

/// Validate a volume value is within acceptable bounds (-60 to +6 dB)
fn validate_volume_db(volume_db: f32) -> Result<(), String> {
    if volume_db < -60.0 || volume_db > 6.0 {
        return Err(format!("Volume out of range: {} (must be -60 to +6 dB)", volume_db));
    }
    if !volume_db.is_finite() {
        return Err("Volume must be a finite number".to_string());
    }
    Ok(())
}

/// Application state shared across Tauri commands
pub struct AppState {
    mixer: Arc<Mutex<MixerEngine>>,
    preset_manager: Arc<Mutex<PresetManager>>,
    config_manager: Arc<ConfigManager>,
    rt: tokio::runtime::Runtime,
    enumerator: Arc<CpalEnumerator>,
    audio_engine: Arc<Mutex<Option<AudioEngine>>>,
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

        // Initialize config manager with 1 second auto-save interval
        let config_dir = ConfigManager::default_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let config_manager = Arc::new(ConfigManager::new(config_dir, 1));

        // Create Tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

        // Initialize audio enumerator
        let enumerator = Arc::new(CpalEnumerator::new());

        // Audio engine will be initialized later when user starts audio
        let audio_engine = Arc::new(Mutex::new(None));

        Self {
            mixer,
            preset_manager,
            config_manager,
            rt,
            enumerator,
            audio_engine,
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
fn list_audio_devices(state: tauri::State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    use troubadour_core::domain::audio::AudioError;
    use serde_json::json;

    state
        .enumerator
        .input_devices()
        .map(|devices: Vec<troubadour_core::domain::audio::DeviceInfo>| {
            devices
                .into_iter()
                .map(|d| {
                    let max_ch = d.channel_counts.first().map(|c| c.count()).unwrap_or(2);
                    // Sanitize device name to prevent XSS
                    let safe_name = d.name.chars()
                        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "()-_.".contains(*c))
                        .collect::<String>();
                    json!({
                        "id": d.id.as_str(),
                        "name": safe_name,
                        "device_type": "Input",
                        "max_channels": max_ch,
                    })
                })
                .collect()
        })
        .map_err(|e: AudioError| e.to_string())
}

/// List all output devices
#[tauri::command]
fn list_output_devices(state: tauri::State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    use troubadour_core::domain::audio::AudioError;
    use serde_json::json;

    state
        .enumerator
        .output_devices()
        .map(|devices: Vec<troubadour_core::domain::audio::DeviceInfo>| {
            devices
                .into_iter()
                .map(|d| {
                    let max_ch = d.channel_counts.first().map(|c| c.count()).unwrap_or(2);
                    // Sanitize device name to prevent XSS
                    let safe_name = d.name.chars()
                        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "()-_.".contains(*c))
                        .collect::<String>();
                    json!({
                        "id": d.id.as_str(),
                        "name": safe_name,
                        "device_type": "Output",
                        "max_channels": max_ch,
                    })
                })
                .collect()
        })
        .map_err(|e: AudioError| e.to_string())
}

/// Set output device for a bus
#[tauri::command]
fn set_bus_output_device(
    state: tauri::State<AppState>,
    bus_id: String,
    device_id: Option<String>,
) -> Result<(), String> {
    use troubadour_core::domain::audio::DeviceId;

    let bus_id = BusId::new(bus_id);
    let device_id = device_id.map(DeviceId::new);

    state
        .mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .set_bus_output_device(&bus_id, device_id)
        .map_err(|e| e.to_string())
}

/// Get output device for a bus
#[tauri::command]
fn get_bus_output_device(
    state: tauri::State<AppState>,
    bus_id: String,
) -> Result<Option<String>, String> {
    let bus_id = BusId::new(bus_id);
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    Ok(mixer
        .get_bus_output_device(&bus_id)
        .map(|d| d.as_str().to_string()))
}

/// Set input device for a bus
#[tauri::command]
fn set_bus_input_device(
    state: tauri::State<AppState>,
    bus_id: String,
    device_id: Option<String>,
) -> Result<(), String> {
    use troubadour_core::domain::audio::DeviceId;

    let bus_id = BusId::new(bus_id);
    let device_id = device_id.map(DeviceId::new);

    state
        .mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .set_bus_input_device(&bus_id, device_id)
        .map_err(|e| e.to_string())
}

/// Get input device for a bus
#[tauri::command]
fn get_bus_input_device(
    state: tauri::State<AppState>,
    bus_id: String,
) -> Result<Option<String>, String> {
    let bus_id = BusId::new(bus_id);
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    Ok(mixer
        .get_bus_input_device(&bus_id)
        .map(|d| d.as_str().to_string()))
}

/// Set volume for a bus (in decibels)
#[tauri::command]
fn set_bus_volume(
    state: tauri::State<AppState>,
    bus_id: String,
    volume_db: f32,
) -> Result<(), String> {
    let bus_id = BusId::new(bus_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let bus = mixer
        .bus_mut(&bus_id)
        .ok_or_else(|| format!("Bus not found: {}", bus_id.as_str()))?;

    bus.set_volume(volume_db);
    Ok(())
}

/// Toggle mute for a bus
#[tauri::command]
fn toggle_bus_mute(state: tauri::State<AppState>, bus_id: String) -> Result<bool, String> {
    let bus_id = BusId::new(bus_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let bus = mixer
        .bus_mut(&bus_id)
        .ok_or_else(|| format!("Bus not found: {}", bus_id.as_str()))?;

    Ok(bus.toggle_mute())
}

/// Get all mixer channels with their current state
#[tauri::command]
fn get_channels(state: tauri::State<AppState>) -> Result<Vec<ChannelInfo>, String> {
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    Ok(mixer
        .channels()
        .map(|ch| {
            let is_master = ch.id.as_str() == "master" || ch.name.to_lowercase() == "master";
            ChannelInfo {
                id: ch.id.as_str().to_string(),
                name: ch.name.clone(),
                volume_db: ch.volume.db(),
                muted: ch.muted,
                solo: ch.solo,
                level_db: ch.level.current_db,
                peak_db: ch.level.peak_db,
                input_device: ch.get_input_device().map(|s| s.to_string()),
                is_master,
            }
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
    validate_channel_id(&channel_id)?;
    validate_volume_db(volume_db)?;

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
    validate_channel_id(&channel_id)?;

    let id = ChannelId::new(channel_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let channel = mixer
        .channel_mut(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?;

    Ok(channel.toggle_mute())
}

/// Toggle solo for a channel (exclusive behavior)
#[tauri::command]
fn toggle_solo(state: tauri::State<AppState>, channel_id: String) -> Result<bool, String> {
    validate_channel_id(&channel_id)?;

    let id = ChannelId::new(channel_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    // Get current state to determine new state
    let current_solo = mixer
        .channel(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?
        .solo;

    let new_solo_state = !current_solo;

    // Use exclusive solo logic
    mixer
        .set_channel_solo_exclusive(&id, new_solo_state)
        .map_err(|e| e.to_string())?;

    Ok(new_solo_state)
}

/// Add a new channel
#[tauri::command]
fn add_channel(
    state: tauri::State<AppState>,
    channel_id: String,
    name: String,
) -> Result<(), String> {
    validate_channel_id(&channel_id)?;
    validate_channel_name(&name)?;

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
    validate_channel_id(&channel_id)?;

    let id = ChannelId::new(channel_id);
    state.mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .remove_channel(&id)
        .map_err(|e| e.to_string())
}

/// Set input device for a channel
#[tauri::command]
fn set_channel_input_device(
    state: tauri::State<AppState>,
    channel_id: String,
    device_id: Option<String>,
) -> Result<(), String> {
    let id = ChannelId::new(channel_id);
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let channel = mixer
        .channel_mut(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?;

    channel.set_input_device(device_id);
    Ok(())
}

/// Get input device for a channel
#[tauri::command]
fn get_channel_input_device(
    state: tauri::State<AppState>,
    channel_id: String,
) -> Result<Option<String>, String> {
    let id = ChannelId::new(channel_id);
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let channel = mixer
        .channel(&id)
        .ok_or_else(|| format!("Channel not found: {}", id.as_str()))?;

    Ok(channel.get_input_device().map(|s| s.to_string()))
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
    validate_channel_id(&from)?;
    validate_channel_id(&to)?;

    let from_id = ChannelId::new(from);
    let to_id = ChannelId::new(to);

    state
        .mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .routing_mut()
        .set_route(&from_id, &to_id, enabled);

    Ok(())
}

/// Get all available buses
#[tauri::command]
fn get_buses(state: tauri::State<AppState>) -> Result<Vec<BusInfo>, String> {
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    Ok(mixer
        .buses()
        .iter()
        .map(|bus| BusInfo {
            id: bus.id.as_str().to_string(),
            name: bus.name.clone(),
            input_device: bus.input_device.as_ref().map(|d| d.as_str().to_string()),
            output_device: bus.output_device.as_ref().map(|d| d.as_str().to_string()),
            volume_db: bus.volume_db,
            muted: bus.muted,
        })
        .collect())
}

/// Get buses assigned to a channel
#[tauri::command]
fn get_channel_buses(state: tauri::State<AppState>, channel_id: String) -> Result<Vec<String>, String> {
    let id = ChannelId::new(channel_id);
    let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    let bus_ids = mixer.get_channel_buses(&id);
    Ok(bus_ids.into_iter().map(|id| id.as_str().to_string()).collect())
}

/// Set which buses a channel is routed to
#[tauri::command]
fn set_channel_buses(
    state: tauri::State<AppState>,
    channel_id: String,
    bus_ids: Vec<String>,
) -> Result<(), String> {
    let id = ChannelId::new(channel_id);
    let bus_ids: Vec<BusId> = bus_ids.into_iter().map(BusId::new).collect();

    state
        .mixer
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .set_channel_buses(&id, bus_ids);

    Ok(())
}

/// Add a new bus to the mixer
#[tauri::command]
fn add_bus(state: tauri::State<AppState>) -> Result<String, String> {
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    mixer
        .add_bus()
        .map(|bus_id| bus_id.as_str().to_string())
        .map_err(|e| e.to_string())
}

/// Remove the last bus from the mixer
#[tauri::command]
fn remove_bus(state: tauri::State<AppState>) -> Result<String, String> {
    let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

    mixer
        .remove_bus()
        .map(|bus_id| bus_id.as_str().to_string())
        .map_err(|e| e.to_string())
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
                    input_device: ch.input_device.clone(),
                })
                .collect();

            let buses: Vec<_> = mixer.buses()
                .iter()
                .map(|bus| troubadour_core::domain::config::BusConfig {
                    id: bus.id.as_str().to_string(),
                    name: bus.name.clone(),
                    volume_db: bus.volume_db,
                    muted: bus.muted,
                    output_device: bus.output_device.as_ref().map(|d| d.as_str().to_string()),
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
                buses,
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
struct ChannelInfo {
    id: String,
    name: String,
    volume_db: f32,
    muted: bool,
    solo: bool,
    level_db: f32,
    peak_db: f32,
    input_device: Option<String>,
    is_master: bool,
}

#[derive(serde::Serialize)]
struct RouteInfo {
    from: String,
    to: String,
    enabled: bool,
}

#[derive(serde::Serialize)]
struct BusInfo {
    id: String,
    name: String,
    input_device: Option<String>,
    output_device: Option<String>,
    volume_db: f32,
    muted: bool,
}

#[derive(serde::Serialize)]
struct PresetInfo {
    name: String,
    channel_count: usize,
    route_count: usize,
}

/// Load configuration from file
#[tauri::command]
fn load_config(state: tauri::State<AppState>) -> Result<(), String> {
    state
        .rt
        .block_on(async {
            let config = state.config_manager.load().await;

            // Apply configuration to mixer
            let mixer_config = config.mixer;
            let mut mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

            // Clear existing channels and rebuild from config
            for channel_config in &mixer_config.channels {
                let id = ChannelId::new(channel_config.id.clone());
                let mut channel = MixerChannel::new(id.clone(), channel_config.name.clone());
                channel.set_volume(channel_config.volume_db);
                if channel_config.muted {
                    channel.toggle_mute();
                }
                if channel_config.solo {
                    channel.toggle_solo();
                }
                channel.input_device = channel_config.input_device.clone();
                mixer.add_channel(channel);
            }

            // Apply bus configurations
            for bus_config in &mixer_config.buses {
                let bus_id = BusId::new(bus_config.id.clone());
                if let Some(bus) = mixer.bus_mut(&bus_id) {
                    bus.volume_db = bus_config.volume_db;
                    bus.muted = bus_config.muted;
                    bus.output_device = bus_config.output_device.clone()
                        .map(|id| troubadour_core::domain::audio::DeviceId::new(id));
                }
            }

            // Apply routing
            for route_config in &mixer_config.routing.routes {
                let from = ChannelId::new(route_config.from.clone());
                let to = ChannelId::new(route_config.to.clone());
                mixer.routing_mut().set_route(&from, &to, route_config.enabled);
            }

            Ok(())
        })
}

/// Save current configuration to file
#[tauri::command]
fn save_config(state: tauri::State<AppState>) -> Result<(), String> {
    state
        .rt
        .block_on(async {
            let mixer = state.mixer.lock().map_err(|e| format!("Lock error: {}", e))?;

            // Build configuration from current mixer state
            let mixer_config = MixerConfig::from_mixer_engine(&mixer);

            let config = TroubadourConfig {
                app: troubadour_core::domain::config::AppConfig::default(),
                audio: troubadour_core::domain::config::AudioDeviceConfig::default(),
                mixer: mixer_config,
            };

            state
                .config_manager
                .save(&config)
                .await
                .map_err(|e| e.to_string())
        })
}

/// Get the config file path
#[tauri::command]
fn get_config_path(state: tauri::State<AppState>) -> Result<String, String> {
    Ok(state
        .config_manager
        .config_path()
        .to_string_lossy()
        .to_string())
}

/// Clear configuration (delete config file)
#[tauri::command]
fn clear_config(state: tauri::State<AppState>) -> Result<(), String> {
    state
        .rt
        .block_on(async {
            state
                .config_manager
                .clear()
                .await
                .map_err(|e| e.to_string())
        })
}

/// Start audio engine with per-channel device routing
///
/// This command reads the input_device field from each mixer channel
/// and creates audio streams accordingly. Channels with the same device
/// share a stream, channels with different devices get separate streams.
#[tauri::command]
fn start_audio(state: tauri::State<AppState>) -> Result<String, String> {
    use troubadour_core::domain::audio::SampleRate;

    info!("Starting audio engine with per-channel device routing");

    // Create audio engine
    let engine = AudioEngine::new(
        state.enumerator.clone(),
        state.mixer.clone(),
        SampleRate::Hz48000,
        512, // buffer size
    );

    // Initialize the engine
    let mut engine_guard = engine;

    // Start streams based on channel device assignments
    engine_guard.start_channel_streams()
        .map_err(|e| format!("Failed to start channel streams: {}", e))?;

    // Store the engine in state
    *state.audio_engine.lock().map_err(|e| format!("Lock error: {}", e))? = Some(engine_guard);

    Ok("Audio engine started successfully".to_string())
}

/// Stop audio engine
#[tauri::command]
fn stop_audio(state: tauri::State<AppState>) -> Result<(), String> {
    info!("Stopping audio engine");

    let mut audio_engine = state.audio_engine.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    *audio_engine = None; // Drop the engine, which will stop all streams

    Ok(())
}

/// Refresh audio streams after device assignment changes
///
/// Call this after modifying channel input_device assignments to restart
/// streams with the new routing configuration.
#[tauri::command]
fn refresh_audio_streams(state: tauri::State<AppState>) -> Result<String, String> {
    info!("Refreshing audio streams");

    let mut audio_engine = state.audio_engine.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    if let Some(engine) = &mut *audio_engine {
        engine.refresh_streams()
            .map_err(|e| format!("Failed to refresh streams: {}", e))?;

        Ok("Audio streams refreshed successfully".to_string())
    } else {
        Err("Audio engine not running. Call start_audio first.".to_string())
    }
}

/// Check if audio engine is running
#[tauri::command]
fn is_audio_running(state: tauri::State<AppState>) -> Result<bool, String> {
    let audio_engine = state.audio_engine.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    Ok(audio_engine.as_ref().map(|e| e.is_running()).unwrap_or(false))
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
            list_output_devices,
            set_bus_output_device,
            get_bus_output_device,
            set_bus_input_device,
            get_bus_input_device,
            set_bus_volume,
            toggle_bus_mute,
            get_channels,
            set_volume,
            toggle_mute,
            toggle_solo,
            add_channel,
            remove_channel,
            set_channel_input_device,
            get_channel_input_device,
            get_routing,
            set_route,
            get_buses,
            get_channel_buses,
            set_channel_buses,
            add_bus,
            remove_bus,
            list_presets,
            load_preset,
            save_preset,
            delete_preset,
            load_config,
            save_config,
            get_config_path,
            clear_config,
            start_audio,
            stop_audio,
            refresh_audio_streams,
            is_audio_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
