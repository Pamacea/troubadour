//! Real-time audio stream processing with resampling support
//!
//! This module provides low-latency audio stream capabilities using CPAL
//! and rubato for transparent resampling.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig as CpalStreamConfig};
use crossbeam::channel::{bounded, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use troubadour_core::domain::audio::{
    AudioError, DeviceId, Result, SampleRate, StreamConfig,
};

/// Audio buffer containing samples
pub type AudioBuffer = Vec<f32>;

/// Ring buffer for lock-free audio data transfer
pub struct RingBuffer {
    buffer: Vec<f32>,
    capacity: usize,
    write_pos: usize,
    read_pos: usize,
}

impl RingBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            capacity,
            write_pos: 0,
            read_pos: 0,
        }
    }

    pub fn write(&mut self, samples: &[f32]) -> Result<usize> {
        let available = self.available_write();
        let to_write = samples.len().min(available);

        for i in 0..to_write {
            self.buffer[(self.write_pos + i) % self.capacity] = samples[i];
        }

        self.write_pos = (self.write_pos + to_write) % self.capacity;
        Ok(to_write)
    }

    pub fn read(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let available = self.available_read();
        let to_read = buffer.len().min(available);

        for i in 0..to_read {
            buffer[i] = self.buffer[(self.read_pos + i) % self.capacity];
        }

        self.read_pos = (self.read_pos + to_read) % self.capacity;
        Ok(to_read)
    }

    pub fn available_write(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.capacity - (self.write_pos - self.read_pos) - 1
        } else {
            self.read_pos - self.write_pos - 1
        }
    }

    pub fn available_read(&self) -> usize {
        if self.read_pos > self.write_pos {
            self.capacity - (self.read_pos - self.write_pos)
        } else {
            self.write_pos - self.read_pos
        }
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.read_pos = 0;
    }
}

/// Simple resampler using linear interpolation
pub struct Resampler {
    channels: usize,
    ratio: f64,
    position: f64,
}

impl Resampler {
    pub fn new(source_rate: u32, target_rate: u32, channels: u16) -> Result<Self> {
        if source_rate == target_rate {
            debug!(
                "Source and target rates match ({}Hz), bypassing resampling",
                source_rate
            );
            return Ok(Self {
                channels: channels as usize,
                ratio: 1.0,
                position: 0.0,
            });
        }

        info!(
            "Creating resampler: {}Hz -> {}Hz, {} channels",
            source_rate, target_rate, channels
        );

        let ratio = target_rate as f64 / source_rate as f64;

        Ok(Self {
            channels: channels as usize,
            ratio,
            position: 0.0,
        })
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<usize> {
        if self.ratio == 1.0 {
            // Bypass - no resampling needed
            let to_copy = input.len().min(output.len());
            output[..to_copy].copy_from_slice(&input[..to_copy]);
            return Ok(to_copy);
        }

        // Linear interpolation resampling
        let input_frames = input.len() / self.channels;
        let max_output_frames = output.len() / self.channels;

        let mut output_frame = 0;

        while self.position < input_frames as f64 && output_frame < max_output_frames {
            let i0 = self.position.floor() as usize;
            let i1 = (i0 + 1).min(input_frames - 1);
            let i0_float = i0 as f64;
            let frac = (self.position - i0_float) as f32;

            for ch in 0..self.channels {
                let idx0 = i0 * self.channels + ch;
                let idx1 = i1 * self.channels + ch;
                let out_idx = output_frame * self.channels + ch;

                if out_idx < output.len() && idx0 < input.len() && idx1 < input.len() {
                    output[out_idx] = input[idx0] + frac * (input[idx1] - input[idx0]);
                }
            }

            output_frame += 1;
            self.position += self.ratio.recip();
        }

        // Wrap position if needed
        if self.position >= input_frames as f64 {
            self.position -= input_frames as f64;
        }

        Ok(output_frame * self.channels)
    }

    pub fn ratio(&self) -> f64 {
        self.ratio
    }
}

/// Audio stream configuration with automatic resampling
pub struct AudioStreamConfig {
    pub device_id: DeviceId,
    pub stream_config: StreamConfig,
    pub target_sample_rate: SampleRate,
}

/// Real-time audio stream with resampling support
pub struct AudioStream {
    _stream: Stream,
    config: StreamConfig,
    sample_sender: Option<Sender<AudioBuffer>>,
    sample_receiver: Option<Receiver<AudioBuffer>>,
    resampler: Option<Arc<Mutex<Resampler>>>,
}

impl AudioStream {
    /// Create an input stream (capture)
    pub fn create_input_stream(
        device_id: &DeviceId,
        config: &StreamConfig,
        target_rate: SampleRate,
    ) -> Result<Self> {
        info!(
            "Creating input stream: device={}, config={:?}",
            device_id.as_str(),
            config
        );

        let host = cpal::default_host();
        #[allow(deprecated)]
        let cpal_device: cpal::Device = host
            .devices()
            .map_err(|e| AudioError::OsError(e.to_string()))?
            .find(|d| d.name().ok().as_deref() == Some(device_id.as_str()))
            .ok_or_else(|| AudioError::DeviceNotFound(device_id.as_str().to_string()))?;

        // Check if resampling is needed
        let needs_resampling = config.sample_rate.hz() != target_rate.hz();
        let resampler = if needs_resampling {
            Some(Arc::new(Mutex::new(Resampler::new(
                config.sample_rate.hz(),
                target_rate.hz(),
                config.channels.count(),
            )?)))
        } else {
            None
        };

        let channels = config.channels.count() as usize;
        let (sender, receiver) = bounded(8);

        let sender_clone = sender.clone();
        let resampler_clone = resampler.clone();

        // Convert to CPAL config
        let cpal_config = CpalStreamConfig {
            channels: config.channels.count(),
            sample_rate: config.sample_rate.hz(),
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size),
        };

        // Build the stream
        let stream = cpal_device
            .build_input_stream(
                &cpal_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buffer = data.to_vec();

                    // Apply resampling if needed
                    if let Some(res) = &resampler_clone {
                        if let Ok(mut resampler) = res.lock() {
                            let mut resampled = vec![
                                0.0;
                                (buffer.len() as f64 * resampler.ratio()) as usize
                                    + channels
                            ];
                            match resampler.process(&buffer, &mut resampled) {
                                Ok(n) => {
                                    buffer.truncate(n);
                                    buffer = resampled;
                                }
                                Err(e) => {
                                    error!("Resampling error: {:?}", e);
                                }
                            }
                        }
                    }

                    let _ = sender_clone.try_send(buffer);
                },
                |err| error!("Input stream error: {}", err),
                None,
            )
            .map_err(|e| AudioError::StreamError(format!("Failed to build stream: {}", e)))?;

        stream.play().map_err(|e| {
            AudioError::StreamError(format!("Failed to start stream: {}", e))
        })?;

        Ok(Self {
            _stream: stream,
            config: config.clone(),
            sample_sender: None,
            sample_receiver: Some(receiver),
            resampler,
        })
    }

    /// Create an output stream (playback)
    pub fn create_output_stream(
        device_id: &DeviceId,
        config: &StreamConfig,
        target_rate: SampleRate,
    ) -> Result<Self> {
        info!(
            "Creating output stream: device={}, config={:?}",
            device_id.as_str(),
            config
        );

        let host = cpal::default_host();
        #[allow(deprecated)]
        let cpal_device: cpal::Device = host
            .devices()
            .map_err(|e| AudioError::OsError(e.to_string()))?
            .find(|d| d.name().ok().as_deref() == Some(device_id.as_str()))
            .ok_or_else(|| AudioError::DeviceNotFound(device_id.as_str().to_string()))?;

        let channels = config.channels.count() as usize;
        let (sender, receiver) = bounded(8);

        let resampler = if config.sample_rate.hz() != target_rate.hz() {
            Some(Arc::new(Mutex::new(Resampler::new(
                target_rate.hz(),
                config.sample_rate.hz(),
                config.channels.count(),
            )?)))
        } else {
            None
        };

        let receiver_clone: Receiver<AudioBuffer> = receiver.clone();
        let resampler_clone = resampler.clone();

        let cpal_config = CpalStreamConfig {
            channels: config.channels.count(),
            sample_rate: config.sample_rate.hz(),
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size),
        };

        let stream = cpal_device
            .build_output_stream(
                &cpal_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Try to get samples from channel
                    match receiver_clone.try_recv() {
                        Ok(mut buffer) => {
                            // Apply resampling if needed
                            if let Some(res) = &resampler_clone {
                                if let Ok(mut resampler) = res.lock() {
                                    let mut resampled = vec![
                                        0.0;
                                        (buffer.len() as f64 * resampler.ratio()) as usize
                                            + channels
                                    ];
                                    match resampler.process(&buffer, &mut resampled) {
                                        Ok(n) => {
                                            buffer.truncate(n);
                                            buffer = resampled;
                                        }
                                        Err(e) => {
                                            error!("Resampling error: {:?}", e);
                                        }
                                    }
                                }
                            }

                            // Copy to output
                            let len = data.len().min(buffer.len());
                            data[..len].copy_from_slice(&buffer[..len]);

                            // Fill remaining with silence
                            if len < data.len() {
                                data[len..].fill(0.0);
                            }
                        }
                        Err(_) => {
                            // No data available, output silence
                            data.fill(0.0);
                        }
                    }
                },
                |err| error!("Output stream error: {}", err),
                None,
            )
            .map_err(|e| AudioError::StreamError(format!("Failed to build stream: {}", e)))?;

        stream.play().map_err(|e| {
            AudioError::StreamError(format!("Failed to start stream: {}", e))
        })?;

        Ok(Self {
            _stream: stream,
            config: config.clone(),
            sample_sender: Some(sender),
            sample_receiver: None,
            resampler,
        })
    }

    /// Receive audio data from input stream
    pub fn receive(&self) -> Result<Option<AudioBuffer>> {
        if let Some(receiver) = &self.sample_receiver {
            Ok(receiver.try_recv().ok())
        } else {
            Err(AudioError::StreamError(
                "Not an input stream".to_string(),
            ))
        }
    }

    /// Send audio data to output stream
    pub fn send(&self, buffer: AudioBuffer) -> Result<()> {
        if let Some(sender) = &self.sample_sender {
            sender.send(buffer).map_err(|_| {
                AudioError::StreamError("Failed to send audio data".to_string())
            })
        } else {
            Err(AudioError::StreamError(
                "Not an output stream".to_string(),
            ))
        }
    }

    /// Get stream configuration
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }

    /// Check if resampling is active
    pub fn is_resampling(&self) -> bool {
        self.resampler.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buffer = RingBuffer::with_capacity(16);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = vec![0.0; 4];

        assert_eq!(buffer.write(&input).unwrap(), 4);
        assert_eq!(buffer.available_read(), 4);
        assert_eq!(buffer.read(&mut output).unwrap(), 4);
        assert_eq!(output, input);
    }

    #[test]
    fn test_ring_buffer_wraparound() {
        let mut buffer = RingBuffer::with_capacity(8);

        // Write 6 samples
        let input1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(buffer.write(&input1).unwrap(), 6);

        // Read 4 samples
        let mut output1 = vec![0.0; 4];
        assert_eq!(buffer.read(&mut output1).unwrap(), 4);
        assert_eq!(output1, vec![1.0, 2.0, 3.0, 4.0]);

        // Write 6 more samples (should wrap around)
        let input2 = vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        assert_eq!(buffer.write(&input2).unwrap(), 5); // Only 5 slots available

        // Read remaining
        let mut output2 = vec![0.0; 10];
        assert_eq!(buffer.read(&mut output2).unwrap(), 7);
        assert_eq!(output2[..7], vec![5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]);
    }

    #[test]
    fn test_resampler_bypass() {
        let mut resampler = Resampler::new(48000, 48000, 2).unwrap();
        assert!((resampler.ratio - 1.0).abs() < 0.01);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = vec![0.0; 4];

        assert_eq!(resampler.process(&input, &mut output).unwrap(), 4);
        assert_eq!(output, input);
    }

    #[test]
    fn test_resampler_conversion() {
        let mut resampler = Resampler::new(44100, 48000, 2).unwrap();
        assert!(resampler.ratio > 1.0);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = vec![0.0; 16];

        let n = resampler.process(&input, &mut output).unwrap();
        assert!(n > 4); // Resampled output should be larger
    }
}
