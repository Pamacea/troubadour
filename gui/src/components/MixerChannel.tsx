import { useState } from "react";

interface MixerChannelProps {
  id: string;
  name: string;
  volumeDb: number;
  muted: boolean;
  solo: boolean;
  levelDb: number;
  peakDb: number;
  onVolumeChange: (volumeDb: number) => void;
  onToggleMute: () => void;
  onToggleSolo: () => void;
}

export function MixerChannel({
  id,
  name,
  volumeDb,
  muted,
  solo,
  levelDb,
  peakDb,
  onVolumeChange,
  onToggleMute,
  onToggleSolo,
}: MixerChannelProps) {
  const [localVolume, setLocalVolume] = useState(volumeDb);

  // Volume in decibels: -60 to +6
  const minVolume = -60;
  const maxVolume = 6;

  // Convert dB to percentage for slider
  const volumePercent = ((localVolume - minVolume) / (maxVolume - minVolume)) * 100;

  // Format dB for display
  const formatDb = (db: number) => {
    if (db <= minVolume) return "-∞";
    return `${db.toFixed(1)} dB`;
  };

  // Handle volume change
  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const percent = parseFloat(e.target.value);
    const newVolume = minVolume + (percent / 100) * (maxVolume - minVolume);
    setLocalVolume(newVolume);
    onVolumeChange(newVolume);
  };

  // Level meter height
  const levelPercent = ((levelDb - minVolume) / (0 - minVolume)) * 100;
  const peakPercent = ((peakDb - minVolume) / (0 - minVolume)) * 100;

  return (
    <div className="flex flex-col items-center gap-3 p-4 bg-slate-900 rounded-lg border border-slate-700 min-w-[140px]">
      {/* Channel Name */}
      <div className="w-full text-center">
        <input
          type="text"
          defaultValue={name}
          className="bg-transparent text-sm font-medium text-slate-200 text-center border-none w-full focus:outline-none focus:ring-1 focus:ring-blue-500 rounded"
        />
      </div>

      {/* Level Meter */}
      <div className="relative w-6 h-48 bg-slate-800 rounded-full overflow-hidden border border-slate-700">
        {/* Peak indicator */}
        <div
          className="absolute left-0 right-0 bg-yellow-400 transition-all duration-100"
          style={{ bottom: `${Math.max(0, Math.min(100, peakPercent))}%`, height: "2px" }}
        />
        {/* Current level */}
        <div
          className="absolute left-0 right-0 bg-gradient-to-t from-green-500 via-yellow-500 to-red-500 transition-all duration-75"
          style={{ bottom: 0, height: `${Math.max(0, Math.min(100, levelPercent))}%` }}
        />
      </div>

      {/* Volume Fader */}
      <div className="flex flex-col items-center gap-2 w-full">
        <span className="text-xs text-slate-400 font-mono">{formatDb(localVolume)}</span>
        <div className="relative w-full h-32">
          {/* Fader track */}
          <div className="absolute inset-0 bg-slate-800 rounded-full border border-slate-700">
            {/* Fader fill */}
            <div
              className="absolute left-0 right-0 bg-blue-500 rounded-full transition-all"
              style={{ bottom: 0, height: `${volumePercent}%` }}
            />
          </div>
          {/* Fader thumb */}
          <input
            type="range"
            min={minVolume}
            max={maxVolume}
            step={0.1}
            value={localVolume}
            onChange={handleVolumeChange}
            className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
            style={{ appearance: "none", background: "transparent" }}
          />
          {/* Visible fader thumb */}
          <div
            className="absolute left-1/2 w-8 h-4 bg-slate-300 rounded shadow-lg border-2 border-slate-500 transform -translate-x-1/2 transition-all pointer-events-none"
            style={{ bottom: `calc(${volumePercent}% - 8px)` }}
          />
        </div>
      </div>

      {/* Mute/Solo Buttons */}
      <div className="flex gap-2 w-full">
        <button
          onClick={onToggleMute}
          className={`
            flex-1 py-2 px-3 rounded font-medium text-xs transition-all
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
            flex-1 py-2 px-3 rounded font-medium text-xs transition-all
            ${solo
              ? "bg-blue-600 text-white hover:bg-blue-700"
              : "bg-slate-700 text-slate-300 hover:bg-slate-600"
            }
          `}
        >
          S
        </button>
      </div>

      {/* Channel ID (hidden but useful for debugging) */}
      <div className="text-xs text-slate-600 font-mono">{id}</div>
    </div>
  );
}
