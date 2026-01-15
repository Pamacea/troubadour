# 🛠️ Troubadour - Development Guide

## 🚀 Quick Start

### Prerequisites

| Platform    | Requirements               |
|-------------|----------------------------|
| **Windows** | Windows 10+, Rust 1.75+    |
| **Linux**   | ALSA dev files, PulseAudio |
| **macOS**   | Xcode command-line tools   |

### Installation

```bash
# Clone repository
git clone https://github.com/Yanis/troubadour.git
cd troubadour

# Build project
cargo build --release

# Run
cargo run --release
```

---

## 🔧 Development Workflow

### Watch Mode

```bash
# Watch for changes and rebuild
cargo watch -x build -x test -x clippy
```

### Testing

```bash
# Run all tests
cargo nextest run

# Run with output
cargo test -- --nocapture
```

### Linting

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy -- -D warnings
```

---

## 📐 Coding Standards

### Key Rules

- ✅ Use `Result<T, E>` for fallible operations
- ✅ Use `thiserror` for error types
- ✅ Avoid `unwrap()` in production
- ✅ Prefer `iter()` over loops
- ✅ Use `#[instrument]` from `tracing`

### Example

```rust
// ❌ Bad
fn process(input: Vec<f32>) -> Vec<f32> {
    input.iter().map(|x| x * 2.0).collect()
}

// ✅ Good
fn process(buffer: &mut [f32]) {
    for sample in buffer.iter_mut() {
        *sample *= 2.0;
    }
}
```

---

## 🧪 Testing Guidelines

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_setter_clamps_values() {
        let mut channel = MixerChannel::new("Test");
        channel.set_volume(100.0);
        assert_eq!(channel.volume(), 6.0);
    }
}
```

---

## 📚 Resources

- [MASTERPLAN.md](./MASTERPLAN.md) - Project overview
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical architecture
- [PLAN.md](./PLAN.md) - Development roadmap

---

*Last updated: 2025-01-14*
