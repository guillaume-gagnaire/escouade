# Codex

This plugin makes Codex, OpenAI’s agent, a harness of Escouade. Your agents run on your machine’s `codex`, with your ChatGPT account, and their conversation shows in Escouade: replies and reasoning live, commands and diffs, permission, question and plan cards, the estimated cost of each turn and the account’s quota.

It is for anyone who already has Codex and a ChatGPT account.

## What it needs

- Codex installed on the machine (`npm install -g @openai/codex`, tried with 0.163.0).
- A Codex account added in Settings › Accounts and harnesses: with no account, no agent starts.

## What it can do

- It runs inside Escouade, isolated: a WASM module, with no network and no command.
- It tells Escouade how to launch `codex app-server`, then translates what Codex writes. Escouade launches the program, never the plugin.
- It accepts on its own `git add` and `git commit` of the agent’s worktree, in Accept edits and Auto, when the command is nothing more.
- Through Escouade’s host, it reads whether the configuration and the hooks of the agent’s repository are protected, and the account’s `config.toml` when Escouade’s server is refused to an agent.
- It is given the token of Escouade’s MCP server for each agent it describes, and hands it to Codex in a variable, never among the arguments.

## The tabs

- **Install**: Codex, the plugin, an account.
- **Use**: the modes, the Codex view, private projects.
- **Permissions**: what the plugin reads, gets and decides, mode by mode.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the folder, the tests, running it alone.
