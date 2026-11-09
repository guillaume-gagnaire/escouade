# Developing isola

`dev.escouade.isola` is a plugin of Escouade’s own hand, in `plugins/isola/` of its repository. It is a WASM module (`run.wasm`) written in Rust on `escouade-plugin-kit`, compiled from this folder and run in Escouade’s host. Its version is its own.

It is the first **launch environment** (`contributes.launchEnvironments`).

## What its manifest declares

| Field | Value | What the app does with it |
| --- | --- | --- |
| `detect` | `.isola.toml` | the file that says a worktree is isola’s; read whole for the user before `up` |
| `program` | `isola` | the program, looked for on the PATH by the app |
| `up` | `isola up` | the command a test launch runs in a terminal (its input is not one: no password) |

The module answers `launch/ls`, `launch/down` and `launch/destroy`. The app lends it the worktree for the time of a call, under the name it gives in `agent`, with its branch (`branch`) and the language of its messages (`lang`): the module holds no path. It starts `isola` through the host’s `proc/spawn`, and reads its output through `proc/out` and `proc/exit`.

## Its folder

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | the manifest |
| `Cargo.toml` | its crate, on `escouade-plugin-kit` |
| `Cargo.lock` | the crates of crates.io the store compiles (`scripts/plugin-lock.sh`) |
| `src/lib.rs` | its code and its unit tests |
| `README.fr.md`, `README.en.md` | its page, on the store and in the app |
| `docs/fr/`, `docs/en/` | this documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | its versions, its license (GPL-3.0-or-later) |

The app’s side is `src-tauri/src/launchenv.rs` (and `src-tauri/src/isola.rs`, which finds the program). The launch tests (`src-tauri/src/core_tests.rs`, “isola”) and `src-tauri/src/plugins_isola_tests.rs` run the module in the app’s host against the test double `tests/fixtures/fake-isola.mjs`, which stands for the `isola` program.

## Testing it alone

From `plugins/`, against the fake Escouade of the plugins’ bench, as every plugin of the app:

```sh
cargo test -p escouade-plugin-isola
escouade-plugin test isola --first-party --escouade 2.2.0
```

The bench starts no `isola`: `launch/*` is called by the app, with a worktree it lends.

`node plugins/ci.mjs` (with `ESCOUADE_PLUGIN_BIN` naming the app’s `escouade-plugin`) does all of this for each plugin of the folder: its LICENSE and CHANGELOG, `cargo test`, `escouade-plugin validate` and the bench.

`--first-party`: its id `dev.escouade.*` is kept for Escouade’s own plugins. The store takes it from this folder of Escouade’s repository alone; its tags are `plugins/isola/v1.2.3`.
