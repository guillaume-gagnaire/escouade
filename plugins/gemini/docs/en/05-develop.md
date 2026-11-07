# Develop

For whoever wants to read or change the plugin. It lives in `plugins/gemini/` of Escouade’s repository, id `dev.escouade.gemini-cli`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone. Its code is a few lines: it serves the harness its manifest declares (`contributes.harnesses[0]`, id `gemini`, its `acp` object with the extension `gemini`) by the ACP client of the crate `escouade-acp` (`escouade_acp::plugin::serve_as`).

- The core finds `gemini`, writes in the agent’s `GEMINI_CLI_HOME` its `settings.json` (no update, no statistics, how it signs in) and, for a private project or a discussion, its rules; it asks the plugin how to launch it (`engine/launch`: `--acp`, the folder trusted by `GEMINI_CLI_TRUST_WORKSPACE`), adds `--policy` when there are rules, checks the launch and starts it.
- What Gemini writes comes by `agent/frames` and goes back as items of the conversation: the session (new, or resumed by `session/load`, whose replay of the history is left out), its mode, its model, texts, reasonings, calls under Claude Code’s names (their tool read from their id, a subagent as `Task`, its MCP tools as `mcp__<server>__<tool>`), permissions, each turn’s end with its tokens.
- Its change of mode, a text of its own (“[MODE_UPDATE] plan”), is read as such and left out of the thread, and can only lower the mode.
- Its questions go by an `<escouade_question>` block at the end of a turn; its plan is the permission of its `exit_plan_mode`, read from the file it names.
- A refusal of the user’s cuts the turn (`session/cancel`); its usage limit ends the turn “limited”, its resume planned.

What stays in the core: the terminal view (`gemini` in a PTY, followed by its hooks), the accounts, the models read for an account’s menu, the written texts (`gemini -p`, every tool refused), the files of its `GEMINI_CLI_HOME`, Escouade’s MCP server.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, its `acp` object |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-gemini` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its `run`: the ACP plugin’s service, with its own harness, and its manifest’s test |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

The client and the extension `gemini` are in `crates/escouade-acp/` (`agent.rs`, `gemini.rs`).

## Its tests

- its manifest’s test in `src/lib.rs`, and those of `crates/escouade-acp`, with `cargo test`;
- the parity with the former native plugin (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/gemini.jsonl`);
- the bench of the harnesses on the fake Gemini (`src-tauri/src/engine/gemini/bench.rs`) and the streams of the study of Gemini CLI 0.66.0 replayed by its client (`src-tauri/src/plugins_acp_tests.rs`, `src-tauri/src/agent/gemini_tests.rs`);
- the core’s tests that go through it (`gemini_tui_tests.rs`) and the e2e `tests/e2e/gemini.spec.ts`;
- the real run `real_gemini`, written and ignored: no account has run it yet.

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build gemini --first-party --out ../packs
escouade-plugin test gemini --first-party --wasm ../packs/dev.escouade.gemini-cli-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh gemini` compiles it against `crates/`, and `sh scripts/plugin-lock.sh gemini` writes its `Cargo.lock` again after a change of the crates.
