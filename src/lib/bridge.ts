// The only place the renderer is allowed to touch Tauri directly.
//
// It re-implements the Electron preload contract (see
// src/shared/ipc/preload-api.ts) on top of `invoke` + `listen`, so every
// component, hook and feature ported from the upstream starter works without
// edits. Rust command names are the channel name with `:` → `_`.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { DesktopApi, DesktopPlatform } from '@shared/ipc/preload-api';
import type { DndBlessAndResolveResponse, DndResolvedPath } from '@shared/ipc/types';

function detectPlatform(): DesktopPlatform {
  const ua = navigator.userAgent;
  if (ua.includes('Mac')) return 'darwin';
  if (ua.includes('Win')) return 'win32';
  return 'linux';
}

/**
 * Subscribe to a Rust-emitted event, returning an unsubscribe function.
 * `listen` is async, so unsubscribing before it resolves is handled explicitly.
 */
function subscribe<T>(event: string, callback: (payload: T) => void): () => void {
  let unlisten: UnlistenFn | null = null;
  let disposed = false;

  void listen<T>(event, (e) => callback(e.payload)).then((off) => {
    if (disposed) off();
    else unlisten = off;
  });

  return () => {
    disposed = true;
    unlisten?.();
    unlisten = null;
  };
}

const EMPTY_RESOLVED: DndResolvedPath = {
  path: '',
  name: '',
  size: 0,
  isFile: false,
  isDirectory: false,
  exists: false,
};

export const api: DesktopApi = {
  platform: detectPlatform(),

  appGetInfo: () => invoke('app_get_info'),
  appOpenExternal: (data) => invoke('app_open_external', { data }),
  appShowInFolder: (data) => invoke('app_show_in_folder', { data }),
  appOpenPath: (data) => invoke('app_open_path', { data }),

  dialogOpenFile: (data) => invoke('dialog_open_file', { data: data ?? {} }),

  logSend: (data) => invoke('log_send', { data }),

  settingsGet: (data) => invoke('settings_get', { data }),
  settingsSet: (data) => invoke('settings_set', { data }),
  settingsReset: () => invoke('settings_reset'),
  onSettingsChange: (cb) => subscribe('settings:on-change', cb),

  secretHas: (data) => invoke('secret_has', { data }),
  secretSet: (data) => invoke('secret_set', { data }),
  secretDelete: (data) => invoke('secret_delete', { data }),
  secretStatus: () => invoke('secret_status'),

  themeGet: () => invoke('theme_get'),
  themeSet: (data) => invoke('theme_set', { data }),
  onThemeChange: (cb) => subscribe('theme:on-change', cb),

  windowMinimize: () => invoke('window_minimize'),
  windowMaximize: () => invoke('window_maximize'),
  windowClose: () => invoke('window_close'),
  windowIsMaximized: () => invoke('window_is_maximized'),
  onWindowMaximizeChange: (cb) => subscribe('window:on-maximize-change', cb),

  menuShowWindowMenu: (data) => invoke('menu_show_window_menu', { data: data ?? {} }),
  onMenuAction: (cb) => subscribe('menu:action', cb),

  notificationShow: (data) => invoke('notification_show', { data }),

  aiComplete: (data) => invoke('ai_complete', { data }),
  aiCancel: (data) => invoke('ai_cancel', { data }),
  onAiStreamChunk: (cb) => subscribe('ai:stream-chunk', cb),

  // There is no DOM way to map a File back to its path, and Tauri does not need
  // one: drops arrive with absolute paths via the webview drag-drop event, which
  // useFileDrop() consumes. Kept for API parity; always returns an empty entry.
  dndGetPathForFile: async (_file: File) => EMPTY_RESOLVED,
  dndResolvePaths: (data) => invoke('dnd_resolve_paths', { data }),

  fsRead: (data) => invoke('fs_read', { data }),
  fsWrite: (data) => invoke('fs_write', { data }),
  fsList: (data) => invoke('fs_list', { data }),
  fsStat: (data) => invoke('fs_stat', { data }),
};

/**
 * Grants fs access to a path that came from a real drop, and returns its
 * metadata in the same round trip.
 *
 * This is the Tauri equivalent of the Electron preload's internal
 * `dnd:bless-and-resolve` hand-off: a drop path is only known once the webview
 * reports it, so the grant has to happen here rather than in Rust's setup.
 * Not part of `window.api` — it is used only by useFileDrop().
 */
export async function blessDroppedPath(path: string): Promise<DndResolvedPath> {
  try {
    const res = await invoke<DndBlessAndResolveResponse>('dnd_bless_and_resolve', {
      data: { path },
    });
    return res.success ? res.resolved : EMPTY_RESOLVED;
  } catch {
    return EMPTY_RESOLVED;
  }
}
