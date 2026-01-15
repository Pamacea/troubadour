//! Real-time audio engine for managing input/output streams
//!
//! This module provides the AudioEngine which coordinates multiple audio streams,
//! routes them through the mixer, and handles device connections.
//!
//! # CRITICAL TODO: Output Audio Not Being Played
//!
//! **MAJOR BUG**: The mixer processes audio correctly, but the output is NOT
//! being sent to the output streams! Users will see level meters moving but
//! hear no audio from output devices.
//!
//! Current state:
//! - Input streams: ✅ Working (audio captured from devices)
//! - Mixer processing: ✅ Working (audio mixed, effects applied)
//! - Output streams: ❌ Created but NOT receiving audio
//!
//! To fix this, in `process_audio()` at line 376:
//! 1. Get output streams for each bus that has an output_device assigned
//! 2. Send the mixed bus audio to the corresponding output stream
//! 3. Handle multiple buses routing to the same output device (mix them)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, trace, warn};
use troubadour_core::domain::audio::{AudioError, AudioEnumerator, DeviceId, Result, SampleRate, StreamConfig, ChannelCount};
use troubadour_core::domain::mixer::{ChannelId, MixerEngine};

use super::stream::AudioStream;

/// Configuration for an audio stream
#[derive(Debug, Clone)]
pub struct EngineStreamConfig {
    pub device_id: DeviceId,
    pub channels: u16,
    pub sample_rate: SampleRate,
    pub buffer_size: u32,
}

/// Active audio stream with routing information
pub struct ActiveStream {
    pub audio_stream: AudioStream,
    pub config: EngineStreamConfig,
    /// Channel IDs that receive audio from this stream
    pub target_channels: Vec<ChannelId>,
}

/// Audio engine managing multiple input/output streams
pub struct AudioEngine {
    enumerator: Arc<dyn AudioEnumerator>,
    mixer: Arc<Mutex<MixerEngine>>,
    /// Map of device ID to active input stream
    input_streams: HashMap<DeviceId, ActiveStream>,
    /// Map of device ID to active output stream
    output_streams: HashMap<DeviceId, ActiveStream>,
    /// Target sample rate for the mixer
    target_sample_rate: SampleRate,
    /// Buffer size for streams
    buffer_size: u32,
    /// Whether the engine is running
    running: bool,
}

impl AudioEngine {
    /// Create a new audio engine
    pub fn new(
        enumerator: Arc<dyn AudioEnumerator>,
        mixer: Arc<Mutex<MixerEngine>>,
        target_sample_rate: SampleRate,
        buffer_size: u32,
    ) -> Self {
        Self {
            enumerator,
            mixer,
            input_streams: HashMap::new(),
            output_streams: HashMap::new(),
            target_sample_rate,
            buffer_size,
            running: false,
        }
    }

    /// Start all input streams based on mixer channel device assignments
    ///
    /// This is the key method that implements per-channel audio routing:
    /// 1. Reads input_device from each mixer channel
    /// 2. Groups channels by their assigned device
    /// 3. Creates one AudioInputStream per unique device
    /// 4. Stores mapping of which channels use each stream
    pub fn start_channel_streams(&mut self) -> Result<()> {
        info!("Starting channel streams based on device assignments");

        // Lock mixer to read channel configurations
        let mixer = self.mixer.lock()
            .map_err(|e| AudioError::StreamError(format!("Mixer lock error: {}", e)))?;

        // Group channels by their assigned input device
        let mut device_channels: HashMap<Option<String>, Vec<ChannelId>> = HashMap::new();

        for channel in mixer.channels() {
            // Skip channels without input device and master channel
            let device_id = channel.input_device.clone();
            device_channels
                .entry(device_id)
                .or_insert_with(Vec::new)
                .push(channel.id.clone());

            debug!(
                "Channel '{}' assigned to device: {:?}",
                channel.name,
                channel.input_device
            );
        }

        // Drop mixer lock before creating streams (to avoid deadlock)
        drop(mixer);

        // Get default input device ID for channels with None
        let default_device_id = self.enumerator
            .default_input_device()
            .map(|d| d.id)
            .ok();

        // Create one stream per unique device
        let mut stream_count = 0;
        for (device_id_opt, channel_ids) in device_channels {
            // Resolve None to default device
            let device_id = match device_id_opt {
                Some(id) => DeviceId::new(id),
                None => {
                    match &default_device_id {
                        Some(default_id) => default_id.clone(),
                        None => {
                            warn!("No default input device available, skipping channels: {:?}", channel_ids);
                            continue;
                        }
                    }
                }
            };

            // Get device info to determine supported config
            let device_info = self.enumerator
                .input_devices()
                .map_err(|e| AudioError::StreamError(e.to_string()))?
                .into_iter()
                .find(|d| d.id == device_id)
                .ok_or_else(|| AudioError::DeviceNotFound(format!(
                    "Input device not found: {}",
                    device_id.as_str()
                )))?;

            info!(
                "Creating input stream for device '{}' ({} channels)",
                device_info.name,
                channel_ids.len()
            );

            // Create stream config
            let stream_config = StreamConfig {
                sample_rate: self.target_sample_rate,
                channels: ChannelCount::Stereo, // TODO: Use device's preferred channel count
                format: troubadour_core::domain::audio::SampleFormat::F32,
                buffer_size: self.buffer_size,
            };

            // Create the actual CPAL audio stream
            let audio_stream = AudioStream::create_input_stream(
                &device_id,
                &stream_config,
                self.target_sample_rate,
            ).map_err(|e| {
                error!("Failed to create input stream for {}: {}", device_id.as_str(), e);
                e
            })?;

            // Store stream with target channel mapping
            let active_stream = ActiveStream {
                audio_stream,
                config: EngineStreamConfig {
                    device_id: device_id.clone(),
                    channels: 2, // Stereo
                    sample_rate: self.target_sample_rate,
                    buffer_size: self.buffer_size,
                },
                target_channels: channel_ids.clone(),
            };

            self.input_streams.insert(device_id.clone(), active_stream);
            stream_count += 1;

            debug!(
                "Created stream for device '{}' serving {} channels: {:?}",
                device_id.as_str(),
                channel_ids.len(),
                channel_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>()
            );
        }

        self.running = true;
        info!("Started {} input streams", stream_count);
        Ok(())
    }

    /// Start an input stream from the specified device (legacy method)
    ///
    /// Note: Use start_channel_streams() for automatic per-channel routing
    #[deprecated(note = "Use start_channel_streams() for per-channel device routing")]
    pub fn start_input_stream(&mut self, config: EngineStreamConfig) -> Result<()> {
        info!(
            "Starting input stream: device={}, channels={}, rate={:?}",
            config.device_id.as_str(),
            config.channels,
            config.sample_rate
        );

        // Create stream config
        let stream_config = StreamConfig {
            sample_rate: config.sample_rate,
            channels: ChannelCount::Stereo,
            format: troubadour_core::domain::audio::SampleFormat::F32,
            buffer_size: config.buffer_size,
        };

        // Create the actual CPAL audio stream
        let audio_stream = AudioStream::create_input_stream(
            &config.device_id,
            &stream_config,
            self.target_sample_rate,
        )?;

        let active_stream = ActiveStream {
            audio_stream,
            config: config.clone(),
            target_channels: Vec::new(), // No specific channels for legacy method
        };

        self.input_streams.insert(config.device_id.clone(), active_stream);
        self.running = true;

        Ok(())
    }

    /// Start an output stream to the specified device
    pub fn start_output_stream(&mut self, config: EngineStreamConfig) -> Result<()> {
        info!(
            "Starting output stream: device={}, channels={}, rate={:?}",
            config.device_id.as_str(),
            config.channels,
            config.sample_rate
        );

        // Create stream config
        let stream_config = StreamConfig {
            sample_rate: config.sample_rate,
            channels: ChannelCount::Stereo,
            format: troubadour_core::domain::audio::SampleFormat::F32,
            buffer_size: config.buffer_size,
        };

        // Create the actual CPAL audio stream
        let audio_stream = AudioStream::create_output_stream(
            &config.device_id,
            &stream_config,
            self.target_sample_rate,
        )?;

        let active_stream = ActiveStream {
            audio_stream,
            config: config.clone(),
            target_channels: Vec::new(),
        };

        self.output_streams.insert(config.device_id.clone(), active_stream);

        Ok(())
    }

    /// Stop an input stream
    pub fn stop_input_stream(&mut self, device_id: &DeviceId) -> Result<()> {
        if let Some(_stream) = self.input_streams.remove(device_id) {
            info!("Stopped input stream: {}", device_id.as_str());
            Ok(())
        } else {
            Err(AudioError::StreamError(format!(
                "No active input stream for device {}",
                device_id.as_str()
            )))
        }
    }

    /// Stop an output stream
    pub fn stop_output_stream(&mut self, device_id: &DeviceId) -> Result<()> {
        if let Some(_stream) = self.output_streams.remove(device_id) {
            info!("Stopped output stream: {}", device_id.as_str());
            Ok(())
        } else {
            Err(AudioError::StreamError(format!(
                "No active output stream for device {}",
                device_id.as_str()
            )))
        }
    }

    /// Check if the engine is currently running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get all active input stream device IDs
    pub fn active_input_streams(&self) -> Vec<DeviceId> {
        self.input_streams.keys().cloned().collect()
    }

    /// Get all active output stream device IDs
    pub fn active_output_streams(&self) -> Vec<DeviceId> {
        self.output_streams.keys().cloned().collect()
    }

    /// Process audio through the mixer
    ///
    /// This method routes audio from input streams to their assigned channels:
    /// 1. Reads audio data from each input stream
    /// 2. Distributes audio to all channels using that device
    /// 3. Processes through mixer engine
    /// 4. Updates channel level meters
    pub fn process_audio(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        // Collect audio from all input streams and route to channels
        let mut channel_audio: HashMap<ChannelId, Vec<f32>> = HashMap::new();

        for (device_id, active_stream) in &self.input_streams {
            // Try to receive audio from this stream
            match active_stream.audio_stream.receive() {
                Ok(Some(audio_buffer)) => {
                    debug!(
                        "Received {} samples from device '{}'",
                        audio_buffer.len(),
                        device_id.as_str()
                    );

                    // Distribute this audio to all channels using this device
                    for channel_id in &active_stream.target_channels {
                        // Clone audio buffer for each channel (they may process independently)
                        channel_audio.insert(channel_id.clone(), audio_buffer.clone());
                    }
                }
                Ok(None) => {
                    // No audio data available this frame (non-blocking)
                    trace!("No audio data available from device '{}'", device_id.as_str());
                }
                Err(e) => {
                    error!("Error receiving audio from device '{}': {}", device_id.as_str(), e);
                }
            }
        }

        // Process audio through mixer if we have data
        if !channel_audio.is_empty() {
            let mut mixer = self.mixer.lock()
                .map_err(|e| AudioError::StreamError(format!("Mixer lock error: {}", e)))?;

            // Update channel level meters
            for (channel_id, audio_buffer) in &channel_audio {
                if let Some(channel) = mixer.channel_mut(channel_id) {
                    // Update level meter with RMS of buffer
                    let rms = (audio_buffer.iter().map(|&s| s * s).sum::<f32>() / audio_buffer.len() as f32).sqrt();
                    channel.update_level(rms);
                }
            }

            // Process through mixer (apply routing, gain, effects, etc.)
            let outputs = mixer.process_with_effects(&channel_audio, &mut HashMap::new());

            // TODO: Send outputs to output streams
            let _ = outputs; // Suppress unused warning for now
        }

        Ok(())
    }

    /// Refresh streams when channel device assignments change
    ///
    /// Call this after modifying channel input_device assignments to restart
    /// streams with the new routing configuration.
    pub fn refresh_streams(&mut self) -> Result<()> {
        info!("Refreshing audio streams due to device assignment changes");

        // Stop all existing streams
        self.stop_all_streams()?;

        // Restart with new configuration
        self.start_channel_streams()?;

        Ok(())
    }

    /// Stop all input streams
    fn stop_all_streams(&mut self) -> Result<()> {
        info!("Stopping all input streams");

        let device_ids: Vec<_> = self.input_streams.keys().cloned().collect();
        for device_id in device_ids {
            if let Some(_stream) = self.input_streams.remove(&device_id) {
                debug!("Stopped stream for device '{}'", device_id.as_str());
            }
        }

        self.running = false;
        Ok(())
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        info!("Shutting down audio engine");
        // Stop all streams
        let _ = self.stop_all_streams();

        // Stop output streams too
        let output_ids: Vec<_> = self.output_streams.keys().cloned().collect();
        for id in output_ids {
            let _ = self.stop_output_stream(&id);
        }
    }
}
