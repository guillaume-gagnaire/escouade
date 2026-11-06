# Changelog

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. The dictation goes by `net/fetch`,
  the voice by `net/stream`, the transcription as one speaks by a WebSocket (`ws/*`), cut and resumed as before. The code of the
  service is the crate `escouade-voice`. It has its own version, is published on the store and can be
  uninstalled.
- Moved to `plugins/openai/` of Escouade's repository (it was `plugins/builtin/dev.escouade.openai/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
