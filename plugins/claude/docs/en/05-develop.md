# Develop

For whoever wants to read or change the plugin. It lives in `plugins/claude/` of Escouade’s repository, id `dev.escouade.claude-code`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone. It is the example of a harness written in Rust: a state machine that turns lines into lines. What it doesn’t do stays in the core.

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `claude`): `claude` driven over stream-json and its control protocol, on the contract of the harnesses in process.

- The core finds `claude`, asks the plugin how to launch it (`engine/launch`: the stream’s arguments, the model, the effort, the mode, the added instructions, the session to resume or copy, the folders it reads, Escouade’s server by a file of the agent’s), checks the description and starts the process, each account in its folder (`CLAUDE_CONFIG_DIR`).
- The plugin writes `initialize` as the first line (`opens`), tells the core the account’s commands and models (`Item::Told`), then that the agent is ready (`Item::Ready`).
- What `claude` writes comes by `agent/frames` and goes back as items of the conversation: texts and reasonings live, calls with their diffs, subagents, permission and question cards, the end of a turn with each model’s cost, the quota of a `rate_limit_event`, the usage limit, a sign-in lost.
- A change of model, effort or mode, an interruption, a title, a task stopped, remote control go as control requests, whose answer the core waits for (`Asked`, `Item::Answer`).
- The “Claude Code view” goes through the module too (`view/translate`): the core hands it the lines of the session’s transcript.

What stays in the core: Claude Code’s place in the registry, the accounts and their sign-in, the quota read from the usage endpoint, an account’s models when no agent runs, the written texts (`claude -p`), the terminal view’s PTY and its hooks, Escouade’s MCP server, each agent’s plan and requests.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, its account folder |
| `Cargo.toml` | The crate `escouade-plugin-claude`, a `cdylib` the core doesn’t link |
| `Cargo.lock` | The crates published on crates.io: what the store compiles |
| `src/lib.rs` | Its `run` and the methods of the contract |
| `src/client.rs` | The client of `claude`, a state machine without a process |
| `src/translate.rs` | Its lines as items of the conversation |
| `src/view.rs` | The transcript read for the “Claude Code view” |
| `src/localcmd.rs` | What it writes of a local command (`/compact`, `! ls`) |
| `src/words.rs` | What is said in the app’s language, its quota’s windows |
| `src/tests.rs` | The client and the translation on their own |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

Three things make the crate a module: `[lib] crate-type` in `Cargo.toml`, `"run": { "wasm": "Cargo.toml" }` in the manifest, and in `src/lib.rs`:

```rust
pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run("claude", json!({ "harnesses": ["claude"] }), move |m, params| plugin.handle(m, params))
}

escouade_plugin_protocol::module!(run);
```

`turn::run` keeps the handler; each turn of the module reads the lines that came and answers them. Nothing waits in a turn: `client.rs` and `translate.rs` keep their state between two lines, and what is asked of `claude` comes back as a later line. No `tokio`, thread, `std::fs` or `std::process`.

## Its tests

- `src/tests.rs`, with `cargo test` in `plugins/`;
- the parity with the former native plugin, line by line on the recorded streams (`src-tauri/src/plugins_claude_acp_parity_tests.rs`, `tests/fixtures/plugins/parity/claude.json`);
- the bench of the harnesses on the fake `claude` (`src-tauri/src/engine/claude/bench.rs`) and the recorded streams (`tests/fixtures/replays/claude/`);
- the tests of the real process (`src-tauri/src/process_tests.rs`), those of the agents in `core_tests.rs`, nearly every e2e of `tests/e2e/`;
- the real runs, started by hand: `real_claude_models` and `real_claude_view` (`src-tauri/src/tui_tests.rs`), `real_fence`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`): the module as the store compiles it, from this folder alone and the crates of its `Cargo.lock`, then the bench’s fake Escouade.

```sh
escouade-plugin build claude --first-party --out ../packs
escouade-plugin test claude --first-party --wasm ../packs/dev.escouade.claude-code-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh claude` compiles it against `crates/`, for the app’s tests and a development build. After a change of the crates it uses, `sh scripts/plugin-lock.sh claude` writes its `Cargo.lock` again.
