# Gitea and Forgejo: the recorded answers

The answers the fake server gives the client of `plugins/gitea`
(`src-tauri/src/forge/gitea_tests.rs`). `node make.mjs` writes them.

**They are not recorded from a live instance** (decided on 2026-10-16: no real trial for Gitea and Forgejo). They are
written from the swagger documents of Forgejo 16 (`codeberg.org/swagger.v1.json`) and of Gitea's main branch
(`gitea.com/swagger.v1.json`), field for field, and they show the differences between the two APIs the client handles:

| File                                         | What it is                                                                                |
| -------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `pull.json`, `forgejo-pull.json`             | a pull request (`content_version` is Gitea's, `flow` Forgejo's)                           |
| `gitea-runs.json`, `gitea-run-jobs.json`     | Gitea's Actions: `run_number`, `conclusion`, the jobs wrapped in `{ jobs }`               |
| `forgejo-runs.json`, `forgejo-run-jobs.json` | Forgejo's Actions: `index_in_repo`, a status only, the jobs as a list                     |
| `statuses.json`, `combined-status.json`      | what Actions post (`target_url` = `.../actions/runs/<number>/jobs/<index>`) and a third-party CI |
| the others                                   | reviews, a review's comments, the conversation, branches, labels, repositories            |

To replace them with real recordings once an instance is tried (docs/TODO-2.2.md), keep the names and the fields the
tests read.
