# ClickUp

`dev.escouade.clickup`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les tâches d'une liste ClickUp dans le Kanban, ses statuts comme colonnes. Le dernier des plugins de première main qui
sont des sources du Kanban (INT-7.4), après Trello, Linear et Jira.

## What it does

It is a ticket source of the Kanban for ClickUp (API v2): a list of the account (chosen through the hierarchy
workspace, space, folder, list) is linked to a project of the Kanban, its open tasks are listed in the import
(filtered by « Mes tâches », by status, by label, by text, or by a task's id such as `#86abx3` or a custom id such as
`ABC-12`), and a task follows its ticket: it takes the status of the column the ticket comes into and is commented when
the project asks for it. The items of the checklist named after the criteria (else the description's list) give the
criteria of the ticket.

ClickUp has its own view of the states: each list has its own statuses, and the plugin gives the status each column
takes by default — by the names, else by the statuses' types (`custom`, `done`).

## What it may do

- **Network**: `api.clickup.com`, and nothing else.
- **Secrets**: the one of its account, `account:clickup` (the personal API token), which it asks of Escouade for each
  call (`secrets/get`); Escouade never puts it in a call.
- No command to run, no fact read, no file.

## Its account

One ClickUp account per Escouade: the personal API token, made on <https://app.clickup.com/settings/apps> and typed in
Réglages › Intégrations. It is kept in the system's keychain under `plugin:dev.escouade.clickup:account:clickup`; the
former name (`clickup`) is kept beside it and read first, so that a version from before reads the account as it is
(docs/PLUGINS.md §4.4). The text of the form (« Indique le jeton. ») is in its manifest
(`contributes.accounts[0].fields[].missing`).

It can be turned off for the app or for a project, never removed, and a plugin of the store with the same id is
refused: this one is the app's. Turned off, or stopped, the sync says so (« Le plugin dev.escouade.clickup ne répond
pas ») and tries again later; nothing is lost.

## What is in this folder

| File                   | What it is                                                        |
| ---------------------- | ----------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                      |
| `Cargo.toml`           | The crate `escouade-plugin-clickup`                               |
| `src/lib.rs`           | The client of ClickUp, and its `SourceKind` (the kit)             |
| `README.md`            | This page                                                         |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the kit
`escouade-plugin-kit`, which answers the protocol, asks Escouade for the account's secret and keeps a client per
account. Its tests are `src-tauri/src/plugins_clickup_tests.rs` (the plugin started as the app starts it, against the
fake ClickUp of `src-tauri/src/integrations/fake.rs`), the unit tests of its crate, and the integration tests of the core
(`integrations_tests.rs`), which go through it.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/clickup --first-party --escouade 2.2.0
```

The bench plays the handshake, the delays, the secret asked for, a crash and the shutdown, then one call of the
source's (`account/me`, `tickets/containers`…) when the manifest declares one.
