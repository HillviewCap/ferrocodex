/// <reference types="vite/client" />

// Tauri API types
declare global {
  interface Window {
    __TAURI__: {
      invoke: (command: string, args?: any) => Promise<any>;
    };
  }
}

export {};