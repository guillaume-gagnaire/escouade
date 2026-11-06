# Changelog

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more, and no tokio. It reaches `opencode serve` by
  `local:shared` (the port the core read, and no other), reads its events by `net/stream`, and opens them again the same way after a
  cut. Its agents converse as they did. It has its own version, is published on the store and can be uninstalled.
- Moved to `plugins/opencode/` of Escouade's repository (it was `plugins/builtin/dev.escouade.opencode/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
