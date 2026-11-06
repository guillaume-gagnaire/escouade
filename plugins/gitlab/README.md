# GitLab

`dev.escouade.gitlab`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Les issues dans le Kanban, les MR, les pipelines, les discussions et les déploiements de tes projets GitLab, sur gitlab.com et tes instances.

## What it does

It serves the code host `gitlab` on one account per instance (gitlab.com, a self-managed GitLab, under a path of its
own too: its API is `<instance>/api/v4`). Its word is « MR ». It:

- opens a merge request: a draft by the `Draft:` its title starts with, its reviewers (their logins made ids), its
  labels, « Closes #n » at the end of its description;
- reads one, those of a branch, the open ones of a project: its state, draft, conflict, approvals, open threads,
  auto-merge, its head pipeline's state, the fork it comes from;
- merges it (a merge commit or squashed; the project's setting says whether GitLab rebases first) or sets it to merge
  once its pipeline passes (`auto_merge`, `merge_when_pipeline_succeeds` before GitLab 17.11: both are sent);
- reads the jobs of its head pipeline (a job allowed to fail does not fail it), a job's log (`/jobs/:id/trace`), and
  runs the pipeline's failed jobs again (one retry);
- reads the approvals and the reviewers' say, the discussions (on a line of the diff or not; system notes aside),
  answers in a discussion, resolves it, comments, and publishes a review the user validated (its lines as
  discussions on the diff, its text, its approval);
- reads the deployments of a branch: the last of each environment.

Projects are named by their path, groups and subgroups included (`gitlab-org/api/client-go`). It spares the limits:
the open threads are read only when GitLab says some are, a fork's path once; it slows down as the limit comes near
(`RateLimit-*`) and waits what a 429 asks (`Retry-After`) before trying once more; each answer tells Escouade what the
account has left (`rateLimit`).

It serves the Kanban's source `gitlab` too, GitLab Issues, on the same accounts (`src/issues.rs`): the opened issues of a
project (by number, searched, assigned to you, by label, a page at a time), its states (opened, closed, the labels of
its boards' lists, its other labels), an issue closed, opened again or given a label in one call, and a note. A
container is `<host>/<groups>/<project>`, so that the instance goes with it: Escouade asks the account whose host the
container names.

Its program is a module (`run.wasm`), which reaches gitlab.com and the instance the account names (`{account}`). The code is the crate of this folder (`src/merges.rs`
for the code host), on the kit of the Rust plugins of Escouade (`crates/escouade-plugin-kit`), which speaks the
protocol, asks Escouade for each account's token and keeps one client per account.

Written from GitLab's documentation and sources, and tried against answers recorded from gitlab.com: not yet against a
real instance with a token (what remains to try is in `docs/TODO-2.2.md`).

## Its account

One account per instance, added in Settings › Integrations › Code hosts: a personal token with `api`, or the one of the
GitLab CLI (`glab auth login`). An instance's account is GitLab Issues' too: one connection, both uses. The token stays
in the system's keychain; Escouade lends it to the plugin when it asks (`secrets/get` of `account:<host>`), and the
CLI's is asked by Escouade, which knows where `glab` is.

## What it may do

- reach `gitlab.com` and the GitLab instances you add;
- read the tokens of its accounts (`account:*`), never another plugin's;
- nothing else: no command, no file, no fact of the board.

Switched off (Settings › Plugins, for the app or a project), its merge requests and its issues are no longer followed: an agent's card
says so, the code hosts' view too, and nothing is lost.

## What is in this folder

| File                   | What it is                                   |
| ---------------------- | -------------------------------------------- |
| `escouade-plugin.json` | The manifest: its account, code host and source |
| `README.md`            | This page                                    |
| `Cargo.toml`, `src/`   | Its crate: `lib.rs` (its kinds), `merges.rs`, `issues.rs` |

Its tests: the crate's own (`cargo test -p escouade-plugin-gitlab`), and those of the app that run its process against
a fake GitLab fed with answers recorded from gitlab.com (`tests/fixtures/forges/gitlab/`, `record.mjs`):
`src-tauri/src/forge/gitlab_tests.rs` for the merge requests, `src-tauri/src/integrations/gitlab_tests.rs` for the
issues (gitlab.com and an instance side by side; `real_gitlab_issues`, ignored, is the real trial by hand with
`ESCOUADE_TEST_GITLAB_TOKEN` and `ESCOUADE_TEST_GITLAB_PROJECT`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/gitlab --first-party --escouade 2.2.0
```
