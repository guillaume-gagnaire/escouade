# Bitbucket

`dev.escouade.bitbucket`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les PR, les statuts, les pipelines, leurs journaux, les commentaires et les déploiements de tes dépôts Bitbucket Cloud.

## What it does

It serves the code host `bitbucket` (Bitbucket Cloud, bitbucket.org; Data Center, whose API differs, waits for another
version) for one account:

- **pull requests**: opens one (a draft, its reviewers, the issue it closes), follows those of an agent's branch and
  the open ones of a repository, with their approvals, requests for changes, open threads and checks;
- **merge**: with the strategy asked (merge commit, squash, rebase, rebase then merge commit); a merge Bitbucket does
  in the background is waited for;
- **statuses and pipelines**: the commit statuses of a pull request's last commit are its checks; a Bitbucket
  Pipelines run has a log, read step after step;
- **comments and reviews**: comments on lines with their threads, answers, resolving a thread, publishing a review the
  user validated (its line comments, then approval or request for changes);
- **deployments**: those of a commit, by environment.

**There is no auto-merge**: Bitbucket's API merges at once (BCLOUD-22055 and BCLOUD-22062 ask for more). The capability is
declared absent (`Capabilities::auto_merge`), `forge/autoMerge` says « no such method », and Escouade waits for the
checks and merges itself, saying so. Labels (no such thing on Bitbucket pull requests) and re-running failed jobs
(no call known) are absent too.

**The hour's limit.** Bitbucket Cloud allows 1000 calls an hour (more for big workspaces) and does not tell how many
remain: the client counts its own calls over the hour, believes the `X-RateLimit-Limit` it is told, takes
`X-RateLimit-NearLimit` for « a fifth left », and slows down as the budget runs out. A refusal (429) is waited out, at
most 30 seconds, then asked once more. Each answer tells Escouade what the account has left (`rateLimit`).

Its program is a module (`run.wasm`), which reaches api.bitbucket.org. The code is the crate of this folder (`src/pulls.rs`),
on the kit of the Rust plugins of Escouade (`crates/escouade-plugin-kit`).

## Its account

One account, added in Settings › Integrations › Code hosts: **an Atlassian API token with scopes, and the e-mail of
the Atlassian account** it belongs to (app passwords are retired). Scopes: `read:user:bitbucket`,
`read:repository:bitbucket`, `read:pullrequest:bitbucket`, `write:pullrequest:bitbucket`, `read:pipeline:bitbucket`.
The token stays in the system's keychain; Escouade lends it to the plugin when it asks (`secrets/get` of
`account:bitbucket.org`).

## What it may do

- reach `api.bitbucket.org`;
- read the tokens of its accounts (`account:*`), never another plugin's;
- nothing else: no command, no file, no fact of the board.

## Written from the documentation

Nobody tried it on bitbucket.org yet (decided on 2026-10-16: real trials on GitHub only). It was written from Bitbucket
Cloud's API 2.0 documentation and tested against a fake server fed with answers in the shape the documentation gives
(`tests/fixtures/forges/bitbucket/`). What remains to try for real is in `docs/TODO-2.2.md`.

## What is in this folder

| File                   | What it is                                                |
| ---------------------- | --------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: its account and its code host               |
| `README.md`            | This page                                                 |
| `Cargo.toml`, `src/`   | Its crate: `lib.rs` (its kind), `pulls.rs` (its client)   |

Its tests: the crate's own (`cargo test -p escouade-plugin-bitbucket`), and those of the app that run its process
against a fake Bitbucket: `src-tauri/src/forge/bitbucket_tests.rs`.
