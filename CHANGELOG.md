# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-03-19

### Added
- **Workspace**: Rust workspace with 3 crates (`troubadour-core`, `troubadour-ui`, `troubadour-shared`)
- **Device enumeration**: List all system audio devices (inputs/outputs) via `cpal`
- **Audio passthrough**: Capture input device and route to output device (F32, 48kHz)
- **Sample rate conversion**: `rubato` FFT-based resampler (44.1kHz, 48kHz, 96kHz, 192kHz)
- **Buffer size configuration**: Support for 64, 128, 256, 512 sample buffers with latency calculation
- **IPC**: crossbeam channels for lock-free UI ↔ Engine communication (Command/Event messages)
- **Configuration**: TOML-based config with serde serialization/deserialization
- **Error handling**: Typed errors via `thiserror` with `TroubadourResult` alias
- **UI skeleton**: Dioxus desktop window displaying detected audio devices
- **CI**: GitHub Actions pipeline (check, test, clippy, fmt) on Windows/macOS/Linux
- **Tests**: 38 automated tests covering audio types, config, messages, devices, engine, and resampler

## [0.0.0] - 2026-03-19

### Added
- Project initialization with documentation
- README with architecture overview
- CHANGELOG, LICENSE (MIT), ROADMAP
- Documentation: ARCHITECTURE.md, GUIDE.md, REFERENCE.md

[Unreleased]: https://github.com/Pamacea/troubadour/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Pamacea/troubadour/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Pamacea/troubadour/releases/tag/v0.0.0
