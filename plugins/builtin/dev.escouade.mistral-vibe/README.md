# Mistral Vibe

`dev.escouade.mistral-vibe`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with
it, and updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is
read).

Mistral Vibe comme harness, par ACP : le deuxième harness sorti du cœur d'Escouade (INT-10.6). Ses agents conversent
comme avant, par un programme à part qu'Escouade lance et surveille.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `vibe`): Mistral Vibe's `vibe-acp`, driven
over ACP v1 by the client of `crates/escouade-acp` with its extension `vibe`. On the contract of the harnesses in
process (INT-9):

- the core finds `vibe-acp` (on the PATH, else where `uv tool install mistral-vibe` leaves it), writes in the agent's
  own `VIBE_HOME` the trust of its folder and, for a private project, the `denylist` of its file tools, asks this
  plugin how to launch it (`engine/launch`: `vibe-acp`, no telemetry), checks the launch and starts it;
- what Vibe writes comes here (`agent/frames`) and goes back as the items of the conversation and the lines to write
  to it: the session (new, resumed or copied by `session/fork`), its agent (`ask`, `accept-edits`, `auto-approve`), its
  model and thinking (`off` for « Bas », `high` otherwise), texts, reasonings, calls under Claude Code's names (its
  `agents.spawn` a `Task`, its MCP tools `mcp__<server>__<tool>`), permissions named by their call alone, its questions
  (`elicitation/create`, a form), its task list, the end of each turn with what the session's cost grew by, and its
  rate limit (« limité », resumed a minute later);
- Escouade's mode Plan, which Vibe does not have, is played here: each prompt says to plan without changing anything,
  its writes are refused, and the turn's last words are the plan to approve on its card; approved, it is carried out in
  the mode chosen;
- a refusal of the user's cuts the turn (`session/cancel`): Vibe would otherwise try the command again under other
  forms, each asking again; « Toujours » is Vibe's `allow_always_permanent`, kept in the agent's own `VIBE_HOME`.

What stays in the core, as the contract says: the terminal view (`vibe` in a PTY, its journal and its hooks followed),
the accounts (the machine's key that `vibe --setup` kept, an account by its API key given as `MISTRAL_API_KEY`), the
models read for an account's menu, the texts it writes (`vibe -p`, no tool, within a price), the files of its
`VIBE_HOME`, Escouade's MCP server.

## What it may do

- describe the launch of Mistral Vibe and translate what it says: the core launches it, never this plugin;
- refuse at once a permission that names a path of a private project (`fenceOnPermissions`: Vibe reads and writes by
  itself; the core also refuses those paths to its file tools in its `config.toml`);
- nothing else: no network, no secret, no file, no command to run. The API key of an account is given to Vibe by the
  core, never to this plugin; the plugin is only told the account goes by one (`byKey`).

A discussion of the Docs tab never goes to it: it has no mode that changes nothing (`readOnlyMode`).

## Turned off

It can be turned off for the app, never removed, and a plugin of the store with the same id is refused: this one is the
app's. Turned off or not loaded, an agent of Mistral Vibe says at its start « Le plugin Mistral Vibe
(dev.escouade.mistral-vibe) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a
plugin that stops while an agent runs is started again, as every plugin is.

## What is in this folder

| File                   | What it is                                                    |
| ---------------------- | ------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, `acp`  |
| `Cargo.toml`           | The crate `escouade-plugin-vibe`                              |
| `src/lib.rs`           | Its `run`: the ACP plugin's service, with its own harness     |
| `README.md`            | This page                                                     |

Its program is `escouade-plugins vibe` (`src-tauri/src/bin/escouade-plugins.rs` calls the crate's `run`), on the service
of `plugins/builtin/dev.escouade.acp` (`serve_as`) and the client of `crates/escouade-acp` (`vibe.rs`). Its tests: the
bench of the harnesses on the fake Vibe (`src-tauri/src/engine/vibe/bench.rs`), the recorded streams of Mistral Vibe
3.0.3 replayed by its client (`src-tauri/src/plugins_acp_tests.rs`, `agent/vibe_tests.rs`), the tests of the core that go
through it (`src-tauri/src/vibe_tests.rs`, `vibe_tui_tests.rs`), the e2e `tests/e2e/vibe.spec.ts`, and the real runs
(`real_vibe`, `real_vibe_writes_a_text`, `real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` and `escouade-plugins` (built with
`cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`, side by side in `target/debug`):

```sh
escouade-plugin test plugins/builtin/dev.escouade.mistral-vibe --builtin
```
