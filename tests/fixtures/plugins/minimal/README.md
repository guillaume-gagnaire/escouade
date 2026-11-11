# Minimal

`dev.escouade.minimal`, the demonstration plugin of the builds of development and the tests of
[Escouade](https://escouade.dev), never in a release. Its version is the app's (the `0.0.0` of its manifest is replaced
when it is read).

The least a plugin of the first hand is, a module (`run.wasm`). It does the handshake of the protocol, hears the facts
`ticket.column` and does nothing with them, and answers `minimal/echo` (what it is given) and `minimal/hello` (the
version of Escouade). It proves the way from the registry (`src-tauri/src/plugins/builtin.rs`) to an instance in WASM.

It is also the proof of the contract of the harnesses in process (INT-9, `docs/PLUGINS-DESIGN.md` §4.5): it declares the
harness « Minimal » (`contributes.harnesses`), the minimal fake engine of the conformance bench made a plugin. The core
asks it how to launch its harness (`engine/launch`), checks the description and launches the program itself
(`tests/fixtures/fake-minimal.mjs` in the tests); the plugin translates the harness's frames into the normalized items of
`escouade_plugin_protocol::engine` (`agent/frames`), and says what to write to it (a message, an answer, an
interruption), the launch of its terminal view and the answers to its hooks (`terminal/launch`, `terminal/hook`).

Le plugin de démonstration des builds de développement et des tests d'Escouade.

## What it may do

- read the facts `ticket.column` (and ignore them);
- describe the launch of its harness and translate what it says: the core launches it, never the plugin;
- write in the agent's folder, for the scenario « plantage » alone (its flag);
- nothing else: no network, no secret, no account, no command to run.

## What is in this folder

| File                   | What it is                                                                        |
| ---------------------- | --------------------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                                      |
| `src/lib.rs`           | Its code, on `crates/escouade-plugin-protocol`                                    |
| `plugin.wasm`          | Its module, committed: `sh build.sh` makes it again (`wasm32-wasip1`)             |
| `README.md`            | This page                                                                         |

Its tests are those of the harness in process (`src-tauri/src/plugins_harness_tests.rs`: the conformance bench, a crash
in the middle of a turn, the checks of the launch).
