# Installing Slack

## 1. Make the bot in Slack

1. On <https://api.slack.com/apps>, create an app (**Create New App › From scratch**) in your workspace.
2. **OAuth & Permissions › Bot Token Scopes**: add `chat:write`. The plugin needs nothing else.
3. **Install to Workspace**, then copy the **Bot User OAuth Token**. It starts with `xoxb-`.
4. Make, if needed, each project’s channel (its name is below), and invite the bot there: `/invite @your-app-name` in the channel.

This token lets anyone post as the bot: keep it like a password.

## 2. Install the plugin

1. In Escouade, open the Plugins view (the puzzle piece icon in the projects bar).
2. **Store**, search for “Slack”, then **Install**. You can also start from its page on <https://store.escouade.dev/plugins/dev.escouade.slack> and **Open in Escouade**.
3. The “Install Slack?” window says what it will be able to do: run inside Escouade, isolated; get `ticket.column` and `agent.status`; connect to `slack.com`; keep a secret. **Install**.

It needs Escouade 2.2.0 or later.

## 3. Paste the token

1. On the plugin’s page, **Settings** tab, **Secrets** section: “Slack bot token (xoxb-…)”.
2. Paste the token, then **Keep**. The row then says “kept in the keychain”.

**Forget** erases the token: nothing goes any more.

## 4. Each project’s channel name

Each project posts to `escouade-<project name>`, written as Slack writes channel names: lower case, no accents, each run of characters other than letters, digits, `_` and `-` replaced by a `-`, 80 characters at most.

| Project | Channel |
| --- | --- |
| `Shop` | `#escouade-shop` |
| `Équipe Web` | `#escouade-equipe-web` |
| `API v2` | `#escouade-api-v2` |

The plugin’s `channels.json` file can give a project another channel, by its name. The published one only names Escouade’s own project (`Escouade` → `#escouade-dev`). Do not change it in the installed folder: Escouade would see that its files changed and turn it off.

## 5. Choose the projects

**Projects** tab: the plugin is on in every project that is not private. A private project is off by default: nothing goes for it until you turn it on there.

## What Escouade keeps, and where

- **The token**: in the system’s keychain, under a name of the plugin’s own. It is written in no file.
- **The plugin**: its files in `~/.escouade/plugins/dev.escouade.slack`, and its install (version, fingerprint, your choices per project) in `~/.escouade/plugins.json`. Its compiled WASM module is kept in Escouade’s cache, and made again if it goes.
- **Its journal**: in the **Journal** tab of its page, and whole in `~/.escouade/plugin-logs/dev.escouade.slack.log`. The token is masked there.

Uninstalling erases its files, its settings, its secret and its journal, unless you tick “Keep its settings and its secrets for a later install”.
