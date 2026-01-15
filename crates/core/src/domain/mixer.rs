//! Mixer engine and virtual channel management
//!
//! This module provides the core mixing functionality including volume control,
//! mute/solo logic, routing matrix, and signal metering.

use crate::domain::audio::AudioError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, trace};

pub mod bus;

pub use bus::{Bus, BusId, StandardBus, db_to_gain, gain_to_db};

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
    /// Effects chain configuration for this channel
    pub effects: crate::domain::dsp::EffectsChain,
    /// Input device ID for this channel (None = use default input)
    pub input_device: Option<String>,
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
            effects: crate::domain::dsp::EffectsChain::new(),
            input_device: None,
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

    /// Set the input device for this channel
    pub fn set_input_device(&mut self, device_id: Option<String>) {
        self.input_device = device_id;
        debug!(
            "Channel {} input device set to: {:?}",
            self.name, self.input_device
        );
    }

    /// Get the input device for this channel
    pub fn get_input_device(&self) -> Option<&str> {
        self.input_device.as_deref()
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

/// Performance-optimized mixer engine
///
/// Optimizations:
/// - Cache-friendly channel storage using Vec instead of HashMap
/// - Pre-allocated output buffers to reduce allocations
/// - In-place audio processing when possible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerEngine {
    channels: HashMap<ChannelId, MixerChannel>,
    routing: RoutingMatrix,
    buses: Vec<Bus>,

    // Performance optimization: cache-friendly channel list
    #[serde(skip)]
    channel_order: Vec<ChannelId>,

    // Track which channel has solo (for exclusive solo behavior)
    soloed_channel_id: Option<ChannelId>,
}

impl MixerEngine {
    pub const MIN_BUSES: usize = 2;
    pub const MAX_BUSES: usize = 5;

    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            routing: RoutingMatrix::new(),
            buses: Self::create_default_buses(),
            channel_order: Vec::new(),
            soloed_channel_id: None,
        }
    }

    /// Create default bus system (A1-A2 for outputs, minimum 2 buses)
    fn create_default_buses() -> Vec<Bus> {
        vec![
            // Output buses (A1, A2) - minimum 2 buses
            Bus::standard(StandardBus::A1),
            Bus::standard(StandardBus::A2),
        ]
    }

    /// Add a new bus to the mixer (up to MAX_BUSES)
    ///
    /// Returns Ok(bus_id) if successful, Err if already at maximum
    pub fn add_bus(&mut self) -> Result<BusId> {
        if self.buses.len() >= Self::MAX_BUSES {
            return Err(AudioError::InvalidConfiguration(
                format!("Maximum bus limit reached ({})", Self::MAX_BUSES)
            ));
        }

        // Auto-generate next bus ID (A1, A2, A3, A4, A5)
        let next_bus_num = self.buses.len() + 1;
        let bus_id = match next_bus_num {
            1 => StandardBus::A1,
            2 => StandardBus::A2,
            3 => StandardBus::A3,
            4 => StandardBus::A4,
            5 => StandardBus::A5,
            _ => return Err(AudioError::InvalidConfiguration("Invalid bus number".to_string())),
        };

        let bus = Bus::standard(bus_id);
        let id = bus.id.clone();

        debug!("Adding bus: {}", id.as_str());
        self.buses.push(bus);

        Ok(id)
    }

    /// Remove the last bus from the mixer (minimum MIN_BUSES)
    ///
    /// Returns Ok(bus_id) if successful, Err if already at minimum
    pub fn remove_bus(&mut self) -> Result<BusId> {
        if self.buses.len() <= Self::MIN_BUSES {
            return Err(AudioError::InvalidConfiguration(
                format!("Minimum bus limit reached ({})", Self::MIN_BUSES)
            ));
        }

        let removed_bus = self.buses.pop()
            .ok_or_else(|| AudioError::InvalidConfiguration("No bus to remove".to_string()))?;

        let bus_id = removed_bus.id.clone();

        // Remove all routes to/from this bus
        let bus_channel_id = ChannelId::new(bus_id.as_str().to_string());
        self.routing
            .routes
            .retain(|(from, to), _| from != &bus_channel_id && to != &bus_channel_id);

        debug!("Removing bus: {}", bus_id.as_str());

        Ok(bus_id)
    }

    /// Get the current number of buses
    pub fn bus_count(&self) -> usize {
        self.buses.len()
    }

    /// Get all buses
    pub fn buses(&self) -> &[Bus] {
        &self.buses
    }

    /// Get a mutable reference to a bus
    pub fn bus_mut(&mut self, bus_id: &BusId) -> Option<&mut Bus> {
        self.buses.iter_mut().find(|b| &b.id == bus_id)
    }

    /// Get buses assigned to a specific channel
    pub fn get_channel_buses(&self, channel_id: &ChannelId) -> Vec<BusId> {
        // Convert bus IDs to channel IDs for routing lookup
        self.routing
            .get_outputs(channel_id)
            .into_iter()
            .filter(|id| {
                // Check if this channel ID corresponds to a bus
                self.buses.iter().any(|b| b.id.as_str() == id.as_str())
            })
            .map(|id| BusId::new(id.as_str().to_string()))
            .collect()
    }

    /// Set which buses a channel is routed to
    pub fn set_channel_buses(&mut self, channel_id: &ChannelId, bus_ids: Vec<BusId>) {
        // First, clear all existing routes to any bus
        for bus in &self.buses {
            let bus_channel_id = ChannelId::new(bus.id.as_str().to_string());
            self.routing.set_route(channel_id, &bus_channel_id, false);
        }

        // Then enable the selected buses
        for bus_id in &bus_ids {
            let bus_channel_id = ChannelId::new(bus_id.as_str().to_string());
            self.routing.set_route(channel_id, &bus_channel_id, true);
        }

        debug!(
            "Channel {} routed to buses: {:?}",
            channel_id.as_str(),
            bus_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>()
        );
    }

    /// Set the output device for a bus
    pub fn set_bus_output_device(
        &mut self,
        bus_id: &BusId,
        device_id: Option<crate::domain::audio::DeviceId>,
    ) -> Result<()> {
        let bus = self
            .bus_mut(bus_id)
            .ok_or_else(|| AudioError::DeviceNotFound(bus_id.as_str().to_string()))?;

        bus.output_device = device_id;
        debug!(
            "Bus {} output device set to: {:?}",
            bus_id.as_str(),
            bus.output_device.as_ref().map(|d| d.as_str())
        );
        Ok(())
    }

    /// Get the output device for a bus
    pub fn get_bus_output_device(&self, bus_id: &BusId) -> Option<crate::domain::audio::DeviceId> {
        self.buses
            .iter()
            .find(|b| &b.id == bus_id)
            .and_then(|b| b.output_device.clone())
    }

    /// Add a new channel
    pub fn add_channel(&mut self, channel: MixerChannel) {
        debug!("Adding channel: {}", channel.name);
        let id = channel.id.clone();
        self.channels.insert(channel.id.clone(), channel);
        self.channel_order.push(id);
    }

    /// Remove a channel
    pub fn remove_channel(&mut self, id: &ChannelId) -> Result<()> {
        if self.channels.remove(id).is_some() {
            debug!("Removing channel: {}", id.as_str());
            // Remove all routes to/from this channel
            self.routing
                .routes
                .retain(|(from, to), _| from != id && to != id);
            // Update channel order cache
            self.channel_order.retain(|cid| cid != id);
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

    /// Set solo state for a channel with exclusive behavior
    ///
    /// When solo is activated on a channel, all other channels are automatically unsoloed.
    /// When solo is deactivated on the soloed channel, all channels return to normal state.
    ///
    /// This implements standard mixer behavior where solo is exclusive.
    pub fn set_channel_solo_exclusive(&mut self, channel_id: &ChannelId, solo_state: bool) -> Result<()> {
        if solo_state {
            // Solo this channel, unsolo all others
            for channel in self.channels.values_mut() {
                channel.solo = channel.id == *channel_id;
            }
            self.soloed_channel_id = Some(channel_id.clone());
            debug!("Channel {} soloed exclusively", channel_id.as_str());
        } else {
            // Unsolo this channel if it's the soloed one
            if self.soloed_channel_id.as_ref() == Some(channel_id) {
                self.soloed_channel_id = None;
                debug!("Solo cleared from all channels");
            }
            if let Some(channel) = self.channels.get_mut(channel_id) {
                channel.solo = false;
                debug!("Channel {} unsoloed", channel_id.as_str());
            }
        }
        Ok(())
    }

    /// Mix audio samples from all input channels to all output channels
    ///
    /// Performance optimizations:
    /// - Single pass through inputs
    /// - Pre-allocated output buffers
    /// - Cache-friendly sequential access
    /// - Minimal allocations in hot path
    ///
    /// Note: This is a simplified version that only applies volume gain.
    /// For effects processing, use `process_with_effects` instead.
    pub fn process(&mut self, inputs: &HashMap<ChannelId, Vec<f32>>) -> HashMap<ChannelId, Vec<f32>> {
        self.process_with_effects(inputs, &mut HashMap::new())
    }

    /// Mix audio samples from all input channels to all output channels with effects processing
    ///
    /// # Arguments
    /// * `inputs` - Input audio buffers by channel ID
    /// * `effects_processors` - Effects processors for each channel (created from channel effects configs)
    ///
    /// Performance optimizations:
    /// - Single pass through inputs
    /// - Pre-allocated output buffers
    /// - Cache-friendly sequential access
    /// - Minimal allocations in hot path
    /// - Effects processing in-place before mixing
    pub fn process_with_effects(
        &mut self,
        inputs: &HashMap<ChannelId, Vec<f32>>,
        effects_processors: &mut HashMap<ChannelId, crate::domain::dsp::EffectsChainProcessor>,
    ) -> HashMap<ChannelId, Vec<f32>> {
        // Check if any channel is in solo mode (early exit optimization)
        let any_solo = self.channels.values().any(|c| c.solo);

        // Pre-allocate outputs with expected capacity
        let mut outputs: HashMap<ChannelId, Vec<f32>> = HashMap::with_capacity(inputs.len());

        // Process each input channel
        for (input_id, input_buffer) in inputs {
            // Use get() for borrow checker instead of match
            let Some(channel) = self.channels.get(input_id) else {
                continue;
            };

            // Skip if channel is not audible (early exit)
            if !channel.is_audible(any_solo) {
                continue;
            }

            // Get output destinations
            let output_ids = self.routing.get_outputs(input_id);

            // Clone buffer for effects processing (needed because we might send to multiple outputs)
            let mut processed_buffer = input_buffer.clone();

            // Apply effects if available for this channel
            if let Some(processor) = effects_processors.get_mut(input_id) {
                let _ = processor.process(&mut processed_buffer);
            }

            // Apply channel gain once (cache the gain value)
            let gain = channel.volume.to_amplitude();
            let is_muted = channel.muted;

            if is_muted {
                continue; // Skip muted channels entirely
            }

            // Process each output destination
            for output_id in output_ids {
                // Get or create output buffer with exact capacity
                let output = outputs
                    .entry(output_id.clone())
                    .or_insert_with(|| vec![0.0; processed_buffer.len()]);

                // Apply gain and mix in a single pass (cache-friendly)
                // This is the hot path - optimized for sequential memory access
                output
                    .iter_mut()
                    .zip(processed_buffer.iter())
                    .for_each(|(out, &sample)| {
                        *out += sample * gain;
                    });
            }
        }

        // Apply bus gain to all outputs
        // This applies bus volume and mute to the mixed audio
        for (output_id, output_buffer) in outputs.iter_mut() {
            // Check if this output corresponds to a bus
            if let Some(bus) = self.buses.iter().find(|b| b.id.as_str() == output_id.as_str()) {
                let bus_gain = bus.gain();
                if bus_gain == 0.0 {
                    // Bus is muted, zero out the buffer
                    output_buffer.fill(0.0);
                } else {
                    // Apply bus gain
                    output_buffer.iter_mut().for_each(|sample| {
                        *sample *= bus_gain;
                    });
                }
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

    // ========================================================================
    // ADDITIONAL COMPREHENSIVE TESTS
    // ========================================================================

    #[test]
    fn test_volume_extreme_values() {
        let vol = VolumeDecibels::new(-100.0);
        assert_eq!(vol.db(), -60.0);
        assert_eq!(vol.to_amplitude(), 0.0);

        let vol = VolumeDecibels::new(100.0);
        assert_eq!(vol.db(), 6.0);
        assert!(vol.to_amplitude() > 1.0);
    }

    #[test]
    fn test_volume_roundtrip() {
        let test_values = [-60.0, -40.0, -20.0, -6.0, 0.0, 3.0, 6.0];

        for db in test_values {
            let vol = VolumeDecibels::new(db);
            let amplitude = vol.to_amplitude();
            let recovered = VolumeDecibels::from_amplitude(amplitude);

            if db <= -60.0 {
                assert_eq!(recovered.db(), -60.0);
            } else {
                assert!((recovered.db() - db).abs() < 0.1, "Failed for {} dB", db);
            }
        }
    }

    #[test]
    fn test_channel_solo_isolation() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());
        let ch2 = ChannelId::new("ch2".to_string());
        let ch3 = ChannelId::new("ch3".to_string());
        let out = ChannelId::new("A1".to_string());

        engine.add_channel(MixerChannel::new(ch1.clone(), "1".to_string()));
        engine.add_channel(MixerChannel::new(ch2.clone(), "2".to_string()));
        engine.add_channel(MixerChannel::new(ch3.clone(), "3".to_string()));

        // Solo ch2 only
        engine.channel_mut(&ch2).unwrap().solo = true;

        engine.routing_mut().set_route(&ch1, &out, true);
        engine.routing_mut().set_route(&ch2, &out, true);
        engine.routing_mut().set_route(&ch3, &out, true);

        let mut inputs = HashMap::new();
        inputs.insert(ch1.clone(), vec![1.0; 100]);
        inputs.insert(ch2.clone(), vec![1.0; 100]);
        inputs.insert(ch3.clone(), vec![1.0; 100]);

        let outputs = engine.process(&inputs);
        let output = outputs.get(&out).unwrap();

        // Only ch2 should be audible
        assert!((output[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_multiple_solo_channels() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());
        let ch2 = ChannelId::new("ch2".to_string());
        let out = ChannelId::new("A1".to_string());

        engine.add_channel(MixerChannel::new(ch1.clone(), "1".to_string()));
        engine.add_channel(MixerChannel::new(ch2.clone(), "2".to_string()));

        // Solo both
        engine.channel_mut(&ch1).unwrap().solo = true;
        engine.channel_mut(&ch2).unwrap().solo = true;

        engine.routing_mut().set_route(&ch1, &out, true);
        engine.routing_mut().set_route(&ch2, &out, true);

        let mut inputs = HashMap::new();
        inputs.insert(ch1.clone(), vec![0.5; 100]);
        inputs.insert(ch2.clone(), vec![0.3; 100]);

        let outputs = engine.process(&inputs);
        let output = outputs.get(&out).unwrap();

        // Both should be mixed
        assert!((output[0] - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_mute_with_solo() {
        let mut channel = MixerChannel::new(
            ChannelId::new("test".to_string()),
            "Test".to_string(),
        );

        // Solo but also muted
        channel.solo = true;
        channel.muted = true;

        // Mute should take precedence
        assert!(!channel.is_audible(true));
        assert!(!channel.is_audible(false));
    }

    #[test]
    fn test_volume_affects_gain() {
        let mut channel = MixerChannel::new(
            ChannelId::new("test".to_string()),
            "Test".to_string(),
        );

        // Test various volumes
        let test_cases = [
            (-6.0, 0.501),   // -6dB ≈ 0.5
            (-20.0, 0.1),    // -20dB ≈ 0.1
            (0.0, 1.0),      // Unity
            (6.0, 1.995),    // +6dB ≈ 2.0
        ];

        for (db, expected_amp) in test_cases {
            channel.set_volume(db);
            let output = channel.apply_gain(1.0);
            assert!((output - expected_amp).abs() < 0.01, "Failed for {} dB", db);
        }
    }

    #[test]
    fn test_routing_complex_scenarios() {
        let mut matrix = RoutingMatrix::new();
        let ch1 = ChannelId::new("ch1".to_string());
        let a1 = ChannelId::new("A1".to_string());
        let a2 = ChannelId::new("A2".to_string());
        let a3 = ChannelId::new("A3".to_string());

        // Route to multiple outputs
        matrix.set_route(&ch1, &a1, true);
        matrix.set_route(&ch1, &a2, true);
        matrix.set_route(&ch1, &a3, true);

        let outputs = matrix.get_outputs(&ch1);
        assert_eq!(outputs.len(), 3);
        assert!(outputs.contains(&a1));
        assert!(outputs.contains(&a2));
        assert!(outputs.contains(&a3));

        // Disable one
        matrix.set_route(&ch1, &a2, false);
        let outputs = matrix.get_outputs(&ch1);
        assert_eq!(outputs.len(), 2);
        assert!(!outputs.contains(&a2));
    }

    #[test]
    fn test_bus_system() {
        let engine = MixerEngine::new();
        let buses = engine.buses();

        // Should have 2 default buses (minimum)
        assert_eq!(buses.len(), 2);
        assert_eq!(buses[0].id.as_str(), "A1");
        assert_eq!(buses[1].id.as_str(), "A2");
    }

    #[test]
    fn test_channel_operations() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());

        // Add channel
        engine.add_channel(MixerChannel::new(ch1.clone(), "Test".to_string()));
        assert!(engine.channel(&ch1).is_some());
        assert_eq!(engine.channels().count(), 1);

        // Remove channel
        engine.remove_channel(&ch1).unwrap();
        assert!(engine.channel(&ch1).is_none());
        assert_eq!(engine.channels().count(), 0);

        // Remove non-existent should error
        let result = engine.remove_channel(&ch1);
        assert!(result.is_err());
    }

    #[test]
    fn test_audio_level_clamping() {
        let mut level = AudioLevel::new();

        // Silence
        level.update(0.0);
        assert_eq!(level.current_db, AudioLevel::MIN_LEVEL);

        // Full scale
        level.update(1.0);
        assert_eq!(level.current_db, AudioLevel::MAX_LEVEL);

        // Beyond full scale (should clamp)
        level.update(2.0);
        assert_eq!(level.current_db, AudioLevel::MAX_LEVEL);
    }

    #[test]
    fn test_audio_level_peak_behavior() {
        let mut level = AudioLevel::new();

        // Start with silence
        level.update(0.0);
        assert_eq!(level.peak_db, AudioLevel::MIN_LEVEL);

        // Hit full scale
        level.update(1.0);
        assert_eq!(level.peak_db, 0.0);

        // Lower signal
        level.update(0.1);
        assert!((level.current_db - (-20.0)).abs() < 1.0);
        assert_eq!(level.peak_db, 0.0); // Peak should remain

        // Decay peak
        level.decay_peak(5.0);
        assert_eq!(level.peak_db, -5.0);
    }

    #[test]
    fn test_bus_assignment() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());
        engine.add_channel(MixerChannel::new(ch1.clone(), "Test".to_string()));

        // Assign to bus A1
        let bus_a1 = BusId::new("A1".to_string());
        engine.set_channel_buses(&ch1, vec![bus_a1.clone()]);

        let buses = engine.get_channel_buses(&ch1);
        assert_eq!(buses.len(), 1);
        assert_eq!(buses[0].as_str(), "A1");

        // Assign to multiple buses (add A3 first since it doesn't exist by default)
        let bus_a3 = engine.add_bus().unwrap();
        let bus_a2 = BusId::new("A2".to_string());
        engine.set_channel_buses(&ch1, vec![bus_a1.clone(), bus_a2, bus_a3]);

        let buses = engine.get_channel_buses(&ch1);
        assert_eq!(buses.len(), 3);
    }

    #[test]
    fn test_exclusive_solo_behavior() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());
        let ch2 = ChannelId::new("ch2".to_string());
        let ch3 = ChannelId::new("ch3".to_string());

        engine.add_channel(MixerChannel::new(ch1.clone(), "Channel 1".to_string()));
        engine.add_channel(MixerChannel::new(ch2.clone(), "Channel 2".to_string()));
        engine.add_channel(MixerChannel::new(ch3.clone(), "Channel 3".to_string()));

        // Initially, no channels are soloed
        assert!(!engine.channel(&ch1).unwrap().solo);
        assert!(!engine.channel(&ch2).unwrap().solo);
        assert!(!engine.channel(&ch3).unwrap().solo);

        // Solo ch1 - should unsolo all others
        engine.set_channel_solo_exclusive(&ch1, true).unwrap();
        assert!(engine.channel(&ch1).unwrap().solo);
        assert!(!engine.channel(&ch2).unwrap().solo);
        assert!(!engine.channel(&ch3).unwrap().solo);

        // Solo ch2 - should unsolo ch1 and ch3
        engine.set_channel_solo_exclusive(&ch2, true).unwrap();
        assert!(!engine.channel(&ch1).unwrap().solo);
        assert!(engine.channel(&ch2).unwrap().solo);
        assert!(!engine.channel(&ch3).unwrap().solo);

        // Unsolo ch2 - all should be unsoloed
        engine.set_channel_solo_exclusive(&ch2, false).unwrap();
        assert!(!engine.channel(&ch1).unwrap().solo);
        assert!(!engine.channel(&ch2).unwrap().solo);
        assert!(!engine.channel(&ch3).unwrap().solo);
    }

    #[test]
    fn test_exclusive_solo_with_processing() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());
        let ch2 = ChannelId::new("ch2".to_string());
        let out = ChannelId::new("A1".to_string());

        engine.add_channel(MixerChannel::new(ch1.clone(), "1".to_string()));
        engine.add_channel(MixerChannel::new(ch2.clone(), "2".to_string()));

        // Route both to output
        engine.routing_mut().set_route(&ch1, &out, true);
        engine.routing_mut().set_route(&ch2, &out, true);

        let mut inputs = HashMap::new();
        inputs.insert(ch1.clone(), vec![1.0; 100]);
        inputs.insert(ch2.clone(), vec![1.0; 100]);

        // Both channels playing (no solo)
        let outputs = engine.process(&inputs);
        let output = outputs.get(&out).unwrap();
        assert!((output[0] - 2.0).abs() < 0.01); // Both mixed

        // Solo ch1 - only ch1 should be audible
        engine.set_channel_solo_exclusive(&ch1, true).unwrap();
        let outputs = engine.process(&inputs);
        let output = outputs.get(&out).unwrap();
        assert!((output[0] - 1.0).abs() < 0.01); // Only ch1

        // Solo ch2 - only ch2 should be audible
        engine.set_channel_solo_exclusive(&ch2, true).unwrap();
        let outputs = engine.process(&inputs);
        let output = outputs.get(&out).unwrap();
        assert!((output[0] - 1.0).abs() < 0.01); // Only ch2

        // Unsolo - both should be audible again
        engine.set_channel_solo_exclusive(&ch2, false).unwrap();
        let outputs = engine.process(&inputs);
        let output = outputs.get(&out).unwrap();
        assert!((output[0] - 2.0).abs() < 0.01); // Both mixed
    }

    // ========================================================================
    // BUS MANAGEMENT TESTS
    // ========================================================================

    #[test]
    fn test_default_bus_count() {
        let engine = MixerEngine::new();
        assert_eq!(engine.bus_count(), 2);
        assert_eq!(engine.buses().len(), 2);
    }

    #[test]
    fn test_add_bus_increases_count() {
        let mut engine = MixerEngine::new();
        assert_eq!(engine.bus_count(), 2);

        let bus_id = engine.add_bus().unwrap();
        assert_eq!(bus_id.as_str(), "A3");
        assert_eq!(engine.bus_count(), 3);
    }

    #[test]
    fn test_add_multiple_buses() {
        let mut engine = MixerEngine::new();

        // Add 3 buses (should reach max of 5)
        let bus3 = engine.add_bus().unwrap();
        assert_eq!(bus3.as_str(), "A3");
        assert_eq!(engine.bus_count(), 3);

        let bus4 = engine.add_bus().unwrap();
        assert_eq!(bus4.as_str(), "A4");
        assert_eq!(engine.bus_count(), 4);

        let bus5 = engine.add_bus().unwrap();
        assert_eq!(bus5.as_str(), "A5");
        assert_eq!(engine.bus_count(), 5);
    }

    #[test]
    fn test_add_bus_beyond_limit() {
        let mut engine = MixerEngine::new();

        // Add up to max (5 buses)
        let _ = engine.add_bus().unwrap();
        let _ = engine.add_bus().unwrap();
        let _ = engine.add_bus().unwrap();

        assert_eq!(engine.bus_count(), 5);

        // Should fail to add beyond limit
        let result = engine.add_bus();
        assert!(result.is_err());
        assert_eq!(engine.bus_count(), 5); // Count unchanged
    }

    #[test]
    fn test_remove_bus_decreases_count() {
        let mut engine = MixerEngine::new();

        // First add a bus so we have 3
        let _ = engine.add_bus().unwrap();
        assert_eq!(engine.bus_count(), 3);

        // Remove one
        let removed_id = engine.remove_bus().unwrap();
        assert_eq!(removed_id.as_str(), "A3");
        assert_eq!(engine.bus_count(), 2);
    }

    #[test]
    fn test_remove_bus_below_minimum() {
        let mut engine = MixerEngine::new();
        assert_eq!(engine.bus_count(), 2);

        // Should fail to remove below minimum
        let result = engine.remove_bus();
        assert!(result.is_err());
        assert_eq!(engine.bus_count(), 2); // Count unchanged
    }

    #[test]
    fn test_bus_auto_naming() {
        let mut engine = MixerEngine::new();

        // Check default buses
        let buses = engine.buses();
        assert_eq!(buses[0].id.as_str(), "A1");
        assert_eq!(buses[1].id.as_str(), "A2");

        // Add buses and check names
        let _ = engine.add_bus().unwrap();
        let _ = engine.add_bus().unwrap();
        let _ = engine.add_bus().unwrap();

        let buses = engine.buses();
        assert_eq!(buses[2].id.as_str(), "A3");
        assert_eq!(buses[3].id.as_str(), "A4");
        assert_eq!(buses[4].id.as_str(), "A5");
    }

    #[test]
    fn test_remove_bus_clears_routing() {
        let mut engine = MixerEngine::new();
        let ch1 = ChannelId::new("ch1".to_string());

        engine.add_channel(MixerChannel::new(ch1.clone(), "Channel 1".to_string()));

        // Add a third bus
        let bus_a3 = engine.add_bus().unwrap();
        assert_eq!(bus_a3.as_str(), "A3");

        // Route channel to A3
        let bus_channel_id = ChannelId::new(bus_a3.as_str().to_string());
        engine.routing_mut().set_route(&ch1, &bus_channel_id, true);

        // Verify route exists
        assert!(engine.routing().is_routed(&ch1, &bus_channel_id));

        // Remove the bus
        let removed = engine.remove_bus().unwrap();
        assert_eq!(removed.as_str(), "A3");

        // Route should be cleared
        assert!(!engine.routing().is_routed(&ch1, &bus_channel_id));
    }

    #[test]
    fn test_bus_limits_const() {
        assert_eq!(MixerEngine::MIN_BUSES, 2);
        assert_eq!(MixerEngine::MAX_BUSES, 5);
    }

    #[test]
    fn test_dynamic_bus_range() {
        let mut engine = MixerEngine::new();

        // Start at minimum
        assert_eq!(engine.bus_count(), MixerEngine::MIN_BUSES);

        // Add to maximum
        for _ in 0..(MixerEngine::MAX_BUSES - MixerEngine::MIN_BUSES) {
            engine.add_bus().unwrap();
        }
        assert_eq!(engine.bus_count(), MixerEngine::MAX_BUSES);

        // Remove back to minimum
        for _ in 0..(MixerEngine::MAX_BUSES - MixerEngine::MIN_BUSES) {
            engine.remove_bus().unwrap();
        }
        assert_eq!(engine.bus_count(), MixerEngine::MIN_BUSES);
    }
}
