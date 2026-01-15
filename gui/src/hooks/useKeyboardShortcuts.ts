import { useEffect, useCallback } from "react";

export interface ShortcutConfig {
  key: string;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  action: () => void;
  description: string;
  preventDefault?: boolean;
}

export interface KeyboardShortcutsOptions {
  shortcuts: ShortcutConfig[];
  disabled?: boolean;
  ignoreInputs?: boolean;
}

/**
 * Hook for managing global keyboard shortcuts
 *
 * Features:
 * - Global keydown event listener
 * - Configurable key combinations (Ctrl, Shift, Alt, Meta)
 * - Option to ignore shortcuts when typing in inputs
 * - Returns list of shortcuts for documentation
 */
export function useKeyboardShortcuts({
  shortcuts,
  disabled = false,
  ignoreInputs = true,
}: KeyboardShortcutsOptions) {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (disabled) return;

      // Ignore if typing in input, textarea, or contentEditable
      if (ignoreInputs) {
        const target = event.target as HTMLElement;
        if (
          target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable
        ) {
          return;
        }
      }

      // Find matching shortcut
      const matchedShortcut = shortcuts.find((shortcut) => {
        return (
          event.key.toLowerCase() === shortcut.key.toLowerCase() &&
          !!event.ctrlKey === !!shortcut.ctrlKey &&
          !!event.shiftKey === !!shortcut.shiftKey &&
          !!event.altKey === !!shortcut.altKey &&
          !!event.metaKey === !!shortcut.metaKey
        );
      });

      if (matchedShortcut) {
        if (matchedShortcut.preventDefault !== false) {
          event.preventDefault();
        }
        matchedShortcut.action();
      }
    },
    [shortcuts, disabled, ignoreInputs]
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return { shortcuts };
}
