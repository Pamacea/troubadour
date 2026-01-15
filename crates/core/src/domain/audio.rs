//! Audio device abstractions and domain models
//!
//! This module defines the core audio interfaces that are platform-agnostic.
//! Implementations for specific platforms (WASAPI, ALSA, CoreAudio) live in
//! the `infra` crate.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in the audio subsystem
#[derive(Debug, Error)]
pub enum AudioError {
    /// Requested audio device was not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// Error in audio stream creation or processing
    #[error("Stream error: {0}")]
    StreamError(String),

    /// Invalid configuration for audio device
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Input/Output error at the OS level
    #[error("OS error: {0}")]
    OsError(String),

    /// Device does not support the requested configuration
    #[error("Unsupported configuration: {0}")]
    UnsupportedConfiguration(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;

/// Unique identifier for an audio device
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Audio sample rate in Hz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleRate {
    Hz44100,
    Hz48000,
    Hz96000,
    Hz192000,
    Custom(u32),
}

impl SampleRate {
    pub fn hz(&self) -> u32 {
        match self {
            SampleRate::Hz44100 => 44100,
            SampleRate::Hz48000 => 48000,
            SampleRate::Hz96000 => 96000,
            SampleRate::Hz192000 => 192000,
            SampleRate::Custom(hz) => *hz,
        }
    }

    pub fn from_hz(hz: u32) -> Self {
        match hz {
            44100 => SampleRate::Hz44100,
            48000 => SampleRate::Hz48000,
            96000 => SampleRate::Hz96000,
            192000 => SampleRate::Hz192000,
            hz => SampleRate::Custom(hz),
        }
    }
}

/// Number of audio channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelCount {
    Mono,
    Stereo,
    Surround(u16),
}

impl ChannelCount {
    pub fn count(&self) -> u16 {
        match self {
            ChannelCount::Mono => 1,
            ChannelCount::Stereo => 2,
            ChannelCount::Surround(n) => *n,
        }
    }
}

/// Supported audio sample formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleFormat {
    I16,
    I32,
    F32,
    F64,
}

/// Configuration for an audio stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub sample_rate: SampleRate,
    pub channels: ChannelCount,
    pub format: SampleFormat,
    pub buffer_size: u32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: SampleRate::Hz48000,
            channels: ChannelCount::Stereo,
            format: SampleFormat::F32,
            buffer_size: 512,
        }
    }
}

/// Type of audio device
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Input,
    Output,
    Duplex,
}

/// Information about an audio device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub device_type: DeviceType,
    pub sample_rates: Vec<SampleRate>,
    pub channel_counts: Vec<ChannelCount>,
    pub default_sample_rate: Option<SampleRate>,
}

/// Trait for platform-agnostic audio device operations
///
/// This trait defines the interface that all platform implementations must support.
pub trait AudioDevice: Send + Sync {
    /// Get information about this device
    fn info(&self) -> &DeviceInfo;

    /// Check if the device supports a specific configuration
    fn supports_config(&self, config: &StreamConfig) -> bool;

    /// Get the default configuration for this device
    fn default_config(&self) -> Result<StreamConfig>;
}

/// Trait for enumerating available audio devices
pub trait AudioEnumerator: Send + Sync {
    /// List all available audio devices
    fn devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Get all input devices
    fn input_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Get all output devices
    fn output_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Get the default input device
    fn default_input_device(&self) -> Result<DeviceInfo>;

    /// Get the default output device
    fn default_output_device(&self) -> Result<DeviceInfo>;

    /// Find a device by its ID
    fn device_by_id(&self, id: &DeviceId) -> Result<DeviceInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_rate_conversion() {
        assert_eq!(SampleRate::Hz48000.hz(), 48000);
        assert_eq!(SampleRate::from_hz(48000), SampleRate::Hz48000);
        assert_eq!(SampleRate::Custom(96000).hz(), 96000);
    }

    #[test]
    fn test_channel_count() {
        assert_eq!(ChannelCount::Mono.count(), 1);
        assert_eq!(ChannelCount::Stereo.count(), 2);
        assert_eq!(ChannelCount::Surround(5).count(), 5);
    }

    #[test]
    fn test_device_id() {
        let id = DeviceId::new("test-device".to_string());
        assert_eq!(id.as_str(), "test-device");
    }

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.sample_rate.hz(), 48000);
        assert_eq!(config.channels.count(), 2);
        assert_eq!(config.buffer_size, 512);
    }
}
