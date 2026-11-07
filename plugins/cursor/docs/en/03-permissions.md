# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network, no access to the disk. Its manifest declares no file or network permission.

Escouade’s core finds `agent` and launches `agent acp`, with your rights: that is what the install window shows. The plugin describes the launch (its arguments, the app’s variables to take away, `MSYSTEM` and `EXEPATH`, the first message), and the core checks that description before it launches anything: Escouade’s token never among the arguments, the fence of a private project held. A description that doesn’t hold is refused, and the agent doesn’t start.

## What it reads

Only what Escouade hands it: the ACP messages Cursor writes. It reads no file: Cursor reads and writes its own (`"fs": false`).

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin hands them to Cursor’s session (`session/new`, the token in a header), on Cursor’s input: never among the arguments. It has no network to send it anywhere.
- **No account key.** An account’s API key is given to Cursor by the core (`CURSOR_API_KEY`). The plugin only knows the account goes by a key, to say the right message when Cursor refuses it.

## What it accepts alone, and what goes to a card

The plugin’s ACP client answers these requests of Cursor alone, and only these:

- **refused at once**: a request that names a fenced path, in every mode;
- **accepted in Accept edits**: a write or a deletion whose paths are all in the agent’s folder, and none in `.git` (whatever its case or its Windows name; a path with `..` that leaves it isn’t in it);
- **accepted in Bypass**: all the rest.

Everything else is a card, with its diff for a write, and “Outside the agent’s folder” when a path leaves it.

| Mode | Without a card |
|---|---|
| Ask | Nothing Cursor asks. What it does without asking (its reads and writes of files) escapes Escouade. |
| Accept edits | Writes and deletions submitted in the agent’s folder, outside `.git`. |
| Plan | Nothing Cursor asks; its plan comes on a card. |
| Bypass | All it asks, but what names a fenced path. |

`git add` and `git commit` aren’t accepted at once: they are commands, and they get their card.

## The fence of a private project

The manifest says Cursor reads and writes by itself (`"fs": false`) and that the fence holds on its requests (`fenceOnPermissions`). Escouade believes the latter only of a plugin of its own hand. The plugin then refuses, without a card, any request that names a fenced path: its title, its arguments, the paths it touches.

If the plugin starts again, Escouade gives it each agent’s fence again: the refusal still holds, and the agent keeps its session, its turn and the requests waiting.

What the fence doesn’t hold: the reads and writes Cursor makes without asking, and a command that reaches a path without naming it.

## What it cannot do

- launch `agent` or any other program;
- read or write a file, open a network connection;
- see an account’s API key or sign-in;
- put Escouade’s token among the arguments: the core refuses the launch;
- stop Cursor from reading or writing a file it doesn’t ask about.
