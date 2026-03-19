# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-03-20

### Added
- **DSP module**: `dsp/` with trait `Processor` and `EffectsChain` for composable audio processing
- **Noise Gate**: Envelope follower with configurable threshold/attack/release, off by default
- **Compressor**: Dynamic range compression with threshold, ratio, attack, release, makeup gain
- **Limiter**: Brickwall limiter with configurable ceiling, always active for clipping protection
- **Parametric EQ**: 3-band biquad filter (low shelf 200Hz, peaking 1kHz, high shelf 8kHz)
- **Effects chain**: Gate -> EQ -> Compressor -> Limiter pipeline, integrated in audio callback
- **DSP shared types**: Serializable configs for all effects (NoiseGateConfig, CompressorConfig, EqConfig, LimiterConfig)
- **Effects presets**: 3 built-in presets (Default, Streaming, Clean) with TOML serialization
- **UI DSP panel**: Controls for Gate, EQ (Low/Mid/High sliders), Compressor (threshold/ratio/makeup), Limiter (ceiling)
- **UI preset selector**: Switch between Default/Streaming/Clean with one click
- **Live DSP wiring**: UI controls rebuild the EffectsChain via Arc<Mutex> shared with audio callback
- **EffectsChain::from_preset**: Reconstruct entire DSP chain from serialized preset config
- **Bypass per effect**: Toggle each processor on/off independently
- **36 new tests**: DSP processors, EQ biquad filters, effects chain, presets serialization

### Changed
- Default DSP chain: Gate (off) -> EQ (flat) -> Compressor (3:1, soft) -> Limiter (0.95 ceiling)
- Audio pipeline: input -> mono downmix -> DSP chain -> gain/pan -> output

## [0.2.1] - 2026-03-20

### Fixed
- Audio controls now work: volume, mute, pan actually control audio output
- Mono to stereo: downmix input to mono then apply pan for both ears
- Command channel: dedicated crossbeam channel prevents mpmc message stealing
- CI: exclude cpal-dependent tests on runners without audio hardware

## [0.2.0] - 2026-03-19

### Added
- Mixer core with channel management, routing matrix N:N, solo logic
- UI: Tailwind CSS v4 dark theme, channel strips, routing matrix
- 34 new mixer tests

## [0.1.0] - 2026-03-19

### Added
- Rust workspace with 3 crates, device enumeration, audio passthrough
- Sample rate conversion (rubato), IPC (crossbeam), TOML config
- Dioxus desktop UI skeleton, CI (GitHub Actions), 38 tests

## [0.0.0] - 2026-03-19

### Added
- Project initialization, documentation, MIT license

[Unreleased]: https://github.com/Pamacea/troubadour/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Pamacea/troubadour/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Pamacea/troubadour/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Pamacea/troubadour/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Pamacea/troubadour/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Pamacea/troubadour/releases/tag/v0.0.0
