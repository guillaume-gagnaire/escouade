# Changelog

## 2.2.7

- Under Windows, its Vibe view puts the folder of `escouade-hook` first on the `PATH` only when it is whole (a drive and its root, or a share, with no `;` nor `"`): an `escouade-hook` at a drive's root no longer makes it `C:`, the drive's current folder (escouade-acp 0.1.6, ESC-433).

## 2.2.6

- Under Windows, the hooks of its Vibe view call `escouade-hook` by its whole path (with `/`, no quotes), by its name only when a line cannot hold its path, and `NoDefaultCurrentDirectoryInExePath=1` keeps `cmd` from running an `escouade-hook.cmd` of the agent's folder (escouade-acp 0.1.5, ESC-430).

## 2.2.5

- Its manifest declares the environment variables its launches set (`permissions.env`: `VIBE_ENABLE_TELEMETRY`, `VIBE_ACTIVE_MODEL`, `VIBE_MCP_SERVERS`, `ESCOUADE_MCP_TOKEN`, `PATH`), shown before it is installed: from Escouade 2.2, a plugin sets no other (ESC-424).

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Accounts and harnesses and Advanced › Network (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use (the voice and Mistral Vibe), permissions, troubleshooting, develop.

## 2.2.1

- Built on escouade-acp 0.1.3: a path with `..` is no longer taken for one in the agent's folder, and a write in `.git` goes to a card in « Édits auto ».
- Its fence and read-only, given again by Escouade, survive a restart of the plugin (a change of language or network).
- An agent whose plugin started again keeps its session, its turn and the permissions it waits for.
- Its page says what is true today: one module, no secret of its own, the MCP token of the launch it describes.

## 2.2.0

- Its « Vue Vibe » is read by the module (`view/translate`), no longer by a translation of the core's.
- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. The dictation goes by `net/fetch`,
  the voice by `net/stream`, the transcription as one speaks by a WebSocket (`ws/*`), cut and resumed as before. The code of the
  service is the crate `escouade-voice`. Mistral Vibe is served in the same module, on the client of `escouade-acp`. It has its own version, is published on the store and can be
  uninstalled.
- Moved to `plugins/mistral/` of Escouade's repository (it was `plugins/builtin/dev.escouade.mistral/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
