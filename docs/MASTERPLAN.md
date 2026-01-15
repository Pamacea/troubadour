# 🎼 Troubadour - Master Plan

## 📋 Project Overview

**Troubadour** is a next-generation virtual audio mixer written in 100% Rust, designed as a modern, reliable, and
user-friendly alternative to Voicemeeter.

### Vision

- **100% Rust** - Memory safety, zero-cost abstractions, fearless concurrency
- **Cross-platform** - Windows, Linux, macOS with native audio APIs
- **Professional grade** - Low latency (< 20ms), high quality, reliable
- **User friendly** - Intuitive UI, no confusing options, sensible defaults

### Problems Solved (vs Voicemeeter)

- ❌ **Resampling artifacts** → ✅ **High-quality rubato-based resampling**
- ❌ **Confusing UX** → ✅ **Clean, modern UI with clear feedback**
- ❌ **Windows-only** → ✅ **True cross-platform support**
- ❌ **Unreliable state** → ✅ **Robust configuration management**
- ❌ **Poor documentation** → ✅ **Comprehensive docs and examples**

---

## 🏗️ Technology Stack

### Core (Rust)

| Category    | Technology                       | Purpose                              |
|-------------|----------------------------------|--------------------------------------|
| **Runtime** | `tokio`                          | Async runtime, task scheduling       |
| **Audio**   | `cpal`                           | Cross-platform audio I/O abstraction |
| **DSP**     | `rubato`                         | High-quality resampling              |
| **DSP**     | `rustfft`                        | FFT for frequency analysis           |
| **GUI**     | `tauri`                          | Desktop app framework (Rust backend) |
| **State**   | `tokio::sync::mpsc`              | Async channels (Actor model)         |
| **Config**  | `serde` + `toml`                 | Serialization & config               |
| **Errors**  | `thiserror`                      | Typed error enums                    |
| **Logging** | `tracing` + `tracing-subscriber` | Structured logging                   |
| **CLI**     | `clap`                           | Command-line interface               |
| **Testing** | `proptest`                       | Property-based testing               |

### Audio APIs (Platform-Specific)

| Platform    | API               | Implementation                 |
|-------------|-------------------|--------------------------------|
| **Windows** | WASAPI            | `cpal` default                 |
| **Linux**   | ALSA / PulseAudio | `cpal` with PulseAudio support |
| **macOS**   | CoreAudio         | `cpal` default                 |

---

## 🎯 Core Features

### Phase 1: Foundation (MVP)

1. ✅ **Device Enumeration** - List all input/output audio devices
2. ✅ **Audio Capture** - Real-time stream capture from inputs
3. ✅ **Audio Playback** - Real-time stream output to devices
4. ✅ **Virtual Channels** - N virtual mixer channels
5. ✅ **Volume Control** - Per-channel volume (0-200%)
6. ✅ **Mute/Solo** - Channel mute and solo functionality
7. ✅ **Routing** - Any input → Any output matrix
8. ✅ **Metering** - Real-time level meters (dB)

### Phase 2: UX & Polish

9. ✅ **GUI** - Cross-platform desktop UI
10. ✅ **Presets** - Save/load mixer configurations
11. ✅ **Settings** - Device selection, buffer size, sample rate
12. ✅ **Auto-resampling** - Transparent sample rate conversion

### Phase 3: Advanced Features

13. ✅ **DSP Effects** - EQ, Compressor, Gate
14. ✅ **MIDI Control** - Hardware controller support
15. ✅ **OSC Support** - Remote control protocol
16. ✅ **Macro Buttons** - Programmable actions

### Phase 4: Distribution

17. ✅ **Installers** - Windows/Linux/macOS packages
18. ✅ **Auto-update** - Seamless version updates
19. ✅ **Digital Signatures** - Code signing for trust

---

## 📐 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Troubadour                           │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Tauri GUI  │    │   CLI Tool   │    │  OSC Server  │  │
│  │  (React TSX) │    │   (Clap)     │    │   (Optional) │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │          │
│         └───────────────────┼───────────────────┘          │
│                             │                               │
│                    ┌────────▼────────┐                      │
│                    │  API Layer      │                      │
│                    │  (Commands)     │                      │
│                    └────────┬────────┘                      │
│                             │                               │
│  ┌──────────────────────────┼──────────────────────────┐  │
│  │                          │                          │  │
│  │  ┌─────────────────────▼─────────────────────┐    │  │
│  │  │         Core Domain Layer                 │    │  │
│  │  │  • Mixer Engine (Channels, Routing)       │    │  │
│  │  │  • DSP Effects (EQ, Comp, Gate)           │    │  │
│  │  │  • State Machine (Config, Presets)        │    │  │
│  │  └─────────────────────┬─────────────────────┘    │  │
│  │                          │                          │  │
│  │  ┌─────────────────────▼─────────────────────┐    │  │
│  │  │         Infrastructure Layer              │    │  │
│  │  │  • Audio Backend (cpal + platform APIs)   │    │  │
│  │  │  • MIDI I/O (midir)                       │    │  │
│  │  │  • File I/O (config, presets)             │    │  │
│  │  │  • Logging (tracing)                      │    │  │
│  │  └───────────────────────────────────────────┘    │  │
│  │                                                   │  │
│  └───────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 🎯 Performance Targets

| Metric        | Target   | Measurement          |
|---------------|----------|----------------------|
| **Latency**   | < 20ms   | End-to-end (in→out)  |
| **CPU Usage** | < 5%     | @ 48kHz, 8 channels  |
| **Memory**    | < 100MB  | Working set          |
| **XRUNs**     | < 1/hour | Audio dropouts       |
| **Startup**   | < 2s     | Cold start to ready  |
| **GUI FPS**   | 60 FPS   | Meter updates smooth |

---

## 📊 Project Status

- **Current Phase**: Planning & Architecture
- **Next Milestone**: US-001 (Project Foundation)
- **Target MVP**: Q2 2025
- **Target v1.0**: Q4 2025

---

## 🔄 Development Workflow

1. **Feature Branch** - `feature/US-XXX-description`
2. **PR Review** - Required for all code
3. **CI/CD** - Automated tests + benchmarks
4. **Documentation** - Updated alongside code
5. **Git Hooks** - Pre-commit lint + format check

---

## 📚 Key Resources

- **CPAL Docs**: https://docs.rs/cpal
- **Tauri Docs**: https://tauri.app
- **Rubato (Resampling)**: https://docs.rs/rubato
- **Tokio**: https://tokio.rs
- **Rust Book**: https://doc.rust-lang.org/book/

---

*Last updated: 2025-01-14*
