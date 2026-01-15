import { useState } from "react";
import { MixerPanel } from "./components/MixerPanel";
import { PresetManager } from "./components/PresetManager";
import "./index.css";

function App() {
  const [presetsVisible, setPresetsVisible] = useState(false);

  return (
    <div className="grid grid-rows-[auto_1fr] h-screen w-screen bg-slate-950 text-slate-200 overflow-hidden">
      {/* Top Bar */}
      <div className="h-14 bg-slate-900 border-b border-slate-800 flex items-center justify-between px-6">
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-bold text-white flex items-center gap-2">
            <span className="text-2xl">🎼</span>
            Troubadour
          </h1>
          <span className="px-2 py-1 bg-slate-800 text-slate-400 text-xs rounded">
            v0.1.0
          </span>
        </div>

        {/* Preset Toggle Button - Discrete */}
        <button
          onClick={() => setPresetsVisible(!presetsVisible)}
          className={`
            px-3 py-1.5 rounded text-sm font-medium transition-all
            ${presetsVisible
              ? "bg-blue-600 text-white hover:bg-blue-700"
              : "bg-slate-800 text-slate-400 hover:bg-slate-700"
            }
          `}
        >
          {presetsVisible ? "✓" : "⚙"} Presets
        </button>
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-[1fr_auto] overflow-hidden">
        {/* Mixer Panel */}
        <div className="overflow-hidden">
          <MixerPanel />
        </div>

        {/* Preset Manager Sidebar - Collapsible */}
        {presetsVisible && (
          <div className="w-80 bg-slate-800 border-l border-slate-700 animate-in slide-in-from-right overflow-hidden">
            <PresetManager onClose={() => setPresetsVisible(false)} />
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
