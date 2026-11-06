# Azure DevOps

`dev.escouade.azure-devops`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with
it, and updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is
read).

Azure Boards dans le Kanban, et les PR d'Azure Repos, leurs stratégies, leurs pipelines, leurs relectures et leur merge,
sur le compte de ton organisation.

## What it does

One account per organization (dev.azure.com, its former `<org>.visualstudio.com` address, or an Azure DevOps Server's
collection) serves both uses.

**The code host `azure`** (Azure Repos and Azure Pipelines):

- **pull requests**: opens one (draft, reviewers, labels, the work item it closes), follows those of an agent's branch,
  reads their state, conflicts, votes and open threads;
- **merge** by the strategies the branch's policy allows (merge, squash, rebase, semi-linear), and **auto-complete**;
- **checks**: the evaluations of the build and status policies, the statuses, and the builds of the merge commit no
  policy holds; the **log** of a build, and its **retry**;
- **reviews and threads**: reads and publishes a review (its comments, then a vote), answers and resolves a thread.

**The Kanban's source `azure`** (Azure Boards): a project is a container (`<org>/<project>`); the states are those of
every work item type of its process (Basic, Agile, Scrum, CMMI or one inherited from them), each with its category, and
the columns' defaults come from the categories; the import is a WIQL query (the backlog's types, not closed, assigned to
me, a tag, a text or a number), the acceptance criteria come from the field of that name or the description, a state is
pushed in JSON Patch and a comment in markdown.

It has no deployments: Azure Pipelines' environments say nothing simple of a branch (the code host's `Capabilities` say
so).

## Its account

One account per organization, added in Settings › Integrations › Code hosts, with a personal access token (the scopes
`vso.code_write`, `vso.build_execute` and `vso.work_write`, made at `/_usersSettings/tokens`), or with the Entra ID token
of the **`az` CLI** (`az login`). Escouade asks `az` itself and lends the token it gets: the plugin runs no program. A
token is sent in Basic (`:token`) when personal, in Bearer when it is a JWT. The token stays in the system's keychain;
Escouade lends it to the plugin when it asks (`secrets/get` of `account:<host>`).

Azure does not tell a token's rights: none is checked, and a token without the Work Items right is told where to make
another one when Azure Boards is refused.

## What it may do

- reach `dev.azure.com`, `*.dev.azure.com` (the profile and identities hosts) and `*.visualstudio.com`, and the Server
  you add;
- read the tokens of its accounts (`account:*`), never another plugin's;
- nothing else: no command, no file, no fact of the board.

## Not tried for real

Both real trials are written and compiled, and ignored, as they were in the core: `real_azure_devops`
(`src-tauri/src/forge/azure_tests.rs`: opens a pull request, reads everything about it, abandons it) and
`real_azure_boards` (`src-tauri/src/integrations/azure_tests.rs`: makes a Task, finds it, moves it, comments on it, puts
it in the recycle bin). They need an organization and a token we do not have: they are still to do, see
`docs/TODO-2.2.md` (« Hébergeurs de code »). Run them with the variables their comments name
(`ESCOUADE_TEST_AZURE_URL`, `ESCOUADE_TEST_AZURE_PAT`, `ESCOUADE_TEST_AZURE_REPO` or `ESCOUADE_TEST_AZURE_PROJECT`):

```sh
cargo test --lib real_azure_devops -- --ignored --nocapture
cargo test --lib real_azure_boards -- --ignored --nocapture
```

## What is in this folder

| File                   | What it is                                                                               |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: its account, its code host and its source                                   |
| `README.md`            | This page                                                                                |
| `Cargo.toml`, `src/`   | Its crate: `lib.rs` (its kinds), `pulls.rs` (the code host's client), `boards.rs` (Azure Boards), `html.rs` (the HTML of the work items' fields as markdown) |

Its tests: the crate's own (`cargo test -p escouade-plugin-azure-devops`: the HTML of a field), and those of the app that
run its module against a fake Azure DevOps: `src-tauri/src/forge/azure_tests.rs` (the pull requests, from the fixtures
of `tests/fixtures/forges/azure/`) and `src-tauri/src/integrations/azure_tests.rs` (Azure Boards, two organizations),
and `tests/e2e/azure-devops.spec.ts` (the window).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/azure-devops --first-party --escouade 2.2.0
```
