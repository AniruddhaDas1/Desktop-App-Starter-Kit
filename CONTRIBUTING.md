# Contributing

Thanks for helping. This repo is a starter template, so contributions are judged on one thing
above all: **does it make the next feature land in the same shape as the last one?**

## Before you start

1. Read [CLAUDE.md](./CLAUDE.md) in full. It is the architecture rulebook, for humans too.
2. Read [NOTICE.md](./NOTICE.md) — this project ports the UI layer from an MIT-licensed upstream.

## Setup

```bash
npm install
npm run tauri dev
```

Prerequisites (Node 20.19+, Rust 1.77+, platform webview deps) are listed in the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## The checks we run

Every PR must pass all of these locally:

```bash
npm run typecheck
npm run lint
npm run test
cargo fmt   --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

CI runs the same list on Ubuntu; macOS and Windows builds are covered by the release workflow.

## Adding a screen

```bash
npm run new:feature MyScreen
```

Then edit `src/features/my-screen/components/MyScreenScreen.tsx`. Do not touch `App.tsx` or
`Sidebar.tsx` — the registry entry is the whole integration.

## Adding a backend command

```bash
npm run new:command demo:ping
```

That writes the Rust command, registers it in `mod.rs` + `lib.rs`, and adds the bridge wrapper and
type. Fill in the body in `src-tauri/src/commands/demo.rs`.

If your command calls a Tauri plugin directly from JS anywhere, add its permission to
`src-tauri/capabilities/default.json`.

## Style

- Match the surrounding file. Predictability beats cleverness.
- No `any` in TypeScript; no `unwrap()` on I/O in Rust.
- Colors, spacing and motion come from the tokens in `src/renderer/styles/global.css`.
- Comments only where the WHY is not obvious.

## Reporting bugs

Include your OS, the Tauri version (`npm run tauri -- info`), and the exact commands you ran.
For permission errors, include `src-tauri/capabilities/default.json`.
