# Discord’s permissions

What the plugin can do, what goes to Discord, and what it cannot do.

## What it is

A **declarative** plugin: no program, nothing of it runs. Its manifest declares two webhooks (an address, the facts that set them off, the text of the message), and Escouade sends them.

## What it asks for

| Permission | What the install window says |
| --- | --- |
| `events`: `ticket.column`, `agent.status` | “Gets these events” |
| `network`: `discord.com` | “Connects to discord.com”, enforced by Escouade: its webhooks go to this host only |
| `secrets`: `webhook` | “Keeps a secret: webhook” |

Each can be taken back in the **Permissions** tab of its page. Without `events:…`, its webhooks no longer get those facts; without `discord.com`, no send goes.

## The facts it gets

Only `ticket.column` (a ticket changes column) and `agent.status` (an agent’s status changes), and only those of the projects it is on in. A private project sends it nothing until you turn it on there.

Escouade posts only two cases: a ticket that reaches “To review”, an agent waiting for your answer. The other facts are received and ignored.

## What goes to Discord

For each message, a `POST` request to the webhook’s address, from your machine, with this text and nothing else:

- a ticket: its key, its title and the project’s name;
- an agent: its name and the project’s name.

Before each send, Escouade checks that the address is `http` or `https` and that its host is exactly `discord.com`. An address to another host is not sent, and the refusal goes to the journal.

## What never goes there

Neither the agent’s conversation, nor the ticket’s description, nor its criteria, nor your code, nor a path of your machine, nor the branch’s name. The text posted is set by the manifest: it holds only the fields above.

## The secret

The webhook’s address is the only secret. Escouade keeps it in the system’s keychain, under a name of the plugin’s own, and uses it only as the address of its webhooks: it never goes into a message’s text. It is masked in the journal.

## What it cannot do

- Run anything on your machine: it has no program.
- Read or write a file.
- Reach another host than `discord.com`.
- Decide your agents’ permissions.
- Read the channel’s messages: a webhook only writes.
