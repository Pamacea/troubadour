# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-03-20

### Added
- **Tab navigation**: Mixer | Effects | Devices tabs with clean header
- **Profile system**: 5 built-in profiles (Default, Gaming, Streaming, Music, Meeting) with TOML serialization
- **Profile bar**: One-click profile switching in header, auto-applies DSP preset
- **Device panel**: Input/output device selection UI with dropdown selectors
- **Footer**: Shows currently selected input/output devices
- **Profile types**: `Profile` struct combining mixer config + DSP preset + device selection
- **4 new tests**: Profile serialization, save/load, builtin profiles

### Changed
- **UI architecture**: Refactored from single-page to tabbed interface
- **MixerView**: Extracted `render_channel_strip` helper to reduce code duplication
- **Header**: Redesigned with profile bar, live indicator, device count

## [0.3.0] - 2026-03-20

### Added
- DSP module: Noise Gate, Compressor, Limiter, Parametric EQ (3-band biquad)
- Effects chain with trait Processor, live DSP wiring via Arc<Mutex>
- 3 built-in effect presets, UI DSP panel with all controls
- 36 new DSP tests

## [0.2.1] - 2026-03-20

### Fixed
- Audio controls (volume/mute/pan) now actually work
- Mono to stereo fix, command channel architecture fix

## [0.2.0] - 2026-03-19

### Added
- Mixer core, routing matrix N:N, Tailwind CSS v4 UI, 34 new tests

## [0.1.0] - 2026-03-19

### Added
- Rust workspace, device enumeration, audio passthrough, rubato, CI, 38 tests

## [0.0.0] - 2026-03-19

### Added
- Project initialization, documentation, MIT license

[Unreleased]: https://github.com/Pamacea/troubadour/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/Pamacea/troubadour/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Pamacea/troubadour/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Pamacea/troubadour/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Pamacea/troubadour/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Pamacea/troubadour/compare/v0.0.0...v0.1.0
[0.0.0]: https://github.com/Pamacea/troubadour/releases/tag/v0.0.0
