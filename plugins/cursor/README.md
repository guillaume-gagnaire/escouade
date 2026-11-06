# Cursor CLI

`dev.escouade.cursor-cli`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Cursor CLI comme harness, par ACP : le premier harness sorti du cœur d'Escouade (INT-10.1). Ses agents conversent comme avant, dans l'hôte des plugins d'Escouade.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `cursor`): Cursor CLI's `agent acp`,
driven over ACP v1 by the client of `crates/escouade-acp` with its extension `cursor` (`cursor/ask_question`,
`cursor/create_plan`, `cursor/update_todos`, its subagents, `cursor/list_available_models` for the efforts of its
models). On the contract of the harnesses in process (INT-9):

- the core finds `agent` (the launcher its installer leaves is run as its `node` and `index.js`), asks this plugin how
  to launch it (`engine/launch`: `agent acp`, `MSYSTEM` and `EXEPATH` taken away), checks the launch and starts it,
  each agent in a configuration folder of its own (`CURSOR_CONFIG_DIR`);
- what Cursor writes comes here (`agent/frames`) and goes back as the items of the conversation and the lines to write
  to it: the session (new or resumed), its mode and model and effort, texts, reasonings, calls under Claude Code's
  names with their diffs, permissions, questions, the plan, the tasks, the subagents, the end of each turn, and
  Cursor's refusals (a usage limit, a model out of the plan, a key or a sign-in refused) said as the core said them;
- a message's images and text files go as the core made them (`content` of `agent/sendMessage`); what was said in its
  terminal view goes before the next message (`relay` of `engine/launch`).

What stays in the core, as the contract says: the terminal view (its PTY, its transcript followed), the accounts (the
machine's sign-in, an account by its API key, `keyVar`), the models read for an account's menu, the texts it writes
(a ticket's title…), the PTY of every terminal, Escouade's MCP server.

## What it may do

- describe the launch of Cursor CLI and translate what it says: the core launches it, never this plugin;
- refuse at once a permission that names a path of a private project (`fenceOnPermissions`: Cursor reads and writes
  by itself, which its modes' meanings say);
- nothing else: no network, no secret, no file, no command to run. The API key of an account is given to Cursor by
  the core, never to this plugin; the plugin is only told the account goes by one (`byKey`).

## Turned off

It is a plugin like any other: it can be turned off, and uninstalled. Turned off or not loaded, an agent of Cursor says at its start « Le plugin Cursor CLI
(dev.escouade.cursor-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a
plugin that stops while an agent runs is started again, as every plugin is.

## What is in this folder

| File                   | What it is                                                     |
| ---------------------- | -------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, `acp`   |
| `Cargo.toml`           | The crate `escouade-plugin-cursor`                             |
| `src/lib.rs`           | Its `run`: the ACP plugin's service, with its own harness      |
| `README.md`            | This page                                                      |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the
service of `plugins/acp` (`serve_as`) and the client of `crates/escouade-acp`. Its tests: the
bench of the harnesses on the fake Cursor (`src-tauri/src/engine/cursor/bench.rs`), the recorded streams of Cursor CLI
2026.10.22 replayed by its client (`src-tauri/src/plugins_acp_tests.rs`), the tests of the core that go through it
(`src-tauri/src/cursor_tests.rs`, `cursor_tui_tests.rs`), the e2e `tests/e2e/cursor.spec.ts`, and the real runs
(`real_cursor`, `real_cursor_view`, `real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/cursor --first-party --escouade 2.2.0
```
