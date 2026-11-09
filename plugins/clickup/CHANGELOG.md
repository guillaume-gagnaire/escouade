# Changelog

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Connected services, a project’s Imported tickets, Advanced › Network and General (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use, permissions, troubleshooting and develop.

## 2.2.1

- Reads answers of up to 8 MiB, within 15 s, as the code hosts do: a large board no longer stops at 1 MiB.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It answers as it did. It has its own version, is
  published on the store and can be uninstalled.
- Moved to `plugins/clickup/` of Escouade's repository (it was `plugins/builtin/dev.escouade.clickup/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
