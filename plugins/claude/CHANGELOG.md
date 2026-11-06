# Changelog

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates what `claude` says as it did. It has
  its own version, is published on the store and can be uninstalled.
- No more `tokio` nor `signin.rs`: what Claude Code keeps of a sign-in is read by the core alone.
- Moved to `plugins/claude/` of Escouade's repository (it was `plugins/builtin/dev.escouade.claude-code/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
