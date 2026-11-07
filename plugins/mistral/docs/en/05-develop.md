# Develop

For whoever wants to read, test or change this plugin, in Escouade’s repository.

## Its folder

`plugins/mistral/`:

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | The manifest: `run.wasm`, the host `api.mistral.ai`, the activation `voice:mistral`, the `mistral` account and the `vibe` harness (its `acp` object), `store.docs` |
| `Cargo.toml`, `Cargo.lock` | The `escouade-plugin-mistral` crate, built as a WASM module (`cdylib`); its `Cargo.lock` pins the crates published on crates.io, those the store builds |
| `src/lib.rs` | The module: `escouade_voice::serve::run("mistral", …)`, which serves `voice/*`, and beside it the service of `escouade_acp::plugin` for the rest, on the harness its manifest declares (embedded by `include_str!`) |
| `README.fr.md`, `README.en.md` | Its page on the store, which the app’s Plugins view links to |
| `docs/fr/`, `docs/en/` | These pages |
| `CHANGELOG.md`, `LICENSE` | Its versions; GPL-3.0-or-later |

Mistral’s code is in `crates/escouade-voice`: `online.rs` (dictation), `live.rs` (real time), `speech.rs` (voice), `catalog.rs` (models and voices), `serve.rs` (the `voice/*` calls), `wire.rs` (their shape). Vibe is in `crates/escouade-acp` (`vibe.rs`, `agent.rs`, `plugin.rs`). In the repository, `plugins/Cargo.toml` replaces them with those of `crates/` (`[patch.crates-io]`).

## Its tests

- `cargo test -p escouade-voice`, in `src-tauri/`: the service without a module, Mistral included.
- `cargo test -p escouade-plugin-mistral`, in `plugins/`: its manifest (the harness, its `vibe` extension).
- In `src-tauri/` (`cargo test`): the module against fake servers (`src/voice/*_tests.rs`), and `src/voice/parity_tests.rs`, which compares it with what the native plugin did (`tests/fixtures/plugins/parity/voice-mistral.json`); the account (`src/plugins_accounts_tests.rs`), the harness bench on a fake Vibe (`src/engine/vibe/bench.rs`), the recorded streams of Mistral Vibe 3.0.3 (`src/plugins_acp_tests.rs`, `src/agent/vibe_tests.rs`), the core (`src/vibe_tests.rs`, `src/vibe_tui_tests.rs`).
- End to end: `tests/e2e/voice.spec.ts`, `tests/e2e/vibe.spec.ts`, and the real runs (`real_vibe`, `real_vibe_writes_a_text`, `tests/e2e/real-harnesses.spec.ts`), which need a real Vibe.

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
../src-tauri/target/debug/escouade-plugin validate mistral --first-party --escouade 2.2.0
../src-tauri/target/debug/escouade-plugin test mistral --first-party --escouade 2.2.0
```

`--escouade` says the Escouade version to judge it for: without it, the tool takes its own, and a tool older than 2.2.0 refuses the plugin. `test` builds the module and plays it against the bench’s fake Escouade. To try it in the app of a development build, `sh scripts/plugins-wasm.sh mistral` writes its module to `src-tauri/target/plugins-wasm/mistral/plugin.wasm`, which the app takes as long as its seed does not give it.

A version is published by Escouade’s chief: a `plugins/mistral/vX.Y.Z` tag, then its submission to the store. Do not bump the version in a contribution.
