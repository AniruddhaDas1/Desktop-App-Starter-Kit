// Declares the surface every component and hook already expects.
// Installed once in src/renderer/main.tsx from src/lib/bridge.ts.
import type { DesktopApi } from '@shared/ipc/preload-api';

declare global {
  interface Window {
    api: DesktopApi;
  }
}

export {};
