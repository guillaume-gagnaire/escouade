# Develop

This page is for whoever wants to read, change or test the plugin. Its code is under GPL-3.0-or-later, in Escouade's repository.

## Its folder

`plugins/gitea/` of Escouade's repository:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: its id `dev.escouade.gitea`, its version, its permissions, its account and its code host |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-gitea`, and the lock of the published crates its build uses |
| `src/lib.rs` | The code host, and the API address of an instance (`<instance>/api/v1`) |
| `src/pulls.rs` | The Gitea and Forgejo client: pull requests, merge and auto-merge, statuses, Actions, reviews and threads, and what depends on the version; with its tests |
| `README.fr.md`, `README.en.md` | Its page on the store and in the app |
| `docs/fr/`, `docs/en/` | This documentation |
| `CHANGELOG.md`, `LICENSE` | Its versions, its licence |

Its program is a module (`run.wasm`), built by `escouade-plugin build` from this folder and its `Cargo.lock`. It stands on the kit of Escouade's Rust plugins (`crates/escouade-plugin-kit`), which answers the protocol, asks Escouade for each account's token and keeps one client per account.

## Its tests

- Its crate's tests, in the workspace of `plugins/`: `cd plugins && cargo test -p escouade-plugin-gitea`.
- The app's, which play its module in Escouade's host against a fake server fed with answers written from the API documents (`tests/fixtures/forges/gitea/`, whose README says how), once as a Gitea, once as a Forgejo: `src-tauri/src/forge/gitea_tests.rs`. They need the module built first: `sh scripts/plugins-wasm.sh gitea`.
- All the first-party plugins, as CI does: `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (on Windows, `escouade-plugin.exe`). It checks its licence and changelog, that the manifest's version is the crate's, runs `cargo test`, `escouade-plugin validate` and the conformance bench.

What is left to try on a real Codeberg and a real Gitea, the Actions routes above all, is in `docs/TODO-2.2.md` ("Hébergeurs de code").

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/gitea --first-party --escouade 2.2.0
```

Its id `dev.escouade.gitea` is reserved to the first party, and Escouade refuses another plugin with the same id. To make it yours, copy the folder, give it an id of your own (`io.github.<your-login>.gitea`), then load it in the Plugins view › "In development" › "Load a folder…".
