# Using Discord

Once the webhook’s address is kept, there is nothing more to do: Escouade posts in the channel when one of these two facts comes, in a project the plugin is on in.

## What it posts

| When | The message |
| --- | --- |
| A ticket reaches the “To review” column | `🧪 **APP-7** to test: The cart (Shop)` |
| An agent waits for your answer (a question, a permission) | `✋ **Mila** waits for you (Shop)` |

The ticket’s key and title, the agent’s name and the project’s are Escouade’s at the time of the fact. The rest of the text is in English, whatever the interface’s language: it is written in the plugin’s manifest.

The other moves (to “In progress”, “Done”…) and an agent’s other statuses (working, done, in error) post nothing.

## Mentions

Mentions are off (`allowed_mentions` empty): an `@everyone` or an `@here` in a ticket’s title, from a public issue for instance, shows as it is without notifying the channel.

## Its limits

- **One channel** for all the projects: the plugin has one webhook address. Turn off in the **Projects** tab those you do not want there.
- **No retry**: a send that fails (Discord does not answer within 10 seconds, refuses, or the machine is offline) is lost. It is written in its journal.
- **No catching up**: with Escouade closed, nothing is sent, and what happened meanwhile is not sent later.
- **A queue of 256 facts** per plugin: if Discord is very slow and facts pile up, the oldest are dropped, and the journal says so.
- **No link** to the ticket or the agent: the message says which, you open it in Escouade.
- The text of the messages cannot be set: it is the manifest’s.

## The journal

Each send is written in the plugin’s journal, with Discord’s answer: `webhook to discord.com: 204 No Content (ticket.column #42)`. The webhook’s address does not show there.

As the plugin has no program, the **Journal** tab of its page does not show these lines (“A plugin without a program has no journal”). They are in the file `~/.escouade/plugin-logs/dev.escouade.discord.log`.
