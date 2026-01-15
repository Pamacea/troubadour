//! Criterion benchmarks for mixer engine performance
//!
//! Measures:
//! - Channel processing throughput
//! - Gain application performance
//! - Routing matrix lookup speed
//! - Solo/mute logic overhead
//! - Audio mixing scalability

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use troubadour_core::domain::mixer::{
    ChannelId, MixerChannel, MixerEngine, RoutingMatrix, VolumeDecibels,
};

/// Generate test audio buffer
fn generate_buffer(size: usize, value: f32) -> Vec<f32> {
    vec![value; size]
}

/// Benchmark mixer channel creation
fn bench_channel_creation(c: &mut Criterion) {
    c.bench_function("channel_creation", |b| {
        b.iter(|| {
            MixerChannel::new(
                ChannelId::new("test".to_string()),
                "Test Channel".to_string(),
            )
        })
    });
}

/// Benchmark gain application (hot path)
fn bench_gain_application(c: &mut Criterion) {
    let channel = MixerChannel::new(
        ChannelId::new("test".to_string()),
        "Test Channel".to_string(),
    );

    let buffer = generate_buffer(512, 0.5);

    c.bench_function("gain_application_512_samples", |b| {
        b.iter(|| {
            black_box(
                buffer
                    .iter()
                    .map(|&s| channel.apply_gain(s))
                    .collect::<Vec<f32>>(),
            )
        })
    });
}

/// Benchmark routing matrix lookup
fn bench_routing_lookup(c: &mut Criterion) {
    let mut matrix = RoutingMatrix::new();

    // Add routes
    for i in 0..100 {
        let from = ChannelId::new(format!("input_{}", i));
        let to = ChannelId::new(format!("output_{}", i));
        matrix.set_route(&from, &to, true);
    }

    let from = ChannelId::new("input_50".to_string());
    let to = ChannelId::new("output_50".to_string());

    c.bench_function("routing_lookup", |b| {
        b.iter(|| black_box(matrix.is_routed(black_box(&from), black_box(&to))))
    });
}

/// Benchmark mixer engine with varying channel counts
fn bench_mixer_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixer_scaling");

    for channel_count in [2, 4, 8, 16, 32].iter() {
        let mut engine = MixerEngine::new();

        // Setup channels
        for i in 0..*channel_count {
            let input_id = ChannelId::new(format!("input_{}", i));
            let output_id = ChannelId::new(format!("output_{}", i));

            engine.add_channel(MixerChannel::new(
                input_id.clone(),
                format!("Input {}", i),
            ));
            engine.add_channel(MixerChannel::new(
                output_id.clone(),
                format!("Output {}", i),
            ));

            // Route: input_i -> output_i
            engine
                .routing_mut()
                .set_route(&input_id, &output_id, true);
        }

        // Prepare input data
        let mut inputs = HashMap::new();
        for i in 0..*channel_count {
            let input_id = ChannelId::new(format!("input_{}", i));
            inputs.insert(input_id, generate_buffer(512, 0.5));
        }

        group.bench_with_input(BenchmarkId::from_parameter(channel_count), channel_count, |b, _| {
            b.iter(|| black_box(engine.process(black_box(&inputs))))
        });
    }

    group.finish();
}

/// Benchmark mixer with different buffer sizes
fn bench_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_size");

    for buffer_size in [64, 128, 256, 512, 1024, 2048].iter() {
        let mut engine = MixerEngine::new();

        // Setup 8 channels
        for i in 0..8 {
            let input_id = ChannelId::new(format!("input_{}", i));
            let output_id = ChannelId::new(format!("output_{}", i));

            engine.add_channel(MixerChannel::new(
                input_id.clone(),
                format!("Input {}", i),
            ));
            engine.add_channel(MixerChannel::new(
                output_id.clone(),
                format!("Output {}", i),
            ));

            engine
                .routing_mut()
                .set_route(&input_id, &output_id, true);
        }

        let mut inputs = HashMap::new();
        for i in 0..8 {
            let input_id = ChannelId::new(format!("input_{}", i));
            inputs.insert(input_id, generate_buffer(*buffer_size, 0.5));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(buffer_size),
            buffer_size,
            |b, _| {
                b.iter(|| black_box(engine.process(black_box(&inputs))))
            },
        );
    }

    group.finish();
}

/// Benchmark solo mode processing overhead
fn bench_solo_overhead(c: &mut Criterion) {
    c.bench_function("solo_check_overhead", |b| {
        let mut engine = MixerEngine::new();

        // Add 8 channels
        for i in 0..8 {
            let input_id = ChannelId::new(format!("input_{}", i));
            let output_id = ChannelId::new(format!("output_{}", i));

            engine.add_channel(MixerChannel::new(
                input_id.clone(),
                format!("Input {}", i),
            ));
            engine.add_channel(MixerChannel::new(
                output_id.clone(),
                format!("Output {}", i),
            ));

            engine
                .routing_mut()
                .set_route(&input_id, &output_id, true);
        }

        // Enable solo on channel 0
        let solo_id = ChannelId::new("input_0".to_string());
        if let Some(ch) = engine.channel_mut(&solo_id) {
            ch.toggle_solo();
        }

        let mut inputs = HashMap::new();
        for i in 0..8 {
            let input_id = ChannelId::new(format!("input_{}", i));
            inputs.insert(input_id, generate_buffer(512, 0.5));
        }

        b.iter(|| black_box(engine.process(black_box(&inputs))))
    });
}

/// Benchmark volume conversion (decibels <-> amplitude)
fn bench_volume_conversion(c: &mut Criterion) {
    c.bench_function("volume_to_amplitude", |b| {
        let volumes: Vec<VolumeDecibels> = (-60..=6).map(|db| VolumeDecibels::new(db as f32)).collect();

        b.iter(|| {
            black_box(
                volumes
                    .iter()
                    .map(|v| v.to_amplitude())
                    .collect::<Vec<f32>>(),
            )
        })
    });
}

/// Benchmark channel state mutations
fn bench_channel_mutations(c: &mut Criterion) {
    c.bench_function("channel_mutations", |b| {
        b.iter(|| {
            let mut channel = MixerChannel::new(
                ChannelId::new("test".to_string()),
                "Test".to_string(),
            );
            channel.set_volume(-6.0);
            channel.toggle_mute();
            channel.toggle_solo();
            black_box(channel)
        })
    });
}

criterion_group!(
    benches,
    bench_channel_creation,
    bench_gain_application,
    bench_routing_lookup,
    bench_mixer_scaling,
    bench_buffer_sizes,
    bench_solo_overhead,
    bench_volume_conversion,
    bench_channel_mutations,
);

criterion_main!(benches);
