# Changelog

The changes of `@escouade-dev/plugin`, version by version.

## [0.5.0] — not published yet

It needs Escouade 2.2.5 for what follows (`ESCOUADE_MIN`, the `escouade.min` of a plugin that uses it).

- `p.secret(key, { profile })`, `p.profiles.list(kind)`, `p.profiles.get(kind, id)` and `p.profiles.of(project, setting)`: the profiles the user makes on the plugin's page, the one each project chose, and their secrets, multi-line ones included. `p.settings(project)` gives a project's settings by their ranks.
- `p.runProgram` gives the program's `pid`; `p.runTyped`, for an entry of `permissions.commands` with jokers, wants it.
- `p.require('profiles' | 'typedCommands')` refuses (`-32601`) before the call when Escouade did not announce it at `initialize`, and these calls check it themselves: an older Escouade would ignore the profile of `secrets/get` and answer another secret.
- The fake Escouade of `@escouade-dev/plugin/testing`, given the manifest (`manifest`), serves them as the app does: the profiles (`profiles`), the settings of each project by their ranks (`projectValues`, `projectSettings`, `off`), the refusals of a profile's secret, `proc/spawn` on `permissions.commands` with its jokers and `limits.processes` (`programs`, `spawned`, `exit`, `refusals`); `capabilities` plays an older Escouade.

## [0.4.0] — 2026-11-17

- `validate` (`escouade-plugin validate`, `@escouade-dev/plugin/manifest-check`) knows the app's whole manifest: `run.wasm`, `permissions.files`, `permissions.env` and `limits`, and no longer wants a `run.command` (issue #43). Its fields are those of `plugin-1.json`, the schema Escouade's reader writes; Escouade's tests have every manifest of its repository judged by both, and fail when they differ.
- `p.fs` (`read`, `text`, `write`, `stat`, `list`, `canon`) for the folders of `permissions.files`, and `p.runProgram` for the programs of `permissions.commands`, with `proc/*` and `fs/*` typed from `plugin-protocol-1.json`: `proc/out` carries text (never base64), `proc/exit` its code, `null` when the program was killed.
- `net/stream`, `ws/*` and `timer/*` typed, with `openStream`, `openSocket` and `setTimeout` in WASM.

## [0.3.0] — 2026-11-01

- `start()` speaks to Escouade's WASM host or to Node's standard input and output, the plugin unchanged; `p.file()` reads a file of its folder; `fetch` goes through `net/fetch` in WASM.

## [0.2.0] — 2026-10-30

- The types of the protocol, the stdio client, the fake Escouade for tests and the `escouade-plugin` command.
