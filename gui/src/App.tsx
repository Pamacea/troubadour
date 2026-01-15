import { MixerPanel } from "./components/MixerPanel";
import { PresetManager } from "./components/PresetManager";
import "./index.css";

function App() {
  return (
    <div className="h-screen w-screen bg-slate-950 text-slate-200">
      {/* Top Bar */}
      <div className="h-14 bg-slate-900 border-b border-slate-800 flex items-center px-6">
        <h1 className="text-xl font-bold text-white flex items-center gap-2">
          <span className="text-2xl">🎼</span>
          Troubadour
        </h1>
        <span className="ml-4 px-2 py-1 bg-slate-800 text-slate-400 text-xs rounded">
          v0.1.0
        </span>
      </div>

      {/* Main Content */}
      <div className="flex" style={{ height: "calc(100vh - 3.5rem)" }}>
        {/* Mixer Panel */}
        <div className="flex-1">
          <MixerPanel />
        </div>

        {/* Preset Manager Sidebar */}
        <div className="w-80">
          <PresetManager />
        </div>
      </div>
    </div>
  );
}

export default App;
