# Linear

`dev.escouade.linear`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les issues d'une équipe Linear dans le Kanban, ses statuts comme colonnes. Le deuxième des plugins de première main qui
sont des sources du Kanban (INT-7.2) : Jira et ClickUp suivent le même modèle.

## What it does

It is a ticket source of the Kanban for Linear (GraphQL API): a team of the account is linked to a project, its issues
not done nor canceled are listed in the import (filtered by « Mes issues », by workflow state, by label, by text or by
number such as `ENG-42`), and an issue follows its ticket: it is moved to the workflow state of the column the ticket
comes into, and commented when the project asks for it. The acceptance criteria of the description give the criteria of
the ticket.

## What it may do

- **Network**: `api.linear.app`, and nothing else.
- **Secrets**: the one of its account, `account:linear` (the personal API key), which it asks of Escouade for each call
  (`secrets/get`); Escouade never puts it in a call.
- No command to run, no fact read, no file.

## Its account

One Linear account per Escouade. The personal API key is made on <https://linear.app/settings/account/security>, typed
in Réglages › Intégrations. It is kept in the system's keychain under `plugin:dev.escouade.linear:account:linear`; the
former name (`linear`) is kept beside it and read first, so that a version from before reads the account as it is
(docs/PLUGINS.md §4.4). Its text (« Indique le jeton. ») is in its manifest (`contributes.accounts[0].fields[].missing`).

It can be turned off for the app or for a project, never removed, and a plugin of the store with the same id is
refused: this one is the app's. Turned off, or stopped, the sync says so (« Le plugin dev.escouade.linear ne répond pas »)
and tries again later; nothing is lost.

## What is in this folder

| File                   | What it is                                            |
| ---------------------- | ----------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                          |
| `Cargo.toml`           | The crate `escouade-plugin-linear`                    |
| `src/lib.rs`           | The client of Linear, and its `SourceKind` (the kit)  |
| `README.md`            | This page                                             |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the kit
`escouade-plugin-kit`, which answers the protocol, asks Escouade for the account's secret and keeps a client per
account. Its tests are `src-tauri/src/plugins_linear_tests.rs` (the plugin started as the app starts it, against the fake
Linear of `src-tauri/src/integrations/fake.rs`) and the integration tests of the core (`integrations_tests.rs`), which go
through it.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/linear --first-party --escouade 2.2.0
```

The bench plays the handshake, the delays, the secret asked for, a crash and the shutdown, then one call of the
source's (`account/me`, `tickets/containers`…) when the manifest declares one.
