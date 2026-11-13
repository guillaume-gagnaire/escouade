# Changelog

## 2.2.7

- Its process sends to a card a write whose path goes back up into Copilot’s settings (`.github/x/../hooks`): a `..` no longer hides `.github/hooks`, `.github/copilot` or `.copilot`, and a folder named of dots and spaces alone, which Windows may read as another one, goes to a card too.

## 2.2.6

- In its Copilot CLI view, every write asks in Accept edits and Auto: `--allow-tool=write` covered the worktree’s `.git` too (a hook, then a `git commit` with no question), and nothing proves a refusal by path covers the files under `.git`. Writes in `.git`, `.copilot`, `.github/copilot`, `.github/hooks` and the joined files’ folder are refused there in every mode (`--deny-tool`), and its process sends those of Copilot’s settings to a card, as those of `.git` (ESC-430).
- Its view no longer says it holds a fence nothing holds: a fenced agent’s view is refused, and says why. A discussion is held read-only there by `--deny-tool=write` and `--deny-tool=shell`.
- Its view gets Escouade’s token in the core’s private file alone (Escouade 2.3), in no variable, and `ESCOUADE_MCP_TOKEN` is among `--secret-env-vars`; its hooks double every quote PowerShell reads as `'` (`‘ ’ ‚ ‛`).

## 2.2.5

- Its manifest declares the environment variables its launches set (`permissions.env`: `ESCOUADE_MCP_TOKEN`, `COPILOT_GITHUB_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`), shown before it is installed: from Escouade 2.2, a plugin sets no other (ESC-424).

## 2.2.4

- The launch of its Copilot CLI view (`terminal/launch`) and the reads of an account by a server of its own (`account/read`: its models, its quota, the models listed before the view starts, the copy of a session) are the plugin’s, checked by the core, which keeps the terminal (ESC-359).

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
