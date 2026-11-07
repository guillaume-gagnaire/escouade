# isola

`dev.escouade.isola`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module (`run.wasm`), compiled
from this folder, run in Escouade's host. Its version is its own.

It is the first **launch environment** (INT-14, `contributes.launchEnvironments`): what runs each git worktree's
services on ports and `*.localhost` addresses of their own, so that a ticket's test launch needs no recipe on the
reserved ports. It drives [isola](https://github.com/cyucelen/isola) (`isola up`, `ls`, `down`, `destroy`).

## Use it

Nothing to do: it is on in every project but the private ones, where it has to be turned on, in Réglages › Plugins. A
worktree is isola's when the project commits an `.isola.toml` **and** the `isola` program is installed. Then:

- « ▶ Tester » shows the `.isola.toml` first (it may declare commands), and runs `isola up` once the user approved
  exactly that file; the services' addresses, from `isola ls --json`, are what the test modal opens and probes;
- `isola down` stops the services when the test ends, the agent is archived or deleted, or its project closed;
- `isola destroy` tears them down, with what they keep (databases), before the worktree goes.

Turned off for the app or for a project, or without `isola` or without the file, a worktree is nobody's: a ticket
starts as it did without isola (a recipe on the reserved ports).

## What it declares

| Field      | Value         | What the app does with it                                                           |
| ---------- | ------------- | ----------------------------------------------------------------------------------- |
| `detect`   | `.isola.toml` | the file that says a worktree is isola's; read whole for the user before `up`       |
| `program`  | `isola`       | the CLI, looked for on the PATH by the app                                          |
| `up`       | `isola up`    | the command a test launch runs in a terminal (its input is not one: no password)   |

The module answers `launch/ls`, `launch/down` and `launch/destroy`. The app lends it the worktree for the time of a
call, under a name it gives in `agent`, with its branch and the language of its messages: the module holds no path.

## What it may do

- run, through the host (`proc/spawn`), the three commands of `permissions.commands`, word for word, in a worktree the
  app lent it: `isola ls --json`, `isola down` and `isola destroy`. They show in the window of its permissions, and the
  host refuses any other. They run no command the `.isola.toml` declares (only `up` does, which the app runs itself, and
  the user reads that file first). That holds for the current upstream isola, and is to be checked again when isola
  changes;
- nothing else: no network, no secret, no account, no event, no file. It is isolated; the program is not, and has the
  user's rights.

The host finds `isola` on the PATH. Under Windows it only starts an `.exe` or a `.com`, never a `.cmd`, and isola has no
release for Windows: a build of it must be on the PATH.

## What is in this folder

| File                   | What it is                                                            |
| ---------------------- | --------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                          |
| `Cargo.toml`           | Its crate, on `escouade-plugin-kit`                                   |
| `Cargo.lock`           | The crates of crates.io the store compiles (`scripts/plugin-lock.sh`) |
| `src/lib.rs`           | Its code and its unit tests                                           |
| `README.md`            | This page                                                             |

The app's side is `src-tauri/src/launchenv.rs`. The tests of the launches (`src-tauri/src/core_tests.rs`, « isola ») and
`src-tauri/src/plugins_isola_tests.rs` run the module in the app's host against the test double
`tests/fixtures/fake-isola.mjs`, which stands for the `isola` CLI.

## Run it alone

Against the fake Escouade of the plugins' bench, as every plugin of the app, from `plugins/`:

```sh
cargo test -p escouade-plugin-isola
escouade-plugin test isola --first-party --escouade 2.2.0
```

The bench starts no `isola`: `launch/*` is the app's to call, with a worktree it lends.
