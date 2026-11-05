# Trello

`dev.escouade.trello`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les cartes de Trello dans le Kanban, leurs listes comme colonnes. Le premier des plugins de première main qui sont des
sources du Kanban (INT-7.1) : Linear, Jira et ClickUp suivent le même modèle.

## What it does

It is a ticket source of the Kanban for Trello (REST API 1): a board of the account is linked to a project, its open
cards are listed in the import (filtered by « Mes cartes », by list, by label, by text or by number), and a card
follows its ticket: it is moved to the list of the column the ticket comes into, and commented when the project asks
for it. The checklist named after the acceptance criteria (else any checklist, else the description's) gives the
criteria of the ticket.

## What it may do

- **Network**: `api.trello.com`, and nothing else.
- **Secrets**: the one of its account, `account:trello` (the API key and the token, `{"key":…,"token":…}`), which it
  asks of Escouade for each call (`secrets/get`); Escouade never puts it in a call.
- No command to run, no fact read, no file.

## Its account

One Trello account per Escouade. The API key and the token are made on <https://trello.com/power-ups/admin> (the key
of a Power-Up, then « Token »), typed in Réglages › Intégrations. They are kept in the system's keychain under
`plugin:dev.escouade.trello:account:trello`; the former name (`trello`) is kept beside it and read first, so that a
version from before reads the account as it is (docs/PLUGINS.md §4.4). Its texts (« Indique le jeton. », « Indique la
clé d'API. ») are in its manifest (`contributes.accounts[0].fields[].missing`).

It can be turned off for the app or for a project, never removed, and a plugin of the store with the same id is
refused: this one is the app's. Turned off, or stopped, the sync says so (« Le plugin dev.escouade.trello ne répond
pas ») and tries again later; nothing is lost.

## What is in this folder

| File                   | What it is                                          |
| ---------------------- | --------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                        |
| `Cargo.toml`           | The crate `escouade-plugin-trello`                  |
| `src/lib.rs`           | The client of Trello, and its `SourceKind` (the kit) |
| `README.md`            | This page                                           |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the kit
`escouade-plugin-kit`, which answers the protocol, asks Escouade for the account's secret and keeps a client per
account. Its tests are `src-tauri/src/plugins_trello_tests.rs` (the plugin started as the app starts it, against the fake
Trello of `src-tauri/src/integrations/fake.rs`) and the integration tests of the core (`integrations_tests.rs`), which go
through it.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/trello --first-party --escouade 2.2.0
```

The bench plays the handshake, the delays, the secret asked for, a crash and the shutdown, then one call of the
source's (`account/me`, `tickets/containers`…) when the manifest declares one.
