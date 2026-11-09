# Troubleshooting Slack

What the plugin writes in its journal, **Journal** tab of its page (the whole file: `~/.escouade/plugin-logs/dev.escouade.slack.log`), what it means and what to do.

## Nothing arrives, and the journal says nothing

No fact reached it. Check:

- that the plugin is **on** for the app (its switch in **Installed**) and that its state is not “Switched off”;
- that it is on for the **project** (**Projects** tab; a private project is off by default);
- that you did not take back `ticket.column` or `agent.status` in the **Permissions** tab;
- that the expected fact happened: a ticket reaching “To review”, an agent waiting for your answer. Other columns and statuses post nothing.

## The journal’s messages

**`posted in #escouade-shop`**
Sent: Slack took the message.

**`no Slack token kept: nothing sent (Plugins › Slack › Settings)`**
No token is kept. Paste the bot’s token in the **Settings** tab of the plugin’s page, **Secrets** section, then **Keep**.

**`the token kept is not a Slack bot token (xoxb-…): nothing sent`**
The token kept does not start with `xoxb-`: it may be a user token (`xoxp-`) or the app’s. Copy the **Bot User OAuth Token** from **OAuth & Permissions**.

**`Slack refused the message in #escouade-shop: {error}`**
Slack answered, but refused. The end is Slack’s error code:
- `not_in_channel`: the bot is not in the channel. Invite it (`/invite @your-app-name`);
- `channel_not_found`: no channel by that name, or a private channel the bot is not in. Make the channel with exactly that name, or invite the bot;
- `invalid_auth`, `token_revoked`, `account_inactive`: the token is no longer good. Install the app in the workspace again and paste the new token;
- `missing_scope`: the bot does not have `chat:write`. Add it, install the app again, and paste the new token;
- `is_archived`: the channel is archived.

**`Slack could not be reached: {reason}`**
The request did not get through: machine offline, proxy, certificate, Slack did not answer within 10 seconds, or its answer was not JSON (a Slack outage). Check your connection and **Settings › Advanced › Network**. That message is lost.

**`the queue of facts is full (256): the oldest, agent.status #17, is dropped`**
Facts came faster than the plugin handled them: the oldest was not posted.

## The plugin is stopped or turned off

The plugin’s state, at the head of its journal and on its row in **Installed**, says why:

**“Its files changed since it was installed (fingerprint …): install it again.”**
A file of `~/.escouade/plugins/dev.escouade.slack` was changed (`channels.json`, for instance). Uninstall it keeping its settings and its secrets, then install it again from the store.

**“Stopped after {n} crashes”**
Its program stopped several times in a row. Read the last lines of the journal, then **Restart**. If it happens again, report it on its store page.
