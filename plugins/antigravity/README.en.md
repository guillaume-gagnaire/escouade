# Antigravity CLI

This plugin makes Antigravity CLI, Google’s agent, a harness of Escouade. Your agents run on your machine’s `agy`, with your Google account or a Gemini API key, and their conversation shows in Escouade: replies live, tools with their diffs, subagents, the tokens of each turn, and the permissions and questions asked once the turn is over.

It is for anyone with a Google account for Antigravity, or a Gemini API key.

## What it needs

- Antigravity CLI installed on the machine (tried with 1.4.9).
- An Antigravity CLI account added in Settings › Accounts and harnesses: the machine’s Google sign-in, or a Gemini API key.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no network and no command.
- It tells Escouade how to launch `agy`, then translates what it writes. Escouade launches the program, and launches it again when it must, never the plugin.
- It tells Escouade which permission rules to write for each agent, in the agent’s own folder, by mode. Outside Bypass, they let nothing through on their own: what `agy` refuses comes back to the plugin at the end of the turn, which accepts it, refuses it or makes a card of it.
- It reads, in that folder of the agent’s own, the settings, the end of the conversation’s transcript and the plan, at the write path the model gave.
- It accepts no command alone, `git add` and `git commit` included, until the real trial of `agy` showed how it reads a rule: each acceptance becomes a “start of a text” rule for the turn’s resumption.
- It is given the token of Escouade’s MCP server for each agent it describes, and has it written in that folder, never among the arguments.

Your `~/.gemini` is never read nor written.

## The tabs

- **Install**: Antigravity CLI, the plugin, an account.
- **Use**: the modes, the requests at the end of a turn, the Antigravity CLI view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
