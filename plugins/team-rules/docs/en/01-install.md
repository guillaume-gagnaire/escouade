# Installing Team rules

## 1. Install it

1. In Escouade, open the Plugins view (the puzzle piece icon in the projects bar).
2. **Store**, search for “Team rules”, then **Install**. You can also start from its page on <https://store.escouade.dev/plugins/dev.escouade.team-rules> and **Open in Escouade**.
3. The “Install Team rules?” window says what it will be able to do: run inside Escouade, isolated, and, in red, “Decides your agents’ permissions”. **Install**.

It needs Escouade 2.2.0 or later.

Escouade starts it at once, and then with the app: its first decision does not wait for it to start.

## 2. Choose the projects

**Projects** tab of its page:

- it is on in every project that is not private: the agents of those projects are subject to it;
- a private project is off by default: its agents are subject to it only if you turn it on there;
- an `.escouade.yml` never turns it on: only your own gesture does.

## 3. Read its rules

It has no setting. Its rules are those of the `rules.json` published with it; the Use tab details them. For other rules, make your own copy of the plugin (the Use tab says how).

## What Escouade keeps, and where

- **The plugin**: its files, `rules.json` included, in `~/.escouade/plugins/dev.escouade.team-rules`, and its install (version, fingerprint, your choices per project) in `~/.escouade/plugins.json`. Its compiled WASM module is kept in Escouade’s cache, and made again if it goes.
- **Its journal**: each decision, in the **Journal** tab of its page (“Decision” filter) and whole in `~/.escouade/plugin-logs/dev.escouade.team-rules.log`.

It has no secret and no setting. Uninstalling erases its files and its journal.
