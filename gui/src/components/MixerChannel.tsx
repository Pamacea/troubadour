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
  const [isExpanded, setIsExpanded] = useState(false);

  // Volume range: -60dB to +6dB
  const minVolume = -60;
  const maxVolume = 6;
  const volumePercent = ((localVolume - minVolume) / (maxVolume - minVolume)) * 100;

  // Format dB for display
  const formatDb = (db: number) => {
    if (db <= minVolume) return "-∞";
    return `${db.toFixed(1)} dB`;
  };

  // Calculate meter height (logarithmic scale)
  const meterHeight = () => {
    if (levelDb <= -60) return 0;
    if (levelDb > 0) return 100;
    // Map -60dB to 0dB → 0% to 90%
    return Math.min(90, ((levelDb + 60) / 60) * 100);
  };

  // Peak meter (yellow)
  const peakHeight = () => {
    if (peakDb <= -60) return 0;
    if (peakDb > 0) return 100;
    return Math.min(100, ((peakDb + 60) / 60) * 100);
  };

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const percent = parseFloat(e.target.value);
    const newVolume = minVolume + (percent / 100) * (maxVolume - minVolume);
    setLocalVolume(newVolume);
    onVolumeChange(newVolume);
  };

  return (
    <div
      className={`
        flex flex-col bg-slate-800 rounded-xl border border-slate-700
        hover:border-blue-500/50 transition-all duration-200 w-[160px] max-h-[600px]
        shadow-lg
      `}
    >
      {/* Channel Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-slate-700">
        <input
          type="text"
          defaultValue={name}
          className="bg-transparent text-sm font-medium text-white border-none w-full focus:outline-none focus:ring-0"
        />
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

      {/* Expanded Section - EQ & Effects (placeholder for now) */}
      {isExpanded && (
        <div className="px-4 py-3 border-b border-slate-700 bg-slate-900">
          <p className="text-xs text-slate-500 text-center italic">
            EQ & Effects coming soon...
          </p>
        </div>
      )}

      {/* Level Meters (Enhanced) */}
      <div className="px-3 py-2">
        <div className="flex gap-1 h-20">
          {/* Left Meter */}
          <div className="flex-1 relative">
            <div className="absolute left-0 right-0 top-0 bottom-0 bg-slate-900 rounded-2xl overflow-hidden">
              {/* Peak indicator (yellow) */}
              <div
                className="absolute left-0 right-0 bg-yellow-400 transition-all duration-75"
                style={{ bottom: `${peakHeight()}%`, height: "2px" }}
              />
              {/* Current level (green → red gradient) */}
              <div
                className="absolute left-0 right-0 bg-gradient-to-t from-green-500 via-yellow-500 to-red-500 transition-all duration-75"
                style={{ bottom: 0, height: `${meterHeight()}%` }}
              />
            </div>
          </div>

          {/* Right Meter */}
          <div className="flex-1">
            <div className="absolute left-0 right-0 top-0 bottom-0 bg-slate-900 rounded-2xl overflow-hidden">
              <div
                className="absolute left-0 right-0 bg-gradient-to-t from-green-500 via-yellow-500 to-red-500 transition-all duration-75"
                style={{ bottom: 0, height: `${meterHeight()}%` }}
              />
              <div
                className="absolute left-0 right-0 bg-yellow-400 transition-all duration-75"
                style={{ bottom: `${peakHeight()}%`, height: "2px" }}
              />
            </div>
          </div>
        </div>

        {/* Peak value display */}
        <div className="text-center mt-1">
          <span className="text-xs font-mono font-bold text-white">
            {formatDb(peakDb)}
          </span>
        </div>
      </div>

      {/* Volume Fader (Enhanced) */}
      <div className="flex flex-col items-center gap-2 px-3 py-3">
        {/* Volume Display */}
        <span className="text-base font-bold text-white font-mono">
          {formatDb(localVolume)}
        </span>

        {/* Fader Track */}
        <div className="relative w-full h-32 bg-slate-900 rounded-3xl border-2 border-slate-700">
          {/* Fader Fill */}
          <div
            className="absolute left-0 right-0 bg-gradient-to-t from-blue-600 to-blue-400 rounded-3xl transition-all duration-75"
            style={{ bottom: `${volumePercent}%`, height: "100%" }}
          />

          {/* Fader Thumb */}
          <input
            type="range"
            min={0}
            max={100}
            step={0.1}
            value={volumePercent}
            onChange={handleVolumeChange}
            className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
            style={{ appearance: "none", background: "transparent" }}
          />

          {/* Visible Fader Thumb */}
          <div
            className="absolute left-1/2 -translate-x-1/2 w-8 h-5 bg-gradient-to-b from-slate-100 to-slate-300 rounded-lg shadow-lg border-2 border-slate-400 transition-all duration-75"
            style={{ bottom: `calc(${volumePercent}% - 10px)` }}
          />
        </div>

        {/* Volume Presets */}
        <div className="flex gap-1 w-full justify-center text-[10px]">
          <button
            onClick={() => onVolumeChange(0)}
            className="flex-1 py-1 text-xs font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            ∞
          </button>
          <button
            onClick={() => onVolumeChange(-6)}
            className="flex-1 py-1 text-xs font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            -6
          </button>
          <button
            onClick={() => onVolumeChange(-12)}
            className="flex-1 py-1 text-xs font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            -12
          </button>
          <button
            onClick={() => onVolumeChange(-18)}
            className="flex-1 py-1 text-xs font-medium bg-slate-700 text-white rounded hover:bg-slate-600 transition-colors"
          >
            -18
          </button>
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
