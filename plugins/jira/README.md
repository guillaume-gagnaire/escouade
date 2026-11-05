# Jira

`dev.escouade.jira`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les tickets d'un projet Jira Cloud dans le Kanban, ses statuts comme colonnes. Le troisième des plugins de première
main qui sont des sources du Kanban (INT-7.3) : ClickUp suit le même modèle.

## What it does

It is a ticket source of the Kanban for Jira Cloud (REST API v3): a project of the account is linked to a project of
the Kanban, its issues not done are listed in the import (filtered by « Assignés à moi », active sprint, to do, by
label, by text or by key such as `ATL-12`), and an issue follows its ticket: it moves along the workflow's transition
that leads to the status of the column the ticket comes into, and is commented (as an Atlassian document) when the
project asks for it. The description's acceptance criteria give the criteria of the ticket.

## What it may do

- **Network**: `*.atlassian.net`, and nothing else.
- **Secrets**: the one of its account, `account:jira` (the API token), which it asks of Escouade for each call
  (`secrets/get`); Escouade never puts it in a call. The site and the e-mail are not secret: Escouade tells them.
- No command to run, no fact read, no file.

## Its account

One Jira account per Escouade: the site (`atlas.atlassian.net`), the e-mail and the API token, made on
<https://id.atlassian.com/manage-profile/security/api-tokens> and typed in Réglages › Intégrations. The token is kept
in the system's keychain under `plugin:dev.escouade.jira:account:jira`; the former name (`jira`) is kept beside it and
read first, so that a version from before reads the account as it is (docs/PLUGINS.md §4.4). The site must be in https
(http only on this machine, for a fake one). The texts of the form (« Indique l'adresse du site. », « Indique l'e-mail. »,
« Indique le jeton. ») are in its manifest (`contributes.accounts[0].fields[].missing`).

It can be turned off for the app or for a project, never removed, and a plugin of the store with the same id is
refused: this one is the app's. Turned off, or stopped, the sync says so (« Le plugin dev.escouade.jira ne répond pas »)
and tries again later; nothing is lost.

## What is in this folder

| File                   | What it is                                                          |
| ---------------------- | ------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                        |
| `Cargo.toml`           | The crate `escouade-plugin-jira`                                    |
| `src/lib.rs`           | The client of Jira, and its `SourceKind` (the kit)                  |
| `src/adf.rs`           | Atlassian documents read as markdown, and a comment written as one |
| `README.md`            | This page                                                           |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the kit
`escouade-plugin-kit`, which answers the protocol, asks Escouade for the account's secret and keeps a client per
account. Its tests are `src-tauri/src/plugins_jira_tests.rs` (the plugin started as the app starts it, against the fake
Jira of `src-tauri/src/integrations/fake.rs`) and the integration tests of the core (`integrations_tests.rs`), which go
through it.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/jira --first-party --escouade 2.2.0
```

The bench plays the handshake, the delays, the secret asked for, a crash and the shutdown, then one call of the
source's (`account/me`, `tickets/containers`…) when the manifest declares one.
