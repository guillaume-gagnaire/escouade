# Changelog

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates what an agent says as it did. It has
  its own version, is published on the store and can be uninstalled.
- The files an agent gives it go by the host (`fs/*`): it reads in the agent's folder and its repository and writes in the agent's folder,
  4 MiB a file at most, and a private project's are refused by the host. Before, it reached any path out of the fence.
- Moved to `plugins/acp/` of Escouade's repository (it was `plugins/builtin/dev.escouade.acp/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
