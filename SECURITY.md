# Security Policy

## Reporting a vulnerability

Please do not open a public issue for security problems. Use this repository's **Security → Report a
vulnerability** (GitHub private vulnerability reporting) instead.

Include: what you did, what happened, and the expected behaviour. A proof of concept helps.

## Scope

This is a starter template used to build other apps. The security properties it promises are:

- **Secrets stay out of the webview.** API keys live in the OS credential store and are read only
  inside Rust. The renderer receives a masked preview.
- **File access is allow-listed.** `fs:*` only touches `app_data_dir`, paths the user picked in a
  native dialog, or paths dropped onto the window. Symlinks are canonicalized before the check.
- **The webview is unprivileged.** `capabilities/default.json` grants `core:default` only; every
  privileged action goes through a Rust command.
- **CSP is on.** `tauri.conf.json` restricts scripts to `'self'`.

Breaking any of these is a security bug — please report it.

## Known limitations

- Installers produced by `npm run tauri build` are unsigned unless you configure signing.
- On Linux systems without a Secret Service backend, secrets fall back to a plaintext file under
  `app_data_dir`. The AI screen shows a warning when this happens. Do not store high-value secrets
  on such a machine.
