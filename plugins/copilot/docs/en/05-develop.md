# Develop

For whoever wants to read or change the plugin. It lives in `plugins/copilot/` of Escouade’s repository, id `dev.escouade.copilot-cli`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone, on the crates `escouade-plugin-protocol` and `escouade-acp` (for the git rules). It serves the harness its manifest declares (`contributes.harnesses[0]`, id `copilot`): `copilot --server --stdio`, driven over its SDK’s protocol (JSON-RPC 2.0, protocol 3) by a client of Escouade’s. It is no ACP harness.

- The core finds `copilot`, asks the plugin how to launch it (`engine/launch`), checks the description and starts it. Its messages go with `Content-Length` headers (`framing`), and `runtime.shutdown` is written before its input is closed (`closeLine`), so that it saves its session.
- The plugin opens the session itself, step by step (`opens`): the protocol checked, the sign-in, the session new, resumed or copied, its modes, Escouade’s server, its model, the account’s models. A refusal stops the agent (`Item::Stop`) and says why.
- What Copilot writes comes by `agent/frames` and goes back as items of the conversation: texts, reasonings, calls under Claude Code’s names with their diffs, permissions, questions, forms, the end of a plan, the ceiling of credits, tasks, subagents, the month’s credits, the context, each turn’s tokens and cost.
- A message’s images and files go as the core made them; `/compact` by its request (`agent/compact`), a title by `session.name.set` (`agent/rename`).
- The Copilot CLI view’s transcript is read by the same translation (`view/translate`, `src/view.rs`).

What stays in the core: the terminal view’s PTY and its hooks, the accounts (`copilot login` in each one’s folder), an account’s quota and models read by a server of their own, the written texts (`copilot -p`), Escouade’s MCP server.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, framing and close |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-copilot` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its `run`: the methods of the contract, one client for each agent |
| `src/client.rs` | The client of Copilot’s server, a state machine without a process |
| `src/translate.rs` | Its events as items of the conversation, the safe requests; the view’s too |
| `src/view.rs` | The transcript read for the Copilot CLI view |
| `src/words.rs` | Its arguments, modes, models, credits, the fence, what it says |
| `src/tests.rs`, `src/rules_tests.rs` | The client on its own, and what its rules hold through the service |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- `src/tests.rs` and `src/rules_tests.rs`, with `cargo test` in `plugins/`;
- the parity with the former native plugin (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/copilot.jsonl`);
- the bench of the harnesses on the fake Copilot (`src-tauri/src/engine/copilot/bench.rs`) and the recorded streams of Copilot CLI 1.0.98 (`src-tauri/src/agent/copilot_tests.rs`, `tests/fixtures/replays/copilot/`);
- the core’s tests that go through it (`src-tauri/src/copilot_tests.rs`, `copilot_tui_tests.rs`) and the e2e `tests/e2e/copilot.spec.ts`;
- the real runs, started by hand: `real_copilot`, `real_copilot_view`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build copilot --first-party --out ../packs
escouade-plugin test copilot --first-party --wasm ../packs/dev.escouade.copilot-cli-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh copilot` compiles it against `crates/`, and `sh scripts/plugin-lock.sh copilot` writes its `Cargo.lock` again after a change of the crates.
