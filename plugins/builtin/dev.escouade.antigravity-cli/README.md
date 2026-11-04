# Antigravity CLI

`dev.escouade.antigravity-cli`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with it,
and updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is read).

Antigravity CLI de Google comme harness. Ses agents conversent comme avant, par un programme à part qu'Escouade lance et
surveille.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `agy`): `agy -p=` driven in stream-json, one
message a line on its input, one event a line on its output (it is no ACP harness). On the contract of the harnesses in
process (INT-9):

- the core finds `agy`, asks this plugin how to launch it (`engine/launch`: stream-json both ways, its model and effort, its
  mode, its conversation, the agent's own home as `USERPROFILE` and `HOME`), checks the launch and starts it;
- it starts it again when it must (`relaunches`): `agy` takes its model, effort and mode at its launch alone, and has no
  message that stops a turn. A turn stopped pauses its program (`Item::Pause`), the next message starts another on the same
  conversation (`--conversation`, `Item::Relaunch`), each description checked as the first one was;
- this plugin tells the core the agent is ready when `agy` said its conversation (`init`, `Item::Ready`); one to resume that is
  gone gives a new one, said in the conversation;
- what `agy` writes comes here (`agent/frames`) and goes back as the items of the conversation, in the form the ACP client reads
  (`escouade-acp`, extension `agy`): its texts, its calls under Claude Code's names with their diffs, its subagents, its
  tokens, the end of each turn. What its events leave out (a call's arguments, a write's text, its reasoning) is read from the
  conversation's transcript;
- `-p` asks nothing: what needs a permission is refused at once, and `ask_question` skipped. At the end of such a turn this
  plugin shows the cards, writes the rules allowed and has the turn go on with a message saying what was allowed and answered;
- in the mode Plan, the plan it wrote in its conversation is shown on the card « Plan à approuver ».

What stays in the core, as the contract says: Antigravity's place in the registry (its capabilities, the meaning of its modes),
its accounts (the machine's Google sign-in, others by their Gemini key), the models of an account (`agy models`), the texts it
writes (`agy -p`), the terminal view (its PTY, its transcript, its hooks), Escouade's MCP server, and taking its token out of
the agent's home when the process is over.

## What it may do

- describe the launch of `agy` and translate what it says: the core launches it, never this plugin;
- read, in the agent's own home: the rules kept for « Toujours », `settings.json`, `mcp_config.json` and the transcript of its
  conversation; and the plan file `agy` names, in that home;
- have the core write, in that home alone (`engine/launch`, `Item::File`, which the core checks): the permission rules, what
  « Toujours » allows, Escouade's server with its token. The token is never among its arguments;
- nothing else: no network, no secret, no command, no process. The user's `~/.gemini` is never read nor written. The sign-in of
  the machine is in its keychain, which this plugin never reads; a key goes by a variable of the core's.

## Accounts

The machine's account is `agy` signed in in a terminal (« Ajouter un compte… »); another account goes by its Gemini API key
(`GEMINI_API_KEY`, `modelProvider: gemini` in the agent's settings). A key of the machine is not given to an agent that does not
go by it.

## Turned off

It can be turned off for the app, never removed, and a plugin of the store with the same id is refused: this one is the app's.
Turned off or not loaded, an agent of Antigravity says at its start « Le plugin Antigravity CLI (dev.escouade.antigravity-cli)
est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its conversation; a plugin that stops while an agent
runs is started again, as every plugin is. A card shown before a restart cannot go on: its answer ends the turn. The texts, the
models of an account and the terminal view don't go through it.

## What is in this folder

| File                   | What it is                                                      |
| ---------------------- | --------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, its folder and key        |
| `Cargo.toml`           | The crate `escouade-plugin-agy`                                 |
| `src/lib.rs`           | Its `run`: the methods of the contract                          |
| `src/client.rs`        | The client of `agy -p`, a state machine without a process       |
| `src/translate.rs`     | Its events as items of the conversation                         |
| `src/words.rs`         | Its launch, rules, home, calls and questions, read by the core  |
| `src/tests.rs`         | The client on its own                                           |
| `README.md`            | This page                                                       |

Its program is `escouade-plugins agy` (`src-tauri/src/bin/escouade-plugins.rs` calls the crate's `run`). Its tests: `src/tests.rs`,
the bench of the harnesses on the fake `agy` (`src-tauri/src/engine/agy/bench.rs`), the recorded streams of `agy` 1.4.9
(`src-tauri/src/agent/agy_tests.rs`, `tests/fixtures/replays/agy/`), the tests of the core that go through it
(`src-tauri/src/agy_tests.rs`, `agy_tui_tests.rs`), the e2e `tests/e2e/agy.spec.ts`, and the real runs (`real_agy`, `real_agy_writes_a_text`,
`real-harnesses.spec.ts`, its view included).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` and `escouade-plugins` (built with
`cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`, side by side in `target/debug`):

```sh
escouade-plugin test plugins/builtin/dev.escouade.antigravity-cli --builtin
```
