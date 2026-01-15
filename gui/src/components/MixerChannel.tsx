import { useState, useEffect, useCallback, useMemo, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { VolumeFader } from "./VolumeFader";

interface BusInfo {
  id: string;
  name: string;
  output_device: string | null;
  volume_db: number;
  muted: boolean;
}

interface DeviceInfo {
  id: string;
  name: string;
  device_type: string;
  max_channels: number;
}

interface MixerChannelProps {
  id: string;
  name: string;
  volumeDb: number;
  muted: boolean;
  solo: boolean;
  levelDb: number;
  peakDb: number;
  inputDevice?: string | null;
  onVolumeChange: (volumeDb: number) => void;
  onToggleMute: () => void;
  onToggleSolo: () => void;
  focused?: boolean;
  onFocus?: () => void;
  is_master?: boolean;
}

export function MixerChannel({
  id,
  name,
  volumeDb,
  muted,
  solo,
  levelDb,
  peakDb,
  inputDevice,
  onVolumeChange,
  onToggleMute,
  onToggleSolo,
  focused = false,
  onFocus,
  is_master = false,
}: MixerChannelProps) {
  const [localVolume, setLocalVolume] = useState(volumeDb);
  const [isExpanded, setIsExpanded] = useState(false);
  const [buses, setBuses] = useState<BusInfo[]>([]);
  const [selectedBuses, setSelectedBuses] = useState<string[]>([]);
  const [inputDevices, setInputDevices] = useState<DeviceInfo[]>([]);
  const [selectedInputDevice, setSelectedInputDevice] = useState<string | null>(inputDevice || null);
  const [isLoadingDevice, setIsLoadingDevice] = useState(false);

  // Load buses on mount - only once when id changes
  useEffect(() => {
    loadBuses();
    loadChannelBuses();
    loadInputDevices();
    loadChannelInputDevice();
  }, [id]);

  // Memoized loadBuses to prevent recreation on every render
  const loadBuses = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<BusInfo[]>("get_buses");
        setBuses(result);
      } else {
        // Mock buses for development
        const mockBuses: BusInfo[] = [
          { id: "A1", name: "A1", output_device: null, volume_db: 0, muted: false },
          { id: "A2", name: "A2", output_device: null, volume_db: 0, muted: false },
          { id: "A3", name: "A3", output_device: null, volume_db: 0, muted: false },
        ];
        setBuses(mockBuses);
      }
    } catch (error) {
      console.error("Failed to load buses:", error);
    }
  }, []);

  // Memoized loadChannelBuses
  const loadChannelBuses = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<string[]>("get_channel_buses", { channelId: id });
        setSelectedBuses(result);
      } else {
        // Mock default: route to A1
        setSelectedBuses(["A1"]);
      }
    } catch (error) {
      console.error("Failed to load channel buses:", error);
    }
  }, [id]);

  // Memoized handleBusToggle
  const handleBusToggle = useCallback(async (busId: string) => {
    const newSelection = selectedBuses.includes(busId)
      ? selectedBuses.filter(b => b !== busId)
      : [...selectedBuses, busId];

    setSelectedBuses(newSelection);

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("set_channel_buses", {
          channelId: id,
          busIds: newSelection,
        });
      } else {
        console.log(`Mock: Set channel ${id} buses to`, newSelection);
      }
    } catch (error) {
      console.error("Failed to set channel buses:", error);
    }
  }, [selectedBuses, id]);

  // Memoized loadInputDevices
  const loadInputDevices = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<DeviceInfo[]>("list_audio_devices");
        setInputDevices(result);
      } else {
        // Mock devices for development
        const mockDevices: DeviceInfo[] = [
          { id: "mock-in-1", name: "Microphone (Realtek)", device_type: "Input", max_channels: 2 },
          { id: "mock-in-2", name: "USB Audio Interface", device_type: "Input", max_channels: 2 },
          { id: "mock-in-3", name: "Headset Microphone", device_type: "Input", max_channels: 1 },
        ];
        setInputDevices(mockDevices);
      }
    } catch (error) {
      console.error("Failed to load input devices:", error);
    }
  }, []);

  // Memoized loadChannelInputDevice
  const loadChannelInputDevice = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<string | null>("get_channel_input_device", { channelId: id });
        setSelectedInputDevice(result);
      } else {
        // Mock default: no device selected
        setSelectedInputDevice(null);
      }
    } catch (error) {
      console.error("Failed to load channel input device:", error);
    }
  }, [id]);

  // Memoized handleInputDeviceChange
  const handleInputDeviceChange = useCallback(async (deviceId: string) => {
    setIsLoadingDevice(true);
    const newDevice = deviceId === "default" ? null : deviceId;

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("set_channel_input_device", {
          channelId: id,
          deviceId: newDevice,
        });
      } else {
        console.log(`Mock: Set channel ${id} input device to`, newDevice);
      }

      setSelectedInputDevice(newDevice);
    } catch (error) {
      console.error("Failed to set channel input device:", error);
    } finally {
      setIsLoadingDevice(false);
    }
  }, [id]);

  // Volume range: -60dB to +18dB
  const minVolume = -60;
  const maxVolume = 18;

  // Format dB for display - memoized
  const formatDb = useCallback((db: number) => {
    if (db <= minVolume) return "-∞";
    return `${db.toFixed(1)} dB`;
  }, [minVolume]);

  // Calculate meter height (logarithmic scale) - memoized
  const meterHeight = useMemo(() => {
    if (levelDb <= -60) return 0;
    if (levelDb > 0) return 100;
    // Map -60dB to 0dB → 0% to 90%
    return Math.min(90, ((levelDb + 60) / 60) * 100);
  }, [levelDb]);

  // Peak meter (yellow) - memoized
  const peakHeight = useMemo(() => {
    if (peakDb <= -60) return 0;
    if (peakDb > 0) return 100;
    return Math.min(100, ((peakDb + 60) / 60) * 100);
  }, [peakDb]);

  // Memoized handlePresetVolume
  const handlePresetVolume = useCallback((volumeDb: number) => {
    setLocalVolume(volumeDb);
    onVolumeChange(volumeDb);
  }, [onVolumeChange]);

  return (
    <div
      className={`
        flex flex-col shrink-0 rounded-xl border transition-all duration-200
        w-1/12 h-[min(75vh,40rem)] min-h-1/4 shadow-lg
        ${is_master
          ? "bg-slate-900 border-2 border-blue-600 hover:border-blue-500"
          : "bg-slate-800 border-slate-700 hover:border-blue-500/50"
        }
        ${focused && !is_master
          ? "border-blue-500 ring-2 ring-blue-500/50 shadow-blue-500/20"
          : ""
        }
        ${focused && is_master
          ? "ring-4 ring-blue-600/50 shadow-blue-600/30"
          : ""
        }
      `}
      onClick={onFocus}
      tabIndex={0}
      role="button"
      aria-label={`Channel ${name} ${focused ? "(focused)" : ""}`}
    >
      {/* Channel Header */}
      <div className={`flex items-center justify-between px-2 py-2 border-b ${is_master ? "border-blue-800/50 bg-slate-900" : "border-slate-700"}`}>
        <div className="flex items-center gap-2 flex-1">
          {is_master && (
            <span className="px-1.5 py-0.5 bg-blue-600 text-white text-[9px] font-bold uppercase tracking-wider rounded">
              Master
            </span>
          )}
          <input
            type="text"
            defaultValue={name}
            className={`bg-transparent ${is_master ? "text-lg font-bold text-blue-400" : "text-sm font-medium text-white"} border-none w-full focus:outline-none focus:ring-0`}
          />
        </div>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="text-slate-400 hover:text-white transition-colors"
          title={isExpanded ? "Show less" : "Show more"}
        >
          <svg
            className={`w-4 h-4 transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
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
        </button>
      </div>

      {/* Expanded Section - Bus Selection & EQ */}
      {isExpanded && (
        <div className="px-2 py-1.5 border-b border-slate-700 bg-slate-900">
          {/* Input Device Selection */}
          <div className="mb-3">
            <p className="text-[10px] font-medium text-slate-400 mb-1">Input Device</p>
            <div className="relative">
              <select
                value={selectedInputDevice || "default"}
                onChange={(e) => handleInputDeviceChange(e.target.value)}
                disabled={isLoadingDevice}
                className={`
                  w-full bg-slate-950 text-white text-[10px] rounded-lg border border-slate-600
                  px-2 py-1.5 pr-6 appearance-none cursor-pointer
                  hover:border-slate-500 focus:outline-none focus:ring-1 focus:ring-blue-500
                  disabled:opacity-50 disabled:cursor-not-allowed
                  transition-all duration-200
                `}
              >
                <option value="default">Default Input</option>
                {inputDevices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {device.name} ({device.max_channels}ch)
                  </option>
                ))}
              </select>

              {/* Custom arrow icon */}
              <div className="absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none">
                <svg
                  className="w-3 h-3 text-slate-400"
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
            {selectedInputDevice && (
              <div className="flex items-center gap-1 mt-1 text-[10px] text-slate-400">
                <svg
                  className="w-3 h-3 text-green-500 flex-shrink-0"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
                <span className="truncate">
                  {inputDevices.find((d) => d.id === selectedInputDevice)?.name || "Unknown device"}
                </span>
              </div>
            )}

            {!selectedInputDevice && (
              <div className="flex items-center gap-1 mt-1 text-[10px] text-slate-500">
                <svg
                  className="w-3 h-3 flex-shrink-0"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                >
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-11a1 1 0 10-2 0v3.586L7.707 9.293a1 1 0 00-1.414 1.414l3 3a1 1 0 001.414 0l3-3a1 1 0 00-1.414-1.414L11 10.586V7z"
                    clipRule="evenodd"
                  />
                </svg>
                <span>Using default input device</span>
              </div>
            )}
          </div>

          {/* Bus Selection */}
          <div className="mb-2">
            <p className="text-[10px] font-medium text-slate-400 mb-1">Output Buses</p>
            <div className="flex flex-wrap gap-2">
              {buses.map((bus) => (
                <button
                  key={bus.id}
                  onClick={() => handleBusToggle(bus.id)}
                  className={`
                    px-2 py-1 rounded-md text-[10px] font-medium transition-all
                    ${selectedBuses.includes(bus.id)
                      ? "bg-blue-600 text-white shadow-lg shadow-blue-600/20"
                      : "bg-slate-700 text-slate-300 hover:bg-slate-600"
                    }
                  `}
                >
                  {bus.name}
                </button>
              ))}
            </div>
            {selectedBuses.length === 0 && (
              <p className="text-[10px] text-slate-500 mt-1.5 italic">
                No buses selected - channel will be silent
              </p>
            )}
          </div>

          {/* EQ & Effects placeholder */}
          <div className="pt-3 border-t border-slate-700">
            <p className="text-[10px] text-slate-500 text-center italic">
              EQ & Effects coming soon...
            </p>
          </div>
        </div>
      )}

      {/* Level Meters (Enhanced) */}
      <div className="px-2 py-1.5">
        <div className="flex gap-1 h-32">
          {/* Left Meter */}
          <div className="flex-1 relative">
            <div className="absolute left-0 right-0 top-0 bottom-0 bg-slate-900 rounded-2xl overflow-hidden">
              {/* Peak indicator (yellow) */}
              <div
                className="absolute left-0 right-0 bg-yellow-400 transition-all duration-75"
                style={{ bottom: `${peakHeight}%`, height: "2px" }}
              />
              {/* Current level (green → red gradient) */}
              <div
                className="absolute left-0 right-0 bg-gradient-to-t from-green-500 via-yellow-500 to-red-500 transition-all duration-75"
                style={{ bottom: 0, height: `${meterHeight}%` }}
              />
            </div>
          </div>

          {/* Right Meter */}
          <div className="flex-1 relative">
            <div className="absolute left-0 right-0 top-0 bottom-0 bg-slate-900 rounded-2xl overflow-hidden">
              <div
                className="absolute left-0 right-0 bg-gradient-to-t from-green-500 via-yellow-500 to-red-500 transition-all duration-75"
                style={{ bottom: 0, height: `${meterHeight}%` }}
              />
              <div
                className="absolute left-0 right-0 bg-yellow-400 transition-all duration-75"
                style={{ bottom: `${peakHeight}%`, height: "2px" }}
              />
            </div>
          </div>
        </div>

        {/* Peak value display */}
        <div className="text-center mt-1">
          <span className="text-[10px] font-mono font-bold text-white">
            {formatDb(peakDb)}
          </span>
        </div>
      </div>

      {/* Volume Fader (Studio One Style) */}
      <div className="flex flex-col items-center gap-2 px-2 py-3">
        {/* Volume Display */}
        <span className="text-sm font-bold text-white font-mono">
          {formatDb(localVolume)}
        </span>

        {/* Custom VolumeFader */}
        <VolumeFader
          value={localVolume}
          onChange={onVolumeChange}
          min={minVolume}
          max={maxVolume}
        />

        {/* Volume Presets */}
        <div className="flex gap-1 w-full justify-center text-[10px]">
          <button
            onClick={() => handlePresetVolume(-60)}
            className="flex-1 py-1 text-[10px] font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            ∞
          </button>
          <button
            onClick={() => handlePresetVolume(-6)}
            className="flex-1 py-1 text-[10px] font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            -6
          </button>
          <button
            onClick={() => handlePresetVolume(-12)}
            className="flex-1 py-1 text-[10px] font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            -12
          </button>
          <button
            onClick={() => handlePresetVolume(-18)}
            className="flex-1 py-1 text-[10px] font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            -18
          </button>
        </div>
      </div>

      {/* Routing Matrix - Compact Bus Selection */}
      <div className="px-3 pb-2">
        <p className="text-[10px] font-medium text-slate-400 mb-1 text-center">To Bus</p>
        <div className="flex gap-1">
          {buses.map((bus) => (
            <button
              key={bus.id}
              onClick={() => handleBusToggle(bus.id)}
              className={`
                flex-1 px-1 py-1 rounded text-[10px] font-bold transition-all
                ${selectedBuses.includes(bus.id)
                  ? "bg-blue-600 text-white shadow-md shadow-blue-600/30"
                  : "bg-slate-700 text-slate-400 hover:bg-slate-600"
                }
              `}
              title={`Route to ${bus.name}`}
            >
              {bus.id}
            </button>
          ))}
        </div>
      </div>

      {/* Mute/Solo Buttons (Enhanced) */}
      <div className="flex gap-2 px-3 pb-3">
        <button
          onClick={onToggleMute}
          className={`
            flex-1 py-1.5 rounded font-bold text-xs transition-all
            ${muted
              ? "bg-yellow-600 text-white hover:bg-yellow-700"
              : "bg-slate-700 text-slate-300 hover:bg-slate-600"
            }
          `}
        >
          M
        </button>
        <button
          onClick={onToggleSolo}
          className={`
            flex-1 py-1.5 rounded font-bold text-xs transition-all
            ${solo
              ? "bg-blue-600 text-white hover:bg-blue-700"
              : "bg-slate-700 text-slate-300 hover:bg-slate-600"
            }
          `}
        >
          S
        </button>
      </div>
    </div>
  );
}

// Memoize MixerChannel to prevent unnecessary re-renders
// Only re-render when props actually change
export const MemoizedMixerChannel = memo(MixerChannel, (prevProps, nextProps) => {
  return (
    prevProps.id === nextProps.id &&
    prevProps.name === nextProps.name &&
    prevProps.volumeDb === nextProps.volumeDb &&
    prevProps.muted === nextProps.muted &&
    prevProps.solo === nextProps.solo &&
    prevProps.levelDb === nextProps.levelDb &&
    prevProps.peakDb === nextProps.peakDb &&
    prevProps.inputDevice === nextProps.inputDevice &&
    prevProps.focused === nextProps.focused &&
    prevProps.is_master === nextProps.is_master &&
    prevProps.onVolumeChange === nextProps.onVolumeChange &&
    prevProps.onToggleMute === nextProps.onToggleMute &&
    prevProps.onToggleSolo === nextProps.onToggleSolo &&
    prevProps.onFocus === nextProps.onFocus
  );
});
