# Cursor CLI

This plugin makes Cursor CLI a harness of Escouade, over ACP. Your agents run on your machine’s `agent`, with your Cursor account, and their conversation shows in Escouade: replies and reasoning live, tools with their diffs, permission, question and plan cards, tasks and subagents.

It is for anyone with a Cursor account, Free included.

## What it needs

- Cursor CLI installed on the machine (tried with 2026.10.22), signed in by `agent login`.
- A Cursor CLI account added in Settings › Accounts and harnesses: this machine’s account, or another one by its API key.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no network, no file and no command.
- It tells Escouade how to launch `agent acp`, then translates what Cursor writes. Escouade launches the program, never the plugin.
- It accepts on its own, in Accept edits, the writes Cursor submits in the agent’s folder (outside `.git`), and everything in Bypass.
- It refuses at once a request that names a path of another project when one of the two is private.
- It is given the token of Escouade’s MCP server for each agent it describes, to hand it to Cursor’s session, never among the arguments.

Cursor reads and writes files by itself, without passing them through Escouade: what it doesn’t ask, Escouade cannot stop.

## The tabs

- **Install**: Cursor CLI, the plugin, an account.
- **Use**: the modes, the Cursor CLI view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
