# Reference

## Audio Concepts

### Virtual Audio Device
A software-emulated audio device that appears in the OS mixer. Applications can select it as input/output, and Troubadour routes the audio internally.

### Audio Callback
The function called by the OS audio driver at regular intervals to fill/read audio buffers. Must complete within the buffer duration (e.g., 128 samples @ 48kHz = 2.67ms). No allocations, no locks.

### Sample Rate Conversion
When routing audio between devices with different sample rates, `rubato` handles high-quality resampling in real-time.

### Buffer Size
Trade-off between latency and stability:
| Buffer Size | Latency @ 48kHz | Stability |
|------------|-----------------|-----------|
| 64 samples | 1.3ms | Requires fast CPU |
| 128 samples | 2.7ms | Recommended |
| 256 samples | 5.3ms | Safe default |
| 512 samples | 10.7ms | Maximum stability |

## Key Dependencies

### troubadour-core

| Crate | Purpose | Docs |
|-------|---------|------|
| `cpal` | Cross-platform audio I/O | [docs.rs/cpal](https://docs.rs/cpal) |
| `dasp` | Audio DSP primitives (samples, buffers, conversion) | [docs.rs/dasp](https://docs.rs/dasp) |
| `rubato` | Asynchronous sample rate conversion | [docs.rs/rubato](https://docs.rs/rubato) |
| `crossbeam-channel` | Lock-free MPMC channels | [docs.rs/crossbeam-channel](https://docs.rs/crossbeam-channel) |

### troubadour-ui

| Crate | Purpose | Docs |
|-------|---------|------|
| `dioxus` | Reactive UI framework (desktop mode) | [dioxuslabs.com](https://dioxuslabs.com) |

### troubadour-shared

| Crate | Purpose | Docs |
|-------|---------|------|
| `serde` | Serialization/deserialization | [docs.rs/serde](https://docs.rs/serde) |
| `toml` | TOML config file parsing | [docs.rs/toml](https://docs.rs/toml) |

## IPC Message Types

### UI -> Core (Commands)
| Message | Description |
|---------|-------------|
| `SetVolume(channel, f32)` | Set channel volume (0.0 - 1.0) |
| `SetMute(channel, bool)` | Mute/unmute a channel |
| `SetRoute(source, dest)` | Connect audio route |
| `RemoveRoute(source, dest)` | Disconnect audio route |
| `SetDevice(role, device_id)` | Select audio device |
| `SetBufferSize(u32)` | Change buffer size |
| `SetSampleRate(u32)` | Change sample rate |

### Core -> UI (Events)
| Message | Description |
|---------|-------------|
| `LevelUpdate(channel, f32)` | Current audio level (for VU-meters) |
| `DeviceList(Vec<Device>)` | Available audio devices |
| `DeviceError(String)` | Device error notification |
| `EngineState(State)` | Engine running/stopped/error |

## Configuration File Format

```toml
[audio]
sample_rate = 48000
buffer_size = 256
backend = "wasapi"  # wasapi | coreaudio | alsa | pipewire

[channels.input]
count = 3
names = ["Mic", "Desktop", "Browser"]

[channels.output]
count = 2
names = ["Headphones", "Speakers"]

[routing]
# source -> destination pairs
routes = [
    { from = "Mic", to = "Headphones" },
    { from = "Desktop", to = "Headphones" },
    { from = "Desktop", to = "Speakers" },
]

[ui]
theme = "dark"
vu_meter_fps = 30
```
