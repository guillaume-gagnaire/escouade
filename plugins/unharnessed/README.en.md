# Unharnessed

This plugin makes Unharnessed, Rayane Rachid’s coding agent with one tool (a Python REPL where the model writes all the rest), a harness of Escouade. It holds no code: its manifest describes Unharnessed over ACP (`unharnessed --mode acp`), and Escouade’s ACP plugin (`dev.escouade.acp`) launches it, speaks to it and shows its conversation: replies live, each Python cell with its code and its output, question cards.

**Unharnessed never asks anything, and its kernel is not a sandbox**: its code and its commands run with your rights, on the whole disk. Its one mode in Escouade is Bypass. Run it in a disposable worktree, a container or a virtual machine (Permissions tab).

## What it needs

- Unharnessed on the machine (`unharnessed --version` answers 0.9.8 or later) and `uv` in the `PATH`, with a model provider set up. It has no official installer: the VS Code extension (Apple Silicon macOS only) or a build from source (Install tab).
- The ACP plugin (`dev.escouade.acp`) installed and on.
- “Allow Bypass mode” on in Settings › Agents.
- An Unharnessed account added in Settings › Accounts and harnesses.

## What it can do

- Nothing of it runs: it is a plugin without a program.
- Its agents run your machine’s `unharnessed --mode acp`, with your rights.
- It asks for no network, no secret, no file. Escouade reads none of Unharnessed’s credentials.

No discussion of the Docs tab, and no Unharnessed agent as soon as one of your projects is private.

## The tabs

- **Install**: Unharnessed, its provider, the plugin, an account.
- **Use**: its Python cell, the model, the turns, the restart, Escouade’s MCP server.
- **Permissions**: what it does without asking, and why it needs Escouade 2.2.2.
- **Troubleshooting**: the messages and what to do.
- **Develop**: the manifest, its ACP, its tests.
