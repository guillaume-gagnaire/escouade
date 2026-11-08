# ACP

This plugin runs in Escouade the harnesses that speak ACP (Agent Client Protocol) and that another plugin describes, without code. Goose is one. Escouade launches the agent, the ACP plugin speaks to it and shows its conversation: replies live, tools with their diffs, permission and question cards, the tokens of each turn.

It is for anyone who wants to use a described harness, like Goose, and for the author who wants to describe one.

## What it needs

- Nothing to install beyond the described harness itself and the plugin that describes it.
- An account of the described harness, in Settings › Accounts and harnesses.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no network and no command.
- It tells Escouade how to launch the harness, as its manifest says, then translates what it writes. Escouade launches the program, never the plugin.
- It reads, for the agent, in the agent’s folder and its repository, and writes in the agent’s folder, 4 MiB a file at most, when the harness passes its files through Escouade. The host judges each path and refuses those of a fenced project.
- It accepts on its own the writes in the agent’s folder in Accept edits, and everything in Bypass.
- It is given the token of Escouade’s MCP server for each agent it describes, to hand it to the harness’s session, never among the arguments.

Off or uninstalled, the agents of described harnesses stay, marked “harness absent”, with their conversations, and come back with it.

## The tabs

- **Install**: the plugin, a described harness, an account.
- **Use**: the modes, discussions, private projects, and how to describe a harness.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the crate, the tests, running it alone.
