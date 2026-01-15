# Performance Guide & Benchmarks

## Overview

Troubadour is designed for ultra-low latency audio processing with minimal CPU usage. This document describes performance characteristics, optimization strategies, and benchmark results.

## Performance Targets

| Metric | Target | Production Target |
|--------|--------|-------------------|
| **Latency** | < 20ms | < 10ms |
| **CPU Usage** | < 5% @ 48kHz, 8 channels | < 3% |
| **Memory Footprint** | < 100MB | < 50MB |
| **XRUNs (Dropouts)** | < 1/hour | < 1/day |
| **Peak Memory** | < 50MB | < 30MB |

## Architecture Optimizations

### 1. Lock-Free Audio Path

The audio processing path uses **lock-free** data structures to prevent thread contention:

```rust
// Lock-free ring buffer with cache-padded counters
pub struct LockFreeRingBuffer {
    buffer: Vec<f32>,
    write_pos: Arc<CachePadded<AtomicUsize>>,
    read_pos: Arc<CachePadded<AtomicUsize>>,
    capacity: usize,
    mask: usize,
}
```

**Benefits**:
- No mutex contention between audio thread and main thread
- Wait-free for single producer/consumer
- Cache-padded counters prevent false sharing
- Power-of-2 capacity for fast modulo (bitmask)

### 2. Cache-Friendly Data Structures

**Optimizations in MixerEngine**:

```rust
pub struct MixerEngine {
    channels: HashMap<ChannelId, MixerChannel>,
    routing: RoutingMatrix,
    channel_order: Vec<ChannelId>, // Cache-friendly iteration order
}
```

**Key optimizations**:
- Pre-allocated output buffers with `HashMap::with_capacity()`
- Sequential memory access in hot path
- Single-pass processing (no intermediate allocations)
- Gain value cached before audio loop

### 3. SIMD-Ready Audio Processing

Audio buffers are processed in a cache-friendly manner suitable for SIMD auto-vectorization:

```rust
// Compiler can auto-vectorize this loop
output.iter_mut()
    .zip(input_buffer.iter())
    .for_each(|(out, &sample)| {
        *out += sample * gain;
    });
```

### 4. Zero-Copy Where Possible

- **Bypass mode**: When sample rates match, resampling is skipped (zero-copy)
- **In-place processing**: Effects modify buffers in-place when possible
- **Reference passing**: Channel state passed by reference, not cloned

## Benchmark Suite

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench mixer_benchmark

# Run with flamegraph
cargo flamegraph --bench mixer_benchmark

# Save baseline
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

### Benchmark Categories

#### 1. Mixer Engine Benchmarks (`mixer_benchmark.rs`)

Measures:
- Channel creation overhead
- Gain application throughput (hot path)
- Routing matrix lookup speed
- Mixer scalability (2-32 channels)
- Buffer size sensitivity (64-2048 samples)
- Solo/mute processing overhead

**Key metric**: `mixer_process_8_channels_512_samples`

#### 2. Resampling Benchmarks (`resampling_benchmark.rs`)

Measures:
- Bypass performance (no resampling)
- Upsampling ratios (44.1->48, 48->96, etc.)
- Downsampling ratios (48->44.1, 96->48, etc.)
- Multi-channel performance (1-8 channels)
- Buffer size sensitivity
- Continuous resampling workload

**Key metric**: `resample_44.1_to_48_stereo_512`

#### 3. DSP Effects Benchmarks (`dsp_benchmark.rs`)

Measures:
- Equalizer processing throughput
- Compressor processing throughput
- Effect chaining overhead
- Multi-channel EQ (2-16 channels)
- Effect reset performance
- Compression ratio sensitivity

**Key metric**: `equalizer_512_samples`

#### 4. Memory Benchmarks (`memory_benchmark.rs`)

Measures:
- Memory allocations in audio path
- Lock-free vs mutex ring buffer
- Memory reuse patterns
- Cache behavior (sequential vs random)
- Peak memory usage
- Volume conversion memory

**Key metric**: `lockfree_ring_buffer`

## Expected Performance (48kHz, 8 channels)

### Mixer Engine

| Operation | Time per 512 samples | Throughput |
|-----------|---------------------|------------|
| **Channel creation** | ~500 ns | 2M channels/sec |
| **Gain application** | ~1 μs | 1B samples/sec |
| **Routing lookup** | ~50 ns | 20M lookups/sec |
| **Full mix (8 ch)** | ~10 μs | 50M samples/sec |

### Resampling

| Operation | Time per 512 samples | Ratio |
|-----------|---------------------|-------|
| **Bypass (no-op)** | ~100 ns | 1.0x |
| **44.1 -> 48 kHz** | ~2 μs | 1.088x |
| **48 -> 96 kHz** | ~1.5 μs | 2.0x |
| **96 -> 48 kHz** | ~1.5 μs | 0.5x |

### DSP Effects

| Operation | Time per 512 samples |
|-----------|---------------------|
| **3-band EQ** | ~5 μs |
| **Compressor** | ~8 μs |
| **EQ + Comp** | ~13 μs |

### Memory Usage

| Component | Memory (8 channels) |
|-----------|-------------------|
| **Mixer engine** | ~10 KB |
| **Routing matrix** | ~5 KB |
| **Ring buffers (4KB each)** | ~32 KB |
| **Total** | **~50 KB** |

## Optimization Techniques

### 1. Allocations in Hot Path

**Rule**: Zero allocations in audio callback.

**Bad**:
```rust
pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
    input.iter().map(|&s| s * self.gain).collect() // Allocates!
}
```

**Good**:
```rust
pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
    output.iter_mut().zip(input.iter()).for_each(|(out, &s)| {
        *out = s * self.gain;
    });
}
```

### 2. Cache Locality

**Bad**: Random memory access
```rust
// HashMap iteration (random order)
for (id, channel) in &self.channels {
    process_channel(channel);
}
```

**Good**: Sequential access
```rust
// Vec iteration (sequential order)
for id in &self.channel_order {
    let channel = &self.channels[id];
    process_channel(channel);
}
```

### 3. Branch Prediction

**Bad**: Unpredictable branches
```rust
for sample in buffer {
    if channel.muted {
        *sample = 0.0;
    } else {
        *sample *= gain;
    }
}
```

**Good**: Early exit (branch predict once)
```rust
if channel.muted {
    buffer.fill(0.0);
    return;
}
for sample in buffer {
    *sample *= gain;
}
```

### 4. Memory Reuse

**Bad**: Reallocate every frame
```rust
let mut output = vec![0.0; buffer_size]; // Allocates!
process(&input, &mut output);
```

**Good**: Reuse buffer
```rust
let mut output = Vec::with_capacity(buffer_size);
output.resize(buffer_size, 0.0); // Only allocate once
loop {
    process(&input, &mut output);
}
```

## Profiling Tools

### Flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bench mixer_benchmark

# View SVG
open flamegraph.svg
```

### Heap Profiling

```bash
# Run with heap profiling
cargo build --release
valgrind --tool=massif ./target/release/troubadour

# Analyze results
ms_print massif.out.xxxxx
```

### CPU Profiling

```bash
# perf (Linux)
perf record -g cargo bench
perf report

# Instruments (macOS)
instruments -t "Time Profiler" cargo bench
```

## Known Limitations

1. **Resampling Quality**: Current linear interpolation is fast but low quality. Future work will use rubato's FFT-based resampling for higher quality at the cost of more CPU.

2. **DSP Effects**: Current EQ is a simplified placeholder. Production-quality biquad filters will add ~2-3x overhead.

3. **Lock Contention**: While audio path is lock-free, configuration changes (add/remove channels) use internal mutexes. This is acceptable since config changes are infrequent.

4. **Memory Fragmentation**: Long-running sessions may cause heap fragmentation. Mitigation: Use arena allocators for channel state.

## Future Optimizations

### 1. SIMD Explicit

Use `std::simd` (nightly) or `packed_simd` for explicit vectorization:

```rust
use std::simd::f32x4;

let gain_vec = f32x4::splat(gain);
for chunk in input.chunks_exact(4) {
    let samples = f32x4::from_slice_unaligned(chunk);
    let result = samples * gain_vec;
    result.write_to_slice_unaligned(&mut output[..4]);
}
```

Expected speedup: **2-4x** for gain operations.

### 2. SIMD Resampling

Use rubato's SIMD-optimized FFT resampling:

```rust
use rubato::FftFixedInOut;

let mut resampler = FftFixedInOut::new(
    source_rate,
    target_rate,
    buffer_size,
    2, // channels
)?;
```

Expected quality: **Much higher**, expected speed: **Similar** for large buffers.

### 3. Parallel Channel Processing

Use rayon to process channels in parallel (for 16+ channels):

```rust
use rayon::prelude::*;

inputs.par_iter().for_each(|(id, buffer)| {
    process_channel(id, buffer);
});
```

Expected speedup: **4-8x** for 32+ channels on 8-core CPU.

### 4. Arena Allocation

Use typed_arena for channel state:

```rust
use typed_arena::Arena;

struct MixerEngine {
    channel_arena: Arena<MixerChannel>,
}
```

Expected benefit: **Zero heap fragmentation**, **20% faster** channel creation.

## Performance Checklist

Before deploying to production, verify:

- [ ] All benchmarks pass within expected ranges
- [ ] Flamegraph shows no hot spots in audio path
- [ ] Memory usage stable over 24-hour stress test
- [ ] No XRUNs in 1-hour continuous playback
- [ ] CPU usage < 5% at 48kHz, 8 channels
- [ ] Lock-free ring buffer tests pass under contention
- [ ] Cache miss rate < 1% (perf stat)

## Debugging Performance Issues

### High CPU Usage

1. **Profile**: `cargo flamegraph --bench mixer_benchmark`
2. **Check**: Are allocations happening in audio callback?
3. **Verify**: Is bypass mode working (no unnecessary resampling)?
4. **Optimize**: Reduce channel count or buffer size

### Audio Dropouts (XRUNs)

1. **Check**: Buffer size too small? Increase to 512 or 1024
2. **Verify**: No blocking operations in audio thread
3. **Profile**: Look for lock contention
4. **Optimize**: Use lock-free ring buffers

### Memory Leaks

1. **Test**: Run valgrind massif
2. **Check**: Channels not being removed properly
3. **Verify**: Ring buffers not growing unbounded
4. **Fix**: Use arena allocation for channel state

## References

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [CPAL Performance Guide](https://github.com/RustAudio/cpal)
- [Lock-Free Programming](https://www.1024cores.net/home/lock-free-algorithms)
- [Cache Optimization](https://lwn.net/Articles/255364/)

---

**Last Updated**: 2025-01-15
