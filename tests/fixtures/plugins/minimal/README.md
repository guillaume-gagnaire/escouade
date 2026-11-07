# Minimal

`dev.escouade.minimal`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with it,
and updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is read).

The demonstration of INT-4: the least a plugin of the app is. It does the handshake of the protocol, hears the facts
`ticket.column` and does nothing with them, and answers `minimal/echo` (what it is given) and `minimal/hello` (the
version of Escouade). It proves the way from the registry of the app (`src-tauri/src/plugins/builtin.rs`) to a process:
the program `{escouade}/escouade-plugins`, started with the name `minimal`.

It is also the proof of the contract of the harnesses in process (INT-9, `docs/PLUGINS.md` §4.5): it declares the
harness « Minimal » (`contributes.harnesses`), the minimal fake engine of the conformance bench made a plugin. The core
asks it how to launch its harness (`engine/launch`), checks the description and launches
`{escouade}/escouade-plugins minimal-harness` itself; the plugin translates the harness's frames into the normalized
items of `escouade_plugin_protocol::engine` (`agent/frames`), and says what to write to it (a message, an answer, an
interruption), the launch of its terminal view and the answers to its hooks (`terminal/launch`, `terminal/hook`). The
harness is not in the registry of the app: the tests build it from this manifest.

Le plugin de démonstration livré avec Escouade : le moins qu'un plugin de l'app fasse.

## Use it

Nothing to do in a build of development: it is in the Plugins view, under « Livrés avec Escouade », on (a release's registry holds it not: it is a demonstration, until the real ones are given). It can be turned off for the app or for
a project, never removed, and a plugin of the store with the same id is refused: this one is the app's.

## What it may do

- read the facts `ticket.column` (and ignore them);
- describe the launch of its harness and translate what it says: the core launches it, never the plugin;
- nothing else: no network, no secret, no account, no command to run.

## What is in this folder

| File                   | What it is                                                       |
| ---------------------- | ---------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                     |
| `README.md`            | This page                                                        |

Its program is `src-tauri/src/bin/escouade-plugins/minimal.rs`, its harness `minimal_harness.rs` beside it; its tests
are those of the registry (`src-tauri/src/plugins_builtin_tests.rs`), of the harness in process
(`src-tauri/src/plugins_harness_tests.rs`: the conformance bench, a crash in the middle of a turn, the checks of the
launch) and of the protocol crate (`crates/escouade-plugin-protocol`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` and `escouade-plugins` (built
with `cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`, side by side in `target/debug`):

```sh
escouade-plugin test tests/fixtures/plugins/minimal --builtin
```
