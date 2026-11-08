# Changelog

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Accounts and harnesses and a project’s Project page (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: README.fr.md, README.en.md and five pages in docs/.

## 2.2.1

- The text of a command is read as its shell reads it, never as an argv: `git add .; node evil.js` goes to a card.
- A write in `.git` goes to a card, whatever its case or its Windows name.
- Its fence and read-only, given again by Escouade, survive a restart of the plugin (a change of language or network).
- It reads `resolvedPath`, `managedApprovalRequired`, `resolvedPaths` and `possibleUrls` as Copilot CLI sends them, and offers no rule of the session to a fenced agent.
- A commit that reads a file (`-F`, `--file`) goes to a card for a fenced agent.

## 2.2.0

- Its « Vue Copilot » is read by the module (`view/translate`): the core no longer links this crate.
- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates as it did; the core
  still starts Copilot CLI. It has its own version, is published on the store and can be uninstalled.
- Moved to `plugins/copilot/` of Escouade's repository (it was `plugins/builtin/dev.escouade.copilot-cli/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
