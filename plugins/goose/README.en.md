# Goose

This plugin makes Goose, Block’s open source agent, a harness of Escouade. It holds no code: its manifest describes Goose over ACP (`goose acp`), and Escouade’s ACP plugin (`dev.escouade.acp`) launches it, speaks to it and shows its conversation: replies live, tools with their diffs, permission and question cards, the tokens of each turn.

It is for anyone who already has Goose and a configured provider.

## What it needs

- Goose installed on the machine (`goose --version` answers 1.57.0 or later), with its provider configured (`goose configure`).
- The ACP plugin (`dev.escouade.acp`) installed and on: it runs the described harnesses.
- A Goose account added in Settings › Accounts and harnesses.

## What it can do

- Nothing of it runs: it is a plugin without a program, which the install window calls “No program”.
- Its agents run your machine’s `goose acp`, with your rights.
- It asks for no network, no secret, no file and no account of its own. Escouade reads none of Goose’s keys.

Goose writes files by itself, without passing them through Escouade: as soon as one of your projects is private, no Goose agent starts.

## The tabs

- **Install**: Goose, the plugin, an account.
- **Use**: the modes, discussions, private projects.
- **Permissions**: what the plugin has run, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the manifest, its test.
