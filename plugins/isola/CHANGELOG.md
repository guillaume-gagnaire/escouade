# Changelog

## Unreleased

- Its page has its documentation in French and in English: README.fr.md, README.en.md and the pages of docs/fr and docs/en (`store.docs`).

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. The host runs `isola ls --json`,
  `isola down` and `isola destroy` (`proc/*`), and nothing else: the three commands of `permissions.commands`, shown in
  the window of its permissions. It answers as it did. It has its own version, is published on the store and can be
  uninstalled.
- A private project has it only once it is turned on there, like any plugin.
- Moved to `plugins/isola/` of Escouade's repository (it was `plugins/builtin/dev.escouade.isola/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
