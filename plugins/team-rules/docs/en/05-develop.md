# Developing Team rules

`dev.escouade.team-rules` is a plugin of Escouade’s own hand, in `plugins/team-rules/` of its repository: a program in TypeScript on `@escouade-dev/plugin`, Escouade’s SDK, that answers `permissions/decide`. Escouade runs `dist/plugin.mjs` in WASM, as the repository holds it; Node 22.18 or later only builds and tests it.

## Its folder

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | the manifest |
| `rules.json` | its rules |
| `src/plugin.ts` | its rules in code: how a line is cut, `decide` |
| `src/main.ts` | its start |
| `dist/plugin.mjs` | its build, committed: Escouade runs it and the store compiles it |
| `test/plugin.test.ts` | its tests, against the SDK’s fake Escouade |
| `README.fr.md`, `README.en.md` | its page, on the store and in the app |
| `docs/fr/`, `docs/en/` | this documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | its versions, its license (MIT) |

It reads `rules.json` once, at start (`readFileSync`, served by Escouade for the JSON files at the root of its folder): a decision has only 500 ms.

Its `package-lock.json` is not committed (`.gitignore`): its dependencies only build and test it, and esbuild is pinned in `package.json`.

## Checking it

In its folder:

```sh
npm install
npm run typecheck && npm test        # its tests
npm run build                        # dist/plugin.mjs again, then commit it
npm run validate && npm run bench    # the manifest, then Escouade's conformance bench
```

The bench plays a decision and measures its delay. `npm run dev` runs it against the bench’s fake Escouade, to ask it requests by hand (`decide {"request": {...}}`).

`node plugins/ci.mjs`, from the repository’s root (with `ESCOUADE_PLUGIN_BIN` naming the app’s `escouade-plugin`), does all of this for each plugin of the folder, and checks that `dist/plugin.mjs` is the build of `src/`: a forgotten `dist/` fails the CI.

Its scripts pass `--first-party`: its id `dev.escouade.*` is kept for Escouade’s own plugins. The store takes it from this folder of Escouade’s repository alone; its tags are `plugins/team-rules/v1.2.3`.
