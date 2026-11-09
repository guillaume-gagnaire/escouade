# Claude Code

This plugin makes Claude Code a harness of Escouade. Your agents run on your machine's `claude`, with your account, and their conversation shows in Escouade: replies and reasoning live, tools with their diffs, permission, question and plan cards, the cost of each turn and the account's quota.

It is for anyone who already uses Claude Code, with a Claude subscription or an API key.

## What it needs

- Claude Code installed on the machine (tried from 2.1.283 to 2.1.301).
- A Claude Code account added in Settings › Accounts and harnesses: with no account, no agent starts.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no access to the disk, the network or commands. Its manifest asks for no permission.
- It tells Escouade how to launch `claude`, then translates what `claude` writes. Escouade launches the program, never the plugin.
- It is given the token of Escouade's MCP server for each agent it describes. It never puts it among the arguments: `claude` reads it from a private file of the agent's.
- It never sees your Claude sign-in: only Escouade reads it.

## The tabs

- **Install**: Claude Code, the plugin, an account.
- **Use**: the modes, the Claude Code view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
