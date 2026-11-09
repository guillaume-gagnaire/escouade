# OpenCode

This plugin makes OpenCode a harness of Escouade, through its server. Your agents run on your machine’s `opencode`, with the providers you connected to it, local models included, and their conversation shows in Escouade: replies and reasoning live, tools with their diffs, permission, question and plan cards, the cost of each turn.

It is for anyone who already has OpenCode 1 and at least one connected provider.

## What it needs

- OpenCode 1 installed on the machine (`npm install -g opencode-ai`, tried with 1.19.3). OpenCode 2 isn’t supported.
- A provider connected to OpenCode (`opencode auth login`), and its account added in Settings › Accounts and harnesses.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no file and no command.
- It tells Escouade how to launch `opencode serve`; Escouade launches it, once for the account’s agents. The plugin draws its password at random.
- It talks to that server on 127.0.0.1, on the one port Escouade read, and to nothing else.
- It gives each session the permission rules of its mode, a private project’s fence and a discussion’s read-only. OpenCode has no sandbox: those rules are what protects.
- It gives the server, for each agent’s folder, Escouade’s MCP server with the agent’s token, never among the arguments.

## The tabs

- **Install**: OpenCode, the plugin, the account.
- **Use**: the modes, the OpenCode view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode, and what its password protects from.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
