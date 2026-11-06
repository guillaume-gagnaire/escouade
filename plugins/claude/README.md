# Claude Code

`dev.escouade.claude-code`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own,
that the app runs in its host. It is the example of a harness in Rust: the plugin is a state machine that turns lines into
lines, and everything it does not do is the core's.

Claude Code comme harness. Ses agents conversent comme avant, par un module isolé qu'Escouade fait tourner et surveille.

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
- nothing else. Its manifest declares no permission at all: no network, no secret, no file, no command. A module can only
  do what its manifest declares, so this is not a promise but what the host applies. Escouade's token is in a private file
  of the agent's, whose path alone is among the arguments.

## Its account

It keeps no account: each account is a folder of Claude Code's (`CLAUDE_CONFIG_DIR`), or the machine's own for Principal.
What Claude Code keeps there (`.credentials.json`, on macOS its keychain entry) is read by the core alone
(`src-tauri/src/engine/claude/signin.rs`): the plugin never sees a sign-in, and has no way to. It only knows the words of a
turn that lost one (`translate::lost`).

## Turned off

Turned off or not loaded, an agent of Claude Code says at its start « Le plugin Claude Code
(dev.escouade.claude-code) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a
plugin that stops while an agent runs is started again, as every plugin is, and an agent says so; after the last crash the
agent stops, its session kept. The texts, the quota of an account and the terminal view don't go through it.

## What is in this folder

| File                   | What it is                                                              |
| ---------------------- | ----------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, its folder       |
| `Cargo.toml`           | The crate `escouade-plugin-claude`: a `cdylib`, and an `rlib` for the core |
| `Cargo.lock`           | The crates published on crates.io: what the store compiles              |
| `src/lib.rs`           | Its `run` and the export of its turn: the methods of the contract       |
| `src/client.rs`        | The client of `claude`, a state machine without a process               |
| `src/translate.rs`     | Its frames as items of the conversation                                 |
| `src/localcmd.rs`      | What it writes of a local command (`/compact`, `! ls`)                  |
| `src/words.rs`         | What is said in the app's language, a local output, its quota's windows |
| `src/tests.rs`         | The client and the translation on their own                             |
| `README.md`            | This page                                                               |

## How a harness is a module

Three lines make the same crate a module. `[lib] crate-type` in `Cargo.toml`, `"run": { "wasm": "Cargo.toml" }` in the
manifest, and in `src/lib.rs`:

```rust
pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run("claude", json!({ "harnesses": ["claude"] }), move |m, params| plugin.handle(m, params))
}

escouade_plugin_protocol::module!(run);
```

`turn::run` keeps the handler; each turn of the module reads the lines that came and answers them. Nothing waits in a turn,
and nothing needs to: `client.rs` and `translate.rs` hold their state between two lines, and what is asked of `claude` comes
back as a later line (`Asked`, `Item::Answer`). No `tokio`, no thread, no `std::fs`, no `std::process`.

The core still reads `translate`, `words`, `localcmd` and `client`'s arguments as a library, for « Vue Claude Code », which
is why this crate is an `rlib` too (until P8, docs/PLUGINS.md §8.10). The store compiles the `cdylib`.

## Its tests

`src/tests.rs`, the parity with the native plugin, line by line on every recorded stream
(`src-tauri/src/plugins_claude_acp_parity_tests.rs`, `tests/fixtures/plugins/parity/claude.json`), the bench of the harnesses on the fake `claude` (`src-tauri/src/engine/claude/bench.rs`), its recorded
streams (`tests/fixtures/replays/claude/`), the tests of the real process (`src-tauri/src/process_tests.rs`), every test of
an agent in `core_tests.rs`, nearly every e2e of `tests/e2e/`, and the real runs (`real_claude_models`, `real_claude_view`,
`real_fence`, `real-harnesses.spec.ts`).

## Run it alone

From `plugins/`, with the `escouade-plugin` of the app (`cargo build --bin escouade-plugin` in `src-tauri/`): the module as the
store compiles it, from this folder alone on the crates of its `Cargo.lock`, and the fake Escouade of the bench:

```sh
escouade-plugin build claude --first-party --out ../packs
escouade-plugin test claude --first-party --escouade 2.2.0 --wasm ../packs/dev.escouade.claude-code-2.2.0.wasm
```

In this repository `sh scripts/plugins-wasm.sh claude` compiles it against `crates/`, for the tests of the app and a build of
development. After a change of the crates it uses, `sh scripts/plugin-lock.sh claude` writes its `Cargo.lock` again.
