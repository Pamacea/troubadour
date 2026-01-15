# 🏗️ Troubadour - Technical Architecture

## 📐 Architecture Overview

Troubadour follows **Hexagonal Architecture** (Ports & Adapters) with clear separation between:

1. **Domain Layer** - Core business logic (mixer, DSP, state)
2. **API Layer** - Interfaces (CLI, GUI, OSC)
3. **Infrastructure Layer** - External concerns (audio, MIDI, files)

```
┌─────────────────────────────────────────────────────────────┐
│                        APP Layer                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │
│  │   CLI   │  │   GUI   │  │   OSC   │  │  Tests  │       │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘       │
└───────┼────────────┼────────────┼────────────┼─────────────┘
        │            │            │            │
        └────────────┼────────────┼────────────┘
                     │
        ┌────────────▼────────────────────────┐
        │      Core Domain Layer              │
        │  ┌────────────────────────────────┐ │
        │  │  Mixer Engine (Channels)       │ │
        │  │  DSP Effects (EQ, Comp)        │ │
        │  │  State Machine (Config)        │ │
        │  │  Routing Matrix                │ │
        │  └────────────────────────────────┘ │
        └────────────┬────────────────────────┘
                     │
        ┌────────────▼────────────────────────┐
        │   Infrastructure Layer              │
        │  ┌────────────────────────────────┐ │
        │  │  Audio Backend (cpal)          │ │
        │  │  MIDI I/O (midir)              │ │
        │  │  File System (config)          │ │
        │  │  Logging (tracing)             │ │
        │  └────────────────────────────────┘ │
        └─────────────────────────────────────┘
```

---

## 📁 Directory Structure

```
troubadour/
├── crates/
│   ├── core/                    # Domain logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── domain/          # Business entities
│   │       │   ├── audio.rs     # AudioDevice trait
│   │       │   ├── mixer.rs     # MixerEngine, Channel
│   │       │   ├── dsp.rs       # Effects (EQ, Comp)
│   │       │   └── config.rs    # Config schema
│   │       ├── use_cases/       # Business logic
│   │       │   ├── mixer.rs     # Mixer operations
│   │       │   └── config.rs    # Config operations
│   │       └── lib.rs
│   │
│   ├── infra/                   # Infrastructure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── audio/           # Audio implementations
│   │       ├── midi/            # MIDI I/O
│   │       └── persistence/     # Config/Presets
│   │
│   ├── app/                     # APP Layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── cli/             # CLI commands
│   │       ├── gui/             # Tauri commands
│   │       └── osc/             # OSC server
│   │
│   └── tests/                   # Integration tests
│
├── Cargo.toml                   # Workspace root
├── docs/                        # Documentation
├── .smite/                      # PRD and Ralph state
└── CLAUDE.md                    # Project rules
```

---

## 🔄 Data Flow

### Audio Processing Flow

```
Input Device → Capture Stream → Resampler → Ring Buffer → Mixer Engine → Output Stream → Output Device
```

### State Management Flow

```
User Action → Command Bus → Command Handler → State Update → Mixer Engine
```

---

## 🔑 Key Components

### 1. Audio Backend (`infra/audio`)

**Purpose**: Platform-agnostic audio I/O abstraction

**Key Traits**:

```rust
pub trait AudioDevice {
    fn name(&self) -> &str;
    fn channels(&self) -> usize;
    fn sample_rate(&self) -> SampleRate;
}

pub trait AudioStream {
    fn start(&mut self) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
}
```

### 2. Mixer Engine (`core/domain/mixer.rs`)

**Purpose**: Core mixing logic

**Key Structs**:

```rust
pub struct MixerEngine {
    channels: Vec<MixerChannel>,
    routing: RoutingMatrix,
    sample_rate: SampleRate,
}

pub struct MixerChannel {
    id: ChannelId,
    name: String,
    volume: Decibels,
    muted: bool,
    solo: bool,
    effects: EffectChain,
}
```

### 3. DSP Effects (`core/domain/dsp.rs`)

**Purpose**: Per-channel audio processing

```rust
pub trait Effect {
    fn process(&mut self, buffer: &mut [f32]) -> Result<()>;
    fn reset(&mut self);
    fn bypass(&mut self, enabled: bool);
}
```

---

## 🔒 Concurrency Model

### Architecture: Actor + Async

- **Main Thread (Tokio)** - Handles all async operations
- **Audio Thread** - Real-time audio processing (lock-free)
- **Command Bus** - `tokio::sync::mpsc` for state updates

---

## 🎯 Performance Optimizations

1. **Zero-Copy** - Process buffers in-place
2. **Lock-Free Audio Path** - No mutex in audio thread
3. **SIMD** - Use SIMD for bulk operations
4. **Pre-allocated Buffers** - No allocations in hot path

---

*Last updated: 2025-01-14*
