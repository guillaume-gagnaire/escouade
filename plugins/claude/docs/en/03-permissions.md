# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network and no access to the disk. Its manifest declares no permission, and the host grants a module only what its manifest declares.

Escouade’s core finds `claude` on the machine and launches it. The plugin only describes the launch (the arguments, the first message), and the core checks that description before it launches anything:

- Escouade’s token is never among the arguments, which anyone on the machine can read;
- for a private project, the fence is in the launch;
- for a discussion, the harness is held read-only;
- a file to write can only be in the agent’s own folder.

A description that doesn’t hold is refused, and the agent doesn’t start.

## What it reads

Only what Escouade hands it: the lines `claude` writes, and for the Claude Code view the lines of the session’s transcript. It reads no file itself.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin puts only the path of the private file holding them among the arguments (`--mcp-config`): `claude` reads the file. The plugin has no network to send that token anywhere.
- **No account sign-in.** The Claude sign-in (`.credentials.json`, or the keychain on macOS) is read by the core alone. The plugin sees none of it; it only knows the words of a turn that lost it.

## What it accepts alone, and what goes to a card

The plugin accepts no permission. Each request of `claude` (`can_use_tool`) becomes a card with its buttons, which you answer in Escouade, or which an agent of Escouade may answer through the MCP server.

It answers two things alone, where Escouade has no part: a form request of an MCP server (`elicitation`) is declined, a request it doesn’t know is refused.

The core accepts one thing without a card: reading a file you attached by its path, that file alone.

Everything else is Claude Code’s. What it doesn’t ask, it does without a card:

| Mode | Without a card |
|---|---|
| Ask | Reads in the agent’s folder and in the folders given by `--add-dir` (attached files, the ticket’s files), and what your Claude Code rules allow. |
| Accept edits | The same, and writes in the agent’s folder. |
| Plan | Reads: Claude Code changes nothing before you approve the plan. |
| Auto | What Claude Code’s classifier deems safe. |
| Bypass | Everything, but what its deny rules forbid. |

## The fence of a private project

For a private project, the core writes a settings file of the agent’s own, with the deny rules for the other projects’ paths and, where Claude Code’s sandbox runs, the same paths for that sandbox. The plugin must name that file in the launch (`--settings`); without it, the core says the plugin doesn’t apply the fence and doesn’t start the agent.

The fence is in the launch of `claude`, not in the plugin: if the plugin starts again, `claude` goes on with the same rules. When you change “Private project”, the agent takes its new rules before its next turn: its process starts again, its conversation stays.

What the fence doesn’t hold: a script that opens a file by itself, where Claude Code’s sandbox doesn’t run (Windows without WSL) or isn’t on.

## What it cannot do

- launch `claude` or any other program;
- read or write a file, open a network connection;
- read an account’s sign-in or a secret of the keychain;
- put Escouade’s token among the arguments, or have a file written outside the agent’s folder: the core refuses the launch;
- accept a permission on your behalf.
