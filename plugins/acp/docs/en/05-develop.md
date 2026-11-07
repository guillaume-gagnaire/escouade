# Develop

For whoever wants to read or change the plugin. It lives in `plugins/acp/` of Escouade’s repository, id `dev.escouade.acp`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone. Its code is three lines over `escouade_acp::plugin::serve_as("acp", None)`: it has no harness of its own. The harness’s description comes with each agent (`described` of `engine/launch` and `agent/attach`), on the contract of the harnesses in process.

The client is the crate `crates/escouade-acp`, the same Cursor CLI, Gemini CLI and Mistral Vibe are served by:

- **the session**: the launch’s first line is `initialize`; its answer opens the session (`session/load` of the one to resume when the agent can, else `session/new`, Escouade’s MCP server given with its token in a header), then the mode and the model are set. A message sent before the session is open, or during a turn, waits for it. The instructions go before the first message;
- **the turn**: texts and reasonings gathered, calls under Claude Code’s names, their input as it comes, their end with their result (a write’s diff); the end of the prompt ends the turn;
- **the permissions**: to the user, but those Escouade’s rules answer (see the Permissions tab);
- **the files** the agent reads and writes through the client (`fs/*`), through the host; what it does by itself during a fence is said once.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: its file permissions and its limits; no harness |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-acp` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its `run` |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

In `crates/escouade-acp/src/`: `agent.rs` (an agent, its permissions and its files), `plugin.rs` (the service), `files.rs` (files through the host), `tools.rs`, `questions.rs`, `texts.rs` (what it says), and the extensions `cursor.rs`, `gemini.rs`, `vibe.rs`.

## Its tests

- those of `crates/escouade-acp`, with `cargo test`;
- `src-tauri/src/plugins_acp_tests.rs`: the bench of the harnesses on a fake ACP agent described by `tests/fixtures/plugins/fake-acp/escouade-plugin.json`, played on the module, and the real run `real_goose`, started by hand;
- the parity with the former native plugin, line by line on the recorded streams (`src-tauri/src/plugins_claude_acp_parity_tests.rs`, `tests/fixtures/plugins/parity/acp.json`).

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build acp --first-party --out ../packs
escouade-plugin test acp --first-party --wasm ../packs/dev.escouade.acp-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh acp` compiles it against `crates/`, and `sh scripts/plugin-lock.sh acp` writes its `Cargo.lock` again after a change of the crates.
