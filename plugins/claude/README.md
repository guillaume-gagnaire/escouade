# Claude Code

`dev.escouade.claude-code`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with it,
and updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is read).

Claude Code comme harness. Ses agents conversent comme avant, par un programme à part qu'Escouade lance et surveille.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `claude`): `claude`, driven over stream-json
and its control protocol (docs/PROTOCOL.md). On the contract of the harnesses in process (INT-9):

- the core finds `claude`, asks this plugin how to launch it (`engine/launch`: the stream's arguments, the model, the
  effort and the mode, the added instructions, the session to resume or copy, the folders it reads, Escouade's server by a
  file of the agent's own), checks the launch and starts it, each account in its folder (`CLAUDE_CONFIG_DIR`);
- this plugin asks `initialize` as the first line of its input (`opens`) and tells the core the commands and the models of
  the account (`Item::Told`), then that the agent is ready (`Item::Ready`);
- what Claude Code writes comes here (`agent/frames`) and goes back as the items of the conversation: texts and reasonings
  as they are written, calls with their diffs, subagents, permission cards and questions, the running estimate of a turn,
  its end with what each model cost, the quota a `rate_limit_event` tells, the usage limit, a sign-in lost;
- a change of model, effort or mode, an interruption, a title, a task stopped, the link to claude.ai go as control requests,
  and the core waits for their answer (`Asked`, `Item::Answer`).

What stays in the core, as the contract says: Claude Code's place in the registry (its capabilities, its models and their
efforts), the accounts and the restart of an agent once its sign-in is good, an account's quota read from the usage
endpoint, the models of an account when none of its agents runs, the texts it writes (`claude -p`), the terminal view (its
PTY, its hooks, its transcript read through this plugin's translation, used as a library), Escouade's MCP server, and the
plan, the requests and the conversation of each agent.

## What it may do

- describe the launch of `claude` and translate what it says: the core launches it, never this plugin;
- answer by itself what Escouade takes no part in: an elicitation is declined, a request it does not know is refused;
- nothing else: no network, no secret, no file written, no command of its own. Escouade's token is in a private file of
  the agent's, whose path alone is among the arguments.

## Its account

It keeps no account: each account is a folder of Claude Code's (`CLAUDE_CONFIG_DIR`), or the machine's own for Principal.
`src/signin.rs` reads, for the core and as a library, what Claude Code keeps there (`.credentials.json`, on macOS its
keychain entry): whether it is signed in, a fingerprint of that sign-in, never a token out of the core. The process of the
plugin never reads it.

## Turned off

It can be turned off for the app, never removed, and a plugin of the store with the same id is refused: this one is the
app's. Turned off or not loaded, an agent of Claude Code says at its start « Le plugin Claude Code
(dev.escouade.claude-code) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a
plugin that stops while an agent runs is started again, as every plugin is, and an agent says so; after the last crash the
agent stops, its session kept. The texts, the quota of an account and the terminal view don't go through it.

## What is in this folder

| File                   | What it is                                                              |
| ---------------------- | ----------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, its folder       |
| `Cargo.toml`           | The crate `escouade-plugin-claude`                                      |
| `src/lib.rs`           | Its `run`: the methods of the contract                                  |
| `src/client.rs`        | The client of `claude`, a state machine without a process               |
| `src/translate.rs`     | Its frames as items of the conversation                                 |
| `src/localcmd.rs`      | What it writes of a local command (`/compact`, `! ls`)                  |
| `src/signin.rs`        | What it keeps of a sign-in, how a turn says it lost it                  |
| `src/words.rs`         | What is said in the app's language, a local output, its quota's windows |
| `src/tests.rs`         | The client and the translation on their own                             |
| `README.md`            | This page                                                               |

Its program is `escouade-plugins claude` (`src-tauri/src/bin/escouade-plugins.rs` calls the crate's `run`). Its tests:
`src/tests.rs`, the bench of the harnesses on the fake `claude` (`src-tauri/src/engine/claude/bench.rs`), its recorded
streams (`tests/fixtures/replays/claude/`), the tests of the real process (`src-tauri/src/process_tests.rs`), every test of
an agent in `core_tests.rs`, nearly every e2e of `tests/e2e/`, and the real runs (`real_claude_models`, `real_claude_view`,
`real_fence`, `real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` and `escouade-plugins` (built with
`cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`, side by side in `target/debug`):

```sh
escouade-plugin test plugins/claude --builtin
```
