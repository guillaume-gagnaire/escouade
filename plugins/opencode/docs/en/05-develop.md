# Develop

For whoever wants to read or change the plugin. It lives in `plugins/opencode/` of Escouade’s repository, id `dev.escouade.opencode`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone. It serves the harness its manifest declares (`contributes.harnesses[0]`, id `opencode`): OpenCode 1 (`opencode-ai`, 1.19) driven through `opencode serve`. OpenCode isn’t a process for each agent: one server serves an account’s agents (0.7 GB a server), each a session of its own.

- The core finds `opencode`, asks the plugin how to launch it (`engine/launch`: `opencode serve`, its config, a password drawn at random, marked `shared`), checks the launch and starts the server in its Job object, once for the agents of the same account; it stops it with the last agent.
- The plugin is the client of that server (`agent/open`): it opens, resumes or copies the agent’s session, sets its mode’s rules before opening it (refused, no session), gives the agent’s folder Escouade’s MCP server with its token, sends its messages, answers its permissions and questions.
- It alone reads the server’s event stream, and translates each event into items of the conversation, sent to the core as they come (`agent/items`): texts, reasonings, calls under Claude Code’s names with their diffs, permissions, questions, the plan, tasks, subagents, context, cost, each turn’s end.
- A message sent during a turn waits for its end.

What stays in the core: the server’s launch and end, the terminal view (`opencode attach` in the core’s PTY, from what `agent/open` describes), the accounts, the models read for an account’s menu, the written texts (`opencode run`), Escouade’s MCP server.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, efforts, account folder, how to install it |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-opencode` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its turn, the methods of the contract, the description of the launch |
| `src/server.rs` | The client of `opencode serve`: its requests, its event stream |
| `src/session.rs` | An agent’s session: messages held, permissions, questions, modes |
| `src/translate.rs` | Its events as items of the conversation |
| `src/rules.rs` | The rules of each mode, the fence, the config, the models as OpenCode lists them |
| `src/rules_verdicts.json` | What those rules do, case by case, as the real OpenCode was seen doing it |
| `src/tests.rs` | The plugin on its lines, without a module |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- those of the crate, played on its lines without a module (`src/tests.rs`, `src/rules.rs`), with `cargo test` in `plugins/`;
- the parity with the former native plugin (`src-tauri/src/plugins_wasm_parity_tests.rs`, `tests/fixtures/plugins/parity/opencode*.json`);
- the bench of the harnesses on the fake `opencode serve` (`src-tauri/src/engine/opencode/bench.rs`) and the recorded streams of OpenCode 1.19.3 (`tests/fixtures/replays/opencode/`);
- the core’s tests that go through it (`src-tauri/src/opencode_tests.rs`, `src-tauri/src/agent/opencode_tests.rs`) and the e2e `tests/e2e/opencode.spec.ts`;
- the real runs, started by hand: `real_opencode`, `real_opencode_view`, `real_fence`, `tests/e2e/real-harnesses.spec.ts`. `node scripts/opencode-real-rules.mjs` plays the rules on a real `opencode serve` with a fake model, without an account.

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build opencode --first-party --out ../packs
escouade-plugin test opencode --first-party --wasm ../packs/dev.escouade.opencode-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh opencode` compiles it against `crates/`, and `sh scripts/plugin-lock.sh opencode` writes its `Cargo.lock` again after a change of the crates.
