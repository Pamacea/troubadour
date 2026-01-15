# 🌍 Cross-Platform Building for Troubadour

## 📋 Overview

Troubadour is designed to run on **Windows, Linux, and macOS** with native performance on each platform.

**Current Status**:

- ✅ **Windows**: Native builds (MSVC or GNU)
- ✅ **Linux**: Native builds (ALSA/PulseAudio)
- ✅ **macOS**: Native builds (CoreAudio)

---

## 🎯 Platform Strategy

### Development Platform

**Recommend**: Develop on your primary platform, use CI/CD for others.

| Primary Platform | Build Target                                              | Docker/VM Needed? |
|------------------|-----------------------------------------------------------|-------------------|
| **Windows**      | ✅ Windows (native), ❌ Linux (Docker), ❌ macOS (needs Mac) |
| **Linux**        | ✅ Linux (native), ✅ Windows (cross), ❌ macOS (needs Mac)  |
| **macOS**        | ✅ macOS (native), ✅ Windows (cross), ✅ Linux (native)     |

---

## 🪟 Windows Builds

### Option 1: Native MSVC Build (RECOMMENDED)

**Prerequisites**:

- Microsoft C++ Build Tools (see `fix-windows-linker.md`)
- Rust toolchain: `stable-x86_64-pc-windows-msvc`

**Build**:

```cmd
cargo build --release
```

**Output**: `target/release/troubadour.exe`

### Option 2: Native GNU Build (MinGW)

**Prerequisites**:

- MinGW-w64 or MSYS2
- Rust toolchain: `stable-x86_64-pc-windows-gnu`

**Switch Toolchain**:

```cmd
rustup default stable-x86_64-pc-windows-gnu
```

**Build**:

```cmd
cargo build --release
```

### Option 3: Cross-Compile from Linux

**Install Toolchain**:

```bash
# Ubuntu/Debian
sudo apt install mingw-w64

# Install target
rustup target add x86_64-pc-windows-gnu
```

**Build**:

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

**⚠️ Limitation**: Won't work for Tauri GUI (needs WiX tools on Windows).

---

## 🐧 Linux Builds

### Native Build

**Prerequisites**:

- ALSA development files: `libasound2-dev`
- PulseAudio (optional): `libpulse-dev`
- GTK/QT dev (for GUI)

**Install**:

```bash
# Ubuntu/Debian
sudo apt install build-essential libasound2-dev libpulse-dev

# Fedora
sudo dnf install alsa-lib-devel pulseaudio-libs-devel

# Arch Linux
sudo pacman -S alsa-lib pulseaudio
```

**Build**:

```bash
cargo build --release
```

**Output**: `target/release/troubadour`

### Cross-Compile from macOS

**Use Docker**:

```bash
# Run Linux container
docker run --rm -v $(pwd):/app -w /app \
  rust:latest \
  cargo build --release
```

### Cross-Compile from Windows (WSL)

**In WSL2 Ubuntu**:

```bash
sudo apt update
sudo apt install build-essential libasound2-dev

cargo build --release
```

---

## 🍎 macOS Builds

### Native Build

**Prerequisites**:

- Xcode Command Line Tools

```bash
xcode-select --install
```

**Build**:

```bash
cargo build --release
```

**Output**: `target/release/troubadour`

### Universal Binary (Apple Silicon + Intel)

**Install Targets**:

```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

**Build Universal**:

```bash
# Build Intel
cargo build --release --target x86_64-apple-darwin

# Build Apple Silicon
cargo build --release --target aarch64-apple-darwin

# Create universal binary
lipo -create -output target/release/troubadour-universal \
  target/x86_64-apple-darwin/release/troubadour \
  target/aarch64-apple-darwin/release/troubadour
```

### Code Signing (Required for Distribution)

**Create Certificate**:

1. Open **Keychain Access**
2. **Certificate Assistant** → **Create a Certificate**
3. Developer ID Application

**Sign Binary**:

```bash
codesign --deep --force --verify --verbose \
  --sign "Developer ID Application: Your Name" \
  target/release/troubadour
```

**Verify**:

```bash
codesign -dv target/release/troubadour
spctl -a -vvv target/release/troubadour
```

---

## 📦 Creating Installers

### Windows Installer (MSI/EXE)

**Option 1: Tauri Bundler (RECOMMENDED)**

```bash
cargo install tauri-cli
cargo tauri build
```

**Output**: `src-tauri/target/release/bundle/msi/`

**Option 2: WiX Toolset**

```xml
<!-- installer.wxs -->
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="*" Name="Troubadour" UpgradeCode="...">
        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="ProgramFilesFolder">
                <Directory Id="INSTALLFOLDER" Name="Troubadour"/>
            </Directory>
        </Directory>
    </Product>
</Wix>
```

```cmd
candle installer.wxs
light -out Troubadour.msi installer.wixobj
```

### Linux Packages

**Debian/Ubuntu (.deb)**:

```bash
cargo install cargo-deb
cargo deb
```

**Red Hat/Fedora (.rpm)**:

```bash
cargo install cargo-rpm
cargo generate-rpm
```

**AppImage (Universal Linux)**:

```bash
# Install appimage-builder
pip3 install appimage-builder

# Build AppImage
appimage-builder
```

### macOS Disk Image (.dmg)

**Option 1: Tauri Bundler**

```bash
cargo tauri build
```

**Output**: `src-tauri/target/release/bundle/dmg/`

**Option 2: Manual DMG**

```bash
# Create DMG
hdiutil create -volname "Troubadour" \
  -srcfolder target/release/ \
  -ov -format UDZO \
  Troubadour.dmg
```

---

## 🚀 CI/CD for Multi-Platform Builds

### GitHub Actions Example

```yaml
name: Build Troubadour

on: [ push, pull_request ]

jobs:
  build:
    strategy:
      matrix:
        os: [ windows-latest, ubuntu-latest, macos-latest ]
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: troubadour.exe
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: troubadour
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: troubadour

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: release
          toolchain: stable
          target: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Upload Artifact
        uses: actions/upload-artifact@v3
        with:
          name: troubadour-${{ matrix.os }}
          path: target/${{ matrix.target }}/release/${{ matrix.artifact }}
```

---

## 🔧 Platform-Specific Dependencies

### Audio Libraries

| Platform    | Library    | Rust Crate          |
|-------------|------------|---------------------|
| **Windows** | WASAPI     | `cpal` (built-in)   |
| **Linux**   | ALSA       | `cpal` + `alsa-sys` |
| **Linux**   | PulseAudio | `cpal` + `libpulse` |
| **macOS**   | CoreAudio  | `cpal` (built-in)   |

### Conditional Compilation

```rust
#[cfg(target_os = "windows")]
mod windows_backend;

#[cfg(target_os = "linux")]
mod linux_backend;

#[cfg(target_os = "macos")]
mod macos_backend;
```

---

## ✅ Verification Checklist

Before releasing, verify on each platform:

### Windows

- [ ] Builds with MSVC
- [ ] Audio devices enumerate correctly
- [ ] WASAPI streams work
- [ ] Installer (MSI) installs and runs
- [ ] Code signature valid

### Linux

- [ ] Builds on Ubuntu 20.04+
- [ ] ALSA devices enumerate
- [ ] PulseAudio works (if available)
- [ ] .deb package installs
- [ ] AppImage runs standalone

### macOS

- [ ] Builds on Intel Mac
- [ ] Builds on Apple Silicon (M1/M2)
- [ ] Universal binary works
- [ ] CoreAudio devices enumerate
- [ .dmg opens and installs
- [ ] Code signature valid
- [ ] Notarized (for distribution)

---

## 📊 Performance Comparison

| Platform               | Typical Latency | CPU Usage | Notes                     |
|------------------------|-----------------|-----------|---------------------------|
| **Windows (WASAPI)**   | 10-15ms         | Low       | Exclusive mode best       |
| **Linux (ALSA)**       | 5-10ms          | Lowest    | Requires real-time kernel |
| **Linux (PulseAudio)** | 15-20ms         | Low       | Easier configuration      |
| **macOS (CoreAudio)**  | 5-10ms          | Low       | Best out-of-box           |

---

## 🆘 Troubleshooting

### Issue: "Audio device not found"

**Windows**: Ensure WASAPI is enabled in Sound Settings
**Linux**: Install `alsa-utils` and run `alsamixer`
**macOS**: Grant microphone permissions in System Preferences

### Issue: "High CPU usage"

**Fix**: Increase buffer size in `AudioConfig`:

```rust
pub const BUFFER_SIZE: usize = 512; // or 1024
```

### Issue: "Audio glitches/dropouts"

**Windows**: Use WASAPI Exclusive Mode
**Linux**: Run with `sudo` or configure `limits.conf`
**macOS**: Increase process priority

---

*Last updated: 2025-01-14*
