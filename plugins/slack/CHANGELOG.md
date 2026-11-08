# Changelog

## 2.0.2

- Its messages, README and documentation send to the settings pages of Escouade 2.2: Advanced › Network and the plugin’s own Settings tab (ESC-387).

## 2.0.1

- Its page has its documentation in French and in English: README.fr.md, README.en.md and the pages of docs/fr and docs/en (`store.docs`).

## 2.0.0 (2026-11-02)

- The program runs in WASM inside Escouade (`"run": { "js": "dist/plugin.mjs" }`): no Node needed on the user's machine, and
  the launch scripts `slack` and `slack.cmd` are gone. It reaches `slack.com` and nothing else: the app enforces `permissions.network`. Node 22.18 or later is only for building and testing it.
- Needs Escouade 2.2.0 or later (`escouade.min`); the 1.x versions stay on the store for older apps. The code does not change.

## 1.1.0 (2026-10-31)

- A plugin of Escouade's own hand: its id is `dev.escouade.slack` (it was `dev.exemple.slack`), and it lives in `plugins/slack/` of
  Escouade's repository, submitted to the store from that folder (its tags `plugins/slack/v1.2.3`). Nothing else
  changes; the old id leaves the store: install this one from the Plugins view.

## 1.0.1 (2026-10-30)

- A title, a key or a name is escaped (`&`, `<`, `>`): `<!channel>` from a public issue no longer notifies the
  channel.

## 1.0.0 (2026-10-30)

- Its first version.
