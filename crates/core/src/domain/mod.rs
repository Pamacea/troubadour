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
pub use config::*;
pub use dsp::*;
pub use mixer::{
    AudioLevel, ChannelId, MixerChannel, MixerEngine, RoutingMatrix, RouteEntry, VolumeDecibels,
};
