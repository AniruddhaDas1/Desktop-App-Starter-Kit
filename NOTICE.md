# NOTICE

**Desktop App Starter Kit - Tauri** is a Tauri v2 port of **Desktop App Starter** by Hasan Aboul Hasan.

- Upstream: https://github.com/hassancs91/desktop-app-starter
- Upstream commit used as the reference: `9b95a85688123169329388d5a7f55cea14504f8d`
- License: MIT (see `LICENSE`)

## What was adapted from upstream

The following are derived from the upstream repository:

- `src/shared/components/**` — the UI component library (21 components, plus 8 layout and
  3 motion primitives)
- `src/renderer/styles/global.css` — design tokens and the six themes
- `src/renderer/contexts/**`, `src/renderer/hooks/**`, `src/renderer/components/**`
- `src/features/**` — Home, Explorer, AI, Components gallery, Settings screens
- `src/shared/validation/**` — Zod schemas
- `src/shared/ai/models.ts` — model catalogue and pricing helpers
- `src/shared/ipc/types.ts` — cross-process request/response types
- `docs/**`, `brand/**`, `resources/icon.png`
- The project conventions documented in `CLAUDE.md`

## What was rewritten for Tauri

- `src-tauri/**` replaces the Electron main process and preload
- `src/lib/bridge.ts` replaces `src/preload/preload.ts`
- `src/renderer/hooks/useFileDrop.ts` uses the webview drag-drop event instead of
  `webUtils.getPathForFile`
- `src/renderer/components/TitleBar.tsx` uses `data-tauri-drag-region`
- `plopfile.mjs` generates Rust commands instead of Electron IPC handlers
- The `src/main`, `src/preload`, `electron-builder.yml` and `electron.vite.config.mjs` layers were
  removed

The upstream copyright notice is preserved in `LICENSE`.
