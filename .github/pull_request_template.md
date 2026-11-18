## What changed

## Why

## Checks

- [ ] `npm run typecheck`
- [ ] `npm run lint`
- [ ] `npm run test`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

## Architecture

- [ ] No `@tauri-apps/*` import outside `src/lib/bridge.ts` / `useFileDrop.ts`
- [ ] New commands are wired in all 3 places
- [ ] No capability entry needed (or added)
- [ ] The file allow-list was not widened
