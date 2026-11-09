# Develop

For whoever wants to read or change the plugin. It lives in `plugins/cursor/` of Escouade’s repository, id `dev.escouade.cursor-cli`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone. Its code is a few lines: it serves the harness its manifest declares (`contributes.harnesses[0]`, id `cursor`, its `acp` object with the extension `cursor`) by the ACP client of the crate `escouade-acp` (`escouade_acp::plugin::serve_as`), as the ACP plugin serves the harnesses other plugins describe.

- The core finds `agent` (on Windows, the launcher its installer leaves runs as its `node` and `index.js`), asks the plugin how to launch it (`engine/launch`: `agent acp`, `MSYSTEM` and `EXEPATH` taken away), checks the description and starts it, each agent in its configuration folder (`CURSOR_CONFIG_DIR`).
- What Cursor writes comes by `agent/frames` and goes back as items of the conversation: the session (new or resumed), its mode, model and effort, texts, reasonings, calls under Claude Code’s names with their diffs, permissions, questions (`cursor/ask_question`), the plan (`cursor/create_plan`), tasks (`cursor/update_todos`), subagents, each turn’s end, and Cursor’s refusals (a usage limit, a model outside the plan, a key or a sign-in refused).
- A message’s images and text files go as the core made them; what was said in its terminal view goes before the next message (`relay` of `engine/launch`).

What stays in the core: the terminal view (its PTY, its transcript followed), the accounts (the machine’s sign-in, an account by its key, `keyVar`), the models read for an account’s menu, the written texts, Escouade’s MCP server.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, its `acp` object |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-cursor` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its `run`: the ACP plugin’s service, with its own harness, and its manifest’s test |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

The client, its permission rules and the extension `cursor` are in `crates/escouade-acp/` (`agent.rs`, `cursor.rs`).

## Its tests

- its manifest’s test in `src/lib.rs`, and those of `crates/escouade-acp`, with `cargo test`;
- the bench of the harnesses on the fake Cursor (`src-tauri/src/engine/cursor/bench.rs`);
- the recorded streams of Cursor CLI 2026.10.22 replayed by its client (`src-tauri/src/plugins_acp_tests.rs`);
- the core’s tests that go through it (`src-tauri/src/cursor_tests.rs`, `cursor_tui_tests.rs`) and the e2e `tests/e2e/cursor.spec.ts`;
- the real runs, started by hand: `real_cursor`, `real_cursor_view`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build cursor --first-party --out ../packs
escouade-plugin test cursor --first-party --wasm ../packs/dev.escouade.cursor-cli-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh cursor` compiles it against `crates/`, and `sh scripts/plugin-lock.sh cursor` writes its `Cargo.lock` again after a change of the crates.
