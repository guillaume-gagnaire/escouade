# Develop

This page is for whoever wants to read, change or test the plugin. Its code is under GPL-3.0-or-later, in Escouade's repository.

## Its folder

`plugins/azure-devops/` of Escouade's repository:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: its id `dev.escouade.azure-devops`, its version, its permissions, its account, its source and its code host |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-azure-devops`, and the lock of the published crates its build uses |
| `src/lib.rs` | Its two points: the code host and the source, and what an organization's address means |
| `src/pulls.rs`, `src/pulls_tests.rs` | The code host (REST API 7.1 of Azure Repos and Azure Pipelines): pull requests, merge and auto-complete, checks, logs and retries, threads and reviews; and its tests |
| `src/boards.rs`, `src/boards_tests.rs` | The Azure Boards source: types and states, WIQL query, criteria, state and comments; and its tests |
| `src/html.rs` | Azure Boards' HTML fields read as Markdown |
| `src/joined.rs` | Several calls at once |
| `README.fr.md`, `README.en.md` | Its page on the store and in the app |
| `docs/fr/`, `docs/en/` | This documentation |
| `CHANGELOG.md`, `LICENSE` | Its versions, its licence |

Its program is a module (`run.wasm`), built by `escouade-plugin build` from this folder and its `Cargo.lock`. It stands on the kit of Escouade's Rust plugins (`crates/escouade-plugin-kit`), which answers the protocol, asks Escouade for each account's token and keeps one client per account.

## Its tests

- Its crate's tests, in the workspace of `plugins/`: `cd plugins && cargo test -p escouade-plugin-azure-devops`.
- The app's, which play its module in Escouade's host against a fake server (`tests/fixtures/forges/azure/`): `src-tauri/src/forge/azure_tests.rs` (pull requests) and `src-tauri/src/integrations/azure_tests.rs` (Azure Boards). They need the module built first: `sh scripts/plugins-wasm.sh azure-devops`.
- All the first-party plugins, as CI does: `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (on Windows, `escouade-plugin.exe`). It checks its licence and changelog, that the manifest's version is the crate's, runs `cargo test`, `escouade-plugin validate` and the conformance bench.

Two real trials are written in the crate's tests, and ignored: `real_azure_devops` (`src/pulls_tests.rs`: opens a draft pull request, reads it back, comments, replies, resolves, publishes a review without a vote) and `real_azure_boards` (`src/boards_tests.rs`: makes a Task, finds it, moves it, comments on it, puts it in the recycle bin). They need an organization and a token, with the variables their comments name (`ESCOUADE_TEST_AZURE_URL`, `ESCOUADE_TEST_AZURE_PAT`, and `ESCOUADE_TEST_AZURE_REPO` and `ESCOUADE_TEST_AZURE_BRANCH`, or `ESCOUADE_TEST_AZURE_PROJECT`):

```sh
cd plugins
cargo test -p escouade-plugin-azure-devops real_azure_devops -- --ignored --nocapture
cargo test -p escouade-plugin-azure-devops real_azure_boards -- --ignored --nocapture
```

What is left to try for real is in `docs/TODO-2.2.md` ("Hébergeurs de code").

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/azure-devops --first-party --escouade 2.2.0
```

Its id `dev.escouade.azure-devops` is reserved to the first party, and Escouade refuses another plugin with the same id. To make it yours, copy the folder, give it an id of your own (`io.github.<your-login>.azure-devops`), then load it in the Plugins view › "In development" › "Load a folder…".
