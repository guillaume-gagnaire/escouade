# Discord

The tickets to test and the agents waiting for you, in a Discord channel. A ticket that reaches “To review” posts `🧪 APP-7 to test: …` there, an agent waiting for your answer posts `✋ Mila waits for you`.

**Who it is for**: you or your team, when Escouade is not in front of you and you want to know there is something to test or to unblock.

**What it needs**: a webhook of a Discord channel, which you make in the channel’s settings. No Discord account to link, no bot, no program.

**What it can do**:

- get two events from Escouade: a ticket that changes column, an agent whose status changes;
- send a message to `discord.com`, and nowhere else;
- keep one secret: the webhook’s address, in the system’s keychain.

It has no program: it declares two webhooks, and Escouade sends them. Nothing of it runs on your machine.

The rest is in the documentation tabs:

1. Install: make the webhook and paste it in Escouade.
2. Use: what it posts, when, and its limits.
3. Permissions: what goes to Discord and what never does.
4. Troubleshooting: what happens when nothing arrives.
5. Develop: its folder, its tests.
