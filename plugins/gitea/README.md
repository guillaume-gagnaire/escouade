# Gitea / Forgejo

`dev.escouade.gitea`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les PR, la CI (statuts et Actions), les relectures et le merge de tes dépôts Gitea, Forgejo et Codeberg, sur un seul
compte.

## What it does

It serves the code host `gitea`, one account per instance (Codeberg and gitea.com are known; any other instance is
added by its address and recognised by `/api/v1/version`):

- **pull requests**: opens one (a draft is a title that starts with `WIP:`, its reviewers, its labels, the issue it
  closes), follows those of an agent's branch, reads their state, reviews, threads and statuses;
- **merge** with each strategy, and **auto-merge** (`merge_when_checks_succeed`), which the instance only schedules on a
  **protected branch**: the plugin reads the branch first and refuses, saying why, when it is not one;
- **checks**: the combined statuses of the last commit; those Actions post are tied to their job, whose **log** can be
  read and whose failed jobs can be **run again**;
- **reviews and threads**: reads and publishes reviews (with their lines), reads the comments, answers and resolves a
  thread where the instance can.

## What it does not do, and where the version matters

It tells it, call by call, instead of failing or answering wrong:

| | Gitea | Forgejo |
| --- | --- | --- |
| Deployments | none (no API) | none (no API) |
| Actions: jobs and logs | 1.25 and later | 12 and later |
| Actions: run the failed jobs again | 1.26 and later | none |
| Answer in a thread, resolve it | 1.26 and later | none |

An instance older than that has its statuses (with their address) but no job, no log; the call that needs more answers
« no such method », and Escouade shows nothing for it. The version is asked of `/api/v1/version` once per account
(Forgejo's carries `+gitea-<version>`). **These minimal versions come from the sources, not from a trial**: see below.

## Its account

One account per instance, added in Settings › Integrations › Code hosts: an access token with the scopes
`write:repository`, `write:issue` and `read:user`, made at `/user/settings/applications` of the instance. The token
stays in the system's keychain; Escouade lends it to the plugin when it asks (`secrets/get` of `account:<host>`).

## What it may do

- reach `codeberg.org`, `gitea.com` and the instances you add;
- read the tokens of its accounts (`account:*`), never another plugin's;
- nothing else: no command, no file, no fact of the board.

## Not tried for real

Decided on 2026-10-16: no real trial for Gitea and Forgejo. The client is written from the swagger documents of Forgejo 16
and of Gitea's main branch, and tested against a fake server fed with answers written from them
(`tests/fixtures/forges/gitea/`, its README says how). What is left to try on a real Codeberg and a Gitea, the routes of
Actions above all, is listed in `docs/TODO-2.2.md` (« Hébergeurs de code »).

## What is in this folder

| File                   | What it is                                                  |
| ---------------------- | ----------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: its account and its code host                 |
| `README.md`            | This page                                                   |
| `Cargo.toml`, `src/`   | Its crate: `lib.rs` (its kind), `pulls.rs` (the client)     |

Its tests: the crate's own (`cargo test -p escouade-plugin-gitea`: versions, the address of a job), and those of the app
that run its module against a fake Gitea and a fake Forgejo: `src-tauri/src/forge/gitea_tests.rs`.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/gitea --first-party --escouade 2.2.0
```
