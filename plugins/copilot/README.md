# Copilot CLI

`dev.escouade.copilot-cli`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

GitHub Copilot CLI comme harness, sorti du cœur d'Escouade (INT-10.3). Ses agents conversent comme avant, dans l'hôte des plugins d'Escouade.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `copilot`): `copilot --server --stdio`,
driven over the protocol of its SDK (JSON-RPC 2.0, protocol 3) by a client of Escouade's own. It is no ACP harness. On
the contract of the harnesses in process (INT-9):

- the core finds `copilot`, asks this plugin how to launch it (`engine/launch`), checks the launch and starts it; its
  messages go with `Content-Length` headers (`framing`), and `runtime.shutdown` is written before its input is closed
  (`closeLine`), so that it saves its session;
- this plugin opens the session itself, step by step (`opens`): the protocol checked, the sign-in, the session new,
  resumed or copied, its modes, Escouade's server, its model, the account's models. The agent is ready when that is
  done (`Item::Ready`); a refusal stops it (`Item::Stop`: another protocol, no sign-in) and says why;
- what Copilot writes comes here (`agent/frames`) and goes back as the items of the conversation and the messages to
  write to it: texts, reasonings, calls under Claude Code's names with their diffs, permissions, questions, forms, the
  end of a plan, the session's ceiling of credits, tasks, subagents, the month's credits, the context, and each turn's
  tokens and cost;
- a message's images and text files go as the core made them (`content` of `agent/sendMessage`); `/compact` goes by
  its request (`agent/compact`), a title by `session.name.set` (`agent/rename`).

What stays in the core, as the contract says: the terminal view (its PTY and its hooks), the accounts (`copilot login`
in each account's folder), an account's quota and models read by a server of their own, the texts it writes
(`copilot -p`), Escouade's MCP server. The view reads the session's log through this plugin's translation, as a library
(`src/translate.rs`): one translation for both.

## What it may do

- describe the launch of Copilot and translate what it says: the core launches it, never this plugin;
- answer the safe permissions by Escouade's rules (a read in the agent's folder; a write there, but in the mode Plan;
  `git add` and `git commit` of its worktree in « Accepter les modifications » and « Auto », for which Escouade reads the
  repository's configuration and hooks, by `fs/gitProtected`, which asks for the root `agentRepo`);
- refuse at once a permission that names a path of a private project: a guard, as its modes' meanings say;
- nothing else: no network and no secret. Escouade's token and the account's sign-in reach Copilot by the core, and
  this client never calls what gives a token in clear (`account.getAllUsers`, `account.getCurrentAuth`).

## Accounts

None of its own. An account of Copilot CLI is a folder (`COPILOT_HOME`) signed in by `copilot login`, in
Réglages › Comptes, as before; this plugin is only told the folder, to take the tokens of the app's environment away
from an account of its own (`GH_TOKEN`, `GITHUB_TOKEN`, `COPILOT_GITHUB_TOKEN`).

## Turned off

It is a plugin like any other: it can be turned off, and uninstalled. Turned off or not loaded, an agent of Copilot says at its start « Le plugin Copilot CLI
(dev.escouade.copilot-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a
plugin that stops while an agent runs is started again, as every plugin is, and says so in the conversation.

## What is in this folder

| File                   | What it is                                                                   |
| ---------------------- | ---------------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, models, efforts, framing and close     |
| `Cargo.toml`           | The crate `escouade-plugin-copilot`                                          |
| `src/lib.rs`           | Its `run`: the contract's methods, one client for each agent                 |
| `src/client.rs`        | The client of Copilot's server, a state machine without a process            |
| `src/translate.rs`     | Its events as the items of the conversation; also the view's                 |
| `src/words.rs`         | Its arguments, modes, models, credits and answers, and what it says          |
| `src/tests.rs`         | The client on its own                                                        |
| `README.md`            | This page                                                                    |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the
crates `escouade-plugin-protocol` and `escouade-acp`. Until P8 the core also links this crate, for its view. The parity
with the native plugin it was: `src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/copilot.jsonl`. Its other
tests: the bench of the harnesses on the fake Copilot (`src-tauri/src/engine/copilot/bench.rs`), the recorded streams of
Copilot CLI 1.0.98 (`src-tauri/src/agent/copilot_tests.rs`, `tests/fixtures/replays/copilot/`), the tests of the core
that go through it (`src-tauri/src/copilot_tests.rs`, `copilot_tui_tests.rs`), the e2e `tests/e2e/copilot.spec.ts`, and
the real runs (`real_copilot`, `real_copilot_view`, `real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/copilot --first-party --escouade 2.2.0
```
