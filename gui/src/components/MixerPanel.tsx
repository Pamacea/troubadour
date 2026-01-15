import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MemoizedMixerChannel } from "./MixerChannel";
import { KeyboardShortcutsModal } from "./KeyboardShortcutsModal";
import { useKeyboardShortcuts } from "../hooks/useKeyboardShortcuts";
import { useAutoSaveConfig } from "../hooks/useAutoSaveConfig";

interface ChannelInfo {
  id: string;
  name: string;
  volume_db: number;
  muted: boolean;
  solo: boolean;
  level_db: number;
  peak_db: number;
  input_device?: string | null;
  is_master?: boolean;
}

interface BusInfo {
  id: string;
  name: string;
  output_device: string | null;
  volume_db: number;
  muted: boolean;
}

export function MixerPanel() {
  const [channels, setChannels] = useState<ChannelInfo[]>([]);
  const [buses, setBuses] = useState<BusInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [showKeyboardShortcuts, setShowKeyboardShortcuts] = useState(false);
  const [focusedChannelId, setFocusedChannelId] = useState<string | null>(null);
  const [toast, setToast] = useState<{ message: string; type: "success" | "info" } | null>(null);
  // Cache of manually set states to prevent backend from overwriting
  const manualStateOverridesRef = useRef<Map<string, {muted?: boolean, solo?: boolean}>>(new Map());
  // Track recently added channel IDs to prevent polling refresh from removing them
  const pendingChannelAddsRef = useRef<Set<string>>(new Set());

  // Auto-save configuration with 1 second debounce
  // Tracks when channels, buses, or routing change
  useAutoSaveConfig(
    () => true, // Always save when deps change
    [channels, buses],
    1000 // 1 second debounce
  );

  // Load channels and buses on mount
  useEffect(() => {
    // Load configuration first, then initialize channels and buses
    const initializeApp = async () => {
      try {
        // Load saved configuration
        if (typeof window !== 'undefined' && window.__TAURI__) {
          await invoke("load_config");
          console.log("Configuration loaded successfully");
        }
      } catch (error) {
        console.error("Failed to load configuration:", error);
        // Continue with default state if config loading fails
      }

      // Initialize channels and buses
      loadChannels();
      loadBuses();
    };

    initializeApp();

    // Refresh channels every 500ms for level meters (balance between responsiveness and performance)
    const interval = setInterval(loadChannels, 500);
    return () => clearInterval(interval);
  }, []);

  // Show toast notification - memoized
  const showToast = useCallback((message: string, type: "success" | "info" = "info") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 2000);
  }, []);

  // Navigate channels with Tab - memoized
  const handleChannelNavigation = useCallback((direction: "next" | "prev") => {
    if (channels.length === 0) return;

    const currentIndex = focusedChannelId
      ? channels.findIndex((ch) => ch.id === focusedChannelId)
      : -1;

    let nextIndex: number;
    if (direction === "next") {
      nextIndex = currentIndex + 1 >= channels.length ? 0 : currentIndex + 1;
    } else {
      nextIndex = currentIndex - 1 < 0 ? channels.length - 1 : currentIndex - 1;
    }

    setFocusedChannelId(channels[nextIndex].id);
  }, [channels, focusedChannelId]);

  // Adjust volume with keyboard - memoized
  const handleVolumeAdjust = useCallback((deltaDb: number) => {
    if (!focusedChannelId) return;

    const channel = channels.find((ch) => ch.id === focusedChannelId);
    if (!channel) return;

    const newVolume = Math.min(6, Math.max(-60, channel.volume_db + deltaDb));

    // Call the volume change handler inline to avoid circular dependency
    (async () => {
      try {
        if (typeof window !== 'undefined' && window.__TAURI__) {
          await invoke("set_volume", { channelId: focusedChannelId, volumeDb: newVolume });
        } else {
          console.log(`Mock: Set volume for ${focusedChannelId} to ${newVolume} dB`);
        }
        setChannels((prev) =>
          prev.map((ch) => (ch.id === focusedChannelId ? { ...ch, volume_db: newVolume } : ch))
        );
      } catch (error) {
        console.error("Failed to set volume:", error);
      }
    })();

    const direction = deltaDb > 0 ? "increased" : "decreased";
    showToast(`${channel.name} volume ${direction} to ${newVolume.toFixed(1)} dB`);
  }, [focusedChannelId, channels, showToast]);

  // Save configuration - memoized
  const handleSaveConfig = useCallback(async () => {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("save_config");
      } else {
        console.log("Mock: Save configuration");
      }
      showToast("Configuration saved", "success");
    } catch (error) {
      console.error("Failed to save configuration:", error);
      showToast("Failed to save configuration", "info");
    }
  }, [showToast]);

  // Setup keyboard shortcuts
  useKeyboardShortcuts({
    shortcuts: [
      {
        key: "m",
        action: () => {
          if (focusedChannelId) {
            handleToggleMute(focusedChannelId);
            const channel = channels.find((ch) => ch.id === focusedChannelId);
            showToast(`${channel?.name}: ${channel?.muted ? "Unmuted" : "Muted"}`);
          }
        },
        description: "Toggle mute",
      },
      {
        key: "s",
        action: () => {
          if (focusedChannelId) {
            handleToggleSolo(focusedChannelId);
            const channel = channels.find((ch) => ch.id === focusedChannelId);
            showToast(`${channel?.name}: ${channel?.solo ? "Solo Off" : "Solo On"}`);
          }
        },
        description: "Toggle solo",
      },
      {
        key: "ArrowUp",
        action: () => handleVolumeAdjust(1),
        description: "Volume +1dB",
      },
      {
        key: "ArrowDown",
        action: () => handleVolumeAdjust(-1),
        description: "Volume -1dB",
      },
      {
        key: "ArrowUp",
        shiftKey: true,
        action: () => handleVolumeAdjust(6),
        description: "Volume +6dB",
      },
      {
        key: "ArrowDown",
        shiftKey: true,
        action: () => handleVolumeAdjust(-6),
        description: "Volume -6dB",
      },
      {
        key: "Tab",
        action: () => handleChannelNavigation("next"),
        description: "Next channel",
        preventDefault: true,
      },
      {
        key: "Tab",
        shiftKey: true,
        action: () => handleChannelNavigation("prev"),
        description: "Previous channel",
        preventDefault: true,
      },
      {
        key: "s",
        ctrlKey: true,
        action: handleSaveConfig,
        description: "Save configuration",
      },
      {
        key: "F1",
        action: () => setShowKeyboardShortcuts(true),
        description: "Show keyboard shortcuts",
      },
      {
        key: "Escape",
        action: () => {
          setFocusedChannelId(null);
          setShowKeyboardShortcuts(false);
        },
        description: "Clear focus / Close modal",
      },
    ],
    disabled: loading,
  });

  async function loadChannels() {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = (await invoke<ChannelInfo[]>("get_channels")) || [];

        // Preserve recently added channels that might not be in backend yet
        const pendingAdds = pendingChannelAddsRef.current;
        const enhancedResult = pendingAdds.size > 0
          ? [
              ...result,
              ...Array.from(pendingAdds).map(id => ({
                id,
                name: `Channel ${result.length + 1}`,
                volume_db: 0,
                muted: false,
                solo: false,
                level_db: -60,
                peak_db: -60,
                is_master: false,
              }))
            ]
          : result;

        // Apply manual overrides and identify master channel
        setChannels(() =>
          enhancedResult.map((newChannel) => {
            const override = manualStateOverridesRef.current.get(newChannel.id);
            return {
              ...newChannel,
              is_master: newChannel.id === "master" || newChannel.name.toLowerCase() === "master",
              ...(override && {
                ...(override.muted !== undefined && { muted: override.muted }),
                ...(override.solo !== undefined && { solo: override.solo }),
              }),
            };
          })
        );
      } else {
        // Mock channels for development - preserve manual overrides
        const mockChannels: ChannelInfo[] = [
          { id: "input-1", name: "Input 1", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60, is_master: false },
          { id: "input-2", name: "Input 2", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60, is_master: false },
          { id: "input-3", name: "Input 3", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60, is_master: false },
          { id: "master", name: "Master", volume_db: 0, muted: false, solo: false, level_db: -60, peak_db: -60, is_master: true },
        ];
        // Apply manual overrides even in mock mode
        setChannels(() =>
          mockChannels.map((channel) => {
            const override = manualStateOverridesRef.current.get(channel.id);
            return {
              ...channel,
              ...(override && {
                ...(override.muted !== undefined && { muted: override.muted }),
                ...(override.solo !== undefined && { solo: override.solo }),
              }),
            };
          })
        );
      }
      setLoading(false);
    } catch (error) {
      console.error("Failed to load channels:", error);
      setLoading(false);
    }
  }

  async function loadBuses() {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const result = await invoke<BusInfo[]>("get_buses");
        setBuses(result);
      } else {
        // Mock buses for development
        const mockBuses: BusInfo[] = [
          { id: "A1", name: "A1", output_device: null, volume_db: 0, muted: false },
          { id: "A2", name: "A2", output_device: null, volume_db: 0, muted: false },
        ];
        setBuses(mockBuses);
      }
    } catch (error) {
      console.error("Failed to load buses:", error);
    }
  }

  // Memoized handlers to prevent recreation on every render
  const handleVolumeChange = useCallback(async (channelId: string, volumeDb: number) => {
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
  }, []);

  const handleToggleMute = useCallback(async (channelId: string) => {
    const channel = channels.find(ch => ch.id === channelId);
    if (!channel) return;

    const newMutedState = !channel.muted;
    const oldMutedState = channel.muted;

    // Store manual override IMMEDIATELY
    manualStateOverridesRef.current.set(channelId, { muted: newMutedState });

    // Optimistic update
    setChannels((prev) =>
      prev.map((ch) => (ch.id === channelId ? { ...ch, muted: newMutedState } : ch))
    );

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("toggle_mute", { channelId });
      } else {
        console.log(`Mock: Toggle mute for ${channelId}`);
      }
      // Success: keep the override
    } catch (error) {
      // Rollback on error
      console.error("Failed to toggle mute:", error);
      manualStateOverridesRef.current.delete(channelId);
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, muted: oldMutedState } : ch))
      );
    }
    // Note: We keep the override indefinitely - only remove on error or explicit new action
  }, [channels]);

  const handleToggleSolo = useCallback(async (channelId: string) => {
    const channel = channels.find(ch => ch.id === channelId);
    if (!channel) return;

    const newSoloState = !channel.solo;
    const oldSoloState = channel.solo;

    // Store manual override IMMEDIATELY
    manualStateOverridesRef.current.set(channelId, { solo: newSoloState });

    // Optimistic update
    setChannels((prev) =>
      prev.map((ch) => (ch.id === channelId ? { ...ch, solo: newSoloState } : ch))
    );

    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        await invoke("toggle_solo", { channelId });
      } else {
        console.log(`Mock: Toggle solo for ${channelId}`);
      }
      // Success: keep the override
    } catch (error) {
      // Rollback on error
      console.error("Failed to toggle solo:", error);
      manualStateOverridesRef.current.delete(channelId);
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, solo: oldSoloState } : ch))
      );
    }
    // Note: We keep the override indefinitely - only remove on error or explicit new action
  }, [channels]);

  const handleFocus = useCallback((channelId: string) => {
    setFocusedChannelId(channelId);
  }, []);

  async function handleAddChannel() {
    const id = `channel-${Date.now()}`;
    const name = `Channel ${channels.length + 1}`;
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        // Add to pending set IMMEDIATELY to preserve it during refreshes
        pendingChannelAddsRef.current.add(id);

        await invoke("add_channel", { channelId: id, name });

        // Remove from pending set after a short delay to ensure backend has processed it
        setTimeout(() => {
          pendingChannelAddsRef.current.delete(id);
        }, 500);

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
      // Clean up pending add on error
      pendingChannelAddsRef.current.delete(id);
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
      {/* Top Bar */}
      <div className="bg-slate-800 border-b border-slate-700 px-6 py-3">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-white">Audio Mixer</h2>

          {/* Add Channel Button */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowKeyboardShortcuts(true)}
              className="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded text-sm font-medium transition-colors"
              title="Keyboard shortcuts (F1)"
            >
              ⌨ Shortcuts
            </button>
            <button
              onClick={handleAddChannel}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm font-medium transition-colors"
            >
              + Add Channel
            </button>
          </div>
        </div>
      </div>

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
          <div className="flex gap-6 items-stretch h-full py-4 min-w-0">
            {/* Input Channels */}
            {channels.filter(ch => !ch.is_master).map((channel) => (
              <MemoizedMixerChannel
                key={channel.id}
                id={channel.id}
                name={channel.name}
                volumeDb={channel.volume_db}
                muted={channel.muted}
                solo={channel.solo}
                levelDb={channel.level_db}
                peakDb={channel.peak_db}
                inputDevice={channel.input_device}
                focused={focusedChannelId === channel.id}
                onFocus={() => handleFocus(channel.id)}
                onVolumeChange={(vol) => handleVolumeChange(channel.id, vol)}
                onToggleMute={() => handleToggleMute(channel.id)}
                onToggleSolo={() => handleToggleSolo(channel.id)}
                is_master={false}
              />
            ))}

            {/* Visual Separator */}
            {channels.filter(ch => !ch.is_master).length > 0 && channels.filter(ch => ch.is_master).length > 0 && (
              <div className="flex flex-col items-center justify-center gap-2 px-4 border-l-2 border-slate-600">
                <div className="w-px h-16 bg-gradient-to-b from-transparent via-slate-500 to-transparent"></div>
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wider">→ Master</span>
                <div className="w-px h-16 bg-gradient-to-b from-transparent via-slate-500 to-transparent"></div>
              </div>
            )}

            {/* Master Channel */}
            {channels.filter(ch => ch.is_master).map((channel) => (
              <MemoizedMixerChannel
                key={channel.id}
                id={channel.id}
                name={channel.name}
                volumeDb={channel.volume_db}
                muted={channel.muted}
                solo={channel.solo}
                levelDb={channel.level_db}
                peakDb={channel.peak_db}
                inputDevice={channel.input_device}
                focused={focusedChannelId === channel.id}
                onFocus={() => handleFocus(channel.id)}
                onVolumeChange={(vol) => handleVolumeChange(channel.id, vol)}
                onToggleMute={() => handleToggleMute(channel.id)}
                onToggleSolo={() => handleToggleSolo(channel.id)}
                is_master={true}
              />
            ))}
          </div>
        )}
      </div>

      {/* Toast Notification */}
      {toast && (
        <div
          className={`
            fixed bottom-6 left-1/2 -translate-x-1/2 px-6 py-3 rounded-lg shadow-lg
            flex items-center gap-2 animate-in slide-in-from-bottom
            ${toast.type === "success" ? "bg-green-600" : "bg-slate-700"}
          `}
        >
          <span className="text-white font-medium text-sm">{toast.message}</span>
        </div>
      )}

      {/* Keyboard Shortcuts Modal */}
      {showKeyboardShortcuts && (
        <KeyboardShortcutsModal onClose={() => setShowKeyboardShortcuts(false)} />
      )}
    </div>
  );
}
