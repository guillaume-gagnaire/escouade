# Changelog

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Accounts and harnesses and a project’s Project page (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: README.fr.md, README.en.md and five pages in docs/.

## 2.2.1

- Its fence and read-only, given again by Escouade, survive a restart of the plugin (a change of language or network).
- A commit that reads a file (`-F`, `--file`) goes to a card for a fenced agent.

## 2.2.0

- Its « Vue Codex » is read by the module (`view/translate`): the core no longer links this crate.
- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates as it did; the core
  still starts Codex. It has its own version, is published on the store and can be uninstalled.
- Moved to `plugins/codex/` of Escouade's repository (it was `plugins/builtin/dev.escouade.codex/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
