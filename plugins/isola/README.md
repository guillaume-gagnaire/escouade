# isola

`dev.escouade.isola`, a plugin [Escouade](https://escouade.dev) gives: it is shipped with the app, signed with it, and
updated with it. Its version is Escouade's (the `0.0.0` of its manifest is replaced by the app's when it is read).

It is the first **launch environment** (INT-14, `contributes.launchEnvironments`): what runs each git worktree's
services on ports and `*.localhost` addresses of their own, so that a ticket's test launch needs no recipe on the
reserved ports. It drives [isola](https://github.com/cyucelen/isola) (`isola up`, `ls`, `down`, `destroy`).

## Use it

Nothing to do: it is on in every project, in Réglages › Plugins under « Livrés avec Escouade ». A worktree is isola's
when the project commits an `.isola.toml` **and** the `isola` program is installed. Then:

- « ▶ Tester » shows the `.isola.toml` first (it may declare commands), and runs `isola up` once the user approved
  exactly that file; the services' addresses, from `isola ls --json`, are what the test modal opens and probes;
- `isola down` stops the services when the test ends, the agent is archived or deleted, or its project closed;
- `isola destroy` tears them down, with what they keep (databases), before the worktree goes.

Turned off for the app or for a project, or without `isola` or without the file, a worktree is nobody's: a ticket
starts as it did without isola (a recipe on the reserved ports). It can be turned off, never removed.

## What it declares

| Field      | Value         | What the app does with it                                                           |
| ---------- | ------------- | ----------------------------------------------------------------------------------- |
| `detect`   | `.isola.toml` | the file that says a worktree is isola's; read whole for the user before `up`       |
| `program`  | `isola`       | the CLI, looked for on the PATH by the app                                          |
| `up`       | `isola up`    | the command a test launch runs in a terminal (its input is not one: no password)   |

The plugin's process answers `launch/ls`, `launch/down` and `launch/destroy`; the app hands it the program it found,
the worktree and its branch.

## What it may do

- run the `isola` program the app found, in a worktree of the user's project: `ls --json`, `down` and `destroy`, which
  run no command the `.isola.toml` declares (only `up` does, and the user reads that file first). That holds for the
  current upstream isola, and is to be checked again when isola changes;
- nothing else: no network, no secret, no account, no event.

## What is in this folder

| File                   | What it is   |
| ---------------------- | ------------ |
| `escouade-plugin.json` | The manifest |
| `README.md`            | This page    |

Its program is `src-tauri/src/bin/escouade-plugins/isola.rs` (with its unit tests); the app's side is
`src-tauri/src/launchenv.rs`. The tests of the launches (`src-tauri/src/core_tests.rs`, « isola ») run the plugin's
real process against the test double `tests/fixtures/fake-isola.mjs`, which stands for the `isola` CLI.

## Run it alone

Against the test double and the app's own `escouade-plugins` (built with `cargo build --bin escouade-plugins` in
`src-tauri/`):

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"launch/ls","params":{"program":"tests/fixtures/fake-isola","dir":".","branch":"main","lang":"en"}}' \
  | src-tauri/target/debug/escouade-plugins isola
```

or against the fake Escouade of the plugins' bench, as every plugin of the app:

```sh
escouade-plugin test plugins/isola --builtin
```
