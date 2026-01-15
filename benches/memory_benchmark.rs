//! Memory profiling benchmarks
//!
//! Measures:
//! - Memory allocations in audio path
//! - Peak memory usage
//! - Memory churn (allocations/deallocations)
//! - Cache behavior

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::collections::HashMap;
use troubadour_core::domain::mixer::{
    ChannelId, MixerChannel, MixerEngine, VolumeDecibels,
};
use troubadour_infra::audio::{LockFreeRingBuffer, RingBuffer};

/// Benchmark memory allocation patterns in mixer processing
fn bench_mixer_memory_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocations");

    for channel_count in [2, 4, 8, 16].iter() {
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

            engine
                .routing_mut()
                .set_route(&input_id, &output_id, true);
        }

        // Prepare input data (reusable)
        let mut inputs = HashMap::new();
        for i in 0..*channel_count {
            let input_id = ChannelId::new(format!("input_{}", i));
            inputs.insert(input_id, vec![0.5; 512]);
        }

        group.throughput(Throughput::Elements(*channel_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(channel_count),
            channel_count,
            |b, _| {
                b.iter(|| {
                    // Process multiple times to see allocation patterns
                    for _ in 0..10 {
                        black_box(engine.process(black_box(&inputs)));
                    }
                })
            },
        );
    }

    group.finish();
}

/// Compare lock-free vs mutex-based ring buffer memory patterns
fn bench_ring_buffer_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_memory");

    // Lock-free implementation
    group.bench_function("lockfree_ring_buffer", |b| {
        b.iter(|| {
            let buffer = LockFreeRingBuffer::with_capacity(1024);
            let input = vec![1.0; 512];
            let mut output = vec![0.0; 512];

            buffer.write(black_box(&input));
            buffer.read(black_box(&mut output));

            black_box(buffer)
        })
    });

    // Original mutex-based implementation
    group.bench_function("mutex_ring_buffer", |b| {
        b.iter(|| {
            let mut buffer = RingBuffer::with_capacity(1024);
            let input = vec![1.0; 512];
            let mut output = vec![0.0; 512];

            buffer.write(black_box(&input)).unwrap();
            buffer.read(black_box(&mut output)).unwrap();

            black_box(buffer)
        })
    });

    group.finish();
}

/// Benchmark memory reuse patterns
fn bench_memory_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_reuse");

    // Pattern 1: Re-allocate each time (inefficient)
    group.bench_function("reallocation", |b| {
        b.iter(|| {
            let buffer: Vec<f32> = (0..512).map(|i| i as f32).collect();
            black_box(buffer)
        })
    });

    // Pattern 2: Reuse buffer (efficient)
    group.bench_function("reuse_buffer", |b| {
        let mut buffer = Vec::with_capacity(512);
        b.iter(|| {
            buffer.clear();
            buffer.extend(0..512);
            black_box(&mut buffer)
        })
    });

    group.finish();
}

/// Benchmark cache behavior with different data layouts
fn bench_cache_behavior(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_behavior");

    // Cache-friendly: Sequential array access
    group.bench_function("sequential_access", |b| {
        let data: Vec<f32> = (0..4096).map(|i| i as f32).collect();
        let mut result = 0.0;

        b.iter(|| {
            for &value in &data {
                result += value;
            }
            black_box(result)
        })
    });

    // Cache-unfriendly: Random access pattern
    group.bench_function("random_access", |b| {
        let data: Vec<f32> = (0..4096).map(|i| i as f32).collect();
        let indices: Vec<usize> = (0..4096).map(|i| (i * 17) % 4096).collect();
        let mut result = 0.0;

        b.iter(|| {
            for &idx in &indices {
                result += data[idx];
            }
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark peak memory usage with varying channel counts
fn bench_peak_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("peak_memory");

    for channels in [2, 8, 16, 32].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(channels), channels, |b, _| {
            b.iter(|| {
                let mut engine = MixerEngine::new();

                // Create many channels
                for i in 0..*channels {
                    engine.add_channel(MixerChannel::new(
                        ChannelId::new(format!("ch_{}", i)),
                        format!("Channel {}", i),
                    ));
                }

                // Create large routing matrix
                for i in 0..*channels {
                    for j in 0..*channels {
                        let from = ChannelId::new(format!("ch_{}", i));
                        let to = ChannelId::new(format!("ch_{}", j));
                        engine.routing_mut().set_route(&from, &to, i != j);
                    }
                }

                black_box(engine)
            })
        });
    }

    group.finish();
}

/// Benchmark memory allocation in volume conversion
fn bench_volume_conversion_memory(c: &mut Criterion) {
    c.bench_function("volume_conversion_bulk", |b| {
        let volumes: Vec<VolumeDecibels> = (-60..=6).map(|db| VolumeDecibels::new(db as f32)).collect();

        b.iter(|| {
            let amplitudes: Vec<f32> = volumes.iter().map(|v| v.to_amplitude()).collect();
            black_box(amplitudes)
        })
    });
}

criterion_group!(
    benches,
    bench_mixer_memory_allocations,
    bench_ring_buffer_memory,
    bench_memory_reuse,
    bench_cache_behavior,
    bench_peak_memory,
    bench_volume_conversion_memory,
);

criterion_main!(benches);
