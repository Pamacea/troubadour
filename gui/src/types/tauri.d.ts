/// <reference types="@tauri-apps/api" />

declare global {
  interface Window {
    __TAURI__?: unknown;
  }
}

export {};
