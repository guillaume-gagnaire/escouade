# Develop

This page is for whoever wants to read, change or test the plugin. Its code is under GPL-3.0-or-later, in Escouade's repository.

## Its folder

`plugins/bitbucket/` of Escouade's repository:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: its id `dev.escouade.bitbucket`, its version, its permissions, its account and its code host |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-bitbucket`, and the lock of the published crates its build uses |
| `src/lib.rs` | The code host and the API's address |
| `src/pulls.rs`, `src/pulls_tests.rs` | The Bitbucket Cloud client (API 2.0): pull requests, merge, statuses, pipelines and logs, comments and reviews, deployments, the hourly limit; and its tests |
| `README.fr.md`, `README.en.md` | Its page on the store and in the app |
| `docs/fr/`, `docs/en/` | This documentation |
| `CHANGELOG.md`, `LICENSE` | Its versions, its licence |

Its program is a module (`run.wasm`), built by `escouade-plugin build` from this folder and its `Cargo.lock`. It stands on the kit of Escouade's Rust plugins (`crates/escouade-plugin-kit`), which answers the protocol, asks Escouade for each account's token and keeps one client per account.

## Its tests

- Its crate's tests, in the workspace of `plugins/`: `cd plugins && cargo test -p escouade-plugin-bitbucket`.
- The app's, which play its module in Escouade's host against a fake server fed with answers in the shape the documentation gives (`tests/fixtures/forges/bitbucket/`): `src-tauri/src/forge/bitbucket_tests.rs`. They need the module built first: `sh scripts/plugins-wasm.sh bitbucket`.
- All the first-party plugins, as CI does: `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (on Windows, `escouade-plugin.exe`). It checks its licence and changelog, that the manifest's version is the crate's, runs `cargo test`, `escouade-plugin validate` and the conformance bench.

Nobody has tried it on bitbucket.org yet: what is left to try is in `docs/TODO-2.2.md`.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/bitbucket --first-party --escouade 2.2.0
```

Its id `dev.escouade.bitbucket` is reserved to the first party, and Escouade refuses another plugin with the same id. To make it yours, copy the folder, give it an id of your own (`io.github.<your-login>.bitbucket`), then load it in the Plugins view › "In development" › "Load a folder…".
