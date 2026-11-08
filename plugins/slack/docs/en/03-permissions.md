# Slack’s permissions

What the plugin can do, what goes to Slack, and what it cannot do.

## What it is

A program in TypeScript (`dist/plugin.mjs`), which Escouade compiles to WASM and runs isolated, inside Escouade: it has no process of its own, no access to your disk, and no Node to install. It only does what Escouade’s host serves it.

## What it asks for

| Permission | What the install window says |
| --- | --- |
| `run`: `dist/plugin.mjs` | “Runs inside Escouade, isolated” |
| `events`: `ticket.column`, `agent.status` | “Gets these events” |
| `network`: `slack.com` | “Connects to slack.com”, enforced by Escouade: it reaches no other host, nor an address of your machine or local network |
| `secrets`: `token` | “Keeps a secret: token” |

The `events`, `network` and `secrets` permissions can be taken back in the **Permissions** tab of its page: its program starts again without them. To stop it altogether, turn it off.

## The facts it gets

Only `ticket.column` and `agent.status`, and only those of the projects it is on in. A private project sends it nothing until you turn it on there. It gets the whole fact (the project, the ticket or the agent concerned, what changed), and posts only two cases of it: a ticket that reached “To review”, an agent waiting for your answer.

## What it reads

- The `channels.json` file of its own folder, which Escouade gives it at start. No other file.
- The token, which it asks Escouade for at each fact.

## What goes to Slack

For each message, a `POST https://slack.com/api/chat.postMessage` request, from your machine, with:

- the bot’s token, in the `Authorization` header;
- the channel’s name (`escouade-<project>`, or the one in `channels.json`);
- the text: the ticket’s key and title, or the agent’s name.

Escouade only lets out https requests to `slack.com` and its subdomains, without following a redirection, 10 seconds and 1 MiB at most, and never to an address of your machine or local network. The proxy of **Settings › Advanced › Network** is followed.

## What never goes there

Neither the agent’s conversation, nor the ticket’s description, nor its criteria, nor your code, nor a path of your machine. The token only goes to `slack.com`: the plugin refuses to send a token that does not start with `xoxb-`.

## The secret

The token is the only secret. Escouade keeps it in the system’s keychain, under a name of the plugin’s own, and gives it to the plugin alone, when it asks. It is masked in the journal.

## What it cannot do

- Run a program on your machine.
- Read your files or your agents’, or write a file.
- Reach another host than `slack.com`.
- Decide your agents’ permissions.
- Read Slack’s messages: it only calls `chat.postMessage`. Give its bot the `chat:write` scope alone, the only one it uses.
