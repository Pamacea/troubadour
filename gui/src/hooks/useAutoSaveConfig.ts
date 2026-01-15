import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Hook for auto-saving configuration with debouncing
 *
 * @param shouldSave - Function that determines if save should trigger
 * @param deps - Dependencies that trigger save when changed
 * @param delayMs - Delay in milliseconds before saving (default: 1000ms)
 */
export function useAutoSaveConfig(
  shouldSave: () => boolean = () => true,
  deps: any[] = [],
  delayMs: number = 1000
) {
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isDirtyRef = useRef(false);

  useEffect(() => {
    // Mark as dirty when dependencies change
    if (deps.length > 0) {
      isDirtyRef.current = true;
    }

    // Clear existing timeout
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    // Set new timeout for debounced save
    saveTimeoutRef.current = setTimeout(async () => {
      if (isDirtyRef.current && shouldSave()) {
        try {
          await invoke("save_config");
          console.log("Configuration auto-saved");
        } catch (error) {
          console.error("Failed to auto-save config:", error);
        }
        isDirtyRef.current = false;
      }
    }, delayMs);

    // Cleanup timeout on unmount
    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, deps); // Re-run when dependencies change

  /**
   * Manually trigger an immediate save
   */
  const manualSave = async () => {
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    try {
      await invoke("save_config");
      isDirtyRef.current = false;
      console.log("Configuration manually saved");
      return true;
    } catch (error) {
      console.error("Failed to save config:", error);
      return false;
    }
  };

  return {
    manualSave,
  };
}
