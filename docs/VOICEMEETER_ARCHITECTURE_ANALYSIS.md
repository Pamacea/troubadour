# 🎼 Troubadour - Voicemeeter-Inspired Architecture Design

## 📋 Analysis Summary

Based on research, Voicemeeter's success comes from its **virtual mixing console** architecture. Here's how we'll adapt it for Troubadour.

---

## 🏗️ Voicemeeter Architecture Key Concepts

### 1. Virtual Device Layer
```
┌─────────────────────────────────────────────────┐
│            Windows/Applications                 │
└─────────────────────────────────────────────────┘
                    ↓
        ┌───────────────────────┐
        │  Virtual Audio Driver │  ← Appears as real hardware
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   Voicemeeter Mixer   │  ← Processing happens here
        │  - Input Strips       │
        │  - Buses (A1, A2, A3) │
        │  - Virtual Outputs   │
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │  Hardware Audio API   │  ← WDM/KS/MME/ASIO
        └───────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   Physical Devices    │  ← DAC, ADC, Speakers
        └───────────────────────┘
```

### 2. Input/Output Model

**Voicemeeter Standard (3x3)**:
- Inputs: 2 Hardware + 1 Virtual
- Outputs: 2 Hardware + 1 Virtual
- Buses: 2 (A1, A2)

**Voicemeeter Potato (8x8)**:
- Inputs: 5 Hardware + 3 Virtual
- Outputs: 8 possible BUS outputs
- Full 8×8 matrix routing

### 3. Strip Processing Chain

Each input strip has:
```
Input → Gain → EQ → Gate → Compressor → Routing → Bus Output
        ↓
     Metering
```

### 4. Bus Processing Chain

Each output bus has:
```
Mixed Inputs → Bus EQ → Limiter → Peak Remover → Master Gain → Output
```

---

## 🎯 Troubadour Architecture Design

### Phase 1: Virtual Device Driver (Future)

**Note**: Creating a virtual audio driver is complex and OS-specific:
- **Windows**: Requires WDM driver development (kernel-mode)
- **Linux**: Can use ALSA virtual devices
- **macOS**: Requires CoreAudio virtual driver

**Recommendation**: Start with **physical device mixing only**, add virtual devices later.

### Phase 2: Hardware Mixing Architecture (Current Focus)

```
┌─────────────────────────────────────────────────────┐
│                  Troubadour GUI                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │ Hardware │  │ Hardware │  │ Hardware │          │
│  │ Input 1  │  │ Input 2  │  │ Input 3  │  ...     │
│  │ Strip    │  │ Strip    │  │ Strip    │          │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘          │
│       │             │             │                 │
│       └─────────────┴─────────────┘                 │
│                     ↓                               │
│            ┌───────────────┐                        │
│            │ Mixing Engine │                        │
│            │ (Rust Backend)│                        │
│            └───────┬───────┘                        │
│                    ↓                                │
│         ┌────────────────────┐                     │
│         │ Output Selection   │                     │
│         │ ┌────┐ ┌────┐ ┌────┐│                     │
│         │ │ A1 │ │ A2 │ │ A3 ││  ← Bus Outputs      │
│         │ └────┘ └────┘ └────┘│                     │
│         └────────────────────┘                     │
└─────────────────────────────────────────────────────┘
```

### Phase 3: Multi-Device Input/Output Selection

**Key Features Needed**:

1. **Hardware Input Selection** (Per Channel)
   ```
   Channel 1: [Microphone (Realtek) ▼]
   Channel 2: [Line In (Focusrite) ▼]
   Channel 3: [USB Audio Interface ▼]
   ```

2. **Hardware Output Selection** (Per Bus)
   ```
   Bus A (Master): [Speakers (Realtek) ▼]
   Bus B (Headphones): [Headphones (USB) ▼]
   Bus C (Streaming): [Virtual Audio Cable ▼]
   ```

3. **Routing Matrix** (Any Input → Any Output)
   ```
           │ A1 │ A2 │ A3 │
   ────────┼────┼────┼────┤
   Input 1 │ ✓  │ ✓  │    │
   Input 2 │ ✓  │    │ ✓  │
   Input 3 │    │ ✓  │ ✓  │
   ```

---

## 🔧 Implementation Plan

### Step 1: Extend MixerChannel for Device Assignment

**Current**:
```rust
pub struct MixerChannel {
    id: ChannelId,
    name: String,
    volume_db: f32,
    muted: bool,
    solo: bool,
}
```

**Enhanced**:
```rust
pub struct MixerChannel {
    id: ChannelId,
    name: String,
    volume_db: f32,
    muted: bool,
    solo: bool,
    input_device: Option<DeviceId>,  // NEW: Hardware input device
    output_buses: Vec<BusId>,         // NEW: Which buses this feeds
    // ... DSP settings (EQ, comp, gate)
}
```

### Step 2: Add Bus Structure

```rust
pub struct Bus {
    id: BusId,
    name: String,
    output_device: DeviceId,
    volume_db: f32,
    muted: bool,
}

pub enum BusId {
    A1,
    A2,
    A3,
    Custom(String),
}
```

### Step 3: Update MixerEngine

```rust
pub struct MixerEngine {
    channels: Vec<MixerChannel>,
    buses: Vec<Bus>,                // NEW: Output buses
    routing: RoutingMatrix,         // ENHANCED: Input → Bus routing
    sample_rate: SampleRate,
}

impl MixerEngine {
    // Assign hardware input device to channel
    pub fn assign_input_device(&mut self, channel_id: &ChannelId, device_id: &DeviceId) -> Result<()>;

    // Assign hardware output device to bus
    pub fn assign_output_device(&mut self, bus_id: &BusId, device_id: &DeviceId) -> Result<()>;

    // Route channel to specific bus
    pub fn set_route_to_bus(&mut self, channel_id: &ChannelId, bus_id: &BusId, enabled: bool) -> Result<()>;
}
```

### Step 4: GUI Updates

**MixerPanel Component**:
```tsx
<div className="channel-strip">
  <select>Hardware Input Device</select>
  <div className="routing-matrix">
    <label>☐ Bus A1</label>
    <label>☐ Bus A2</label>
    <label>☐ Bus A3</label>
  </div>
  <VolumeFader />
  <MuteSoloButtons />
</div>
```

**BusPanel Component** (NEW):
```tsx
<div className="bus-panel">
  <h2>Output Buses</h2>
  <BusStrip name="A1" device={selectedDeviceA1} />
  <BusStrip name="A2" device={selectedDeviceA2} />
  <BusStrip name="A3" device={selectedDeviceA3} />
</div>
```

---

## 📊 Comparison: Troubadour vs Voicemeeter

| Feature                | Voicemeeter Standard | Voicemeeter Potato | Troubadour (Current) | Troubadour (Planned) |
|------------------------|---------------------|-------------------|---------------------|---------------------|
| **Hardware Inputs**    | 2                   | 5                 | 0 (not implemented) | 8                   |
| **Virtual Inputs**     | 1                   | 3                 | 0                   | 0 (Phase 2)          |
| **Hardware Outputs**   | 2                   | 8                 | 0 (not implemented) | 8                   |
| **Virtual Outputs**    | 1                   | 3                 | 0                   | 0 (Phase 2)          |
| **Buses**              | 2 (A1, A2)          | 8 (A1-A8)        | 1 (Master only)     | 8 (A1-A8)           |
| **Matrix Routing**     | ✓                   | ✓ (8×8)           | ✓                   | ✓ (8×8)             |
| **DSP Effects**        | Basic EQ            | Advanced          | Backend exists      | Full UI             |
| **Virtual Drivers**    | ✓                   | ✓                 | ✗                   | Phase 2             |

---

## 🚀 Development Priority

### High Priority (MVP - v0.2.0)
1. ✅ Mixer engine with channels
2. ✅ Volume, mute, solo controls
3. ✅ Presets save/load
4. 🔲 **Assign hardware input devices to channels**
5. 🔲 **Assign hardware output devices to buses**
6. 🔲 **Bus A1, A2, A3 outputs**
7. 🔲 **Routing matrix UI**

### Medium Priority (v0.3.0)
8. 🔲 Real-time audio streaming
9. 🔲 DSP UI controls (EQ, compressor)
10. 🔲 Level metering with real audio
11. 🔲 Device hot-plug detection

### Low Priority (v1.0.0)
12. 🔲 Virtual audio drivers
13. 🔲 Application audio capture
14. 🔲 Advanced routing (8×8 matrix)
15. 🔲 Macro buttons

---

## 💡 Key Insights from Voicemeeter

### What Voicemeeter Does Right

1. **Virtual Device Abstraction**
   - Applications don't know they're routing through Voicemeeter
   - Seamless integration with Windows audio stack

2. **Bus Architecture**
   - Separate monitor mix vs. recording mix
   - Independent processing per bus
   - Flexible routing

3. **Strip-Based UI**
   - Each input has its own strip
   - Clear visual hierarchy
   - Easy to understand workflow

4. **Hardware Outs**
   - Can send different mixes to different physical outputs
   - Simultaneous streaming + monitoring

### What We Can Improve

1. **Modern UI**
   - Voicemeeter's UI is dated (Windows 95 style)
   - Troubadour can have a modern, clean interface

2. **Cross-Platform**
   - Voicemeeter is Windows-only
   - Troubadour: Windows + Linux + macOS

3. **Open Source**
   - Voicemeeter is proprietary
   - Troubadour: Community-driven development

4. **Better DSP**
   - Voicemeeter's DSP is basic
   - Troubadour can use modern Rust DSP libraries

---

## 🎯 Next Steps

1. **Implement Bus Structure** - Add `Bus` type to mixer domain
2. **Device Assignment** - Allow assigning devices to channels/buses
3. **Multi-Output Streaming** - Handle multiple output devices simultaneously
4. **Routing UI** - Visual matrix for input→bus routing
5. **Testing** - Test with real hardware devices

---

**Sources**:
- [Voicemeeter Virtual Inputs and Outputs Guide](https://voicemeeter.com/quick-tips-voicemeeter-virtual-inputs-and-outputs-windows-10-and-up/)
- [Voicemeeter User Manual (PDF)](https://vb-audio.com/Voicemeeter/Voicemeeter_UserManual.pdf)
- [Voicemeeter Potato Manual (PDF)](https://vb-audio.com/Voicemeeter/VoicemeeterPotato_UserManual.pdf)
- [Mix-Down and Mix-Up: The VoiceMeeter Bus Modes](https://voicemeeter.com/mix-down-and-mix-up-the-voicemeeter-bus-modes/)
- [VB-Audio Official Website](https://vb-audio.com/Voicemeeter/)
