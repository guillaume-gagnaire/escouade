# Example plugins of Escouade

Four plugins for [Escouade](https://escouade.dev), each a folder you can copy, read alone, and install as it is. They are
checked like a plugin of the store: `node ci.mjs` runs the TypeScript ones' tests, checks that their `dist/` is the build
of `src/`, then `escouade-plugin validate` and the conformance bench of Escouade on every one. The repository's CI runs
it on each change.

| Folder             | What it shows                                                                                      | Kind               |
| ------------------ | -------------------------------------------------------------------------------------------------- | ------------------ |
| `high-contrast/`   | A theme, dark and light, and a palette of project colors (`contributes.themes`)                   | Declarative        |
| `discord-webhook/` | The tickets to test and the agents waiting, in a Discord channel (`contributes.webhooks`)          | Declarative        |
| `slack/`           | The same in Slack, in each project's channel, the bot's token kept in the keychain (`secrets/get`) | TypeScript program |
| `team-rules/`      | The agents' commands allowed or denied by `rules.json` (`permissions/decide`)                      | TypeScript program |

Gemini CLI ships inside Escouade; a harness described by a plugin (ACP) will have its example with that extension point.

```sh
npx tsc -p ../sdk/plugin/tsconfig.json                       # the SDK the examples use (file:../../sdk/plugin)
ESCOUADE_PLUGIN_BIN=/path/to/escouade-plugin node ci.mjs     # all of them, as the CI does
cd slack && npm install && npm test && npm run bench         # one of them
```

To try one in Escouade: Plugins view › “In development”, then its folder. The documentation shows each one whole:
https://escouade.dev/en/docs/plugins/examples

MIT license (`LICENSE`).
