# OpenCode

`dev.escouade.opencode`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

OpenCode comme harness, par son serveur HTTP : ses agents conversent comme avant, par un module qu'Escouade fait tourner.

## What it does

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `opencode`): OpenCode 1 (`opencode-ai`,
1.19), driven through `opencode serve`. OpenCode is not a process for each agent: one server serves the agents of an
account (0,7 Go a server), each a session of its own. On the contract of the harnesses in process (INT-9):

- the core finds `opencode`, asks this plugin how to launch it (`engine/launch`: `opencode serve`, its config, a password
  drawn at random, marked `shared`), checks the launch and starts the server in its Job object, once for the agents of
  the same account, with one agent a folder on a server; it stops the server with its last agent;
- this plugin is the client of that server (`agent/open`): it opens, resumes or copies the agent's session, sets the rules of
  its mode before the session is opened (refused, no session), gives its folder Escouade's MCP server with the agent's own
  token, sends its messages, answers its permissions and questions;
- it alone reads the server's event stream, and translates each event into the items of the conversation, sent to the
  core as they come (`agent/items`): texts, reasonings, calls under Claude Code's names with their diffs, permissions,
  questions, the plan, the tasks, the subagents, the context, what a turn cost, the end of each turn;
- a message sent during a turn is held until it ends (OpenCode would lose the first one's request).

What stays in the core, as the contract says: the launch and the end of the server, the terminal view (`opencode attach`
in the core's PTY, from what `agent/open` describes), the accounts, the models read for an account's menu, the texts it
writes (`opencode run`), Escouade's MCP server.

## What it may do

- describe the launch of OpenCode: the core launches it, never this plugin;
- speak to the server on 127.0.0.1 (`local:shared`): the host opens the port the core read in the server's own output, while it
  runs, and no other. Requests go by `net/fetch`, its events by `net/stream`, up to 16 Mio an answer: a copy of a session whose
  messages weigh more than that is refused;
- hold the fence of a private project and the read-only of a discussion as rules of the agent's session (OpenCode has
  no sandbox: only they protect; a plugin of the app alone is believed on this, `fenceByClient`). For a command the fence is a
  guard on its text: one that reaches a path without naming it passes;
- nothing else: no other address, no secret of the keychain, no file, no command to run. An account's folder and its
  providers are OpenCode's; Escouade's token goes to the agent's folder on the server, never among its arguments.

## Its password

The server stands behind a password drawn at random, and listens on `127.0.0.1` alone. That keeps out the other users of the
machine and the pages of a browser, not the agents: **OpenCode gives its whole environment to the commands it runs**, so any
command of an agent reads `OPENCODE_SERVER_PASSWORD` (tried on 1.19.3). With it and the port, such a command can change the
rules of its session, answer its own cards, and reach the sessions of the account's other agents on that server, whatever their
folder. OpenCode has no setting that keeps the variable from its commands: only a plugin file of its
own can blank it (`shell.env`), and this module writes no file; the command could still read it from its parent process.

So a card and a fence hold what an agent asks through OpenCode's tools, not what a command it was allowed does on purpose: as the
window says, OpenCode has no sandbox, and a command you accept runs with your rights.

## What goes to a card

What `rules.rs` keeps on a card in « Demander » and « Accepter les modifications », whatever the mode lets through; only « Sans
permissions » lets it go. Its cases, played on the real OpenCode, are `src/rules_verdicts.json`.

- An edit of what git or OpenCode run or take as rules: `.git` (a file in a worktree), `.opencode`, `opencode.json` and
  `opencode.jsonc`, `.husky`, `.githooks`, at any depth, under the short names of Windows and its streams too. A plugin of
  `.opencode/plugin` runs at the next start of the folder's instance, an `opencode.json` gives a subagent its rules, a hook runs
  at the next `git commit`.
- `git add` and `git commit` are accepted at once in « Accepter les modifications » only as `git add <paths>`, `git add -A`,
  `-u`, `--all`, `--update`, `git commit -m <message>` and `-am <message>`, with nothing else: a second option, a redirection,
  or what a shell would expand or unquote goes to a card.
- Under a fence, « Toujours » is not offered: what OpenCode keeps comes after the fence's `deny`s and wins over them (tried).
  `git commit -F` and the options that read a file are refused, in « Sans permissions » as well.

## Accounts

An account is a configuration folder of OpenCode's (`OPENCODE_CONFIG_DIR`), the providers connected in it its own
(`opencode auth login`). OpenCode has no quota of its own: the cost of a turn is what its steps say, and an account takes
a budget per day.

## Turned off

It can be turned off for the app, or uninstalled: it is a plugin like any other, and comes back from the store. Turned off or not loaded, an agent of OpenCode says at its start « Le plugin OpenCode
(dev.escouade.opencode) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… », and keeps its session.

A plugin that crashes is started again, as every plugin is: the server goes on, the agent's session is heard again, and
the conversation says « Le plugin de OpenCode a redémarré. ». After its last crash the plugin is stopped: its agents
stop, they say why, and their sessions are kept for their next message once it is started again.

## What is in this folder

| File                   | What it is                                                                         |
| ---------------------- | ---------------------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the harness, its modes, efforts, account folder, how to install it   |
| `Cargo.toml`           | The crate `escouade-plugin-opencode`                                               |
| `src/lib.rs`           | Its turn, the methods of the contract, the description of the launch               |
| `src/server.rs`        | The client of `opencode serve`: its requests, its event stream                     |
| `src/session.rs`       | An agent's session on it: messages held, permissions, questions, modes             |
| `src/translate.rs`     | Its events into the items of the conversation                                      |
| `src/rules.rs`         | The rules of each mode, the fence, the config, the models as OpenCode lists them   |
| `src/rules_verdicts.json` | What those rules do, case by case, as the real OpenCode was seen doing it       |
| `src/tests.rs`         | The plugin on its lines, without a module                                          |
| `Cargo.lock`           | The crates published on crates.io: what the store compiles                         |
| `README.md`            | This page                                                                          |

It compiles to a module by `escouade-plugin build` (`sh scripts/plugins-wasm.sh opencode` for the tests of the app). Its tests: those of the crate, played on
its lines without a module (`src/tests.rs`), its parity with the native plugin
(`src-tauri/src/plugins_wasm_parity_tests.rs`, `tests/fixtures/plugins/parity/opencode*.json`), the bench of the harnesses on the fake `opencode serve` (`src-tauri/src/engine/opencode/bench.rs`), the
recorded streams of OpenCode 1.19.3 replayed through it (`tests/fixtures/replays/opencode/`), the tests of the core that go
through it (`src-tauri/src/opencode_tests.rs`, `src-tauri/src/agent/opencode_tests.rs`), the e2e
`tests/e2e/opencode.spec.ts`, and the real runs (`real_opencode`, `real_opencode_view`, `real_fence`,
`real-harnesses.spec.ts`). `node scripts/opencode-real-rules.mjs` plays the rules on a real `opencode serve` with a fake model,
without an account.

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/opencode --first-party --escouade 2.2.0
```
