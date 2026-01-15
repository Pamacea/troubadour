//! Mixer engine and virtual channel management
//!
//! This module provides the core mixing functionality including volume control,
//! mute/solo logic, routing matrix, and signal metering.

use crate::domain::audio::AudioError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

pub type Result<T> = std::result::Result<T, AudioError>;

/// Unique identifier for a mixer channel
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(String);

impl ChannelId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Volume level in decibels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolumeDecibels(f32);

impl VolumeDecibels {
    pub const MIN_GAIN: f32 = -60.0; // -60 dB (effectively silent)
    pub const UNITY_GAIN: f32 = 0.0;  // 0 dB (no change)
    pub const MAX_GAIN: f32 = 6.0;    // +6 dB (200% amplitude)

    pub fn new(db: f32) -> Self {
        Self(db.clamp(Self::MIN_GAIN, Self::MAX_GAIN))
    }

    pub fn db(&self) -> f32 {
        self.0
    }

    /// Convert decibels to linear amplitude factor
    pub fn to_amplitude(&self) -> f32 {
        if self.0 <= Self::MIN_GAIN {
            0.0
        } else {
            10.0_f32.powf(self.0 / 20.0)
        }
    }

    /// Create from linear amplitude factor
    pub fn from_amplitude(amp: f32) -> Self {
        let db = if amp <= 0.0 {
            Self::MIN_GAIN
        } else {
            20.0 * amp.log10()
        };
        Self::new(db)
    }
}

/// Audio level meter in decibels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioLevel {
    pub current_db: f32,
    pub peak_db: f32,
}

impl AudioLevel {
    pub const MIN_LEVEL: f32 = -60.0;
    pub const MAX_LEVEL: f32 = 0.0;

    pub fn new() -> Self {
        Self {
            current_db: Self::MIN_LEVEL,
            peak_db: Self::MIN_LEVEL,
        }
    }

    /// Update level with new sample value
    pub fn update(&mut self, sample: f32) {
        let level = if sample.abs() > 0.0 {
            let db = 20.0 * sample.abs().log10();
            db.clamp(Self::MIN_LEVEL, Self::MAX_LEVEL)
        } else {
            Self::MIN_LEVEL
        };

        self.current_db = level;
        self.peak_db = self.peak_db.max(level);
    }

    /// Decay peak level (call periodically)
    pub fn decay_peak(&mut self, amount: f32) {
        self.peak_db = (self.peak_db - amount).max(Self::MIN_LEVEL);
    }
}

impl Default for AudioLevel {
    fn default() -> Self {
        Self::new()
    }
}

/// Virtual audio channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerChannel {
    pub id: ChannelId,
    pub name: String,
    pub volume: VolumeDecibels,
    pub muted: bool,
    pub solo: bool,
    pub level: AudioLevel,
}

impl MixerChannel {
    pub fn new(id: ChannelId, name: String) -> Self {
        Self {
            id,
            name,
            volume: VolumeDecibels::new(VolumeDecibels::UNITY_GAIN),
            muted: false,
            solo: false,
            level: AudioLevel::new(),
        }
    }

    /// Apply gain to audio sample
    pub fn apply_gain(&self, sample: f32) -> f32 {
        if self.muted {
            0.0
        } else {
            sample * self.volume.to_amplitude()
        }
    }

    /// Check if channel is audible (not muted and not isolated by solo)
    pub fn is_audible(&self, any_solo: bool) -> bool {
        if self.muted {
            return false;
        }
        if any_solo && !self.solo {
            return false;
        }
        true
    }

    /// Set volume in decibels
    pub fn set_volume(&mut self, db: f32) {
        self.volume = VolumeDecibels::new(db);
        trace!("Channel {} volume set to {} dB", self.name, db);
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        debug!("Channel {} muted: {}", self.name, self.muted);
        self.muted
    }

    /// Toggle solo state
    pub fn toggle_solo(&mut self) -> bool {
        self.solo = !self.solo;
        debug!("Channel {} solo: {}", self.name, self.solo);
        self.solo
    }

    /// Update level meter
    pub fn update_level(&mut self, sample: f32) {
        self.level.update(sample);
    }
}

/// Routing matrix entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub from: ChannelId,
    pub to: ChannelId,
    pub enabled: bool,
}

impl RouteEntry {
    pub fn new(from: ChannelId, to: ChannelId) -> Self {
        Self {
            from,
            to,
            enabled: true,
        }
    }
}

/// Routing matrix for connecting inputs to outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMatrix {
    routes: HashMap<(ChannelId, ChannelId), bool>,
}

impl RoutingMatrix {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Enable or disable a route
    pub fn set_route(&mut self, from: &ChannelId, to: &ChannelId, enabled: bool) {
        self.routes.insert((from.clone(), to.clone()), enabled);
        trace!(
            "Route {} -> {} set to {}",
            from.as_str(),
            to.as_str(),
            enabled
        );
    }

    /// Check if a route is enabled
    pub fn is_routed(&self, from: &ChannelId, to: &ChannelId) -> bool {
        *self.routes.get(&(from.clone(), to.clone())).unwrap_or(&false)
    }

    /// Get all enabled routes from a channel
    pub fn get_outputs(&self, from: &ChannelId) -> Vec<ChannelId> {
        self.routes
            .iter()
            .filter(|((src, _), enabled)| src == from && **enabled)
            .map(|((_, dst), _)| dst.clone())
            .collect()
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
    }
}

impl Default for RoutingMatrix {
    fn default() -> Self {
        Self::new()
    }
}

/// Mixer engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerEngine {
    channels: HashMap<ChannelId, MixerChannel>,
    routing: RoutingMatrix,
}

impl MixerEngine {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            routing: RoutingMatrix::new(),
        }
    }

    /// Add a new channel
    pub fn add_channel(&mut self, channel: MixerChannel) {
        debug!("Adding channel: {}", channel.name);
        self.channels.insert(channel.id.clone(), channel);
    }

    /// Remove a channel
    pub fn remove_channel(&mut self, id: &ChannelId) -> Result<()> {
        if self.channels.remove(id).is_some() {
            debug!("Removing channel: {}", id.as_str());
            // Remove all routes to/from this channel
            self.routing
                .routes
                .retain(|(from, to), _| from != id && to != id);
            Ok(())
        } else {
            Err(AudioError::DeviceNotFound(id.as_str().to_string()))
        }
    }

    /// Get a mutable reference to a channel
    pub fn channel_mut(&mut self, id: &ChannelId) -> Option<&mut MixerChannel> {
        self.channels.get_mut(id)
    }

    /// Get a reference to a channel
    pub fn channel(&self, id: &ChannelId) -> Option<&MixerChannel> {
        self.channels.get(id)
    }

    /// Get all channels
    pub fn channels(&self) -> impl Iterator<Item = &MixerChannel> {
        self.channels.values()
    }

    /// Get routing matrix reference
    pub fn routing(&self) -> &RoutingMatrix {
        &self.routing
    }

    /// Get routing matrix mutable reference
    pub fn routing_mut(&mut self) -> &mut RoutingMatrix {
        &mut self.routing
    }

    /// Mix audio samples from all input channels to all output channels
    pub fn process(&mut self, inputs: &HashMap<ChannelId, Vec<f32>>) -> HashMap<ChannelId, Vec<f32>> {
        // Check if any channel is in solo mode
        let any_solo = self.channels.values().any(|c| c.solo);

        let mut outputs: HashMap<ChannelId, Vec<f32>> = HashMap::new();

        // Process each input channel
        for (input_id, input_buffer) in inputs {
            let channel = match self.channels.get(input_id) {
                Some(ch) => ch,
                None => continue,
            };

            // Skip if channel is not audible
            if !channel.is_audible(any_solo) {
                continue;
            }

            // Get output destinations
            let output_ids = self.routing.get_outputs(input_id);

            for output_id in output_ids {
                // Apply channel gain
                let processed: Vec<f32> = input_buffer
                    .iter()
                    .map(|&sample| {
                        
                        // Update level meter (would need &mut self, simplified here)
                        channel.apply_gain(sample)
                    })
                    .collect();

                // Mix into output buffer
                outputs
                    .entry(output_id.clone())
                    .or_insert_with(|| vec![0.0; processed.len()])
                    .iter_mut()
                    .zip(processed.iter())
                    .for_each(|(out, sample)| {
                        *out += sample;
                    });
            }
        }

        outputs
    }

    /// Clear all level meters (call periodically)
    pub fn decay_meters(&mut self, amount: f32) {
        for channel in self.channels.values_mut() {
            channel.level.decay_peak(amount);
        }
    }
}

impl Default for MixerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_decibels() {
        let vol = VolumeDecibels::new(0.0);
        assert_eq!(vol.db(), 0.0);
        assert!((vol.to_amplitude() - 1.0).abs() < 0.01);

        let vol = VolumeDecibels::new(-6.0);
        assert!((vol.to_amplitude() - 0.501).abs() < 0.01);

        let vol = VolumeDecibels::new(6.0);
        assert!((vol.to_amplitude() - 1.995).abs() < 0.01);
    }

    #[test]
    fn test_volume_clamping() {
        let vol = VolumeDecibels::new(-100.0);
        assert_eq!(vol.db(), VolumeDecibels::MIN_GAIN);

        let vol = VolumeDecibels::new(20.0);
        assert_eq!(vol.db(), VolumeDecibels::MAX_GAIN);
    }

    #[test]
    fn test_mixer_channel() {
        let id = ChannelId::new("test".to_string());
        let mut channel = MixerChannel::new(id.clone(), "Test".to_string());

        assert_eq!(channel.volume.db(), 0.0);
        assert!(!channel.muted);
        assert!(!channel.solo);

        channel.set_volume(-6.0);
        assert_eq!(channel.volume.db(), -6.0);

        assert!(channel.toggle_mute());
        assert!(channel.muted);

        assert!(!channel.toggle_mute());
        assert!(!channel.muted);
    }

    #[test]
    fn test_channel_gain() {
        let mut channel = MixerChannel::new(
            ChannelId::new("test".to_string()),
            "Test".to_string(),
        );

        // Unity gain
        assert!((channel.apply_gain(1.0) - 1.0).abs() < 0.01);

        // -6 dB gain (~0.5 amplitude)
        channel.set_volume(-6.0);
        assert!((channel.apply_gain(1.0) - 0.501).abs() < 0.01);

        // Muted
        channel.toggle_mute();
        assert_eq!(channel.apply_gain(1.0), 0.0);
    }

    #[test]
    fn test_channel_audibility() {
        let mut channel = MixerChannel::new(
            ChannelId::new("test".to_string()),
            "Test".to_string(),
        );

        // Normal state, no solo elsewhere
        assert!(channel.is_audible(false));
        assert!(!channel.is_audible(true)); // Not audible when solo is active elsewhere but this is not solo

        // Muted
        channel.toggle_mute();
        assert!(!channel.is_audible(false));
        assert!(!channel.is_audible(true));

        // Unmuted, solo active
        channel.toggle_mute();
        channel.toggle_solo();
        assert!(channel.is_audible(false));
        assert!(channel.is_audible(true)); // Audible because THIS channel has solo

        // Another channel is solo, this one is not
        // Turn off solo, now another channel has solo
        channel.toggle_solo();
        assert!(channel.is_audible(false));
        assert!(!channel.is_audible(true)); // Not audible when another channel is solo
    }

    #[test]
    fn test_routing_matrix() {
        let mut matrix = RoutingMatrix::new();
        let ch1 = ChannelId::new("ch1".to_string());
        let ch2 = ChannelId::new("ch2".to_string());

        assert!(!matrix.is_routed(&ch1, &ch2));

        matrix.set_route(&ch1, &ch2, true);
        assert!(matrix.is_routed(&ch1, &ch2));

        let outputs = matrix.get_outputs(&ch1);
        assert_eq!(outputs, vec![ch2.clone()]);

        matrix.set_route(&ch1, &ch2, false);
        assert!(!matrix.is_routed(&ch1, &ch2));
    }

    #[test]
    fn test_mixer_engine() {
        let mut engine = MixerEngine::new();

        let ch1 = ChannelId::new("ch1".to_string());
        let ch2 = ChannelId::new("ch2".to_string());

        engine.add_channel(MixerChannel::new(ch1.clone(), "Channel 1".to_string()));
        engine.add_channel(MixerChannel::new(ch2.clone(), "Channel 2".to_string()));

        // Set up routing: ch1 -> ch2
        engine.routing_mut().set_route(&ch1, &ch2, true);

        assert!(engine.channel(&ch1).is_some());
        assert!(engine.channel(&ch2).is_some());

        // Test mixing
        let mut inputs = HashMap::new();
        inputs.insert(ch1.clone(), vec![1.0, 0.5, 0.25]);

        let outputs = engine.process(&inputs);

        assert!(outputs.contains_key(&ch2));
        let output = outputs.get(&ch2).unwrap();
        assert_eq!(output.len(), 3);
        assert!((output[0] - 1.0).abs() < 0.01); // Unity gain
    }

    #[test]
    fn test_audio_level() {
        let mut level = AudioLevel::new();

        level.update(1.0);
        assert_eq!(level.current_db, 0.0);
        assert_eq!(level.peak_db, 0.0);

        level.update(0.5);
        assert!((level.current_db - (-6.02)).abs() < 0.1);
        assert_eq!(level.peak_db, 0.0); // Peak remains

        level.decay_peak(3.0);
        assert_eq!(level.peak_db, -3.0);
    }
}
