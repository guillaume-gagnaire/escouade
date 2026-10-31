# Escouade's own plugins

The plugins of [Escouade](https://escouade.dev)'s own hand, each a folder of its own: its manifest, its README, its
LICENSE (MIT), its CHANGELOG and its tests, readable alone. The store publishes them under the ids `dev.escouade.*`,
which it takes from this repository alone, submitted from their folder like anyone's plugin (docs/STORE.md §3.6).

| Folder           | Id                           | What it shows                                                                         | Kind               |
| ---------------- | ---------------------------- | ------------------------------------------------------------------------------------- | ------------------ |
| `high-contrast/` | `dev.escouade.high-contrast` | A theme, dark and light, and a palette of project colors (`contributes.themes`)       | Declarative        |
| `discord/`       | `dev.escouade.discord`       | The tickets to test and the agents waiting, in a Discord channel (`contributes.webhooks`) | Declarative        |
| `slack/`         | `dev.escouade.slack`         | The same in Slack, in each project's channel, the bot's token in the keychain         | TypeScript program |
| `team-rules/`    | `dev.escouade.team-rules`    | The agents' commands allowed or denied by `rules.json` (`permissions/decide`)         | TypeScript program |

A version is a tag `plugins/<folder>/v1.2.3` (never `v1.2.3`, the app's releases), then its submission on the store
from its folder, its review and its signature.

They are checked as an author checks a plugin before a tag: `node ci.mjs` runs each one's tests (the TypeScript ones'
typecheck too, and checks that their `dist/` is the build of `src/`), then `escouade-plugin validate` and the
conformance bench of Escouade, with `--first-party` for their reserved ids. The repository's CI runs it on each change.

```sh
npx tsc -p ../sdk/plugin/tsconfig.json                       # the SDK they use (file:../../sdk/plugin)
ESCOUADE_PLUGIN_BIN=/path/to/escouade-plugin node ci.mjs     # all of them, as the CI does
cd slack && npm install && npm test && npm run bench         # one of them
```

To try one in Escouade: Plugins view › “In development”, then its folder. To make one yours, copy its folder and give it
an id of your own (`io.github.<your-login>.<plugin>`). The documentation shows each one whole:
https://escouade.dev/en/docs/plugins/examples
