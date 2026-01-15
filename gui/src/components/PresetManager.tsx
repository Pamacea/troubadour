import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function PresetManager() {
  const [presets, setPresets] = useState<string[]>([]);
  const [newPresetName, setNewPresetName] = useState("");
  const [loading, setLoading] = useState(false);

  // Load presets on mount
  useEffect(() => {
    loadPresets();
  }, []);

  async function loadPresets() {
    try {
      const result = await invoke<string[]>("list_presets");
      setPresets(result);
    } catch (error) {
      console.error("Failed to load presets:", error);
    }
  }

  async function handleLoadPreset(name: string) {
    setLoading(true);
    try {
      await invoke("load_preset", { name });
      await loadPresets(); // Refresh
    } catch (error) {
      console.error("Failed to load preset:", error);
      alert(`Failed to load preset: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleSavePreset() {
    if (!newPresetName.trim()) {
      alert("Please enter a preset name");
      return;
    }

    setLoading(true);
    try {
      await invoke("save_preset", { name: newPresetName });
      setNewPresetName("");
      await loadPresets(); // Refresh
    } catch (error) {
      console.error("Failed to save preset:", error);
      alert(`Failed to save preset: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleDeletePreset(name: string) {
    if (!confirm(`Delete preset "${name}"?`)) {
      return;
    }

    setLoading(true);
    try {
      await invoke("delete_preset", { name });
      await loadPresets(); // Refresh
    } catch (error) {
      console.error("Failed to delete preset:", error);
      alert(`Failed to delete preset: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex flex-col h-full bg-slate-800 border-l border-slate-700">
      {/* Header */}
      <div className="px-6 py-4 border-b border-slate-700">
        <h2 className="text-lg font-semibold text-slate-200">Presets</h2>
      </div>

      {/* Save New Preset */}
      <div className="p-4 border-b border-slate-700">
        <div className="flex gap-2">
          <input
            type="text"
            value={newPresetName}
            onChange={(e) => setNewPresetName(e.target.value)}
            placeholder="Preset name..."
            className="flex-1 px-3 py-2 bg-slate-900 text-slate-200 border border-slate-600 rounded focus:outline-none focus:ring-1 focus:ring-blue-500 text-sm"
            disabled={loading}
          />
          <button
            onClick={handleSavePreset}
            disabled={loading || !newPresetName.trim()}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-slate-700 disabled:text-slate-500 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            Save
          </button>
        </div>
      </div>

      {/* Preset List */}
      <div className="flex-1 overflow-y-auto p-4">
        {presets.length === 0 ? (
          <div className="text-center text-slate-500 text-sm py-8">
            No presets saved yet
          </div>
        ) : (
          <div className="space-y-2">
            {presets.map((name) => (
              <div
                key={name}
                className="flex items-center gap-2 p-3 bg-slate-900 rounded border border-slate-700 hover:border-slate-600 transition-colors"
              >
                <span className="flex-1 text-sm text-slate-200 truncate">{name}</span>
                <button
                  onClick={() => handleLoadPreset(name)}
                  disabled={loading}
                  className="px-3 py-1 bg-blue-600 text-white rounded text-xs hover:bg-blue-700 disabled:bg-slate-700 disabled:cursor-not-allowed transition-colors"
                >
                  Load
                </button>
                <button
                  onClick={() => handleDeletePreset(name)}
                  disabled={loading}
                  className="px-3 py-1 bg-red-600 text-white rounded text-xs hover:bg-red-700 disabled:bg-slate-700 disabled:cursor-not-allowed transition-colors"
                >
                  Delete
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
