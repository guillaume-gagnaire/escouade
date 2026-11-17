# Changelog

The changes of `@escouade-dev/plugin`, version by version.

## [0.4.0] — not published yet

- `validate` (`escouade-plugin validate`, `@escouade-dev/plugin/manifest-check`) knows the app's whole manifest: `run.wasm`, `permissions.files`, `permissions.env` and `limits`, and no longer wants a `run.command` (issue #43). Its fields are those of `plugin-1.json`, the schema Escouade's reader writes; Escouade's tests have every manifest of its repository judged by both, and fail when they differ.
- `p.fs` (`read`, `text`, `write`, `stat`, `list`, `canon`) for the folders of `permissions.files`, and `p.runProgram` for the programs of `permissions.commands`, with `proc/*` and `fs/*` typed from `plugin-protocol-1.json`: `proc/out` carries text (never base64), `proc/exit` its code, `null` when the program was killed.
- `net/stream`, `ws/*` and `timer/*` typed, with `openStream`, `openSocket` and `setTimeout` in WASM.

## [0.3.0] — 2026-11-01

- `start()` speaks to Escouade's WASM host or to Node's standard input and output, the plugin unchanged; `p.file()` reads a file of its folder; `fetch` goes through `net/fetch` in WASM.

## [0.2.0] — 2026-10-30

- The types of the protocol, the stdio client, the fake Escouade for tests and the `escouade-plugin` command.
