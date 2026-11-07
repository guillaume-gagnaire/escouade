# Discord

`dev.escouade.discord`, a plugin of [Escouade](https://escouade.dev)'s own hand.

The tickets to test and the agents waiting for you, in a Discord channel: two webhooks, no code.

Les tickets à tester et les agents qui t'attendent, dans un salon Discord : deux webhooks, sans code.

## Use it

1. In Discord: the channel's settings › Integrations › Webhooks › New webhook, then copy its URL.
2. In Escouade: install the plugin from the Plugins view (or from
   <https://store.escouade.dev/plugins/dev.escouade.discord>), then paste the URL in its page › Settings › « Discord
   webhook URL ». It is kept in your system's keychain.

It posts `🧪 APP-7 to test: …` when a ticket goes to the column to test, and `✋ Mila waits for you` when an agent waits.
Mentions are off (`allowed_mentions`): a title taken from a public issue never pings the channel.

## What it may do

- read the facts `ticket.column` and `agent.status`;
- reach `discord.com`;
- keep one secret, `webhook`.

## What is in this folder

| File                     | What it is                                                     |
| ------------------------ | -------------------------------------------------------------- |
| `escouade-plugin.json`   | The manifest: its webhooks (`contributes.webhooks`) and rights |
| `test/webhooks.test.mjs` | Which facts post what, and what it may not do                  |
| `CHANGELOG.md`           | Its versions                                                   |

## Check it

```sh
node --test                                 # the webhooks, on the manifest
escouade-plugin validate . --first-party    # the manifest
escouade-plugin test . --first-party        # Escouade's conformance bench
```

`--first-party`: its id `dev.escouade.*` is kept for Escouade's own plugins. The store takes it from this folder of
Escouade's repository alone; its tags are `plugins/discord/v1.2.3`.

MIT license (`LICENSE`).
