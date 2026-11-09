# Copilot CLI

This plugin makes GitHub Copilot CLI a harness of Escouade. Your agents run on your machine’s `copilot`, with your GitHub account, and their conversation shows in Escouade: replies and reasoning live, commands and diffs, permission, question and plan cards, MCP servers’ forms, the cost of each turn in AI credits and the month’s credits.

It is for anyone with a GitHub account that has Copilot, Free included.

## What it needs

- Copilot CLI installed on the machine (`npm install -g @github/copilot`, tried with 1.0.98).
- A Copilot CLI account added in Settings › Accounts and harnesses, signed in by `copilot login`: with no account, no agent starts.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no network and no command.
- It tells Escouade how to launch `copilot --server --stdio`, then translates what Copilot writes. Escouade launches the program, never the plugin.
- It answers the safe requests on its own: a read in the agent’s folder, a write there outside `.git` and outside the Plan mode, and `git add` and `git commit` of the worktree in Accept edits and Auto.
- It refuses at once a request that names a path of another project when one of the two is private.
- Through Escouade’s host, it reads whether the configuration and the hooks of the agent’s repository are protected.
- It is given the token of Escouade’s MCP server for each agent it describes, and hands it to Copilot in a variable, never among the arguments.

## The tabs

- **Install**: Copilot CLI, the plugin, an account.
- **Use**: the modes, the Copilot CLI view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
