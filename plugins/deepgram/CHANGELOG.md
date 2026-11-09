# Changelog

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Advanced › Network (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use, permissions, troubleshooting, develop.

## 2.2.1

- Its page says what is true today: a plugin of the store, with its own version, that can be uninstalled.
- It says a plugin of another author cannot serve this service: the key of a service only goes to Escouade's own plugin.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. The dictation goes by `net/fetch`,
  the voice by `net/stream`, the transcription as one speaks by a WebSocket (`ws/*`), cut and resumed as before. The code of the
  service is the crate `escouade-voice`. It has its own version, is published on the store and can be
  uninstalled.
- Moved to `plugins/deepgram/` of Escouade's repository (it was `plugins/builtin/dev.escouade.deepgram/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
