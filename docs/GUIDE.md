# Guide

## Getting Started

### Prerequisites

- Rust toolchain (stable, latest) via [rustup](https://rustup.rs/)
- Dioxus CLI: `cargo install dioxus-cli`
- Platform audio libraries:
  - **Windows:** No extra dependencies (WASAPI built-in)
  - **macOS:** No extra dependencies (CoreAudio built-in)
  - **Linux:** `libasound2-dev` (ALSA) or PipeWire/PulseAudio dev packages

### Build

```bash
# Clone the repository
git clone https://github.com/Pamacea/troubadour.git
cd troubadour

# Build all crates
cargo build

# Run the desktop application
cargo run -p troubadour-ui

# Run with Dioxus hot-reload (development)
dx serve --platform desktop
```

### Test

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p troubadour-core

# Run with output
cargo test -- --nocapture
```

## Project Structure

```
troubadour/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── troubadour-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs       # Public API
│   │       ├── engine.rs    # Audio engine loop
│   │       ├── device.rs    # Device management
│   │       ├── mixer.rs     # Channel mixing
│   │       ├── dsp/         # DSP processors
│   │       └── routing.rs   # Audio routing
│   ├── troubadour-ui/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs      # App entry point
│   │       ├── app.rs       # Root component
│   │       ├── components/  # UI components
│   │       └── styles/      # CSS stylesheets
│   └── troubadour-shared/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs       # Re-exports
│           ├── config.rs    # Configuration types
│           ├── messages.rs  # IPC message types
│           └── audio.rs     # Audio types
├── doc/                     # Documentation
├── LICENSE
├── README.md
└── CHANGELOG.md
```

## Development Workflow

1. **Audio changes** → Work in `troubadour-core`, test with `cargo test -p troubadour-core`
2. **UI changes** → Work in `troubadour-ui`, use `dx serve --platform desktop` for hot-reload
3. **Shared types** → Update `troubadour-shared`, both crates will see changes

## Configuration

Troubadour stores its configuration in:
- **Windows:** `%APPDATA%\troubadour\config.toml`
- **macOS:** `~/Library/Application Support/troubadour/config.toml`
- **Linux:** `~/.config/troubadour/config.toml`
