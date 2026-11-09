# Develop

This page is for whoever wants to read, change or test the plugin. Its code is under GPL-3.0-or-later, in Escouade's repository.

## Its folder

`plugins/github/` of Escouade's repository:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: its id `dev.escouade.github`, its version, its permissions, its account, its source and its code host |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-github`, and the lock of the published crates its build uses |
| `src/lib.rs` | Its two points: the code host and the source, and the API address of an instance |
| `src/pulls.rs`, `src/pulls_tests.rs` | The code host: pull requests, merge and auto-merge, checks, Actions logs and reruns, reviews, deployments, limits; and its tests |
| `src/issues.rs` | The GitHub Issues source |
| `README.fr.md`, `README.en.md` | Its page on the store and in the app |
| `docs/fr/`, `docs/en/` | This documentation |
| `CHANGELOG.md`, `LICENSE` | Its versions, its licence |

Its program is a module (`run.wasm`), built by `escouade-plugin build` from this folder and its `Cargo.lock`. It stands on the kit of Escouade's Rust plugins (`crates/escouade-plugin-kit`), which answers the protocol, asks Escouade for each account's token and keeps one client per account.

## Its tests

- Its crate's tests, in the workspace of `plugins/`: `cd plugins && cargo test -p escouade-plugin-github`.
- The app's, which play its module in Escouade's host against a fake GitHub: `src-tauri/src/forge/github_tests.rs` (pull requests, checks, reviews, deployments, limits) and `src-tauri/src/integrations/github_tests.rs` (GitHub Issues). They need the module built first: `sh scripts/plugins-wasm.sh github`.
- All the first-party plugins, as CI does: `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (on Windows, `escouade-plugin.exe`). It checks its licence and changelog, that the manifest's version is the crate's, runs `cargo test`, `escouade-plugin validate` and the conformance bench.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/github --first-party --escouade 2.2.0
```

Against the real github.com, read only, through the plugin as the app runs it (it takes `gh`'s token and writes nothing):

```sh
cd src-tauri && ESCOUADE_TEST_REAL_GITHUB=1 cargo test --lib real_github_reads -- --ignored --nocapture
```

Its id `dev.escouade.github` is reserved to the first party, and Escouade refuses another plugin with the same id. To make it yours, copy the folder, give it an id of your own (`io.github.<your-login>.github`), then load it in the Plugins view › "In development" › "Load a folder…".
