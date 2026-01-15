//! CPAL-based audio device implementation
//!
//! Provides a cross-platform interface to audio devices using the CPAL library.

use cpal::traits::{DeviceTrait, HostTrait};
use std::fmt;
use tracing::{debug, info, warn};
use troubadour_core::domain::audio::{
    AudioDevice, AudioEnumerator, AudioError, ChannelCount, DeviceId, DeviceInfo, DeviceType,
    Result, SampleFormat, SampleRate, StreamConfig,
};

/// CPAL-based audio device wrapper
pub struct CpalDevice {
    info: DeviceInfo,
    cpal_device: cpal::Device,
}

impl CpalDevice {
    pub fn new(cpal_device: cpal::Device, device_type: DeviceType) -> Result<Self> {
        #[allow(deprecated)]
        let name = cpal_device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string());

        // Get supported configurations
        let mut sample_rates = Vec::new();
        let mut channel_counts = Vec::new();

        // Query input configs first
        if let Ok(configs) = cpal_device.supported_input_configs() {
            for config in configs {
                let rate = config.min_sample_rate();
                let channels = config.channels();

                // Convert CPAL sample rate to our domain type
                let rate_hz = rate;
                if rate_hz == 44100 {
                    sample_rates.push(SampleRate::Hz44100);
                } else if rate_hz == 48000 {
                    sample_rates.push(SampleRate::Hz48000);
                } else if rate_hz == 96000 {
                    sample_rates.push(SampleRate::Hz96000);
                } else if rate_hz == 192000 {
                    sample_rates.push(SampleRate::Hz192000);
                } else {
                    sample_rates.push(SampleRate::Custom(rate_hz));
                }

                // Convert channel count
                if channels == 1 {
                    channel_counts.push(ChannelCount::Mono);
                } else if channels == 2 {
                    channel_counts.push(ChannelCount::Stereo);
                } else {
                    channel_counts.push(ChannelCount::Surround(channels));
                }
            }
        }

        // Query output configs if needed
        if let Ok(configs) = cpal_device.supported_output_configs() {
            for config in configs {
                let rate = config.min_sample_rate();
                let channels = config.channels();

                // Convert CPAL sample rate to our domain type
                let rate_hz = rate;
                if rate_hz == 44100 && !sample_rates.contains(&SampleRate::Hz44100) {
                    sample_rates.push(SampleRate::Hz44100);
                } else if rate_hz == 48000 && !sample_rates.contains(&SampleRate::Hz48000) {
                    sample_rates.push(SampleRate::Hz48000);
                } else if rate_hz == 96000 && !sample_rates.contains(&SampleRate::Hz96000) {
                    sample_rates.push(SampleRate::Hz96000);
                } else if rate_hz == 192000 && !sample_rates.contains(&SampleRate::Hz192000) {
                    sample_rates.push(SampleRate::Hz192000);
                } else if !sample_rates.iter().any(|sr| sr.hz() == rate_hz) {
                    sample_rates.push(SampleRate::Custom(rate_hz));
                }

                // Convert channel count
                if channels == 1 && !channel_counts.contains(&ChannelCount::Mono) {
                    channel_counts.push(ChannelCount::Mono);
                } else if channels == 2 && !channel_counts.contains(&ChannelCount::Stereo) {
                    channel_counts.push(ChannelCount::Stereo);
                } else if !channel_counts.iter().any(|cc| cc.count() == channels) {
                    channel_counts.push(ChannelCount::Surround(channels));
                }
            }
        }

        // Remove duplicates
        sample_rates.sort_by_key(|sr| sr.hz());
        sample_rates.dedup_by_key(|sr| sr.hz());
        channel_counts.sort_by_key(|cc| cc.count());
        channel_counts.dedup_by_key(|cc| cc.count());

        // Get default config
        let default_config = cpal_device
            .default_input_config()
            .or_else(|_| cpal_device.default_output_config());

        let default_sample_rate = default_config
            .ok()
            .map(|config| SampleRate::from_hz(config.sample_rate()));

        // Create DeviceId (use name as ID for simplicity)
        let id = DeviceId::new(name.clone());

        let info = DeviceInfo {
            id: id.clone(),
            name,
            device_type,
            sample_rates: sample_rates.clone(),
            channel_counts,
            default_sample_rate,
        };

        debug!("Created device: {}", info.name);

        Ok(Self {
            info,
            cpal_device,
        })
    }
}

impl AudioDevice for CpalDevice {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn supports_config(&self, config: &StreamConfig) -> bool {
        // Check if sample rate is supported
        let rate_supported = self
            .info
            .sample_rates
            .iter()
            .any(|sr| sr.hz() == config.sample_rate.hz());

        // Check if channel count is supported
        let channels_supported = self
            .info
            .channel_counts
            .iter()
            .any(|cc| cc.count() == config.channels.count());

        rate_supported && channels_supported
    }

    fn default_config(&self) -> Result<StreamConfig> {
        let cpal_config = self
            .cpal_device
            .default_input_config()
            .or_else(|_| self.cpal_device.default_output_config())
            .map_err(|e| AudioError::InvalidConfiguration(e.to_string()))?;

        let sample_format = match cpal_config.sample_format() {
            cpal::SampleFormat::I16 => SampleFormat::I16,
            cpal::SampleFormat::I32 => SampleFormat::I32,
            cpal::SampleFormat::F32 => SampleFormat::F32,
            cpal::SampleFormat::F64 => SampleFormat::F64,
            _ => SampleFormat::F32,
        };

        let buffer_size = match cpal_config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, .. } => *min,
            cpal::SupportedBufferSize::Unknown => 512,
        };

        Ok(StreamConfig {
            sample_rate: SampleRate::from_hz(cpal_config.sample_rate()),
            channels: match cpal_config.channels() {
                1 => ChannelCount::Mono,
                2 => ChannelCount::Stereo,
                n => ChannelCount::Surround(n),
            },
            format: sample_format,
            buffer_size,
        })
    }
}

impl fmt::Debug for CpalDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpalDevice")
            .field("info", &self.info)
            .finish()
    }
}

/// CPAL-based audio enumerator
pub struct CpalEnumerator {
    host: cpal::Host,
}

impl Default for CpalEnumerator {
    fn default() -> Self {
        info!("Initializing CPAL enumerator");
        Self::new()
    }
}

impl CpalEnumerator {
    pub fn new() -> Self {
        let host = cpal::default_host();
        debug!("Using audio host: {:?}", host.id());
        Self { host }
    }

    /// Convert CPAL device type to our domain type
    fn determine_device_type(&self, device: &cpal::Device) -> Result<DeviceType> {
        let has_input = device.supported_input_configs().is_ok();
        let has_output = device.supported_output_configs().is_ok();

        match (has_input, has_output) {
            (true, true) => Ok(DeviceType::Duplex),
            (true, false) => Ok(DeviceType::Input),
            (false, true) => Ok(DeviceType::Output),
            (false, false) => Err(AudioError::UnsupportedConfiguration(
                "Device has no inputs or outputs".to_string(),
            )),
        }
    }
}

impl AudioEnumerator for CpalEnumerator {
    fn devices(&self) -> Result<Vec<DeviceInfo>> {
        info!("Enumerating all audio devices");
        let mut devices = Vec::new();

        let cpal_devices = self
            .host
            .devices()
            .map_err(|e| AudioError::OsError(e.to_string()))?;

        for device in cpal_devices {
            let device_type = match self.determine_device_type(&device) {
                Ok(dt) => dt,
                Err(_) => continue,
            };

            match CpalDevice::new(device, device_type) {
                Ok(cp_device) => {
                    devices.push(cp_device.info().clone());
                    debug!("Found device: {}", cp_device.info().name);
                }
                Err(e) => {
                    warn!("Skipping device due to error: {}", e);
                }
            }
        }

        info!("Found {} audio devices", devices.len());
        Ok(devices)
    }

    fn input_devices(&self) -> Result<Vec<DeviceInfo>> {
        let all_devices = self.devices()?;
        Ok(all_devices
            .into_iter()
            .filter(|d| matches!(d.device_type, DeviceType::Input | DeviceType::Duplex))
            .collect())
    }

    fn output_devices(&self) -> Result<Vec<DeviceInfo>> {
        let all_devices = self.devices()?;
        Ok(all_devices
            .into_iter()
            .filter(|d| matches!(d.device_type, DeviceType::Output | DeviceType::Duplex))
            .collect())
    }

    fn default_input_device(&self) -> Result<DeviceInfo> {
        let cpal_device = self
            .host
            .default_input_device()
            .ok_or_else(|| AudioError::DeviceNotFound("No default input device".to_string()))?;

        CpalDevice::new(cpal_device, DeviceType::Input).map(|d| d.info().clone())
    }

    fn default_output_device(&self) -> Result<DeviceInfo> {
        let cpal_device = self
            .host
            .default_output_device()
            .ok_or_else(|| AudioError::DeviceNotFound("No default output device".to_string()))?;

        CpalDevice::new(cpal_device, DeviceType::Output).map(|d| d.info().clone())
    }

    fn device_by_id(&self, id: &DeviceId) -> Result<DeviceInfo> {
        let devices = self.devices()?;
        devices
            .into_iter()
            .find(|d| d.id == *id)
            .ok_or_else(|| AudioError::DeviceNotFound(id.as_str().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerator_creation() {
        let enumerator = CpalEnumerator::default();
        assert_eq!(enumerator.host.id(), cpal::default_host().id());
    }

    #[test]
    fn test_enumerate_devices() {
        let enumerator = CpalEnumerator::default();
        match enumerator.devices() {
            Ok(devices) => {
                assert!(!devices.is_empty(), "Should have at least one device");
                for device in &devices {
                    assert!(!device.name.is_empty());
                }
            }
            Err(e) => {
                // On CI or headless systems, there might not be audio devices
                eprintln!("Skipping test: {}", e);
            }
        }
    }

    #[test]
    fn test_get_default_devices() {
        let enumerator = CpalEnumerator::default();

        match (enumerator.default_input_device(), enumerator.default_output_device()) {
            (Ok(input), Ok(output)) => {
                assert!(!input.name.is_empty());
                assert!(!output.name.is_empty());
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("Skipping test: {}", e);
            }
        }
    }
}
