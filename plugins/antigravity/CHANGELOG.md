# Changelog

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It says what it said on the recorded streams
  of `agy`. It reads the agent's home by the host (`permissions.files`, `agentHome`, in reading), and only the last 4 MiB of a
  transcript. A transcript written late no longer holds the plugin: what follows is held and sent once it is read. It has its own
  version, is published on the store and can be uninstalled.
- Moved to `plugins/antigravity/` of Escouade's repository (it was `plugins/builtin/dev.escouade.antigravity-cli/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
