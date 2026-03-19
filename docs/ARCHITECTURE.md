# Architecture

## Overview

Troubadour is a virtual audio mixer built as a Rust workspace with three crates, each with a clear responsibility boundary.

## Crate Structure

```
troubadour/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── troubadour-core/    # Audio engine
│   ├── troubadour-ui/      # Desktop GUI
│   └── troubadour-shared/  # Common types
```

## troubadour-core (Audio Engine)

The heart of Troubadour. Runs on a dedicated high-priority thread, completely decoupled from the UI.

**Responsibilities:**
- Audio device enumeration and management (via `cpal`)
- Real-time audio routing between virtual and physical devices
- DSP processing: mixing, gain, EQ, effects
- Sample rate conversion (via `rubato`)
- Audio buffer management (via `dasp`)

**Constraints:**
- No heap allocations in the audio callback
- No mutex locks in the hot path
- Target latency: < 5ms
- Lock-free communication with UI via crossbeam channels

## troubadour-ui (Desktop Interface)

Desktop application built with Dioxus in native desktop mode.

**Responsibilities:**
- Faders, knobs, VU-meters
- Audio routing matrix visualization
- Device configuration panel
- Settings and preset management

**Communication with core:**
- Sends: volume changes, routing changes, device selection
- Receives: audio levels (for VU-meters), device state updates

## troubadour-shared (Common Types)

Shared types between core and UI. No logic, only data structures.

**Contains:**
- Audio configuration types (sample rate, buffer size, channels)
- IPC message types (commands from UI, events from core)
- Serializable config structures (via `serde`)
- Error types

## Data Flow

```
User Action (UI)
    │
    ▼
Command (troubadour-shared)
    │
    ▼ crossbeam channel
Audio Engine (troubadour-core)
    │
    ▼ processes audio
Level Update (troubadour-shared)
    │
    ▼ crossbeam channel
VU-meter refresh (UI)
```

## Threading Model

```
┌─────────────────────┐
│ Main Thread          │  Dioxus UI event loop
├─────────────────────┤
│ Audio Thread         │  cpal callback, real-time priority
├─────────────────────┤
│ Engine Thread        │  Command processing, state management
└─────────────────────┘
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Separate crates | Enforce boundary between real-time audio and UI |
| Dioxus (desktop) | CSS flexibility for modern UI, React-like DX |
| crossbeam channels | Lock-free, bounded channels for real-time safety |
| No shared mutable state | All communication through message passing |
