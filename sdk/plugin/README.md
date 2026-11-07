# @escouade-dev/plugin

Write a plugin of [Escouade](https://escouade.dev) in TypeScript: the types of its protocol (generated from `schemas/plugin-protocol-1.json`), a client that speaks it (`createPlugin`, `onEvent`, `decidePermission`, `secret`, `settings`, `file`, `log`), inside Escouade in WASM when the manifest says `"run": { "js": "dist/plugin.mjs" }` (no Node needed: `fetch` and `setTimeout` go through Escouade, to the hosts of `permissions.network` only; `openStream` and `openSocket` read a stream or a WebSocket the same way) or under Node on the standard input and output, the same code on both, a fake Escouade for its tests (`@escouade-dev/plugin/testing`) and the `escouade-plugin` command (`validate`, `test`, `dev`, `pack`).

```sh
npm create escouade-plugin@latest my-plugin
```

Documentation: https://escouade.dev/en/docs/plugins/sdk (in French: https://escouade.dev/docs/plugins/sdk). MIT license.

In this repository, `node scripts/generate.mjs` writes the generated files again from the schemas; a test fails when they differ.
