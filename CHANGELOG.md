# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-03-19

### Added
- **Mixer core**: `Mixer` struct with full channel management (add/remove/modify channels)
- **Channel strips**: Named channels with volume (0-200%), mute, solo, pan (-1.0 to 1.0)
- **Routing matrix**: N:N routing between any input and any output, toggle on/off
- **Solo logic**: Standard console behavior (no solo = all audible, any solo = only solos pass)
- **Pan law**: Constant power panning (equal energy across L/R)
- **VU-meters**: RMS + peak level calculation with attack/release smoothing and peak hold
- **Mixer config types**: `ChannelConfig`, `ChannelKind`, `Route`, `ChannelLevel`, `MixerConfig`
- **Default setup**: Mic, Desktop, Browser inputs → Headphones, Speakers outputs
- **UI Mixer view**: Tailwind CSS v4 dark theme with channel strips, faders, mute/solo buttons
- **UI Routing matrix**: Interactive grid (inputs × outputs), click to connect/disconnect
- **UI Channel strips**: VU-meter bars, volume sliders, pan knobs, IN/OUT badges
- **New commands**: `SetSolo`, `SetPan`, `AddRoute`, `RemoveRoute`
- **Tailwind CSS v4**: Integrated with Dioxus desktop via `include_str!` + `with_custom_head`
- **34 new tests**: Mixer logic, channel config, routing, solo/mute/pan, VU-meter convergence

### Changed
- `Event::LevelUpdate` now carries `Vec<ChannelLevel>` (batch updates for efficiency)
- Engine `process_commands` handles all new command types

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

[Unreleased]: https://github.com/Pamacea/troubadour/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Pamacea/troubadour/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Pamacea/troubadour/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Pamacea/troubadour/releases/tag/v0.0.0
