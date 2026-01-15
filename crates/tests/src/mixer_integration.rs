//! Integration tests for the mixer engine
//!
//! These tests verify the complete audio processing pipeline from inputs to outputs,
//! including complex scenarios like multi-channel mixing, effects, and routing.

use troubadour_core::domain::mixer::{
    MixerEngine, MixerChannel, ChannelId, VolumeDecibels, AudioLevel,
    RoutingMatrix, RouteEntry, Bus, BusId, StandardBus, db_to_gain, gain_to_db
};
use std::collections::HashMap;

fn create_test_channel(id: &str, name: &str) -> MixerChannel {
    MixerChannel::new(ChannelId::new(id.to_string()), name.to_string())
}

fn generate_sine_wave(frequency: f32, sample_rate: u32, duration_ms: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_ms / 1000.0) as usize;
    (0..num_samples)
        .map(|i| {
            2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32
        })
        .map(|phase| phase.sin())
        .collect()
}

fn generate_silence(num_samples: usize) -> Vec<f32> {
    vec![0.0; num_samples]
}

// ============================================================================
// BASIC MIXING TESTS
// ============================================================================

#[test]
fn test_single_channel_passthrough() {
    let mut engine = MixerEngine::new();
    let input_id = ChannelId::new("input".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("input", "Input"));
    engine.routing_mut().set_route(&input_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(input_id.clone(), vec![0.5, 0.3, 0.7]);

    let outputs = engine.process(&inputs);

    assert!(outputs.contains_key(&output_id));
    let output = outputs.get(&output_id).unwrap();
    assert_eq!(output.len(), 3);

    // Unity gain passthrough
    assert!((output[0] - 0.5).abs() < 0.01);
    assert!((output[1] - 0.3).abs() < 0.01);
    assert!((output[2] - 0.7).abs() < 0.01);
}

#[test]
fn test_multiple_channel_mix() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));

    // Route both channels to same output
    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.5, 0.5, 0.5]);
    inputs.insert(ch2_id.clone(), vec![0.3, 0.3, 0.3]);

    let outputs = engine.process(&inputs);

    assert!(outputs.contains_key(&output_id));
    let output = outputs.get(&output_id).unwrap();

    // Should be mixed: 0.5 + 0.3 = 0.8
    assert!((output[0] - 0.8).abs() < 0.01);
    assert!((output[1] - 0.8).abs() < 0.01);
    assert!((output[2] - 0.8).abs() < 0.01);
}

#[test]
fn test_channel_volume_control() {
    let mut engine = MixerEngine::new();
    let input_id = ChannelId::new("input".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("input", "Input"));
    engine.routing_mut().set_route(&input_id, &output_id, true);

    // Set volume to -6 dB (~0.5 amplitude)
    if let Some(channel) = engine.channel_mut(&input_id) {
        channel.set_volume(-6.0);
    }

    let mut inputs = HashMap::new();
    inputs.insert(input_id.clone(), vec![1.0, 1.0, 1.0]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Should be approximately 0.5
    assert!((output[0] - 0.501).abs() < 0.01);
}

#[test]
fn test_muted_channel() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));

    // Mute ch1
    if let Some(channel) = engine.channel_mut(&ch1_id) {
        channel.toggle_mute();
    }

    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![1.0, 1.0, 1.0]);
    inputs.insert(ch2_id.clone(), vec![0.5, 0.5, 0.5]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Should only hear ch2 (0.5), ch1 is muted
    assert!((output[0] - 0.5).abs() < 0.01);
}

// ============================================================================
// SOLO FUNCTIONALITY TESTS
// ============================================================================

#[test]
fn test_single_solo() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));

    // Solo ch1
    if let Some(channel) = engine.channel_mut(&ch1_id) {
        channel.toggle_solo();
    }

    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.5, 0.5, 0.5]);
    inputs.insert(ch2_id.clone(), vec![0.3, 0.3, 0.3]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Should only hear ch1 (0.5)
    assert!((output[0] - 0.5).abs() < 0.01);
}

#[test]
fn test_multiple_solo() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let ch3_id = ChannelId::new("ch3".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));
    engine.add_channel(create_test_channel("ch3", "Channel 3"));

    // Solo ch1 and ch2
    if let Some(channel) = engine.channel_mut(&ch1_id) {
        channel.toggle_solo();
    }
    if let Some(channel) = engine.channel_mut(&ch2_id) {
        channel.toggle_solo();
    }

    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);
    engine.routing_mut().set_route(&ch3_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.5, 0.5, 0.5]);
    inputs.insert(ch2_id.clone(), vec![0.3, 0.3, 0.3]);
    inputs.insert(ch3_id.clone(), vec![0.7, 0.7, 0.7]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Should hear ch1 + ch2 (0.5 + 0.3 = 0.8), but not ch3
    assert!((output[0] - 0.8).abs() < 0.01);
}

#[test]
fn test_solo_with_mute() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));

    // Solo ch1, mute ch2 (should still be silent)
    if let Some(channel) = engine.channel_mut(&ch1_id) {
        channel.toggle_solo();
    }
    if let Some(channel) = engine.channel_mut(&ch2_id) {
        channel.toggle_mute();
    }

    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.5, 0.5, 0.5]);
    inputs.insert(ch2_id.clone(), vec![0.3, 0.3, 0.3]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Should only hear ch1
    assert!((output[0] - 0.5).abs() < 0.01);
}

#[test]
fn test_solo_toggle() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.routing_mut().set_route(&ch1_id, &output_id, true);

    // Initially no solo, should be audible
    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.5]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();
    assert!((output[0] - 0.5).abs() < 0.01);

    // Enable solo on ch1
    if let Some(channel) = engine.channel_mut(&ch1_id) {
        channel.toggle_solo();
    }

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();
    assert!((output[0] - 0.5).abs() < 0.01);

    // Disable solo
    if let Some(channel) = engine.channel_mut(&ch1_id) {
        channel.toggle_solo();
    }

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();
    assert!((output[0] - 0.5).abs() < 0.01);
}

// ============================================================================
// ROUTING MATRIX TESTS
// ============================================================================

#[test]
fn test_multiple_outputs() {
    let mut engine = MixerEngine::new();
    let input_id = ChannelId::new("input".to_string());
    let output1_id = ChannelId::new("A1".to_string());
    let output2_id = ChannelId::new("A2".to_string());

    engine.add_channel(create_test_channel("input", "Input"));

    // Route to both outputs
    engine.routing_mut().set_route(&input_id, &output1_id, true);
    engine.routing_mut().set_route(&input_id, &output2_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(input_id.clone(), vec![0.5, 0.3, 0.7]);

    let outputs = engine.process(&inputs);

    // Should have both outputs
    assert!(outputs.contains_key(&output1_id));
    assert!(outputs.contains_key(&output2_id));

    // Both should have same content
    let output1 = outputs.get(&output1_id).unwrap();
    let output2 = outputs.get(&output2_id).unwrap();

    assert_eq!(output1, output2);
}

#[test]
fn test_selective_routing() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let output1_id = ChannelId::new("A1".to_string());
    let output2_id = ChannelId::new("A2".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));

    // Route ch1 -> A1 only
    engine.routing_mut().set_route(&ch1_id, &output1_id, true);

    // Route ch2 -> A2 only
    engine.routing_mut().set_route(&ch2_id, &output2_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.5, 0.5, 0.5]);
    inputs.insert(ch2_id.clone(), vec![0.3, 0.3, 0.3]);

    let outputs = engine.process(&inputs);

    // A1 should only have ch1
    assert!(outputs.contains_key(&output1_id));
    let output1 = outputs.get(&output1_id).unwrap();
    assert!((output1[0] - 0.5).abs() < 0.01);

    // A2 should only have ch2
    assert!(outputs.contains_key(&output2_id));
    let output2 = outputs.get(&output2_id).unwrap();
    assert!((output2[0] - 0.3).abs() < 0.01);
}

#[test]
fn test_routing_disabled() {
    let mut engine = MixerEngine::new();
    let input_id = ChannelId::new("input".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("input", "Input"));

    // Enable then disable routing
    engine.routing_mut().set_route(&input_id, &output_id, true);
    engine.routing_mut().set_route(&input_id, &output_id, false);

    let mut inputs = HashMap::new();
    inputs.insert(input_id.clone(), vec![0.5, 0.3, 0.7]);

    let outputs = engine.process(&inputs);

    // Should not have output (routing disabled)
    assert!(!outputs.contains_key(&output_id));
}

#[test]
fn test_multiple_inputs_to_same_output() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let ch3_id = ChannelId::new("ch3".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));
    engine.add_channel(create_test_channel("ch3", "Channel 3"));

    // Route all to same output
    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);
    engine.routing_mut().set_route(&ch3_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![0.3, 0.3, 0.3]);
    inputs.insert(ch2_id.clone(), vec![0.2, 0.2, 0.2]);
    inputs.insert(ch3_id.clone(), vec![0.5, 0.5, 0.5]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Should be mixed: 0.3 + 0.2 + 0.5 = 1.0
    assert!((output[0] - 1.0).abs() < 0.01);
}

// ============================================================================
// BUS ROUTING TESTS
// ============================================================================

#[test]
fn test_bus_assignment() {
    let mut engine = MixerEngine::new();
    let channel_id = ChannelId::new("ch1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));

    // Assign channel to bus A1
    engine.set_channel_buses(&channel_id, vec![BusId::new("A1".to_string())]);

    let buses = engine.get_channel_buses(&channel_id);
    assert_eq!(buses.len(), 1);
    assert_eq!(buses[0].as_str(), "A1");
}

#[test]
fn test_multiple_bus_assignment() {
    let mut engine = MixerEngine::new();
    let channel_id = ChannelId::new("ch1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));

    // Add A3 bus since it doesn't exist by default anymore
    let _ = engine.add_bus().unwrap();

    // Assign channel to multiple buses
    engine.set_channel_buses(&channel_id, vec![
        BusId::new("A1".to_string()),
        BusId::new("A2".to_string()),
        BusId::new("A3".to_string()),
    ]);

    let buses = engine.get_channel_buses(&channel_id);
    assert_eq!(buses.len(), 3);
}

#[test]
fn test_bus_output_device() {
    let mut engine = MixerEngine::new();
    let bus_id = BusId::new("A1".to_string());

    // Set output device for bus
    let device_id = troubadour_core::domain::audio::DeviceId::new("test_device".to_string());
    engine.set_bus_output_device(&bus_id, Some(device_id.clone())).unwrap();

    // Retrieve output device
    let retrieved = engine.get_bus_output_device(&bus_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().as_str(), "test_device");
}

#[test]
fn test_bus_gain() {
    let bus = Bus::standard(StandardBus::A1);

    // Test unity gain
    assert!((bus.gain() - 1.0).abs() < 0.01);

    // Test with volume
    let mut bus = Bus::standard(StandardBus::A1);
    bus.volume_db = -6.0;
    assert!((bus.gain() - 0.501).abs() < 0.01);
}

#[test]
fn test_bus_mute() {
    let mut bus = Bus::standard(StandardBus::A1);
    bus.volume_db = 6.0;

    // Unmuted should have gain
    assert!(bus.gain() > 1.0);

    // Muted should have zero gain
    bus.muted = true;
    assert_eq!(bus.gain(), 0.0);
}

// ============================================================================
// DB <-> GAIN CONVERSION TESTS
// ============================================================================

#[test]
fn test_db_to_gain_conversions() {
    // Test key values
    assert!((db_to_gain(0.0) - 1.0).abs() < 0.001);
    assert!((db_to_gain(-6.0) - 0.501).abs() < 0.01);
    assert!((db_to_gain(-20.0) - 0.1).abs() < 0.01);
    assert_eq!(db_to_gain(-60.0), 0.0);

    // Test positive gain
    assert!((db_to_gain(6.0) - 1.995).abs() < 0.01);
}

#[test]
fn test_gain_to_db_conversions() {
    assert!((gain_to_db(1.0) - 0.0).abs() < 0.001);
    assert!((gain_to_db(0.5) - (-6.02)).abs() < 0.1);
    assert!((gain_to_db(0.1) - (-20.0)).abs() < 0.1);
    assert_eq!(gain_to_db(0.0), -60.0);
}

#[test]
fn test_db_gain_roundtrip() {
    let test_values = vec![-60.0, -40.0, -20.0, -6.0, 0.0, 3.0, 6.0];

    for db in test_values {
        let gain = db_to_gain(db);
        let recovered = gain_to_db(gain);

        if db <= -60.0 {
            assert_eq!(recovered, -60.0);
        } else {
            assert!((recovered - db).abs() < 0.1, "Failed for {} dB", db);
        }
    }
}

// ============================================================================
// AUDIO LEVEL METERING TESTS
// ============================================================================

#[test]
fn test_audio_level_metering() {
    let mut level = AudioLevel::new();

    // Test silence
    level.update(0.0);
    assert_eq!(level.current_db, AudioLevel::MIN_LEVEL);

    // Test full scale
    level.update(1.0);
    assert_eq!(level.current_db, 0.0);
    assert_eq!(level.peak_db, 0.0);

    // Test half scale
    level.update(0.5);
    assert!((level.current_db - (-6.02)).abs() < 0.1);
    assert_eq!(level.peak_db, 0.0); // Peak should remain at 0 dB
}

#[test]
fn test_peak_decay() {
    let mut level = AudioLevel::new();

    // Hit full scale
    level.update(1.0);
    assert_eq!(level.peak_db, 0.0);

    // Decay
    level.decay_peak(3.0);
    assert_eq!(level.peak_db, -3.0);

    // Decay more
    level.decay_peak(10.0);
    assert_eq!(level.peak_db, -13.0);

    // Should clamp to minimum
    level.decay_peak(100.0);
    assert_eq!(level.peak_db, AudioLevel::MIN_LEVEL);
}

// ============================================================================
// COMPLEX SCENARIOS
// ============================================================================

#[test]
fn test_real_world_scenario() {
    // Simulate a real-world scenario: microphones, music, system audio
    let mut engine = MixerEngine::new();

    let mic1_id = ChannelId::new("mic1".to_string());
    let mic2_id = ChannelId::new("mic2".to_string());
    let music_id = ChannelId::new("music".to_string());
    let system_id = ChannelId::new("system".to_string());
    let output_id = ChannelId::new("A1".to_string());

    // Add channels
    engine.add_channel(create_test_channel("mic1", "Microphone 1"));
    engine.add_channel(create_test_channel("mic2", "Microphone 2"));
    engine.add_channel(create_test_channel("music", "Music"));
    engine.add_channel(create_test_channel("system", "System Audio"));

    // Set levels
    if let Some(ch) = engine.channel_mut(&mic1_id) {
        ch.set_volume(0.0); // Unity
    }
    if let Some(ch) = engine.channel_mut(&mic2_id) {
        ch.set_volume(-3.0); // Slightly lower
    }
    if let Some(ch) = engine.channel_mut(&music_id) {
        ch.set_volume(-6.0); // Background music
    }
    if let Some(ch) = engine.channel_mut(&system_id) {
        ch.set_volume(-12.0); // System audio quieter
    }

    // Route all to output
    engine.routing_mut().set_route(&mic1_id, &output_id, true);
    engine.routing_mut().set_route(&mic2_id, &output_id, true);
    engine.routing_mut().set_route(&music_id, &output_id, true);
    engine.routing_mut().set_route(&system_id, &output_id, true);

    // Create inputs
    let mut inputs = HashMap::new();
    inputs.insert(mic1_id.clone(), vec![0.5, 0.5, 0.5]);
    inputs.insert(mic2_id.clone(), vec![0.4, 0.4, 0.4]);
    inputs.insert(music_id.clone(), vec![0.3, 0.3, 0.3]);
    inputs.insert(system_id.clone(), vec![0.2, 0.2, 0.2]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // Calculate expected output with all gains applied
    // mic1: 0.5 * 1.0 = 0.5
    // mic2: 0.4 * 0.708 (-3dB) ≈ 0.283
    // music: 0.3 * 0.501 (-6dB) ≈ 0.150
    // system: 0.2 * 0.251 (-12dB) ≈ 0.050
    // Total: ~0.983

    let expected = 0.5 + 0.283 + 0.150 + 0.050;
    assert!((output[0] - expected).abs() < 0.01);
}

#[test]
fn test_all_channels_muted() {
    let mut engine = MixerEngine::new();
    let ch1_id = ChannelId::new("ch1".to_string());
    let ch2_id = ChannelId::new("ch2".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));
    engine.add_channel(create_test_channel("ch2", "Channel 2"));

    // Mute all
    if let Some(ch) = engine.channel_mut(&ch1_id) {
        ch.toggle_mute();
    }
    if let Some(ch) = engine.channel_mut(&ch2_id) {
        ch.toggle_mute();
    }

    engine.routing_mut().set_route(&ch1_id, &output_id, true);
    engine.routing_mut().set_route(&ch2_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(ch1_id.clone(), vec![1.0, 1.0, 1.0]);
    inputs.insert(ch2_id.clone(), vec![1.0, 1.0, 1.0]);

    let outputs = engine.process(&inputs);

    // Should have no output (or empty)
    assert!(!outputs.contains_key(&output_id) || outputs.get(&output_id).unwrap().is_empty());
}

#[test]
fn test_empty_inputs() {
    let mut engine = MixerEngine::new();
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("ch1", "Channel 1"));

    let inputs = HashMap::new();
    let outputs = engine.process(&inputs);

    // Should have no output
    assert!(!outputs.contains_key(&output_id) || outputs.get(&output_id).unwrap().is_empty());
}

#[test]
fn test_zero_input_signal() {
    let mut engine = MixerEngine::new();
    let input_id = ChannelId::new("input".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(create_test_channel("input", "Input"));
    engine.routing_mut().set_route(&input_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(input_id.clone(), vec![0.0, 0.0, 0.0]);

    let outputs = engine.process(&inputs);
    let output = outputs.get(&output_id).unwrap();

    // All zeros should produce zeros
    assert!(output.iter().all(|&s| s == 0.0));
}
