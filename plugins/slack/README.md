# Slack

`dev.escouade.slack`, a plugin of [Escouade](https://escouade.dev)'s own hand.

The tickets to test and the agents waiting for you, in each project's Slack channel. A program in TypeScript on
`@escouade-dev/plugin`, its bot's token kept in your system's keychain.

Les tickets à tester et les agents qui t'attendent, dans le salon Slack de chaque projet.

## Use it

1. In Slack: create an app with the scope `chat:write`, install it in the workspace, copy its bot token (`xoxb-…`),
   and invite the bot in the channels.
2. In Escouade: install the plugin from the Plugins view (or from
   <https://store.escouade.dev/plugins/dev.escouade.slack>), then paste the token in its page › Settings.

A project posts in the channel `channels.json` names for it, else in `#escouade-<project>`. A title, a key or a name is
escaped: `<!channel>` coming from a public issue is shown, never a mention.

## What it may do

- read the facts `ticket.column` and `agent.status`;
- reach `slack.com`;
- keep one secret, `token`;
- run its program, `dist/plugin.mjs` (Node 22.18 or later), as this folder holds it.

## What is in this folder

| File                   | What it is                                                     |
| ---------------------- | -------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest                                                   |
| `src/`                 | The program: `plugin.ts` (its handlers), `main.ts` (its start) |
| `dist/plugin.mjs`      | Its build, committed: Escouade runs it as the repository holds |
| `test/`                | Its tests, against the SDK's fake Escouade and a fake Slack    |
| `channels.json`        | The channel of each project                                    |
| `CHANGELOG.md`         | Its versions                                                   |

## Check it

```sh
npm install
npm run typecheck && npm test        # its tests
npm run build                        # dist/plugin.mjs again, then commit it
npm run validate && npm run bench    # the manifest, then Escouade's conformance bench
```

Its scripts pass `--first-party`: its id `dev.escouade.*` is kept for Escouade's own plugins. The store takes it from
this folder of Escouade's repository alone; its tags are `plugins/slack/v1.2.3`.

MIT license (`LICENSE`).
