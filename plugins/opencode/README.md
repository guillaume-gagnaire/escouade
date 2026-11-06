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
- this plugin is the client of that server (`agent/open`): it opens, resumes or copies the agent's session, gives its
  folder Escouade's MCP server with the agent's own token, sets the rules of its mode, sends its messages, answers its
  permissions and questions;
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
  runs, and no other; the server stands behind a password that never leaves Escouade. Requests go by `net/fetch`, its events by
  `net/stream`, up to 16 Mio an answer: a copy of a session whose messages weigh more than that is refused;
- hold the fence of a private project and the read-only of a discussion as rules of the agent's session (OpenCode has
  no sandbox: only they protect; a plugin of the app alone is believed on this, `fenceByClient`);
- nothing else: no other address, no secret of the keychain, no file, no command to run. An account's folder and its
  providers are OpenCode's; Escouade's token goes to the agent's folder on the server, never among its arguments.

## Accounts

An account is a configuration folder of OpenCode's (`OPENCODE_CONFIG_DIR`), the providers connected in it its own
(`opencode auth login`). OpenCode has no quota of its own: the cost of a turn is what its steps say, and an account takes
a budget per day.

## Turned off

It can be turned off for the app, never removed, and a plugin of the store with the same id is refused: this one is
the app's. Turned off or not loaded, an agent of OpenCode says at its start « Le plugin OpenCode
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
| `src/host.rs`          | Its one thread: its tasks, on the lines of the host (requests, stream, timers)     |
| `src/server.rs`        | The client of `opencode serve`: its requests, its event stream                     |
| `src/session.rs`       | An agent's session on it: messages held, permissions, questions, modes             |
| `src/translate.rs`     | Its events into the items of the conversation                                      |
| `src/rules.rs`         | The rules of each mode, the fence, the config, the models as OpenCode lists them   |
| `README.md`            | This page                                                                          |

It compiles to a module by `escouade-plugin build` (`sh scripts/plugins-wasm.sh opencode` for the tests of the app). The
core still links the crate as a library, for its « Vue OpenCode » and its accounts. Its tests: those of the crate, played on
its lines without a module (`src/tests.rs`), its parity with the native plugin
(`src-tauri/src/plugins_wasm_parity_tests.rs`, `tests/fixtures/plugins/parity/opencode*.json`), the bench of the harnesses on the fake `opencode serve` (`src-tauri/src/engine/opencode/bench.rs`), the
recorded streams of OpenCode 1.19.3 replayed through it (`tests/fixtures/replays/opencode/`), the tests of the core that go
through it (`src-tauri/src/opencode_tests.rs`, `src-tauri/src/agent/opencode_tests.rs`), the e2e
`tests/e2e/opencode.spec.ts`, and the real runs (`real_opencode`, `real_opencode_view`, `real_fence`,
`real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` (`cargo build --bin escouade-plugin`
in `src-tauri/`):

```sh
escouade-plugin test plugins/opencode --first-party --escouade 2.2.0
```
