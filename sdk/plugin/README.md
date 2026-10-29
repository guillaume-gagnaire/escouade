# @escouade-dev/plugin

Write a plugin of [Escouade](https://escouade.dev) in TypeScript: the types of its protocol (generated from `schemas/plugin-protocol-1.json`), a client that speaks it on the standard input and output (`createPlugin`, `onEvent`, `decidePermission`, `secret`, `log`), a fake Escouade for its tests (`@escouade-dev/plugin/testing`) and the `escouade-plugin` command (`validate`, `test`, `dev`, `pack`).

```sh
npm create escouade-plugin@latest my-plugin
```

Documentation: https://escouade.dev/en/docs/plugins/sdk (in French: https://escouade.dev/docs/plugins/sdk). MIT license.

In this repository, `node scripts/generate.mjs` writes the generated files again from the schemas; a test fails when they differ.
