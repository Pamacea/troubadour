# 🎼 Troubadour

<div align="center">

**A modern, cross-platform virtual audio mixer written in Rust**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

</div>

---

## 📖 About

**Troubadour** is a next-generation virtual audio mixer designed as a modern, reliable, and user-friendly alternative to
Voicemeeter. Written in 100% Rust, it provides:

- ✅ **Cross-platform** - Windows, Linux, macOS
- ✅ **Low latency** - < 20ms end-to-end
- ✅ **High quality** - Transparent resampling, professional DSP
- ✅ **User friendly** - Intuitive GUI, sensible defaults
- ✅ **Reliable** - Robust error handling, state persistence

---

## 🚀 Quick Start

```bash
# Clone repository
git clone https://github.com/Yanis/troubadour.git
cd troubadour

# Build
cargo build --release

# Run
cargo run --release
```

---

## 📚 Documentation

- [**MASTERPLAN**](./docs/MASTERPLAN.md) - Complete project overview
- [**ARCHITECTURE**](./docs/ARCHITECTURE.md) - Technical architecture
- [**DEVELOPMENT GUIDE**](./docs/DEVELOPMENT_GUIDE.md) - Development workflow
- [**ROADMAP**](./docs/PLAN.md) - Development roadmap

---

## 🎯 Features

- 🎚️ **Virtual Mixing** - Unlimited channels with routing
- 🔊 **Volume Control** - Per-channel volume (0-200%)
- 🔇 **Mute/Solo** - Channel mute and solo
- 🎛️ **Metering** - Real-time level meters (dB)
- 💾 **Presets** - Save/load mixer configurations
- 🎨 **Modern GUI** - Cross-platform desktop interface (Tauri)
- ⚡ **High Performance** - Low CPU usage (< 5%)

---

## 🏗️ Architecture

Troubadour follows **Hexagonal Architecture**:

- **Core Domain** - Mixer engine, DSP, configuration
- **Infrastructure** - Audio backends, MIDI, persistence
- **API Layer** - CLI, GUI, OSC

For details, see [ARCHITECTURE.md](./docs/ARCHITECTURE.md).

---

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

<div align="center">

**Made with ❤️ and 🦀 by Yanis**

</div>
