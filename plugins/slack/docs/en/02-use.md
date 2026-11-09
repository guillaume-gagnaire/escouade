# Using Slack

Once the token is kept and the bot invited in the channels, there is nothing more to do: the plugin posts in the project’s channel when one of these two facts comes, in a project it is on in.

## What it posts

| When | The message, in `#escouade-<project>` |
| --- | --- |
| A ticket reaches the “To review” column | 🧪 **APP-7** to test: The cart |
| An agent waits for your answer (a question, a permission) | ✋ **Mila** waits for you |

The ticket’s key and title and the agent’s name are Escouade’s at the time of the fact; the channel says the project. The rest of the text is in English, whatever the interface’s language: it is written in its code.

The other moves (to “In progress”, “Done”…) and an agent’s other statuses post nothing. Nor does a fact without a project.

## Mentions

The key, the title and the name are escaped as Slack asks (`&`, `<`, `>`): a `<!channel>` or a `<@U…>` in a ticket’s title, from a public issue for instance, shows as it is without notifying anyone.

## When it runs

Its program starts with the first fact it gets, inside Escouade, in WASM. It asks Escouade for the token at each fact, and does not keep it.

## Its limits

- **One channel per project**, by its name: the bot must be invited there. The channel is not made for you.
- **No retry**: a message Slack refuses, or that does not go (Slack does not answer within 10 seconds, the machine is offline), is lost. The journal says so.
- **No catching up**: with Escouade closed, nothing is sent, and what happened meanwhile is not sent later.
- **A queue of 256 facts** per plugin: if facts pile up, the oldest are dropped, and the journal says so.
- **No link** to the ticket or the agent: the message says which, you open it in Escouade.
- The text of the messages cannot be set.

## The journal

The **Journal** tab of its page shows each message posted (`posted in #escouade-shop`) and each refusal, read again every 2 seconds. The whole file is `~/.escouade/plugin-logs/dev.escouade.slack.log`.
