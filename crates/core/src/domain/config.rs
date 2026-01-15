//! Configuration management for Troubadour
//!
//! This module provides:
//! - Configuration structs for mixer, audio devices, and application settings
//! - Preset system with TOML serialization
//! - Command bus pattern for runtime state management
//! - Hot-reload support via file system watcher

use crate::domain::audio::StreamConfig;
use crate::domain::mixer::{ChannelId, MixerChannel, MixerEngine};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument};

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Errors that can occur during configuration operations
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("File watch error: {0}")]
    WatchError(#[from] notify::Error),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Preset not found: {0}")]
    PresetNotFound(String),

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),
}

/// Application-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Audio buffer size in frames
    pub buffer_size: u32,

    /// Sample rate
    pub sample_rate: u32,

    /// Enable automatic resampling
    pub enable_resampling: bool,

    /// Metering decay rate in dB per second
    pub meter_decay_rate: f32,

    /// Preset directory
    pub preset_dir: PathBuf,

    /// Auto-save interval in seconds (0 = disabled)
    pub auto_save_interval_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            buffer_size: 512,
            sample_rate: 48000,
            enable_resampling: true,
            meter_decay_rate: 12.0,
            preset_dir: PathBuf::from("presets"),
            auto_save_interval_secs: 30,
        }
    }
}

/// Audio device configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioDeviceConfig {
    /// Input device ID (empty = use default)
    #[serde(default)]
    pub input_device: String,

    /// Output device ID (empty = use default)
    #[serde(default)]
    pub output_device: String,

    /// Stream configuration
    #[serde(default)]
    pub stream_config: StreamConfig,
}

/// Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub name: String,
    pub volume_db: f32,
    pub muted: bool,
    pub solo: bool,
    /// Input device ID for this channel (None = use default input)
    #[serde(default)]
    pub input_device: Option<String>,
}

impl From<&MixerChannel> for ChannelConfig {
    fn from(channel: &MixerChannel) -> Self {
        Self {
            id: channel.id.as_str().to_string(),
            name: channel.name.clone(),
            volume_db: channel.volume.db(),
            muted: channel.muted,
            solo: channel.solo,
            input_device: channel.input_device.clone(),
        }
    }
}

impl ChannelConfig {
    pub fn to_channel(&self) -> MixerChannel {
        let mut channel = MixerChannel::new(
            ChannelId::new(self.id.clone()),
            self.name.clone(),
        );
        channel.set_volume(self.volume_db);
        if self.muted {
            channel.toggle_mute();
        }
        if self.solo {
            channel.toggle_solo();
        }
        channel.input_device = self.input_device.clone();
        channel
    }
}

/// Bus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusConfig {
    pub id: String,
    pub name: String,
    pub volume_db: f32,
    pub muted: bool,
    /// Output device ID for this bus (None = use default output)
    #[serde(default)]
    pub output_device: Option<String>,
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub routes: Vec<RouteConfig>,
}

/// Single route configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub from: String,
    pub to: String,
    pub enabled: bool,
}

impl From<&RouteConfig> for crate::domain::mixer::RouteEntry {
    fn from(config: &RouteConfig) -> Self {
        Self {
            from: ChannelId::new(config.from.clone()),
            to: ChannelId::new(config.to.clone()),
            enabled: config.enabled,
        }
    }
}

/// Mixer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerConfig {
    pub channels: Vec<ChannelConfig>,
    pub buses: Vec<BusConfig>,
    pub routing: RoutingConfig,
}

impl MixerConfig {
    /// Create mixer engine from configuration
    pub fn to_mixer_engine(&self) -> MixerEngine {
        let mut engine = MixerEngine::new();

        // Add channels
        for channel_config in &self.channels {
            let channel = channel_config.to_channel();
            engine.add_channel(channel);
        }

        // Set up bus configurations - add buses dynamically based on config
        // First, clear default buses and add configured ones
        let target_bus_count = self.buses.len();

        // Add buses until we reach the target count
        while engine.bus_count() < target_bus_count {
            if engine.add_bus().is_err() {
                break; // Max limit reached
            }
        }

        // Remove buses if we have too many
        while engine.bus_count() > target_bus_count {
            if engine.remove_bus().is_err() {
                break; // Min limit reached
            }
        }

        // Now configure each bus
        for bus_config in &self.buses {
            if let Some(bus) = engine.bus_mut(&crate::domain::mixer::BusId::new(bus_config.id.clone())) {
                bus.volume_db = bus_config.volume_db;
                bus.muted = bus_config.muted;
                bus.output_device = bus_config.output_device.clone()
                    .map(|id| crate::domain::audio::DeviceId::new(id));
            }
        }

        // Set up routing
        for route_config in &self.routing.routes {
            let from = ChannelId::new(route_config.from.clone());
            let to = ChannelId::new(route_config.to.clone());
            engine.routing_mut().set_route(&from, &to, route_config.enabled);
        }

        engine
    }

    /// Create configuration from mixer engine
    pub fn from_mixer_engine(engine: &MixerEngine) -> Self {
        let channels: Vec<ChannelConfig> = engine
            .channels()
            .map(ChannelConfig::from)
            .collect();

        let buses: Vec<BusConfig> = engine.buses()
            .iter()
            .map(|bus| BusConfig {
                id: bus.id.as_str().to_string(),
                name: bus.name.clone(),
                volume_db: bus.volume_db,
                muted: bus.muted,
                output_device: bus.output_device.as_ref().map(|d| d.as_str().to_string()),
            })
            .collect();

        let channel_ids: Vec<_> = engine.channels().map(|ch| ch.id.clone()).collect();

        let routes: Vec<RouteConfig> = channel_ids
            .iter()
            .flat_map(|from| {
                engine.routing().get_outputs(from)
                    .into_iter()
                    .map(move |to| RouteConfig {
                        from: from.as_str().to_string(),
                        to: to.as_str().to_string(),
                        enabled: true,
                    })
            })
            .collect();

        let routing = RoutingConfig { routes };

        Self { channels, buses, routing }
    }
}

/// Complete Troubadour configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TroubadourConfig {
    pub app: AppConfig,
    pub audio: AudioDeviceConfig,
    pub mixer: MixerConfig,
}

impl Default for TroubadourConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            audio: AudioDeviceConfig::default(),
            mixer: MixerConfig {
                channels: Vec::new(),
                buses: Vec::new(),
                routing: RoutingConfig { routes: Vec::new() },
            },
        }
    }
}

impl TroubadourConfig {
    /// Load configuration from TOML file
    #[instrument(skip(path))]
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        info!(path = %path.display(), "Loading configuration");

        let contents = fs::read_to_string(path).await?;
        let config: Self = toml::from_str(&contents)?;

        debug!("Configuration loaded successfully");
        Ok(config)
    }

    /// Save configuration to TOML file
    #[instrument(skip(self, path))]
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!(path = %path.display(), "Saving configuration");

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let toml_str = toml::to_string_pretty(self)?;
        fs::write(path, toml_str).await?;

        debug!("Configuration saved successfully");
        Ok(())
    }

    /// Create factory default configuration
    pub fn factory_default() -> Self {
        let mut config = Self::default();

        // Add some default channels
        config.mixer.channels.push(ChannelConfig {
            id: "mic".to_string(),
            name: "Microphone".to_string(),
            volume_db: 0.0,
            muted: false,
            solo: false,
            input_device: None,
        });

        config.mixer.channels.push(ChannelConfig {
            id: "music".to_string(),
            name: "Music".to_string(),
            volume_db: -6.0,
            muted: false,
            solo: false,
            input_device: None,
        });

        config.mixer.channels.push(ChannelConfig {
            id: "system".to_string(),
            name: "System Audio".to_string(),
            volume_db: -12.0,
            muted: false,
            solo: false,
            input_device: None,
        });

        // Add 2 default buses (A1, A2)
        config.mixer.buses.push(BusConfig {
            id: "A1".to_string(),
            name: "A1".to_string(),
            volume_db: 0.0,
            muted: false,
            output_device: None,
        });

        config.mixer.buses.push(BusConfig {
            id: "A2".to_string(),
            name: "A2".to_string(),
            volume_db: 0.0,
            muted: false,
            output_device: None,
        });

        // Default routing: all to output buses
        let output_buses = vec!["A1", "A2"];
        for input in &["mic", "music", "system"] {
            for output in &output_buses {
                config.mixer.routing.routes.push(RouteConfig {
                    from: input.to_string(),
                    to: output.to_string(),
                    enabled: true,
                });
            }
        }

        config
    }
}

/// Command types for runtime state management
#[derive(Debug, Clone)]
pub enum Command {
    SetVolume {
        channel_id: String,
        volume_db: f32,
    },
    ToggleMute {
        channel_id: String,
    },
    ToggleSolo {
        channel_id: String,
    },
    AddChannel {
        id: String,
        name: String,
    },
    RemoveChannel {
        channel_id: String,
    },
    SetRoute {
        from: String,
        to: String,
        enabled: bool,
    },
    LoadPreset {
        name: String,
    },
    SavePreset {
        name: String,
    },
    SetConfig {
        config: TroubadourConfig,
    },
}

/// Result of command execution
#[derive(Debug, Clone)]
pub enum CommandResult {
    Ok,
    VolumeChanged {
        channel_id: String,
        new_volume_db: f32,
    },
    MuteToggled {
        channel_id: String,
        muted: bool,
    },
    SoloToggled {
        channel_id: String,
        solo: bool,
    },
    ChannelAdded {
        id: String,
    },
    ChannelRemoved {
        id: String,
    },
    RouteChanged {
        from: String,
        to: String,
        enabled: bool,
    },
    PresetLoaded {
        name: String,
    },
    PresetSaved {
        name: String,
    },
    ConfigUpdated,
    Error(String),
}

/// Trait for command execution
#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, command: Command) -> CommandResult;
}

/// File system watcher for hot-reload
pub struct ConfigWatcher {
    _watcher: notify::RecommendedWatcher,
    config_tx: broadcast::Sender<PathBuf>,
}

impl ConfigWatcher {
    /// Create a new config watcher
    pub async fn new(preset_dir: PathBuf) -> Result<Self> {
        use notify::Watcher;

        let (config_tx, _config_rx) = broadcast::channel(32);

        // Create preset directory if it doesn't exist
        fs::create_dir_all(&preset_dir).await?;

        let tx_clone = config_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if matches!(
                    event.kind,
                    notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                ) {
                    for path in event.paths {
                        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                            if let Err(e) = tx_clone.send(path) {
                                error!("Failed to send config change event: {}", e);
                            }
                        }
                    }
                }
            }
        })?;

        watcher.watch(&preset_dir, notify::RecursiveMode::Recursive)?;

        info!(
            path = %preset_dir.display(),
            "Config watcher started"
        );

        Ok(Self {
            _watcher: watcher,
            config_tx,
        })
    }

    /// Subscribe to config change events
    pub fn subscribe(&self) -> broadcast::Receiver<PathBuf> {
        self.config_tx.subscribe()
    }
}

/// Preset manager
pub struct PresetManager {
    preset_dir: PathBuf,
}

impl PresetManager {
    /// Create a new preset manager
    pub fn new(preset_dir: PathBuf) -> Self {
        Self { preset_dir }
    }

    /// List all available presets
    #[instrument(skip(self))]
    pub async fn list_presets(&self) -> Result<Vec<String>> {
        let mut presets = Vec::new();

        let mut entries = fs::read_dir(&self.preset_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Some(name) = path.file_stem() {
                    if let Some(name_str) = name.to_str() {
                        presets.push(name_str.to_string());
                    }
                }
            }
        }

        presets.sort();
        debug!(count = presets.len(), "Listed presets");
        Ok(presets)
    }

    /// Load a preset by name
    #[instrument(skip(self))]
    pub async fn load_preset(&self, name: &str) -> Result<TroubadourConfig> {
        let path = self.preset_dir.join(format!("{}.toml", name));

        if !path.exists() {
            return Err(ConfigError::PresetNotFound(name.to_string()));
        }

        TroubadourConfig::load_from_file(&path).await
    }

    /// Save a preset by name
    #[instrument(skip(self, config))]
    pub async fn save_preset(&self, name: &str, config: &TroubadourConfig) -> Result<()> {
        let path = self.preset_dir.join(format!("{}.toml", name));
        config.save_to_file(&path).await
    }

    /// Delete a preset by name
    #[instrument(skip(self))]
    pub async fn delete_preset(&self, name: &str) -> Result<()> {
        let path = self.preset_dir.join(format!("{}.toml", name));

        if !path.exists() {
            return Err(ConfigError::PresetNotFound(name.to_string()));
        }

        fs::remove_file(&path).await?;
        info!(name, "Preset deleted");
        Ok(())
    }

    /// Check if a preset exists
    pub async fn preset_exists(&self, name: &str) -> bool {
        let path = self.preset_dir.join(format!("{}.toml", name));
        path.exists()
    }
}

/// Configuration manager for the main Troubadour config
///
/// Manages the main configuration file at `~/.config/troubadour/config.toml`
/// with auto-save functionality and debouncing.
pub struct ConfigManager {
    config_dir: PathBuf,
    config_path: PathBuf,
    _auto_save_interval_secs: u64, // Reserved for future use
}

impl ConfigManager {
    /// Create a new ConfigManager
    ///
    /// # Arguments
    /// * `config_dir` - Configuration directory path (e.g., `~/.config/troubadour`)
    /// * `auto_save_interval_secs` - Auto-save interval in seconds (0 = disabled)
    pub fn new(config_dir: PathBuf, auto_save_interval_secs: u64) -> Self {
        let config_path = config_dir.join("config.toml");

        Self {
            config_dir,
            config_path,
            _auto_save_interval_secs: auto_save_interval_secs,
        }
    }

    /// Get the default config directory path
    ///
    /// Returns `~/.config/troubadour` on Linux/Mac
    /// Returns `%APPDATA%\troubadour` on Windows
    pub fn default_config_dir() -> Result<PathBuf> {
        let config_dir = if cfg!(windows) {
            dirs::config_dir()
                .map(|p| p.join("troubadour"))
                .ok_or_else(|| ConfigError::Invalid("Could not determine config directory".to_string()))?
        } else {
            dirs::config_dir()
                .map(|p| p.join("troubadour"))
                .ok_or_else(|| ConfigError::Invalid("Could not determine config directory".to_string()))?
        };

        Ok(config_dir)
    }

    /// Get the config file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Load configuration from file
    ///
    /// If the config file doesn't exist, returns factory default.
    /// If the config file is corrupt, logs an error and returns factory default.
    #[instrument(skip(self))]
    pub async fn load(&self) -> TroubadourConfig {
        if !self.config_path.exists() {
            info!(
                path = %self.config_path.display(),
                "Config file not found, creating factory default"
            );

            let config = TroubadourConfig::factory_default();

            // Save the factory default for next time
            if let Err(e) = config.save_to_file(&self.config_path).await {
                error!(
                    path = %self.config_path.display(),
                    error = %e,
                    "Failed to save factory default config"
                );
            }

            return config;
        }

        match TroubadourConfig::load_from_file(&self.config_path).await {
            Ok(config) => {
                info!(
                    path = %self.config_path.display(),
                    "Configuration loaded successfully"
                );
                config
            }
            Err(e) => {
                error!(
                    path = %self.config_path.display(),
                    error = %e,
                    "Failed to load config, using factory default"
                );

                // Backup the corrupt config
                let backup_path = self.config_path.with_extension("toml.corrupt");
                if let Err(copy_err) = fs::copy(&self.config_path, &backup_path).await {
                    error!(
                        path = %backup_path.display(),
                        error = %copy_err,
                        "Failed to backup corrupt config"
                    );
                }

                TroubadourConfig::factory_default()
            }
        }
    }

    /// Save configuration to file
    #[instrument(skip(self, config))]
    pub async fn save(&self, config: &TroubadourConfig) -> Result<()> {
        // Create config directory if it doesn't exist
        fs::create_dir_all(&self.config_dir).await?;

        config.save_to_file(&self.config_path).await
    }

    /// Clear configuration (delete config file)
    #[instrument(skip(self))]
    pub async fn clear(&self) -> Result<()> {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path).await?;
            info!(
                path = %self.config_path.display(),
                "Configuration cleared"
            );
        }

        Ok(())
    }

    /// Check if config file exists
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mixer::VolumeDecibels;
    use tempfile::TempDir;

    #[test]
    fn test_config_serialization() {
        let config = TroubadourConfig::factory_default();

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: TroubadourConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.app.buffer_size, parsed.app.buffer_size);
        assert_eq!(config.mixer.channels.len(), parsed.mixer.channels.len());
    }

    #[test]
    fn test_channel_config_conversion() {
        let mixer_channel = MixerChannel::new(
            ChannelId::new("test".to_string()),
            "Test Channel".to_string(),
        );

        let config = ChannelConfig::from(&mixer_channel);
        assert_eq!(config.id, "test");
        assert_eq!(config.name, "Test Channel");
        assert_eq!(config.volume_db, 0.0);

        let converted = config.to_channel();
        assert_eq!(converted.id.as_str(), "test");
        assert_eq!(converted.name, "Test Channel");
    }

    #[tokio::test]
    async fn test_preset_manager() {
        let temp_dir = TempDir::new().unwrap();
        let preset_dir = temp_dir.path().to_path_buf();

        let manager = PresetManager::new(preset_dir.clone());
        let config = TroubadourConfig::factory_default();

        // Save preset
        manager.save_preset("test_preset", &config).await.unwrap();

        // Check it exists
        assert!(manager.preset_exists("test_preset").await);

        // List presets
        let presets = manager.list_presets().await.unwrap();
        assert_eq!(presets, vec!["test_preset"]);

        // Load preset
        let loaded = manager.load_preset("test_preset").await.unwrap();
        assert_eq!(loaded.app.buffer_size, config.app.buffer_size);

        // Delete preset
        manager.delete_preset("test_preset").await.unwrap();
        assert!(!manager.preset_exists("test_preset").await);
    }

    #[tokio::test]
    async fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = TroubadourConfig::factory_default();
        config.save_to_file(&config_path).await.unwrap();

        assert!(config_path.exists());

        let loaded = TroubadourConfig::load_from_file(&config_path).await.unwrap();
        assert_eq!(loaded.app.buffer_size, config.app.buffer_size);
        assert_eq!(loaded.mixer.channels.len(), config.mixer.channels.len());
    }

    #[test]
    fn test_volume_config_clamping() {
        let mut config = ChannelConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            volume_db: -100.0, // Below minimum
            muted: false,
            solo: false,
            input_device: None,
        };

        let channel = config.to_channel();
        assert_eq!(channel.volume.db(), VolumeDecibels::MIN_GAIN);

        config.volume_db = 100.0; // Above maximum
        let channel = config.to_channel();
        assert_eq!(channel.volume.db(), VolumeDecibels::MAX_GAIN);
    }
}
