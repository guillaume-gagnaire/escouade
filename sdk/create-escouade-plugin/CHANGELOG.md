# Changelog

The changes of `create-escouade-plugin`, version by version.

## [0.6.0] — not published yet

- The model in TypeScript asks for `@escouade-dev/plugin` 0.5, whose fake Escouade serves the profiles, the settings of each project, the jokers of `permissions.commands` and `limits.processes` as the app does. Nothing else changes in the models.

## [0.5.0] — 2026-11-17

- The model in TypeScript asks for `@escouade-dev/plugin` 0.4, whose `validate` knows the app's whole manifest (`run.wasm`, `permissions.files`, `limits`) and which types `p.fs` and `p.runProgram` (issue #43).
- The model in Rust says how to try it without a hidden step: `escouade-plugin build .`, then load the folder in Plugins › In development, which takes the module of `packs/` and loads again at each build. No more copying the module as `plugin.wasm`.

## [0.4.0] — 2026-11-05

- The model in Rust (`--kind rust`), with `escouade-plugin-kit`, its tests and its `Cargo.lock`.

## [0.3.0] — 2026-11-01

- The model in TypeScript writes `run.js`: a bundle Escouade runs in WASM, without Node.

## [0.2.0] — 2026-10-30

- The model asks for `@escouade-dev/plugin` 0.2.
