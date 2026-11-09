# Develop

This page is for whoever wants to read, change or test the plugin. Its code is under GPL-3.0-or-later, in Escouade's repository.

## Its folder

`plugins/linear/` of Escouade's repository:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: its id `dev.escouade.linear`, its version, its permissions, its account and its source |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-linear`, and the lock of the published crates its build uses |
| `src/lib.rs` | The Linear client (GraphQL API) and its Kanban source |
| `README.fr.md`, `README.en.md` | Its page on the store and in the app |
| `docs/fr/`, `docs/en/` | This documentation |
| `CHANGELOG.md`, `LICENSE` | Its versions, its licence |

Its program is a module (`run.wasm`), built by `escouade-plugin build` from this folder and its `Cargo.lock`. It stands on the kit of Escouade's Rust plugins (`crates/escouade-plugin-kit`), which answers the protocol, asks Escouade for the account's secret and keeps one client per account.

## Its tests

- Its crate's tests, in the workspace of `plugins/`: `cd plugins && cargo test -p escouade-plugin-linear`.
- The app's, which play its module in Escouade's host against a fake Linear (`src-tauri/src/integrations/fake.rs`): `src-tauri/src/plugins_linear_tests.rs`, and the core's integration tests (`src-tauri/src/integrations_tests.rs`), which go through it. They need the module built first: `sh scripts/plugins-wasm.sh linear`.
- All the first-party plugins, as CI does: `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (on Windows, `escouade-plugin.exe`). It checks its licence and changelog, that the manifest's version is the crate's, runs `cargo test`, `escouade-plugin validate` and the conformance bench.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/linear --first-party --escouade 2.2.0
```

The bench plays the handshake, the delays, the secret asked for, a crash and the shutdown, then one call of the source (`account/me`, `tickets/containers`…).

Its id `dev.escouade.linear` is reserved to the first party, and Escouade refuses another plugin with the same id. To make it yours, copy the folder, give it an id of your own (`io.github.<your-login>.linear`), then load it in the Plugins view › "In development" › "Load a folder…".
