# Changelog

## 2.3.0

- A pull request read whole, for Escouade 2.3 (ESC-452): its description (`forge/description`, with its tip's commit and when it last changed), its commits (`forge/commits`), its files and their counts (`forge/files`; their lines are git's, read by Escouade) and its threads (`forge/threads`), with their lines, their answers and their state: open, resolved or outdated. Read only: nothing is written on GitHub. It stands on escouade-plugin-kit 0.1.3.

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Connected services and Advanced › Network (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: install, use, permissions, troubleshooting and develop.

## 2.2.1

- A next page outside the API is not followed: the token stays on it.
- Its page names the hosts of a job's log, reached without the token.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It answers as it did. It has its own version, is
  published on the store and can be uninstalled. It may reach the instances its accounts name (`{account}`), as it did as a process.
- Moved to `plugins/github/` of Escouade's repository (it was `plugins/builtin/dev.escouade.github/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id does not change.
