# {{name}}

A plugin of [Escouade](https://escouade.dev), in TypeScript with [`@escouade-dev/plugin`](https://escouade.dev/en/docs/plugins/sdk).

- `src/plugin.ts`: what it does (a fact received, a permission decided);
- `src/main.ts`: the program Escouade starts, bundled into `dist/plugin.mjs` by `npm run build`;
- `plugin` and `plugin.cmd`: what `run` in `escouade-plugin.json` starts, on macOS and Linux, and on Windows.

```sh
npm install
npm test            # the handlers, against the SDK's fake Escouade, in memory
npm run validate    # the manifest, field by field
npm run bench       # the conformance bench of Escouade, on the real program
npm run dev         # the fake Escouade: type facts, decisions and calls
npm run archive     # the .tar.gz and its SHA-256, in packs/
```

`npm run bench` and `npm run dev` use the `escouade-plugin` that ships with Escouade: install the app, or give its path in `ESCOUADE_PLUGIN_BIN`.

To try it in Escouade, load this folder in Settings › Plugins (developer mode). Before you tag a version, run `npm run build` and commit `dist/`: Escouade runs the plugin as the repository holds it, without `npm install`. For people without Node, build one executable per system: https://escouade.dev/en/docs/plugins/sdk#one-executable
