// The typed surface the bridge exposes as `window.api`.
//
// This intentionally mirrors the upstream Electron preload contract one-for-one
// so that components, hooks and features are portable without edits. Each
// function maps to a Tauri command in `src-tauri/src/commands/`.
import type {
  AiCancelRequest,
  AiCancelResponse,
  AiCompleteRequest,
  AiCompleteResponse,
  AiStreamChunkEvent,
  AppGetInfoResponse,
  AppOpenExternalRequest,
  AppOpenExternalResponse,
  AppOpenPathRequest,
  AppOpenPathResponse,
  AppShowInFolderRequest,
  AppShowInFolderResponse,
  DialogOpenFileRequest,
  DialogOpenFileResponse,
  DndResolvePathsRequest,
  DndResolvePathsResponse,
  DndResolvedPath,
  FsListRequest,
  FsListResponse,
  FsReadRequest,
  FsReadResponse,
  FsStatRequest,
  FsStatResponse,
  FsWriteRequest,
  FsWriteResponse,
  LogSendRequest,
  MenuActionEvent,
  MenuShowWindowMenuRequest,
  NotificationShowRequest,
  NotificationShowResponse,
  SecretDeleteRequest,
  SecretDeleteResponse,
  SecretHasRequest,
  SecretHasResponse,
  SecretSetRequest,
  SecretSetResponse,
  SecretStatusResponse,
  SettingsChangeEvent,
  SettingsGetRequest,
  SettingsGetResponse,
  SettingsResetResponse,
  SettingsSetRequest,
  SettingsSetResponse,
  ThemeChangeEvent,
  ThemeGetResponse,
  ThemeSetRequest,
  ThemeSetResponse,
  WindowIsMaximizedResponse,
  WindowMaximizeChangeEvent,
} from './types';

/**
 * Runtime platform, narrowed for cross-process sharing (no Node types here).
 * Tauri reports the same triples Electron did: darwin, linux, win32.
 */
export type DesktopPlatform = 'darwin' | 'linux' | 'win32' | (string & {});

export interface DesktopApi {
  platform: DesktopPlatform;

  appGetInfo: () => Promise<AppGetInfoResponse>;
  appOpenExternal: (data: AppOpenExternalRequest) => Promise<AppOpenExternalResponse>;
  appShowInFolder: (data: AppShowInFolderRequest) => Promise<AppShowInFolderResponse>;
  appOpenPath: (data: AppOpenPathRequest) => Promise<AppOpenPathResponse>;

  dialogOpenFile: (data?: DialogOpenFileRequest) => Promise<DialogOpenFileResponse>;

  logSend: (data: LogSendRequest) => Promise<void>;

  settingsGet: (data: SettingsGetRequest) => Promise<SettingsGetResponse>;
  settingsSet: (data: SettingsSetRequest) => Promise<SettingsSetResponse>;
  settingsReset: () => Promise<SettingsResetResponse>;
  onSettingsChange: (callback: (event: SettingsChangeEvent) => void) => () => void;

  secretHas: (data: SecretHasRequest) => Promise<SecretHasResponse>;
  secretSet: (data: SecretSetRequest) => Promise<SecretSetResponse>;
  secretDelete: (data: SecretDeleteRequest) => Promise<SecretDeleteResponse>;
  secretStatus: () => Promise<SecretStatusResponse>;

  themeGet: () => Promise<ThemeGetResponse>;
  themeSet: (data: ThemeSetRequest) => Promise<ThemeSetResponse>;
  onThemeChange: (callback: (event: ThemeChangeEvent) => void) => () => void;

  windowMinimize: () => Promise<void>;
  windowMaximize: () => Promise<void>;
  windowClose: () => Promise<void>;
  windowIsMaximized: () => Promise<WindowIsMaximizedResponse>;
  onWindowMaximizeChange: (callback: (event: WindowMaximizeChangeEvent) => void) => () => void;

  menuShowWindowMenu: (data?: MenuShowWindowMenuRequest) => Promise<void>;
  onMenuAction: (callback: (event: MenuActionEvent) => void) => () => void;

  notificationShow: (data: NotificationShowRequest) => Promise<NotificationShowResponse>;

  aiComplete: (data: AiCompleteRequest) => Promise<AiCompleteResponse>;
  aiCancel: (data: AiCancelRequest) => Promise<AiCancelResponse>;
  onAiStreamChunk: (callback: (event: AiStreamChunkEvent) => void) => () => void;

  dndGetPathForFile: (file: File) => Promise<DndResolvedPath>;
  dndResolvePaths: (data: DndResolvePathsRequest) => Promise<DndResolvePathsResponse>;

  fsRead: (data: FsReadRequest) => Promise<FsReadResponse>;
  fsWrite: (data: FsWriteRequest) => Promise<FsWriteResponse>;
  fsList: (data: FsListRequest) => Promise<FsListResponse>;
  fsStat: (data: FsStatRequest) => Promise<FsStatResponse>;
}
