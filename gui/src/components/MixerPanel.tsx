import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MixerChannel } from "./MixerChannel";

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
  const [loading, setLoading] = useState(true);

  // Load channels on mount
  useEffect(() => {
    loadChannels();
    // Refresh channels every 100ms for level meters
    const interval = setInterval(loadChannels, 100);
    return () => clearInterval(interval);
  }, []);

  async function loadChannels() {
    try {
      const result = (await invoke<ChannelInfo[]>("get_channels")) || [];
      setChannels(result);
      setLoading(false);
    } catch (error) {
      console.error("Failed to load channels:", error);
      setLoading(false);
    }
  }

  async function handleVolumeChange(channelId: string, volumeDb: number) {
    try {
      await invoke("set_volume", { channelId, volumeDb });
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
      await invoke("toggle_mute", { channelId });
      // Optimistic update
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, muted: !ch.muted } : ch))
      );
    } catch (error) {
      console.error("Failed to toggle mute:", error);
    }
  }

  async function handleToggleSolo(channelId: string) {
    try {
      await invoke("toggle_solo", { channelId });
      // Optimistic update
      setChannels((prev) =>
        prev.map((ch) => (ch.id === channelId ? { ...ch, solo: !ch.solo } : ch))
      );
    } catch (error) {
      console.error("Failed to toggle solo:", error);
    }
  }

  async function handleAddChannel() {
    const id = `channel-${channels.length + 1}`;
    const name = `Channel ${channels.length + 1}`;
    try {
      await invoke("add_channel", { channelId: id, name });
      await loadChannels();
    } catch (error) {
      console.error("Failed to add channel:", error);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-slate-400">Loading mixer...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 bg-slate-800 border-b border-slate-700">
        <h2 className="text-lg font-semibold text-slate-200">Mixer</h2>
        <button
          onClick={handleAddChannel}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors text-sm font-medium"
        >
          + Add Channel
        </button>
      </div>

      {/* Channel Strips */}
      <div className="flex-1 overflow-x-auto overflow-y-hidden p-6">
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
          <div className="flex gap-4 h-full">
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
