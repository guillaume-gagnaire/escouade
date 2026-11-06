# Changelog

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates as it did; the core
  still starts Cursor CLI. It has its own version, is published on the store and can be uninstalled.
- Moved to `plugins/cursor/` of Escouade's repository (it was `plugins/builtin/dev.escouade.cursor-cli/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
