//! Criterion benchmarks for DSP effects performance
//!
//! Measures:
//! - Equalizer processing throughput
//! - Compressor processing throughput
//! - Effect chaining overhead
//! - Buffer size sensitivity

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::f32::consts::PI;

/// Simple 3-band equalizer for benchmarking
pub struct Equalizer {
    low: f32,
    mid: f32,
    high: f32,
}

impl Equalizer {
    pub fn new(low_db: f32, mid_db: f32, high_db: f32) -> Self {
        Self {
            low: db_to_gain(low_db),
            mid: db_to_gain(mid_db),
            high: db_to_gain(high_db),
        }
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        // Simplified 3-band EQ using rough frequency splitting
        // In production, use proper biquad filters
        for sample in buffer.iter_mut() {
            // This is a simplified placeholder - real EQ uses proper filters
            *sample = *sample * self.mid; // Placeholder
        }
    }
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Simple compressor for benchmarking
pub struct Compressor {
    threshold: f32,
    ratio: f32,
    envelope: f32,
    attack: f32,
    release: f32,
}

impl Compressor {
    pub fn new(threshold_db: f32, ratio: f32, sample_rate: u32) -> Self {
        Self {
            threshold: db_to_gain(threshold_db),
            ratio,
            envelope: 0.0,
            attack: 1.0 - (-0.01_f32).exp(), // ~10ms attack
            release: 1.0 - (-0.1_f32).exp(),  // ~100ms release
        }
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            let input_level = sample.abs();

            // Update envelope
            if input_level > self.envelope {
                self.envelope = input_level * self.attack + self.envelope * (1.0 - self.attack);
            } else {
                self.envelope = input_level * self.release + self.envelope * (1.0 - self.release);
            }

            // Calculate gain reduction
            let gain = if self.envelope > self.threshold {
                let excess = self.envelope / self.threshold;
                excess.powf(1.0 / self.rating).recip()
            } else {
                1.0
            };

            *sample *= gain;
        }
    }

    pub fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

impl Compressor {
    fn rating(&self) -> f32 {
        self.ratio
    }
}

/// Generate test audio buffer with multiple frequencies
fn generate_test_buffer(frames: usize, sample_rate: u32) -> Vec<f32> {
    (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Mix of 100Hz, 1kHz, and 10kHz
            (2.0 * PI * 100.0 * t).sin() * 0.3
                + (2.0 * PI * 1000.0 * t).sin() * 0.3
                + (2.0 * PI * 10000.0 * t).sin() * 0.3
        })
        .collect()
}

/// Benchmark EQ processing
fn bench_equalizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("equalizer");

    for size in [64, 256, 512, 1024, 2048].iter() {
        let mut eq = Equalizer::new(0.0, 0.0, 0.0);
        let mut buffer = generate_test_buffer(*size, 48000);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                eq.process(black_box(&mut buffer));
                black_box(&mut buffer)
            })
        });
    }

    group.finish();
}

/// Benchmark compressor processing
fn bench_compressor(c: &mut Criterion) {
    let mut group = c.benchmark_group("compressor");

    for size in [64, 256, 512, 1024, 2048].iter() {
        let mut comp = Compressor::new(-20.0, 4.0, 48000);
        let mut buffer = generate_test_buffer(*size, 48000);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                comp.process(black_box(&mut buffer));
                black_box(&mut buffer)
            })
        });
    }

    group.finish();
}

/// Benchmark effect chaining
fn bench_effect_chain(c: &mut Criterion) {
    c.bench_function("effect_chain_eq_comp", |b| {
        let mut eq = Equalizer::new(0.0, 3.0, -3.0);
        let mut comp = Compressor::new(-18.0, 4.0, 48000);
        let mut buffer = generate_test_buffer(512, 48000);

        b.iter(|| {
            eq.process(black_box(&mut buffer));
            comp.process(black_box(&mut buffer));
            black_box(&mut buffer)
        })
    });
}

/// Benchmark multiple EQ instances (simulating multiple channels)
fn bench_multi_channel_eq(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_channel_eq");

    for channels in [2, 4, 8, 16].iter() {
        let mut eqs: Vec<Equalizer> = (0..*channels)
            .map(|_| Equalizer::new(0.0, 0.0, 0.0))
            .collect();

        let buffers: Vec<Vec<f32>> = (0..*channels)
            .map(|_| generate_test_buffer(512, 48000))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(channels), channels, |b, _| {
            b.iter(|| {
                for (eq, buffer) in eqs.iter_mut().zip(buffers.iter()) {
                    let mut buf_copy = buffer.clone();
                    eq.process(black_box(&mut buf_copy));
                    black_box(&mut buf_copy);
                }
            })
        });
    }

    group.finish();
}

/// Benchmark effect reset
fn bench_effect_reset(c: &mut Criterion) {
    let mut comp = Compressor::new(-20.0, 4.0, 48000);

    // Prime with some audio
    let mut buffer = generate_test_buffer(512, 48000);
    comp.process(&mut buffer);

    c.bench_function("compressor_reset", |b| {
        b.iter(|| {
            comp.reset();
            black_box(&mut comp)
        })
    });
}

/// Benchmark different compression ratios
fn bench_compression_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    for ratio in [2.0, 4.0, 8.0, 20.0].iter() {
        let mut comp = Compressor::new(-20.0, *ratio, 48000);
        let mut buffer = generate_test_buffer(512, 48000);

        group.bench_with_input(BenchmarkId::from_parameter(ratio), ratio, |b, _| {
            b.iter(|| {
                comp.process(black_box(&mut buffer));
                black_box(&mut buffer)
            })
        });
    }

    group.finish();
}

/// Benchmark in-place vs copy processing
fn bench_in_place_vs_copy(c: &mut Criterion) {
    let mut eq = Equalizer::new(0.0, 0.0, 0.0);
    let input = generate_test_buffer(512, 48000);

    c.bench_function("eq_in_place", |b| {
        b.iter(|| {
            let mut buffer = input.clone();
            eq.process(black_box(&mut buffer));
            black_box(&mut buffer)
        })
    });

    // Compare with copy-based approach (avoided in production)
    c.bench_function("eq_copy_based", |b| {
        b.iter(|| {
            let buffer: Vec<f32> = input
                .iter()
                .map(|&s| {
                    // Simulate copy-based processing (inefficient)
                    s * 1.0
                })
                .collect();
            black_box(buffer)
        })
    });
}

criterion_group!(
    benches,
    bench_equalizer,
    bench_compressor,
    bench_effect_chain,
    bench_multi_channel_eq,
    bench_effect_reset,
    bench_compression_ratios,
    bench_in_place_vs_copy,
);

criterion_main!(benches);
