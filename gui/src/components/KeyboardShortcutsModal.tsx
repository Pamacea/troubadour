import { X } from "lucide-react";

interface Shortcut {
  key: string;
  description: string;
  category: string;
}

const shortcuts: Shortcut[] = [
  { key: "M", description: "Toggle mute on focused channel", category: "Channel Control" },
  { key: "S", description: "Toggle solo on focused channel", category: "Channel Control" },
  { key: "↑ / ↓", description: "Adjust volume ±1dB", category: "Volume" },
  { key: "Shift + ↑ / ↓", description: "Adjust volume ±6dB", category: "Volume" },
  { key: "Tab / Shift+Tab", description: "Navigate between channels", category: "Navigation" },
  { key: "Ctrl + S", description: "Save configuration", category: "System" },
  { key: "F1", description: "Show keyboard shortcuts", category: "Help" },
  { key: "Escape", description: "Close modal / Clear focus", category: "Navigation" },
];

interface KeyboardShortcutsModalProps {
  onClose: () => void;
}

export function KeyboardShortcutsModal({ onClose }: KeyboardShortcutsModalProps) {
  // Group shortcuts by category
  const categories = shortcuts.reduce((acc, shortcut) => {
    if (!acc[shortcut.category]) {
      acc[shortcut.category] = [];
    }
    acc[shortcut.category].push(shortcut);
    return acc;
  }, {} as Record<string, Shortcut[]>);

  return (
    <div
      className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-slate-800 rounded-2xl shadow-2xl border border-slate-700 max-w-2xl w-full mx-4"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700">
          <h2 className="text-xl font-bold text-white flex items-center gap-2">
            <span className="text-2xl">⌨️</span>
            Keyboard Shortcuts
          </h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white transition-colors"
            aria-label="Close"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-4 max-h-[60vh] overflow-y-auto">
          {Object.entries(categories).map(([category, categoryShortcuts]) => (
            <div key={category} className="mb-6 last:mb-0">
              <h3 className="text-sm font-semibold text-blue-400 mb-3 uppercase tracking-wide">
                {category}
              </h3>
              <div className="space-y-2">
                {categoryShortcuts.map((shortcut) => (
                  <div
                    key={shortcut.key}
                    className="flex items-center justify-between py-2 px-3 bg-slate-900/50 rounded-lg hover:bg-slate-900 transition-colors"
                  >
                    <span className="text-sm text-slate-300">{shortcut.description}</span>
                    <kbd className="px-3 py-1.5 bg-slate-700 text-white text-xs font-mono font-semibold rounded border border-slate-600 shadow-sm">
                      {shortcut.key}
                    </kbd>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-slate-700 bg-slate-900/30 rounded-b-2xl">
          <p className="text-xs text-slate-400 text-center">
            Press <kbd className="px-2 py-1 bg-slate-700 text-white text-xs font-mono rounded mx-1">Escape</kbd>
            or click outside to close
          </p>
        </div>
      </div>
    </div>
  );
}
