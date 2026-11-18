# CLAUDE.md — Instructions for Claude Code

> **Read this file before writing any code.** It says where every kind of code goes in this repo.
> **These rules override generic best practices.** When this file conflicts with a habit, follow this file.

## Project context

**Desktop App Starter Kit - Tauri** is a desktop app starter: **Tauri v2 (Rust), React 19, TypeScript 5.9 (strict), Tailwind CSS 4, Vite 7.**
It already runs. The person using it asks you for features in plain words; you add them the way the
rest of the app is built, reusing what is already here.

This is a port of the Electron-based [desktop-app-starter](https://github.com/hassancs91/desktop-app-starter).
The React side is intentionally the same. The backend is Rust, not Node.

Targets: Windows (NSIS), macOS (DMG), Linux (AppImage + deb). All code must work on all three.

Every decision favors **predictability**: the same request should produce the same structure every time.

---

## Quick file orientation

| When you need to…                       | Look here / put it here                                                                                   |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Add a new screen                        | `npm run new:feature MyScreen`, or `src/features/{name}/` + one entry in `src/renderer/screens-registry.ts` |
| Add a new backend command               | `npm run new:command domain:action`, or the 3 steps below                                                  |
| Call an AI provider                      | `src-tauri/src/commands/ai/`; the renderer reaches it only through `window.api.aiComplete`                  |
| Let the user pick a file or folder      | `window.api.dialogOpenFile({ directory?, multiSelections?, filters? })` (picked paths are auto-blessed)     |
| Accept files or folders by drag-and-drop | `useFileDrop()` from `src/renderer/hooks/useFileDrop.ts`                                                   |
| Read / write / list files               | `src-tauri/src/commands/fs.rs` (allow-listed) via the `fs:*` bridge calls                                  |
| Open a file or reveal it in the OS      | `window.api.appOpenPath` / `window.api.appShowInFolder`                                                    |
| Add a backend service/module            | A new file under `src-tauri/src/commands/`, registered in `mod.rs` and `lib.rs`                             |
| Add a setting                           | Extend `AppSettings` in `src-tauri/src/state.rs` **and** the Zod schema in `src/shared/validation/settings-schema.ts` |
| Read/write a setting                    | `useSetting('key')` (renderer). The value is persisted by Rust in `settings.json`                           |
| Store a secret (API key)                | `src-tauri/src/commands/secrets.rs`; renderer uses `useApiKey(name)` (never sees the raw value)              |
| Log                                     | Rust: `log::info!` · renderer: `useLogger('Name')` (forwards to `log:send`)                                  |
| Show a toast                            | `useToast()` from `src/renderer/contexts/ToastContext.tsx`                                                  |
| Ask the user to confirm                 | `useConfirm()` from `src/renderer/contexts/DialogContext.tsx`                                               |
| Show a native OS notification           | `window.api.notificationShow({ title, body })`                                                              |
| Validate a payload                      | Zod schemas in `src/shared/validation/` — validate at the boundary, both directions                          |
| Animate                                 | `motion` (`import { motion, AnimatePresence } from 'motion/react'`) or `FadeIn`/`SlideIn`/`ScaleIn`          |
| Use a UI component                      | `src/shared/components/` (index.ts). See them live on the **Components** screen                              |
| Add an icon                             | `lucide-react` through `src/renderer/components/Icon.tsx` (never inline SVG)                                 |
| Keyboard shortcut in the window         | `useShortcut({ id, accelerator: 'CmdOrCtrl+K' }, handler)`                                                  |
| Build a form with validation            | `react-hook-form` + a Zod schema (see `src/features/settings/components/SettingsForm.tsx`)                   |
| Change window chrome                    | `src-tauri/tauri.conf.json` + the per-OS overrides                                                          |

---

## Architecture rules (HARD — never break)

1. **The renderer never talks to Tauri directly.** `src/renderer/`, `src/features/` and
   `src/shared/` must not import `@tauri-apps/*`. The only exceptions are
   `src/lib/bridge.ts` and `src/renderer/hooks/useFileDrop.ts`. Everything else calls `window.api`.
2. **`window.api` is a fixed contract.** It is typed in `src/shared/ipc/preload-api.ts` and
   implemented in `src/lib/bridge.ts`. Adding a function means updating both, plus the Rust command.
3. **Feature modules are isolated.** `src/features/a/` MUST NEVER import from `src/features/b/`.
   Shared code goes in `src/shared/`.
4. **Rust holds business logic and every privileged operation.** Components → hooks → `window.api`
   → Rust command. Components contain no business logic, file I/O, network calls or OS calls.
5. **One file, one concern, ~400 lines max.** Split when it grows past that. One Rust module per
   domain (`secrets.rs`, `fs.rs`, `ai.rs`, …), not one giant `lib.rs`.
6. **Types live next to usage.** Feature types in `src/features/{x}/types.ts`. Cross-boundary types
   in `src/shared/ipc/types.ts`.

### Dependency direction (NEVER violate)

```
src-tauri/ (Rust)  ──┐
                     ├─→  window.api (typed façade, src/lib/bridge.ts)
src/features/      ──┤
src/renderer/      ──┘         src/shared/ depends on nothing in this project
```

- `features/` may also use `renderer/contexts/` and `renderer/hooks/`.
- Rust and the renderer never import from each other. They talk over the bridge.

---

## Folder structure

```
src-tauri/                     # Rust backend (replaces Electron main + preload)
  src/
    main.rs                    # Entry point
    lib.rs                     # Builder, plugins, window events, generate_handler![]
    state.rs                   # AppState: settings, blessed paths, cancel tokens
    error.rs                   # AppError -> { success: false, error, code? }
    commands/                  # One module per domain
      mod.rs                   # `pub mod ...` list
      app.rs window.rs menu.rs settings.rs theme.rs secrets.rs
      fs.rs dialog.rs notification.rs log.rs ai.rs
  tauri.conf.json              # Window, bundle, CSP
  tauri.macos.conf.json        # Overlay titlebar (traffic lights)
  tauri.windows.conf.json      # decorations: false
  tauri.linux.conf.json        # decorations: false
  capabilities/default.json    # Permissions granted to the webview

src/
  lib/bridge.ts                # window.api implementation (the ONLY Tauri import, + useFileDrop)
  renderer/
    App.tsx                    # Shell + providers + screen routing
    screens-registry.ts        # Every screen: id, label, icon, component
    components/                # Shell: Sidebar, TitleBar, StatusBar, ErrorBoundary, Icon
    contexts/                  # Toast, Dialog (confirm), Theme
    hooks/                     # useApiKey, useFileDrop, useLogger, useSetting, useShortcut, ...
    styles/global.css          # Tailwind 4 + design tokens (@theme)
    types/desktop-api.d.ts     # Declares window.api
  features/                    # One folder per screen: components/, hooks/, types.ts
  shared/
    ai/models.ts               # provider catalogue, model prices, defaults
    ipc/channels.ts            # Channel names (documentation; Rust uses the snake_case twin)
    ipc/types.ts               # Request/response types per channel
    ipc/preload-api.ts         # The DesktopApi interface
    components/                # UI components + layout/ + motion/
    validation/                # Zod schemas
  logging/                     # Renderer logger, forwards to log:send
```

---

## Design patterns (use these; don't invent new ones for the same problem)

### Rust command (canonical shape)

```rust
// src-tauri/src/commands/example.rs
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]   // JS sends camelCase
pub struct ExampleDoRequest {
    pub input: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleDoResponse {
    pub success: bool,
    pub result: String,
}

#[tauri::command]
pub fn example_do(state: State<'_, AppState>, data: ExampleDoRequest) -> Result<ExampleDoResponse, AppError> {
    // 1. Re-validate anything crossing the boundary (the webview is untrusted input).
    // 2. Do the work; use `state` for shared data.
    Ok(ExampleDoResponse { success: true, result: data.input })
}
```

- Command name is the channel name with `:` → `_` (`fs:read` → `fs_read`).
- Every command that touches disk, network or the OS is `async` and never blocks the main thread.
- Never `unwrap()` on user input or I/O. Return `AppError`.

### Bridge wrapper (canonical shape)

```ts
// src/lib/bridge.ts
exampleDo: (data) => invoke<ExampleDoResponse>('example_do', { data }),
```

The wrapper is one line. Payloads go in the `data` argument so Rust's `data:` parameter matches.

### Hook-as-controller (renderer)

Components never call `window.api` for data; a hook does, and the component renders its state.

```ts
export function useExample() {
  const [state, setState] = useState<AsyncState<Result>>({ kind: 'idle' });
  const run = useCallback(async (input: string) => {
    setState({ kind: 'loading' });
    const res = await window.api.exampleDo({ input });
    setState(res.success ? { kind: 'data', value: res.result } : { kind: 'error', error: 'Failed' });
  }, []);
  return { state, run };
}

type AsyncState<T> = { kind: 'idle' } | { kind: 'loading' } | { kind: 'data'; value: T } | { kind: 'error'; error: string };
```

### Push events (Rust → renderer)

Rust: `app.emit("domain:event", payload)`. Bridge: `onSomething: (cb) => subscribe('domain:event', cb)`.
`subscribe()` returns an unsubscribe function; hooks must return it from `useEffect`.
`ai:stream-chunk` is the worked example.

### AI calls (providers)

Anthropic, OpenAI-compatible endpoints and Google Gemini are adapters under
`src-tauri/src/commands/ai/`. The API key is stored by the user on the **AI** screen and read inside
Rust only.

```rust
// in src-tauri/src/commands/ai/mod.rs
let api_key = read_secret(state, provider.secret_key())?;   // never returned to JS
```

- Provider comes from the `aiProvider` setting; the model comes from `aiModel` unless the request
  passes one.
- OpenAI-compatible endpoints use `aiBaseUrl`. A localhost base URL does not require a key.
- Adding a provider means a new adapter module plus a new entry in `src/shared/ai/models.ts`
  (renderer) and `provider.rs` (Rust).
- Streaming uses `ai:stream-chunk` events keyed by `requestId`; the invoke resolves when the stream ends.
- Errors to handle: missing key (tell the user to add one on the AI screen) and API errors.
- The renderer never sees the key. `aiComplete` is the only door.

### Files and folders

`fs.rs` only touches paths the user chose (native dialog or drag-and-drop) or `app_data_dir`.
Picking or dropping a folder allows everything inside it. **Never widen this allow-list to make a
feature work**; ask the user to pick the folder. New file operations (move, rename, delete) belong
in a Rust command that re-checks `is_allowed()` for every source and destination path.

### Settings

`useSetting('key')` in the renderer; the value persists in `settings.json` under `app_data_dir`.
Rust owns the store. Never `localStorage` for user-visible state. A new key needs **three** edits:
`AppSettings` (Rust), the Zod schema (TS), and `value_of`/`set_value` (Rust).

### UI

- Use the components in `src/shared/components/` first. Check the Components screen before writing a new one.
- Layout primitives: `Stack`, `HStack`, `VStack`, `Grid`, `Spacer`, `Divider`, `Container`, `PageHeader`.
- Screens follow the existing shape: a 40px toolbar with the title, then a scrolling content area
  (copy `src/features/explorer/`).
- Colors and spacing come from the design tokens in `global.css` (`bg-app-surface`, `text-text-muted`,
  `text-accent-light`, ...). Never hardcode hex colors in components.
- Animation: `motion` for layout and list animation, the `motion/` primitives for simple enters.
  Respect `prefers-reduced-motion` (`useReducedMotion()` from `motion/react`).
- Icons: `lucide-react` via `Icon`. Never inline `<svg>`.

---

## Command wiring (the 3 places)

Every new command is wired in **exactly these places**. `npm run new:command domain:action`
scaffolds all of them.

1. **Rust command**: `src-tauri/src/commands/{domain}.rs`, listed in `commands/mod.rs`
2. **Registration**: an import in `src-tauri/src/lib.rs` + an entry in `generate_handler![...]`
3. **Bridge surface**: `src/lib/bridge.ts` (invoke wrapper) + `src/shared/ipc/preload-api.ts`
   (declaration) + types in `src/shared/ipc/types.ts`

Names are `domain:action`, lowercase (`files:scan`, not `scanFiles`).

### Capabilities

The webview gets `core:default` only, because every privileged operation goes through a Rust
command. **If you call a Tauri plugin from JS**, you must add that plugin's permission to
`src-tauri/capabilities/default.json` — otherwise it fails silently at runtime. This is the most
common Tauri v2 contributor trap.

---

## Code style rules

TypeScript:

- Strict + `noUncheckedIndexedAccess`. **Never `any`**: use `unknown` and narrow.
- Functional React components. Named exports (no `export default`).
- `async/await`, never raw `.then()` chains.
- Path aliases: `@renderer/`, `@features/`, `@shared/`, `@lib/`, `@logging/`. No deep relative imports.
- Files: components `PascalCase.tsx`, hooks `useCamelCase.ts`, everything else `kebab-case.ts`,
  schemas `*-schema.ts`, tests in `__tests__/*.test.ts`.

Rust:

- `cargo fmt` clean, `cargo clippy -- -D warnings` clean.
- `snake_case` functions, `PascalCase` types, one module per domain.
- `serde(rename_all = "camelCase")` on every struct that crosses to JS.
- No `unwrap()`/`expect()` outside `main.rs` and tests.
- Comments only when the WHY is not obvious.

---

## Security rules (HARD — never relax)

1. `window.api` is the only door. The renderer must not import `@tauri-apps/*` (2 exceptions above).
2. CSP stays on in `tauri.conf.json`. Never relax it to make a feature work. Network calls belong in Rust.
3. Validate every payload that crosses the boundary. The webview is untrusted input.
4. File access stays inside the allow-list (see Files and folders). Canonicalize before checking, so
   symlinks cannot escape a blessed directory.
5. Secrets live in the OS credential store only. Never in settings, env files, logs, or the renderer.
6. External URLs open with `app_open_external`, never inside the window.
7. Never log secrets, API keys, or file contents.

---

## Cross-platform checklist (before writing code that touches the OS)

1. Paths: `PathBuf`/`Path` in Rust, `path.join()` in TS. Never literal `/` or `\`.
2. OS folders: `app.path().app_data_dir()`, `document_dir()`, `download_dir()`. Never hardcode.
3. Keyboard: `CmdOrCtrl` in accelerators.
4. Reading text files: handle `\r\n` and a UTF-8 BOM.
5. Subprocesses: avoid. If unavoidable, absolute path, args array, no shell.
6. Platform-only APIs get a `#[cfg(...)]` guard and a fallback.
7. Window chrome differs per OS: macOS uses `titleBarStyle: "Overlay"` (traffic lights), Windows and
   Linux use `decorations: false`. Keep `tauri.{os}.conf.json` in sync.

If you cannot test on an OS, say so. Don't claim it works there.

### Drag and drop

Tauri's native drag-drop **swallows HTML5 drop events on Windows**. All drops must go through
`useFileDrop()`, which listens to `onDragDropEvent` and blesses paths. Never read `dataTransfer.files`
for paths — a DOM `File` cannot be mapped back to a path in a webview.

---

## Error handling

- Rust commands return `Result<T, AppError>`; `AppError` serializes to `{ success: false, error, code? }`.
- The bridge does not throw for domain errors; it resolves the response object, as upstream did.
- Hooks surface a real message (toast or inline). Never a silent failure.
- `ErrorBoundary` is the renderer's safety net; don't wrap components in try/catch "just in case".

---

## Testing

- Unit-test pure functions, schemas and hooks with Vitest in `__tests__/` next to them.
- Rust logic gets `#[cfg(test)]` tests in the same module (`fs.rs` for allow-list/canonicalize cases).
- Every new feature gets at least one test of its core logic.
- Before saying a feature is done: `npm run typecheck`, `npm run lint`, `npm run test`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`. A clean typecheck alone is
  not proof it works.

---

## Not in this starter

There is no database, sign-in, auto-update, license keys, tray, i18n, command palette, or background
job runner here. If a request needs one, tell the user and propose the smallest addition that fits
the patterns above. Don't build a whole subsystem silently.

---

## Common AI mistakes to avoid

1. **Importing `@tauri-apps/*` outside `bridge.ts` / `useFileDrop.ts`.** Go through `window.api`.
2. **Re-implementing what exists.** Check `src/shared/components/`, `src/renderer/hooks/`,
   `src-tauri/src/commands/` first.
3. **Putting file, network or AI calls in the renderer.** They belong in a Rust command.
4. **Hand-writing CSS or a new styling system.** Use Tailwind classes with the existing tokens.
5. **Rewriting the app shell** (`App.tsx`, `Sidebar.tsx`) to add a screen. Add one registry entry.
6. **Short-circuiting the 3-place command wiring.** All three, every time.
7. **Forgetting a capability entry** when calling a plugin from JS.
8. **Blocking the Rust main thread** with sync I/O in an async command.
9. **Over-abstracting after one example.** Extract on the third use.
10. **Generic catch-all error handling** (`catch (e) { console.log(e) }`).
11. **Inventing config options nobody asked for.**
12. **Using `any`** to quiet TypeScript.
13. **Hardcoding a model id.** Use the setting or `src/shared/ai/models.ts`.
14. **Widening the file allow-list** instead of asking the user to pick a folder.
15. **Claiming work is done before running it.**
16. **Modifying this file without being asked.**

---

## Commands

```bash
npm install            # once after clone
npm run tauri dev      # start the app (Vite + Rust, hot reload)
npm run dev            # frontend only (browser at http://localhost:1420)
npm run typecheck      # tsc --noEmit
npm run lint           # ESLint
npm run test           # Vitest
npm run new:feature    # scaffold a screen (registered in the Sidebar)
npm run new:command    # scaffold a Rust command in all 3 places
npm run rebrand -- --name "My App" --id com.me.myapp
npm run tauri build    # native installer for the current OS
```

Rust checks:

```bash
cargo check  --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt    --manifest-path src-tauri/Cargo.toml
```

## When in doubt

1. Re-read this file.
2. Find the closest existing example in the same domain and copy its shape.
3. Ask the user rather than invent a new pattern.
