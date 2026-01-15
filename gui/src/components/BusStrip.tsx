import { useState, useEffect, useCallback, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { VolumeFader } from "./VolumeFader";

interface DeviceInfo {
  id: string;
  name: string;
  device_type: string;
  max_channels: number;
}

interface BusInfo {
  id: string;
  name: string;
  output_device: string | null;
  volume_db: number;
  muted: boolean;
}

interface BusStripProps {
  bus: BusInfo;
}

export function BusStrip({ bus }: BusStripProps) {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(bus.output_device);
  const [localVolume, setLocalVolume] = useState(bus.volume_db);
  const [isMuted, setIsMuted] = useState(bus.muted);
  const [isLoading, setIsLoading] = useState(false);

  // Update local volume when bus volume changes (but not during user interaction)
  useEffect(() => {
    setLocalVolume(bus.volume_db);
  }, [bus.volume_db]);

  // Load output devices on mount - memoized
  const loadOutputDevices = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<DeviceInfo[]>("list_output_devices");
        setDevices(result);
      } else {
        // Mock devices for development
        const mockDevices: DeviceInfo[] = [
          { id: "mock-out-1", name: "Speakers (Realtek)", device_type: "Output", max_channels: 2 },
          { id: "mock-out-2", name: "Headphones (USB)", device_type: "Output", max_channels: 2 },
          { id: "mock-out-3", name: "Monitor Speakers", device_type: "Output", max_channels: 2 },
        ];
        setDevices(mockDevices);
      }
    } catch (error) {
      console.error("Failed to load output devices:", error);
    }
  }, []);

  useEffect(() => {
    loadOutputDevices();
  }, [loadOutputDevices]);

  // Memoized handleDeviceChange
  const handleDeviceChange = useCallback(async (deviceId: string) => {
    setIsLoading(true);
    const newDevice = deviceId === "none" ? null : deviceId;

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("set_bus_output_device", {
          busId: bus.id,
          deviceId: newDevice,
        });
      } else {
        console.log(`Mock: Set bus ${bus.id} output device to`, newDevice);
      }

      setSelectedDevice(newDevice);
    } catch (error) {
      console.error("Failed to set bus output device:", error);
    } finally {
      setIsLoading(false);
    }
  }, [bus.id]);

  // Memoized handleVolumeChange
  const handleVolumeChange = useCallback(async (newVolume: number) => {
    // Update local state immediately for responsive UI
    setLocalVolume(newVolume);

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("set_bus_volume", {
          busId: bus.id,
          volumeDb: newVolume,
        });
      } else {
        console.log(`Mock: Set bus ${bus.id} volume to`, newVolume);
      }
    } catch (error) {
      console.error("Failed to set bus volume:", error);
      // Revert on error
      setLocalVolume(bus.volume_db);
    }
    // No delay needed - local state is authoritative during interaction
  }, [bus.id, bus.volume_db]);

  // Memoized handleMuteToggle
  const handleMuteToggle = useCallback(async () => {
    const newMutedState = !isMuted;
    setIsMuted(newMutedState);

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("toggle_bus_mute", {
          busId: bus.id,
        });
      } else {
        console.log(`Mock: Toggle bus ${bus.id} mute to`, newMutedState);
      }
    } catch (error) {
      console.error("Failed to toggle bus mute:", error);
      // Revert on error
      setIsMuted(bus.muted);
    }
  }, [bus.id, bus.muted, isMuted]);

  // Memoized handlePresetVolume
  const handlePresetVolume = useCallback((presetDb: number) => {
    setLocalVolume(presetDb);
    handleVolumeChange(presetDb);
  }, [handleVolumeChange]);

  return (
    <div className="bg-slate-800 rounded-xl border border-slate-700 p-4 hover:border-blue-500/50 transition-all duration-200">
      {/* Bus Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <h3 className="text-lg font-bold text-white">{bus.name}</h3>
          <span className="px-2 py-0.5 bg-slate-700 text-slate-300 text-xs rounded">
            Bus
          </span>
        </div>
        <div className="text-xs text-slate-400">
          {isMuted ? (
            <span className="text-yellow-500 font-medium">MUTED</span>
          ) : (
            <span>{localVolume.toFixed(1)} dB</span>
          )}
        </div>
      </div>

      {/* Volume Fader (Studio One Style) */}
      <div className="mb-3">
        <label className="text-[10px] font-medium text-slate-400 mb-1.5 block">Volume</label>

        {/* Custom VolumeFader */}
        <div className="flex justify-center mb-3">
          <VolumeFader
            value={localVolume}
            onChange={handleVolumeChange}
            min={-60}
            max={18}
            disabled={isMuted}
          />
        </div>

        {/* dB Presets */}
        <div className="flex gap-2 mb-3">
          {[-6, -12, -18].map((db) => (
            <button
              key={db}
              onClick={() => handlePresetVolume(db)}
              disabled={isMuted}
              className={`
                flex-1 px-1.5 py-1 text-[10px] rounded font-medium transition-all duration-200
                ${isMuted
                  ? 'opacity-50 cursor-not-allowed bg-slate-700 text-slate-500'
                  : 'bg-slate-700 text-slate-300 hover:bg-slate-600 hover:text-white'
                }
              `}
            >
              {db}
            </button>
          ))}
        </div>

        {/* Mute Button */}
        <button
          onClick={handleMuteToggle}
          className={`
            w-full px-3 py-1.5 rounded-lg font-medium transition-all duration-200
            ${isMuted
              ? 'bg-yellow-500/20 text-yellow-500 border-2 border-yellow-500 hover:bg-yellow-500/30'
              : 'bg-slate-700 text-slate-300 hover:bg-slate-600 hover:text-white border-2 border-transparent'
            }
          `}
        >
          {isMuted ? '🔇 Unmute' : '🔊 Mute'}
        </button>
      </div>

      {/* Device Selection */}
      <div className="space-y-2">
        <label className="text-xs font-medium text-slate-400">Output Device</label>
        <div className="relative">
          <select
            value={selectedDevice || "none"}
            onChange={(e) => handleDeviceChange(e.target.value)}
            disabled={isLoading}
            className={`
              w-full bg-slate-900 text-white text-sm rounded-lg border border-slate-600
              px-3 py-2 pr-8 appearance-none cursor-pointer
              hover:border-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500
              disabled:opacity-50 disabled:cursor-not-allowed
              transition-all duration-200
            `}
          >
            <option value="none">No Device</option>
            {devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name} ({device.max_channels}ch)
              </option>
            ))}
          </select>

          {/* Custom arrow icon */}
          <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
            <svg
              className="w-4 h-4 text-slate-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 9l-7 7-7-7"
              />
            </svg>
          </div>
        </div>

        {/* Device info */}
        {selectedDevice && (
          <div className="flex items-center gap-2 text-xs text-slate-400">
            <svg
              className="w-3 h-3 text-green-500"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                clipRule="evenodd"
              />
            </svg>
            <span>
              {devices.find((d) => d.id === selectedDevice)?.name || "Unknown device"}
            </span>
          </div>
        )}

        {!selectedDevice && (
          <div className="flex items-center gap-2 text-xs text-slate-500">
            <svg
              className="w-3 h-3"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                clipRule="evenodd"
              />
            </svg>
            <span>No output device selected</span>
          </div>
        )}
      </div>
    </div>
  );
}

// Memoize BusStrip to prevent unnecessary re-renders
export const MemoizedBusStrip = memo(BusStrip, (prevProps, nextProps) => {
  return (
    prevProps.bus.id === nextProps.bus.id &&
    prevProps.bus.name === nextProps.bus.name &&
    prevProps.bus.output_device === nextProps.bus.output_device &&
    prevProps.bus.volume_db === nextProps.bus.volume_db &&
    prevProps.bus.muted === nextProps.bus.muted
  );
});
