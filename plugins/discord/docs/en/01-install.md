# Installing Discord

## 1. Make the webhook in Discord

1. In Discord, open the settings of the channel the messages should reach.
2. **Integrations › Webhooks › New Webhook**.
3. Give it a name and an avatar if you like: the channel shows them.
4. **Copy Webhook URL**. It starts with `https://discord.com/api/webhooks/`.

This address is enough to post in the channel: keep it like a password.

## 2. Install the plugin

1. In Escouade, open the Plugins view (the puzzle piece icon in the projects bar).
2. **Store**, search for “Discord”, then **Install**. You can also start from its page on <https://store.escouade.dev/plugins/dev.escouade.discord> and **Open in Escouade**.
3. The “Install Discord?” window says what it will be able to do: get `ticket.column` and `agent.status`, connect to `discord.com`, keep a secret. **Install**.

## 3. Paste the webhook’s address

1. On the plugin’s page, **Settings** tab, **Secrets** section: “Discord webhook URL”.
2. Paste the URL in the field, then **Keep**. The row then says “kept in the keychain”.

**Forget** erases the address: nothing goes any more.

## 4. Choose the projects

**Projects** tab: the plugin is on in every project that is not private. A private project is off by default: nothing goes for it until you turn it on there. Turn off a project whose messages you do not want in that channel.

All the projects it is on in post to the same channel: there is only one webhook address.

## What Escouade keeps, and where

- **The webhook’s address**: in the system’s keychain, under a name of the plugin’s own. It is written in no file.
- **The plugin**: its files in `~/.escouade/plugins/dev.escouade.discord`, and its install (version, fingerprint, your choices per project) in `~/.escouade/plugins.json`.
- **The journal of what it sent**: `~/.escouade/plugin-logs/dev.escouade.discord.log`. The webhook’s address is masked there.

Uninstalling erases its files, its settings, its secret and its journal, unless you tick “Keep its settings and its secrets for a later install”.
