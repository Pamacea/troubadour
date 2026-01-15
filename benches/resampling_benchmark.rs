//! Criterion benchmarks for resampling performance
//!
//! Measures:
//! - Linear interpolation resampling throughput
//! - Different sample rate conversion ratios
//! - Multi-channel performance
//! - Cache behavior with varying buffer sizes

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use troubadour_core::domain::audio::SampleRate;
use troubadour_infra::audio::stream::Resampler;

/// Generate sine wave test buffer
fn generate_sine_wave(freq: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
}

/// Generate stereo test buffer
fn generate_stereo_buffer(frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / 48000.0;
        let left = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        let right = (2.0 * std::f32::consts::PI * 880.0 * t).sin();
        buffer.push(left);
        buffer.push(right);
    }
    buffer
}

/// Benchmark bypass (no resampling)
fn bench_bypass(c: &mut Criterion) {
    let mut resampler = Resampler::new(48000, 48000, 2).unwrap();

    let input = generate_stereo_buffer(512);
    let mut output = vec![0.0; 512];

    c.bench_function("resample_bypass_512_stereo", |b| {
        b.iter(|| {
            resampler.process(black_box(&input), black_box(&mut output)).unwrap();
            black_box(&mut output)
        })
    });
}

/// Benchmark upsampling ratios
fn bench_upsampling_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("upsampling");

    let ratios = vec![
        (44100, 48000, "44.1->48"),
        (48000, 96000, "48->96"),
        (44100, 88200, "44.1->88.2"),
    ];

    for (src_rate, dst_rate, label) in ratios {
        let mut resampler = Resampler::new(src_rate, dst_rate, 2).unwrap();
        let input_frames = 512;
        let input = generate_stereo_buffer(input_frames);
        let ratio = dst_rate as f64 / src_rate as f64;
        let output_capacity = (input_frames as f64 * ratio).ceil() as usize * 2;
        let mut output = vec![0.0; output_capacity];

        group.bench_with_input(label, &label, |b, _| {
            b.iter(|| {
                black_box(resampler.process(black_box(&input), black_box(&mut output)).unwrap())
            })
        });
    }

    group.finish();
}

/// Benchmark downsampling ratios
fn bench_downsampling_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("downsampling");

    let ratios = vec![
        (48000, 44100, "48->44.1"),
        (96000, 48000, "96->48"),
        (88200, 44100, "88.2->44.1"),
    ];

    for (src_rate, dst_rate, label) in ratios {
        let mut resampler = Resampler::new(src_rate, dst_rate, 2).unwrap();
        let input_frames = 512;
        let input = generate_stereo_buffer(input_frames);
        let ratio = dst_rate as f64 / src_rate as f64;
        let output_capacity = (input_frames as f64 * ratio).ceil() as usize * 2;
        let mut output = vec![0.0; output_capacity];

        group.bench_with_input(label, &label, |b, _| {
            b.iter(|| {
                black_box(resampler.process(black_box(&input), black_box(&mut output)).unwrap())
            })
        });
    }

    group.finish();
}

/// Benchmark mono vs stereo vs multi-channel
fn bench_channel_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel_count");

    for channels in [1, 2, 4, 8].iter() {
        let mut resampler = Resampler::new(44100, 48000, *channels as u16).unwrap();

        let frames = 512;
        let input: Vec<f32> = (0..frames * *channels)
            .map(|i| ((i as f32 / 100.0) * 2.0 * std::f32::consts::PI).sin())
            .collect();

        let ratio = 48000.0 / 44100.0;
        let output_capacity = (frames as f64 * ratio).ceil() as usize * *channels;
        let mut output = vec![0.0; output_capacity];

        group.bench_with_input(BenchmarkId::from_parameter(channels), channels, |b, _| {
            b.iter(|| {
                black_box(resampler.process(black_box(&input), black_box(&mut output)).unwrap())
            })
        });
    }

    group.finish();
}

/// Benchmark different buffer sizes
fn bench_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_size");

    for size in [64, 128, 256, 512, 1024, 2048].iter() {
        let mut resampler = Resampler::new(44100, 48000, 2).unwrap();
        let input = generate_stereo_buffer(*size);
        let ratio = 48000.0 / 44100.0;
        let output_capacity = (*size as f64 * ratio).ceil() as usize * 2;
        let mut output = vec![0.0; output_capacity];

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                black_box(resampler.process(black_box(&input), black_box(&mut output)).unwrap())
            })
        });
    }

    group.finish();
}

/// Benchmark continuous resampling (realistic workload)
fn bench_continuous_resampling(c: &mut Criterion) {
    c.bench_function("continuous_resample_44.1_to_48", |b| {
        let mut resampler = Resampler::new(44100, 48000, 2).unwrap();
        let input = generate_sine_wave(440.0, 44100, 512 * 2);
        let mut output = vec![0.0; 512 * 2 * 2];

        b.iter(|| {
            // Process in chunks
            for chunk in input.chunks(512) {
                let ratio = 48000.0 / 44100.0;
                let output_capacity = (chunk.len() as f64 * ratio).ceil() as usize;
                black_box(
                    resampler
                        .process(chunk, &mut output[..output_capacity])
                        .unwrap(),
                );
            }
        })
    });
}

/// Benchmark resampler creation overhead
fn bench_resampler_creation(c: &mut Criterion) {
    c.bench_function("resampler_creation", |b| {
        b.iter(|| {
            black_box(Resampler::new(44100, 48000, 2).unwrap())
        })
    });
}

criterion_group!(
    benches,
    bench_bypass,
    bench_upsampling_ratios,
    bench_downsampling_ratios,
    bench_channel_counts,
    bench_buffer_sizes,
    bench_continuous_resampling,
    bench_resampler_creation,
);

criterion_main!(benches);
