# Minimal

`dev.escouade.minimal`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with it,
and updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is read).

The demonstration of INT-4: the least a plugin of the app is. It does the handshake of the protocol, hears the facts
`ticket.column` and does nothing with them, and answers `minimal/echo` (what it is given) and `minimal/hello` (the
version of Escouade). It proves the way from the registry of the app (`src-tauri/src/plugins/builtin.rs`) to a process:
the program `{escouade}/escouade-plugins`, started with the name `minimal`.

Le plugin de démonstration livré avec Escouade : le moins qu'un plugin de l'app fasse.

## Use it

Nothing to do: it is in Settings › Plugins, under « Livrés avec Escouade », on. It can be turned off for the app or for
a project, never removed, and a plugin of the store with the same id is refused: this one is the app's.

## What it may do

- read the facts `ticket.column` (and ignore them);
- nothing else: no network, no secret, no account, no command to run.

## What is in this folder

| File                   | What it is                                                       |
| ---------------------- | ---------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                     |
| `README.md`            | This page                                                        |

Its program is `src-tauri/src/bin/escouade-plugins/minimal.rs`; its tests are those of the registry
(`src-tauri/src/plugins_builtin_tests.rs`) and of the protocol crate (`crates/escouade-plugin-protocol`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` and `escouade-plugins` (built
with `cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`, side by side in `target/debug`):

```sh
escouade-plugin test plugins/builtin/dev.escouade.minimal --builtin
```
