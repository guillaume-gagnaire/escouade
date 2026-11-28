# Changelog

## 2.2.7

- The 2.2 and 2.3 lines of Escouade ship the same plugin again: each turn’s spend by model, a subagent’s steps on their message’s model (`turnModels`, 2.2.6 and ESC-662), still written in JSON on the published protocol crate (ESC-701).

## 2.2.6

- Each step of a subagent now counts on its message's model (`providerID/modelID`), another provider included, not its agent's: each turn tells Escouade its spend by model (`turnModels`, written in JSON on the published protocol crate), once in the agent's totals (ESC-691).

## 2.2.5

- Its manifest declares the environment variables its launches set (`permissions.env`: `OPENCODE_CONFIG_CONTENT`, `OPENCODE_DISABLE_AUTOUPDATE`, `OPENCODE_ENABLE_QUESTION_TOOL`, `OPENCODE_SERVER_PASSWORD`), shown before it is installed: from Escouade 2.2, a plugin sets no other (ESC-424).

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Accounts and harnesses and a project’s Project page (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: README.fr.md, README.en.md and five pages in docs/.

## 2.2.1

- `git add` and `git commit` are accepted at once only in their plain forms: a redirection, `-F`, a second option go to a card.
- An edit of `.git`, `.opencode`, `opencode.json`, `.husky` or `.githooks` goes to a card, for a subagent too.
- The rules of a resumed or copied session are set before it opens, and refused they refuse it; a change of mode waits for them.
- Under a fence: its `edit` rules hold inside the agent's folder, « Toujours » is not offered, `git commit -F` is refused.
- `websearch` asks as `webfetch` does. The README says what its password protects from.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more, and no tokio. It reaches `opencode serve` by
  `local:shared` (the port the core read, and no other), reads its events by `net/stream`, and opens them again the same way after a
  cut. Its agents converse as they did. It has its own version, is published on the store and can be uninstalled.
- Moved to `plugins/opencode/` of Escouade's repository (it was `plugins/builtin/dev.escouade.opencode/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
