//! Bus (output) management for the mixer
//!
//! Buses represent output destinations that can be assigned to physical devices.
//! Multiple inputs can be routed to each bus, creating separate mixes.

use crate::domain::audio::DeviceId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a bus
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BusId(String);

impl BusId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BusId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Predefined bus identifiers (like Voicemeeter's A1, A2, A3...)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardBus {
    A1,
    A2,
    A3,
    A4,
    A5,
}

impl StandardBus {
    pub fn to_id(self) -> BusId {
        BusId::new(format!("{:?}", self))
    }
}

/// Audio bus (output mix) with processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bus {
    pub id: BusId,
    pub name: String,
    pub output_device: Option<DeviceId>,
    pub volume_db: f32,
    pub muted: bool,
}

impl Bus {
    /// Create a new bus
    pub fn new(id: BusId, name: String) -> Self {
        Self {
            id,
            name,
            output_device: None,
            volume_db: 0.0,
            muted: false,
        }
    }

    /// Create a standard bus (A1, A2, A3...)
    pub fn standard(bus: StandardBus) -> Self {
        let name = format!("{:?}", bus);
        Self::new(bus.to_id(), name)
    }

    /// Set the output device for this bus
    pub fn with_output_device(mut self, device_id: DeviceId) -> Self {
        self.output_device = Some(device_id);
        self
    }

    /// Get the current gain (linear scale)
    pub fn gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            db_to_gain(self.volume_db)
        }
    }

    /// Set volume in decibels (clamped to -60..+6 dB range)
    pub fn set_volume(&mut self, db: f32) {
        self.volume_db = db.clamp(-60.0, 6.0);
    }

    /// Toggle mute state, returning new state
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }

    /// Set mute state directly
    pub fn set_mute(&mut self, muted: bool) {
        self.muted = muted;
    }
}

/// Convert decibels to linear gain
pub fn db_to_gain(db: f32) -> f32 {
    if db <= -60.0 {
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}

/// Convert linear gain to decibels
pub fn gain_to_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        -60.0
    } else {
        20.0 * gain.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_to_gain() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 0.001);
        assert!((db_to_gain(-6.0) - 0.501).abs() < 0.01);
        assert_eq!(db_to_gain(-60.0), 0.0);
    }

    #[test]
    fn test_gain_to_db() {
        assert!((gain_to_db(1.0) - 0.0).abs() < 0.001);
        assert!((gain_to_db(0.5) - (-6.02)).abs() < 0.1);
        assert_eq!(gain_to_db(0.0), -60.0);
    }

    #[test]
    fn test_standard_bus() {
        let bus = Bus::standard(StandardBus::A1);
        assert_eq!(bus.name, "A1");
        assert_eq!(bus.id.as_str(), "A1");
    }
}
