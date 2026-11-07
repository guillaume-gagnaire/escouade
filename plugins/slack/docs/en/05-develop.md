# Developing Slack

`dev.escouade.slack` is a plugin of Escouade’s own hand, in `plugins/slack/` of its repository: a program in TypeScript on `@escouade-dev/plugin`, Escouade’s SDK. Escouade runs `dist/plugin.mjs` in WASM, as the repository holds it; Node 22.18 or later only builds and tests it.

## Its folder

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | the manifest |
| `src/plugin.ts` | its handlers: a project’s channel, a fact’s text, the send |
| `src/main.ts` | its start |
| `dist/plugin.mjs` | its build, committed: Escouade runs it and the store compiles it |
| `channels.json` | the channel of some projects, by their name |
| `test/plugin.test.ts` | its tests, against the SDK’s fake Escouade and a fake Slack, with no network |
| `README.fr.md`, `README.en.md` | its page, on the store and in the app |
| `docs/fr/`, `docs/en/` | this documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | its versions, its license (MIT) |

Its `package-lock.json` is not committed (`.gitignore`): its dependencies only build and test it, and esbuild is pinned in `package.json`.

## Checking it

In its folder:

```sh
npm install
npm run typecheck && npm test        # its tests
npm run build                        # dist/plugin.mjs again, then commit it
npm run validate && npm run bench    # the manifest, then Escouade's conformance bench
```

`npm run dev` runs it against the bench’s fake Escouade, to send it facts by hand.

`node plugins/ci.mjs`, from the repository’s root (with `ESCOUADE_PLUGIN_BIN` naming the app’s `escouade-plugin`), does all of this for each plugin of the folder, and checks that `dist/plugin.mjs` is the build of `src/`: a forgotten `dist/` fails the CI.

Its scripts pass `--first-party`: its id `dev.escouade.*` is kept for Escouade’s own plugins. The store takes it from this folder of Escouade’s repository alone; its tags are `plugins/slack/v1.2.3`.
