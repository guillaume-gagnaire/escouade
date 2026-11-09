# Develop

For whoever wants to read or change the plugin. It lives in `plugins/codex/` of Escouade’s repository, id `dev.escouade.codex`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone, on the crates `escouade-plugin-protocol` and `escouade-acp` (for the git rules). It serves the harness its manifest declares (`contributes.harnesses[0]`, id `codex`): `codex app-server`, driven over its own JSON-RPC protocol, one message a line. It is no ACP harness.

- The core finds `codex`, asks the plugin how to launch it (`engine/launch`: `app-server`, the plan tool, a private project’s permission profiles, Escouade’s server by `-c`), checks the description and starts the process, each account in its folder (`CODEX_HOME`).
- The plugin opens the conversation over the protocol (`opens`: new, resumed or copied), reads the account’s models (`model/list`, every page) and tells the core the agent is ready (`Item::Ready`).
- What Codex writes comes by `agent/frames` and goes back as items of the conversation: reasonings and texts live, calls under Claude Code’s names with their diffs, cards, questions, the Plan mode’s plan, the tasks, the subagents, the context gauge, the quota, each turn’s end.
- A message during a turn goes into it (`turn/steer`); the next turn starts with the model, effort and mode of then (`turn/start`).
- It estimates a turn’s cost from its tokens: `src/prices.rs` is OpenAI’s public grid.
- The “Codex view” is read here too (`view/translate`): the core hands it the lines of the session’s transcript.

What stays in the core: Codex’s place in the registry, the accounts and their sign-in, an account’s quota and models when no agent runs, the written texts (`codex exec`), the terminal view’s PTY and its hooks, Escouade’s MCP server.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, its folder variable |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-codex` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its `run` and the methods of the contract |
| `src/client.rs` | The client of its app-server, a state machine without a process |
| `src/translate.rs` | Its messages as items of the conversation, `git add` and `git commit` accepted |
| `src/words.rs` | Its modes, the fence, Escouade’s server among its options, its quota |
| `src/prices.rs` | The public prices of OpenAI’s models |
| `src/view.rs` | The transcript read for the “Codex view” |
| `src/tests.rs`, `src/rules_tests.rs` | The client on its own, and what its rules hold through the service (a restart, a commit that reads a file) |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- `src/tests.rs`, `src/rules_tests.rs` and `src/prices.rs`, with `cargo test` in `plugins/`;
- the bench of the harnesses on the fake Codex (`src-tauri/src/engine/codex/bench.rs`);
- the recorded streams of Codex 0.163.0 (`src-tauri/src/agent/codex_tests.rs`, `tests/fixtures/replays/codex/`) and the parity with the former native plugin, call by call (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/codex.jsonl`);
- the core’s tests that go through it (`src-tauri/src/codex_tests.rs`, `codex_tui_tests.rs`) and the e2e `tests/e2e/codex.spec.ts`;
- the real runs, started by hand: `real_codex_compact`, `real_codex_view`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build codex --first-party --out ../packs
escouade-plugin test codex --first-party --wasm ../packs/dev.escouade.codex-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh codex` compiles it against `crates/`, and `sh scripts/plugin-lock.sh codex` writes its `Cargo.lock` again after a change of the crates.
