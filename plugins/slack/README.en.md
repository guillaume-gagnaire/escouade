# Slack

The tickets to test and the agents waiting for you, in each project’s Slack channel. A ticket that reaches “To review” posts `🧪 APP-7 to test: …` there, an agent waiting for your answer posts `✋ Mila waits for you`.

**Who it is for**: a team that lives in Slack, with a channel per project, and wants to see there what is to test or to unblock.

**What it needs**: a Slack app of your own, with a bot that has the `chat:write` scope, and its token (`xoxb-…`). The bot must be invited in each channel it posts to.

**What it can do**:

- get two events from Escouade: a ticket that changes column, an agent whose status changes;
- reach `slack.com` over https, and nowhere else;
- keep one secret: the bot’s token, in the system’s keychain;
- run its program inside Escouade, isolated (in WASM): no Node to install.

Each project posts to `#escouade-<project name>`.

The rest is in the documentation tabs:

1. Install: make the Slack bot and paste its token in Escouade.
2. Use: what it posts, in which channel, and its limits.
3. Permissions: what goes to Slack and what never does.
4. Troubleshooting: the messages of its journal.
5. Develop: its folder, its tests, its build.
