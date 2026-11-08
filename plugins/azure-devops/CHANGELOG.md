# Changelog

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Connected services and Advanced › Network (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use, permissions, troubleshooting and develop.

## 2.2.1

- Its page says what is true today: a plugin of the store, with its own version, that can be uninstalled.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It answers as it did. It has its own version, is
  published on the store and can be uninstalled. It may reach the instances its accounts name (`{account}`), as it did as a process.
- Moved to `plugins/azure-devops/` of Escouade's repository (it was `plugins/builtin/dev.escouade.azure-devops/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
