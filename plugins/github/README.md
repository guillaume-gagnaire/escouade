# GitHub

`dev.escouade.github`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les issues dans le Kanban, les PR, la CI, les relectures et les déploiements de tes dépôts GitHub, sur un seul compte.

## What it does

It serves two points of Escouade on one account per instance (github.com, GHE.com, a GitHub Enterprise Server):

- **the code host `github`**: it opens a pull request (a draft, its reviewers, its labels, the issue it closes), follows
  those of an agent's branch (one GraphQL request for every pull request a project follows), merges them or turns
  auto-merge on, reads their checks and statuses, a job's log, runs the failed jobs again, reads and publishes reviews,
  their threads and answers, and the deployments of a commit. Its REST reads are conditional (a `304` costs nothing of
  the limit) and it slows down as the limit comes near; each answer tells Escouade what the account has left
  (`rateLimit`);
- **the Kanban's source `github`** (GitHub Issues, on github.com's account): the repositories you reach, open, closed
  or a label as states, the open issues listed or searched (pull requests aside), a state pushed back (closed, opened
  again, a label given) and the closing comment.

Its program is a module (`run.wasm`), which reaches github.com, GHE.com and the Enterprise Server the account names (`{account}`), and the storage of the Actions logs: GitHub keeps a job's log on Azure's blobs or
under `*.actions.githubusercontent.com`, where it goes without the token. An Enterprise Server that keeps its logs on another host
than itself is not reached, as nothing names that host: the log says so. The code is the crate of this folder (`src/pulls.rs`
for the code host, `src/issues.rs` for the issues), on the kit of the Rust plugins of Escouade
(`crates/escouade-plugin-kit`), which speaks the protocol, asks Escouade for each account's token and keeps one client
per account.

## Its account

One account per instance, added in Settings › Integrations › Code hosts: a token (classic, with `repo`; or
fine-grained: pull requests and contents in writing, checks, statuses, Actions and deployments in reading), or the one
of the GitHub CLI (`gh auth login`). github.com's account is GitHub Issues' too: one connection, both uses. The token
stays in the system's keychain; Escouade lends it to the plugin when it asks (`secrets/get` of `account:<host>`), and
the CLI's is asked by Escouade, which knows where `gh` is.

## What it may do

- reach `api.github.com`, `api.<name>.ghe.com` and the GitHub Enterprise Servers you add;
- read the tokens of its accounts (`account:*`), never another plugin's;
- nothing else: no command, no file, no fact of the board.

Switched off (Settings › Plugins, for the app or a project), its pull requests are no longer followed and the issues no
longer synced: an agent's card says so, the code hosts' view too, and nothing is lost.

## What is in this folder

| File                   | What it is                                                          |
| ---------------------- | ------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: its account, its source and its code host             |
| `README.md`            | This page                                                           |
| `Cargo.toml`, `src/`   | Its crate: `lib.rs` (its kinds), `pulls.rs`, `issues.rs`            |

Its tests: the crate's own (`cargo test -p escouade-plugin-github`), and those of the app that run its process against a
fake GitHub, unchanged since the code left the core: `src-tauri/src/forge/github_tests.rs` (the pull requests, the
checks, the reviews, the deployments, the limits; `real_github_reads` reads github.com with `gh`'s token, nothing
written) and `src-tauri/src/integrations/github_tests.rs` (GitHub Issues).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/github --first-party --escouade 2.2.0
```

The real github.com, read only, through the plugin as the app runs it:

```sh
cd src-tauri && ESCOUADE_TEST_REAL_GITHUB=1 cargo test --lib real_github_reads -- --ignored --nocapture
```
