//! Domain entities and business rules

pub mod audio;
pub mod mixer;
pub mod dsp;
pub mod config;

// Re-export specific items to avoid ambiguous glob imports
pub use audio::{
    AudioDevice, AudioEnumerator, AudioError, ChannelCount, DeviceId, DeviceInfo, DeviceType,
    SampleFormat, SampleRate, StreamConfig,
};
// Use specific imports for config to avoid Result conflict
pub use config::{
    AppConfig, AudioDeviceConfig, ChannelConfig, Command, CommandExecutor, CommandResult,
    ConfigError, ConfigWatcher, MixerConfig, PresetManager, RouteConfig, RoutingConfig,
    TroubadourConfig,
};
pub use dsp::*;
pub use mixer::{
    AudioLevel, ChannelId, MixerChannel, MixerEngine, RoutingMatrix, RouteEntry, VolumeDecibels,
};
