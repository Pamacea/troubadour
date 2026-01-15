// Performance benchmarks for the mixer engine
//
// Run with: cargo bench --bench mixer_bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use troubadour_core::domain::mixer::*;
use std::collections::HashMap;

fn bench_volume_to_amplitude(c: &mut Criterion) {
    let mut group = c.benchmark_group("volume_conversion");

    for db in [-60.0, -40.0, -20.0, -6.0, 0.0, 6.0].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(db), db, |b, &db| {
            let vol = VolumeDecibels::new(db);
            b.iter(|| {
                black_box(vol.to_amplitude());
            });
        });
    }

    group.finish();
}

fn bench_channel_apply_gain(c: &mut Criterion) {
    let channel = MixerChannel::new(
        ChannelId::new("test".to_string()),
        "Test".to_string(),
    );

    let samples: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();

    c.bench_function("channel_apply_gain_1000_samples", |b| {
        b.iter(|| {
            let result: Vec<f32> = samples.iter()
                .map(|&s| black_box(channel.apply_gain(s)))
                .collect();
            black_box(result);
        });
    });
}

fn bench_mixer_process_single_channel(c: &mut Criterion) {
    let mut engine = MixerEngine::new();
    let channel_id = ChannelId::new("ch1".to_string());
    let output_id = ChannelId::new("A1".to_string());

    engine.add_channel(MixerChannel::new(
        channel_id.clone(),
        "Channel 1".to_string(),
    ));
    engine.routing_mut().set_route(&channel_id, &output_id, true);

    let mut inputs = HashMap::new();
    inputs.insert(channel_id.clone(), vec![0.5; 512]);

    c.bench_function("mixer_process_single_channel_512_samples", |b| {
        b.iter(|| {
            black_box(engine.process(black_box(&inputs)));
        });
    });
}

fn bench_mixer_process_multiple_channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixer_process_multiple");

    for num_channels in [2, 4, 8, 16].iter() {
        let mut engine = MixerEngine::new();
        let output_id = ChannelId::new("A1".to_string());

        let mut inputs = HashMap::new();

        for i in 0..*num_channels {
            let channel_id = ChannelId::new(format!("ch{}", i));
            engine.add_channel(MixerChannel::new(
                channel_id.clone(),
                format!("Channel {}", i),
            ));
            engine.routing_mut().set_route(&channel_id, &output_id, true);
            inputs.insert(channel_id, vec![0.5; 512]);
        }

        group.bench_with_input(
            BenchmarkId::new("channels_512_samples", num_channels),
            num_channels,
            |b, _| {
                b.iter(|| {
                    black_box(engine.process(black_box(&inputs)));
                });
            },
        );
    }

    group.finish();
}

fn bench_routing_matrix(c: &mut Criterion) {
    let mut matrix = RoutingMatrix::new();
    let ch1 = ChannelId::new("ch1".to_string());
    let a1 = ChannelId::new("A1".to_string());
    let a2 = ChannelId::new("A2".to_string());

    matrix.set_route(&ch1, &a1, true);
    matrix.set_route(&ch1, &a2, true);

    c.bench_function("routing_matrix_check_route", |b| {
        b.iter(|| {
            black_box(matrix.is_routed(black_box(&ch1), black_box(&a1)));
        });
    });

    c.bench_function("routing_matrix_get_outputs", |b| {
        b.iter(|| {
            black_box(matrix.get_outputs(black_box(&ch1)));
        });
    });
}

fn bench_audio_level_update(c: &mut Criterion) {
    let mut level = AudioLevel::new();
    let samples: Vec<f32> = (0..1000).map(|i| (i as f32) / 1000.0).collect();

    c.bench_function("audio_level_update_1000_samples", |b| {
        b.iter(|| {
            for &sample in &samples {
                level.update(black_box(sample));
            }
        });
    });
}

fn bench_lockfree_buffer(c: &mut Criterion) {
    use troubadour_infra::audio::LockFreeRingBuffer;

    let mut group = c.benchmark_group("lockfree_buffer");

    for size in [64, 256, 1024, 4096].iter() {
        let mut buffer = LockFreeRingBuffer::with_capacity(*size);
        let input: Vec<f32> = (0..*size).map(|i| i as f32).collect();
        let mut output = vec![0.0; *size];

        group.bench_with_input(
            BenchmarkId::new("write", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(buffer.write(black_box(&input)));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("read", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(buffer.read(black_box(&mut output)));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_volume_to_amplitude,
    bench_channel_apply_gain,
    bench_mixer_process_single_channel,
    bench_mixer_process_multiple_channels,
    bench_routing_matrix,
    bench_audio_level_update,
    bench_lockfree_buffer
);

criterion_main!(benches);
