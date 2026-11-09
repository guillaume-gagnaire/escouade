# Troubleshooting Discord

As the plugin has no program, it shows nothing in the app. What Escouade does for it is written in its journal, the file `~/.escouade/plugin-logs/dev.escouade.discord.log` (the **Journal** tab of its page does not show it for a plugin without a program). Open it to know what happened.

## Nothing reaches the channel, and the journal says nothing

No send was tried. Check:

- that the plugin is **on** for the app (its switch in **Installed**);
- that it is on for the **project** (**Projects** tab; a private project is off by default);
- that you did not take back `ticket.column`, `agent.status` or `discord.com` in the **Permissions** tab;
- that the expected fact happened: a ticket reaching “To review”, an agent waiting for your answer. Other columns and statuses post nothing.

## The journal’s lines

**`webhook to discord.com: 204 No Content (ticket.column #42)`**
Sent: Discord took the message.

**`webhook to discord.com: {code} (ticket.column #42) {answer}`**
Discord refused the message; the end of the line is the start of its answer (300 characters at most). By the code:
- `404 Not Found`: the webhook no longer exists (deleted in Discord) or the address is wrong. Make it again, then paste its new address in the **Settings** tab of the plugin’s page, **Secrets** section;
- `401 Unauthorized`: the end of the URL (the webhook’s token) is wrong. Copy the whole URL again;
- `429 Too Many Requests`: Discord limits the number of messages, that one is lost. Turn off the chattiest projects in the **Projects** tab.

**`webhook to discord.com: no answer (ticket.column #42): timed out`**
Discord did not answer within 10 seconds. The message is lost; the next ones go if Discord answers.

**`webhook to discord.com: no answer (…): {reason}`**
The request did not get through: machine offline, proxy, certificate. Check your connection and **Settings › Advanced › Network**.

**`a webhook's address is not one (ticket.column)`**
No address is kept, or what is kept is not an address. Paste the webhook’s URL in the **Settings** tab of the plugin’s page, **Secrets** section, then **Keep**.

**``a webhook to {host} is not sent: not an http(s) address of a host of `permissions.network` (ticket.column)``**
The address kept does not point to `discord.com`. An address on `discordapp.com` (Discord’s old domain) is refused: replace `discordapp.com` with `discord.com` in the URL, or copy it again from Discord.

**`the queue of facts is full (256): the oldest, agent.status #17, is dropped`**
Facts came faster than Discord answered: the oldest was not sent.

## The plugin is turned off

Its row in **Installed** says why:

**“Its files changed since it was installed (fingerprint …): install it again.”**
A file of `~/.escouade/plugins/dev.escouade.discord` was changed. Uninstall it keeping its settings and its secrets, then install it again from the store.
