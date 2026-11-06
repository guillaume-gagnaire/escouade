# Codex

`dev.escouade.codex`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Codex d'OpenAI comme harness. Ses agents conversent comme avant, dans l'hôte des plugins d'Escouade.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `codex`): `codex app-server`, driven over
its own JSON-RPC protocol, one message a line (it is no ACP harness). On the contract of the harnesses in process
(INT-9):

- the core finds `codex`, asks this plugin how to launch it (`engine/launch`: `app-server`, the plan tool on, the
  permissions profiles of a private project, Escouade's server by `-c`), checks the launch and starts it, each account
  in its folder (`CODEX_HOME`);
- this plugin opens the thread over the protocol (`opens`: new, resumed or copied), reads the account's models
  (`model/list`, every page), and tells the core the agent is ready (`Item::Ready`);
- what Codex writes comes here (`agent/frames`) and goes back as the items of the conversation and the messages to
  write to it: reasonings and texts as they are written, calls under Claude Code's names with their diffs, permission
  cards, questions, the plan of the mode Plan and its approval, the tasks of its plan tool, subagents, the context
  gauge, the quota, the end of each turn;
- it steers a message into the turn under way (`turn/steer`), and starts the next turn with the model, effort and mode
  of then: Codex takes them at `turn/start`;
- it estimates what a turn costs from its tokens (Codex tells none): `src/prices.rs` is OpenAI's public grid.

What stays in the core, as the contract says: Codex's place in the registry (its capabilities, the meaning of its
modes), the accounts and their sign-in, an account's quota and models when none of its agents runs (an app-server of
its own), the texts it writes (`codex exec`), the terminal view (its PTY, its hooks and their trust, its rollout read
through this plugin's translation, used as a library), Escouade's MCP server.

## What it may do

- describe the launch of Codex and translate what it says: the core launches it, never this plugin;
- accept without a card `git add` and `git commit` of the agent's own worktree, in « Accepter les modifications » and
  « Auto » (the rules of `crates/escouade-acp/src/git.rs`; Escouade reads the repository's configuration and its hooks for
  it, by `fs/gitProtected`, which asks for the root `agentRepo`);
- read `config.toml` of the account's folder (`agentHome`, through the host), only when Escouade's server is refused to an
  agent: an entry of the user's of that name is then turned off;
- nothing else: no network, no secret, no file written, no command of its own.
  Escouade's token is given to Codex in a variable, never among its arguments; this plugin never reads the sign-in of an
  account, though the folder it holds is one it may read.

## Turned off

It is a plugin like any other: it can be turned off, and uninstalled. Turned off or not loaded, an agent of Codex says at its start « Le plugin Codex (dev.escouade.codex) est
coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a plugin that stops while an agent
runs is started again, as every plugin is. What Codex asked before a restart is still answered, by its id; the texts,
the quota of an account, and the terminal view don't go through it.

## What is in this folder

| File                   | What it is                                                                 |
| ---------------------- | -------------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, its folder variable |
| `Cargo.toml`           | The crate `escouade-plugin-codex`                                          |
| `src/lib.rs`           | Its `run`: the methods of the contract                                     |
| `src/client.rs`        | The client of its app-server, a state machine without a process            |
| `src/translate.rs`     | Its messages as items of the conversation                                  |
| `src/words.rs`         | Its modes, the fence, Escouade's server among its options, its quota       |
| `src/prices.rs`        | The list prices of OpenAI's models                                         |
| `src/tests.rs`         | The client on its own                                                      |
| `README.md`            | This page                                                                  |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the
crates `escouade-plugin-protocol` and `escouade-acp`. Until P8 the core also links this crate, for its view. Its tests:
`src/tests.rs` and `src/prices.rs`, the bench of the harnesses on the fake Codex (`src-tauri/src/engine/codex/bench.rs`),
the recorded streams of Codex 0.163.0 (`src-tauri/src/agent/codex_tests.rs`, `tests/fixtures/replays/codex/`), the parity with the native plugin it was, call by call on these recordings (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/codex.jsonl`), the tests
of the core that go through it (`src-tauri/src/codex_tests.rs`, `codex_tui_tests.rs`), the e2e `tests/e2e/codex.spec.ts`,
and the real runs (`real_codex_compact`, `real_codex_view`, `real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/codex --first-party --escouade 2.2.0
```
