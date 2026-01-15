import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MixerChannel } from "./MixerChannel";

interface DeviceInfo {
  id: string;
  name: string;
  device_type: string;
  max_channels: number;
}

interface ChannelInfo {
  id: string;
  name: string;
  volume_db: number;
  muted: boolean;
  solo: boolean;
  level_db: number;
  peak_db: number;
}

export function MixerPanel() {
  const [channels, setChannels] = useState<ChannelInfo[]>([]);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [showDeviceInfo, setShowDeviceInfo] = useState(false);

  // Load devices and channels on mount
  useEffect(() => {
    loadDevices();
    loadChannels();
    // Refresh channels every 100ms for level meters
    const interval = setInterval(loadChannels, 100);
    return () => clearInterval(interval);
  }, []);

  async function loadDevices() {
    try {
      // Check if running in Tauri context
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<DeviceInfo[]>("list_audio_devices");
        console.log("Loaded devices:", result);
        setDevices(result);
        if (result.length > 0 && !selectedDevice) {
          setSelectedDevice(result[0].id);
        }
      } else {
        console.warn("Not running in Tauri context - using mock devices");
        // Mock devices for development
        const mockDevices: DeviceInfo[] = [
          { id: "mock-1", name: "Microphone (Realtek)", device_type: "Input", max_channels: 2 },
          { id: "mock-2", name: "Speakers (Realtek)", device_type: "Output", max_channels: 2 },
          { id: "mock-3", name: "Headphones (USB)", device_type: "Output", max_channels: 2 },
        ];
        setDevices(mockDevices);
        if (!selectedDevice) {
          setSelectedDevice(mockDevices[0].id);
        }
      }
    } catch (error) {
      console.error("Failed to load devices:", error);
      // Set empty array to prevent infinite loading
      setDevices([]);
      setLoading(false);
    }
  }

  async function loadChannels() {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = (await invoke<ChannelInfo[]>("get_channels")) || [];
        setChannels(result);
      } else {
        // Mock channels for development
        const mockChannels: ChannelInfo[] = [
          { id: "input-1", name: "Input 1", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60 },
          { id: "input-2", name: "Input 2", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60 },
          { id: "input-3", name: "Input 3", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60 },
          { id: "master", name: "Master", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60 },
        ];
        setChannels(mockChannels);
      }
      setLoading(false);
    } catch (error) {
      console.error("Failed to load channels:", error);
      setLoading(false);
    }
  }

  async function handleVolumeChange(channelId: string, volumeDb: number) {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("set_volume", { channelId, volumeDb });
      } else {
        console.log(`Mock: Set volume for ${channelId} to ${volumeDb} dB`);
      }
      // Optimistic update
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, volume_db: volumeDb } : ch))
      );
    } catch (error) {
      console.error("Failed to set volume:", error);
    }
  }

  async function handleToggleMute(channelId: string) {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("toggle_mute", { channelId });
      } else {
        console.log(`Mock: Toggle mute for ${channelId}`);
      }
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, muted: !ch.muted } : ch))
      );
    } catch (error) {
      console.error("Failed to toggle mute:", error);
    }
  }

  async function handleToggleSolo(channelId: string) {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("toggle_solo", { channelId });
      } else {
        console.log(`Mock: Toggle solo for ${channelId}`);
      }
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, solo: !ch.solo } : ch))
      );
    } catch (error) {
      console.error("Failed to toggle solo:", error);
    }
  }

  async function handleAddChannel() {
    const id = `channel-${Date.now()}`;
    const name = `Channel ${channels.length + 1}`;
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("add_channel", { channelId: id, name });
        await loadChannels();
      } else {
        console.log(`Mock: Add channel ${id} (${name})`);
        const newChannel: ChannelInfo = {
          id,
          name,
          volume_db: 0,
          muted: false,
          solo: false,
          level_db: -60,
          peak_db: -60,
        };
        setChannels([...channels, newChannel]);
      }
    } catch (error) {
      console.error("Failed to add channel:", error);
      alert("Failed to add channel: " + error);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full bg-slate-900">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-4"></div>
          <div className="text-slate-400">Loading mixer...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-gradient-to-br from-slate-900 via-slate-900 to-slate-950">
      {/* Top Bar - Device Selection */}
      <div className="bg-slate-800 border-b border-slate-700 px-6 py-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h2 className="text-lg font-semibold text-white">Audio Mixer</h2>

            {/* Device Selector */}
            <div className="flex items-center gap-2">
              <label className="text-sm text-slate-400">Audio Device:</label>
              <select
                value={selectedDevice}
                onChange={(e) => setSelectedDevice(e.target.value)}
                className="bg-slate-700 text-white text-sm rounded px-3 py-1.5 border border-slate-600 focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[200px]"
              >
                {devices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {device.name} ({device.max_channels}ch)
                  </option>
                ))}
              </select>

              <button
                onClick={() => setShowDeviceInfo(!showDeviceInfo)}
                className="text-slate-400 hover:text-slate-200 p-1"
                title="Device info"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 9h6m-6 0h.01" />
                </svg>
              </button>
            </div>
          </div>

          {/* Add Channel Button */}
          <button
            onClick={handleAddChannel}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm font-medium transition-colors"
          >
            + Add Channel
          </button>
        </div>
      </div>

      {/* Device Info Panel (collapsible) */}
      {showDeviceInfo && (
        <div className="bg-slate-800 border-b border-slate-700 px-6 py-3">
          <div className="text-sm text-slate-400">
            Selected: <span className="text-white font-medium"> {devices.find(d => d.id === selectedDevice)?.name || 'None'}</span>
          </div>
        </div>
      )}

      {/* Channel Strips */}
      <div className="flex-1 overflow-x-auto overflow-y-hidden p-8">
        {channels.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <p className="text-slate-400 mb-4">No channels yet</p>
              <button
                onClick={handleAddChannel}
                className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
              >
                Add First Channel
              </button>
            </div>
          </div>
        ) : (
          <div className="flex gap-6 h-full">
            {channels.map((channel) => (
              <MixerChannel
                key={channel.id}
                id={channel.id}
                name={channel.name}
                volumeDb={channel.volume_db}
                muted={channel.muted}
                solo={channel.solo}
                levelDb={channel.level_db}
                peakDb={channel.peak_db}
                onVolumeChange={(vol) => handleVolumeChange(channel.id, vol)}
                onToggleMute={() => handleToggleMute(channel.id)}
                onToggleSolo={() => handleToggleSolo(channel.id)}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
