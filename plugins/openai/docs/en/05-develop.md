# Develop

For whoever wants to read, test or change this plugin, in Escouade’s repository.

## Its folder

`plugins/openai/`:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: `run.wasm`, the host `api.openai.com`, the activation `voice:openai`, `store.docs` |
| `Cargo.toml`, `Cargo.lock` | The `escouade-plugin-openai` crate, built as a WASM module (`cdylib`); its `Cargo.lock` pins the crates published on crates.io, those the store builds |
| `src/lib.rs` | One line: `escouade_voice::serve::run("openai", …)`. The whole service is in the `escouade-voice` crate |
| `README.fr.md`, `README.en.md` | Its page on the store, which the app’s Plugins view links to |
| `docs/fr/`, `docs/en/` | These pages |
| `CHANGELOG.md`, `LICENSE` | Its versions; GPL-3.0-or-later |

OpenAI’s code is in `crates/escouade-voice`: `online.rs` (dictation), `live.rs` (real time), `speech.rs` (voice), `catalog.rs` (models and voices), `serve.rs` (the `voice/*` calls), `wire.rs` (their shape). In the repository, `plugins/Cargo.toml` replaces them with those of `crates/` (`[patch.crates-io]`).

## Its tests

- `cargo test -p escouade-voice`, in `src-tauri/`: the service without a module, OpenAI included.
- `cargo test -p escouade-plugin-openai`, in `plugins/`.
- In `src-tauri/` (`cargo test`): the module against fake servers (`src/voice/*_tests.rs`), and `src/voice/parity_tests.rs`, which compares it with what the native plugin did (`tests/fixtures/plugins/parity/voice-openai.json`).
- End to end: `tests/e2e/voice.spec.ts`.

## Check it as the CI does

Run cargo **from `src-tauri/`**: its `rust-toolchain.toml` pins the Rust version. Build the app’s tool, then run the plugins’ check:

```sh
cd src-tauri && cargo build --bin escouade-plugin && cd ..
ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs --changed
```

(on Windows, `escouade-plugin.exe`). For this plugin, `ci.mjs` reads its LICENSE and CHANGELOG, compares the manifest’s version with the crate’s, runs `cargo test`, then `escouade-plugin validate`, `build` (the module, as the store builds it) and `test` (the conformance bench, in the app’s host), with `--first-party` for its reserved id.

## Run it alone

In `plugins/`, with the tool built above:

```sh
../src-tauri/target/debug/escouade-plugin validate openai --first-party --escouade 2.2.0
../src-tauri/target/debug/escouade-plugin test openai --first-party --escouade 2.2.0
```

`--escouade` says the Escouade version to judge it for: without it, the tool takes its own, and a tool older than 2.2.0 refuses the plugin. `test` builds the module and plays it against the bench’s fake Escouade. To try it in the app of a development build, `sh scripts/plugins-wasm.sh openai` writes its module to `src-tauri/target/plugins-wasm/openai/plugin.wasm`, which the app takes as long as its seed does not give it.

A version is published by Escouade’s chief: a `plugins/openai/vX.Y.Z` tag, then its submission to the store. Do not bump the version in a contribution.
