# Gemini CLI

This plugin makes Gemini CLI, Google’s agent, a harness of Escouade, over ACP. Your agents run on your machine’s `gemini`, with your Google account or a Gemini API key, and their conversation shows in Escouade: replies and reasoning live, tools with their diffs, permission, question and plan cards, the tokens of each turn.

It is for anyone with a Google account or a Gemini API key.

Gemini CLI has not yet been tried with a real account: the plugin follows its documentation and what its program does against a fake server.

## What it needs

- Gemini CLI installed on the machine (`npm install -g @google/gemini-cli`, Node 20 or later; tried with 0.66.0).
- A Gemini CLI account added in Settings › Accounts and harnesses: a Google account signed in by `gemini`, or an API key.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no network and no command.
- It tells Escouade how to launch `gemini --acp`, then translates what Gemini writes. Escouade launches the program, never the plugin.
- It has Gemini trust the agent’s folder (`GEMINI_CLI_TRUST_WORKSPACE`): without it, Gemini doesn’t start Escouade’s MCP server. Gemini then runs the hooks and reads the settings of the repository open in that folder.
- It accepts on its own Gemini’s writes in the agent’s folder (outside `.git`), and everything in Bypass; it refuses at once a request that names a path of another project when one of the two is private.
- It reads, in the agent’s own folder, the plan Gemini wrote.
- It is given the token of Escouade’s MCP server for each agent it describes, to hand it to Gemini’s session, never among the arguments.

## The tabs

- **Install**: Gemini CLI, the plugin, an account.
- **Use**: the modes, the Gemini CLI view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
