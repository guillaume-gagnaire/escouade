# Gemini CLI

`dev.escouade.gemini-cli`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Gemini CLI comme harness, par ACP : le troisième harness sorti du cœur d'Escouade (INT-10.7). Ses agents conversent comme avant, dans l'hôte des plugins d'Escouade.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `gemini`): Gemini CLI's `gemini --acp`,
driven over ACP v1 by the client of `crates/escouade-acp` with its extension `gemini`. On the contract of the harnesses
in process (INT-9):

- the core finds `gemini`, writes in the agent's own `GEMINI_CLI_HOME` its `settings.json` (no update, no statistics, how
  it signs in) and, for a private project or a discussion of the Docs tab, its rules; it asks this plugin how to launch
  it (`engine/launch`: `--acp`, its folder trusted by `GEMINI_CLI_TRUST_WORKSPACE`), adds `--policy` when there are
  rules, checks the launch and starts it;
- what Gemini writes comes here (`agent/frames`) and goes back as the items of the conversation and the lines to write
  to it: the session (new, or resumed by `session/load`, whose replay of the history is left out), its mode (`default`,
  `plan`, `autoEdit`, `yolo`), its model when its session offers it, texts, reasonings, calls under Claude Code's names
  (their tool read from their id, a subagent a `Task`, its MCP tools `mcp__<server>__<tool>`), permissions, and the end
  of each turn with its tokens;
- its change of mode, a text of its own (« [MODE_UPDATE] plan »), is read as such and left out of the thread;
- it has no tool to ask in ACP: the first message tells the model to end its turn on a block `<escouade_question>`,
  taken out of the text and shown as a question's card; the turn waits for the answers, which go on in the same turn;
- its plan is the permission of its `exit_plan_mode`, shown as the card « Plan à approuver », the plan read from the
  file it names; approved, the mode chosen is set once Gemini has left its mode plan;
- a refusal of the user's cuts the turn (`session/cancel`), its word sent after; « Toujours » is Gemini's own, for the
  session;
- its usage limit ends the turn « limité », its resume planned when it says, else a minute (an hour for its daily quota).

What stays in the core, as the contract says: the terminal view (`gemini` in a PTY, followed by its hooks alone), the
accounts (a Google sign-in by the file of the account's folder, given as `GOOGLE_APPLICATION_CREDENTIALS`, or an API key
as `GEMINI_API_KEY`), the models read for an account's menu, the texts it writes (`gemini -p`, every tool refused), the
files of its `GEMINI_CLI_HOME`, Escouade's MCP server.

## What it may do

- describe the launch of Gemini CLI and translate what it says: the core launches it, never this plugin;
- read the plan Gemini wrote, from the file its `exit_plan_mode` names, through the host: the folder of the agent's account
  (`agentHome`), and nothing else;
- refuse at once a permission that names a path of a private project (`fenceOnPermissions`: Gemini reads and writes by
  itself; its rules refuse those paths to its file tools and its commands, in every mode);
- refuse the writes and commands of a discussion of the Docs tab, as its rules do (`readOnlyMode`);
- nothing else: no network, no secret, no file written, no command to run. An account's key or sign-in is given to Gemini by the core,
  never to this plugin.

## Accounts

Nothing is declared here: an account is the core's, by its Google sign-in or its key, as Réglages › Comptes says it.
Each agent has a home of its own and never reads `~/.gemini`, shared with Antigravity CLI.

## Turned off

It is a plugin like any other: it can be turned off, and uninstalled. Turned off or not loaded, an agent of Gemini CLI
says at its start « Le plugin Gemini CLI
(dev.escouade.gemini-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session; a
plugin that stops while an agent runs is started again, as every plugin is.

## What is in this folder

| File                   | What it is                                                |
| ---------------------- | --------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, its `acp`           |
| `Cargo.toml`           | The crate `escouade-plugin-gemini`                        |
| `src/lib.rs`           | Its `run`: the ACP plugin's service, with its own harness |
| `README.md`            | This page                                                 |

Its program is a module (`run.wasm`), compiled by `escouade-plugin build` from this folder and its `Cargo.lock`, on the
service and the client of `crates/escouade-acp` (`plugin.rs`, `gemini.rs`). The parity with the native plugin it was:
`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/gemini.jsonl`. Its
tests: the bench of the harnesses on the fake Gemini (`src-tauri/src/engine/gemini/bench.rs`), the streams of the study
of Gemini CLI 0.66.0 replayed by its client (`src-tauri/src/plugins_acp_tests.rs`, `agent/gemini_tests.rs`), the tests of
the core that go through it (`gemini_tui_tests.rs`), the e2e `tests/e2e/gemini.spec.ts`, and the real run `real_gemini`,
written and ignored: no account has run it yet.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/gemini --first-party --escouade 2.2.0
```
