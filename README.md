# Troubadour

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> A modern, full-Rust virtual audio mixer. The Voicemeeter alternative you've been waiting for.

## Why Troubadour?

Voicemeeter is powerful but painful: manual configuration, clunky UI, no automation. Troubadour reimagines the virtual audio mixer with a clean architecture, modern interface, and zero compromise on performance.

## Architecture

```
┌─────────────────────────────────────┐
│  troubadour-core    (Audio Engine)  │  cpal, rubato, dasp
│  Real-time DSP, routing, mixing     │  < 5ms latency
└──────────────┬──────────────────────┘
               │ crossbeam channels
┌──────────────┴──────────────────────┐
│  troubadour-ui      (Desktop GUI)   │  Dioxus (native desktop)
│  Faders, VU-meters, routing matrix  │  CSS styling
└──────────────┬──────────────────────┘
               │
┌──────────────┴──────────────────────┐
│  troubadour-shared  (Common Types)  │  serde
│  Config, audio types, IPC messages  │
└─────────────────────────────────────┘
```

## Stack

| Crate | Role | Key Dependencies |
|-------|------|-----------------|
| `troubadour-core` | Audio engine | `cpal`, `rubato`, `dasp` |
| `troubadour-ui` | Desktop interface | `dioxus` (desktop) |
| `troubadour-shared` | Shared types/config | `serde` |

## Development

```bash
cargo build             # Build all crates
cargo run -p troubadour-ui   # Run the desktop app
cargo test              # Run all tests
```

## Tooling

- **Formatter:** oxfmt
- **Linter:** oxlint + clippy
- **CI:** GitHub Actions

## License

[MIT](LICENSE)
