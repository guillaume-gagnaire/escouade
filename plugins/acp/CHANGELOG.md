# Changelog

## 2.2.3

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Accounts and harnesses (ESC-387).

## 2.2.2

- Its page has its documentation, in French and in English: README.fr.md, README.en.md and five pages in docs/.

## 2.2.1

- Built on escouade-acp 0.1.3: a path with `..` is no longer taken for one in the agent's folder, and a write in `.git` goes to a card in « Édits auto ».
- Its fence and read-only, given again by Escouade, survive a restart of the plugin (a change of language or network).
- An agent whose plugin started again keeps its session, its turn and the permissions it waits for.
- Its page says it is given the MCP token of the launch it describes.

## 2.2.0

- A WASM module (`run.wasm`), run in Escouade's host: no process of its own any more. It translates what an agent says as it did. It has
  its own version, is published on the store and can be uninstalled.
- The files an agent gives it go by the host (`fs/*`): it reads in the agent's folder and its repository and writes in the agent's folder,
  4 MiB a file at most, and a private project's are refused by the host. Before, it reached any path out of the fence.
- Moved to `plugins/acp/` of Escouade's repository (it was `plugins/builtin/dev.escouade.acp/`), with its own LICENSE
  (GPL-3.0-or-later) and this file. Its id, its code and what it does do not change; its version is still the app's.
