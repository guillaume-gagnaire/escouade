# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds `copilot` and launches `copilot --server --stdio`. The plugin describes the launch (the options, the variables to set and take away, the first message), and the core checks that description before it launches anything: Escouade’s token never among the arguments, the fence of a private project held, a discussion held read-only. A description that doesn’t hold is refused, and the agent doesn’t start.

## What it reads

- **The agent’s repository** (`agentRepo`, read), through Escouade’s host: to know whether its configuration and, for a commit, its hooks are protected (`fs/gitProtected`). The plugin doesn’t read `.git` itself: the host answers yes or no, and refuses a path of a fenced project.
- What Copilot writes and, for the Copilot CLI view, the lines of the session’s transcript.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin puts the token in a variable of the launch (`ESCOUADE_MCP_TOKEN`), which Escouade’s entry names in its header; the entry itself is a private file of the agent’s (`--additional-mcp-config`). The token is never among the arguments, and the plugin has no network to send it anywhere.
- **No account sign-in.** The GitHub sign-in stays in the system keychain. The plugin never calls what gives a token in clear (`account.getAllUsers`, `account.getCurrentAuth`): `auth.getStatus` only tells it how Copilot is signed in.

## What it accepts alone, and what goes to a card

The plugin answers without a card, for the agent’s process (not for the Copilot CLI view), these requests alone:

- **a read** whose paths are all in the agent’s folder, in every mode, discussions included;
- **a write** whose paths are all in the agent’s folder and none in `.git` (whatever its case or its Windows name), but in Plan and in a discussion;
- **`git add` and `git commit`**, in Accept edits and Auto, when the command runs in the agent’s worktree, writes no file by a redirection, and is nothing more, read as its shell (PowerShell or a POSIX shell) reads it: `;`, `|`, a lone `&`, a redirection, `$`, a backquote or a line break go to a card. The repository’s configuration must be protected, and for a commit its hooks those of its `.git`; for a fenced agent, a commit that reads a file (`-F`, `--file`) goes to a card.

Each path is taken as asked and as Copilot resolved it, links followed. What Copilot’s policy keeps for a human (`managedApprovalRequired`) and a way out of its sandbox (`requestSandboxBypass`) always get their card.

Everything else is a card: the other commands, the web, MCP tools, paths outside the folder.

| Mode | Without a card |
|---|---|
| Ask | Reads and writes in the agent’s folder, but `.git`. |
| Accept edits | The same, and `git add` and `git commit` of the worktree. |
| Plan | Reads in the agent’s folder. |
| Auto | As Accept edits; Copilot goes through the steps of its task by itself. |
| Bypass | Everything: Copilot asks nothing (`allow-all`). |

In a discussion, only reads in the folder go through; Escouade refuses writes and commands.

## The fence of a private project

The plugin refuses at once, without a card, a request that names a fenced path, with these words for Copilot: “Refused by Escouade: this path is another project’s, and one of the two is private.” It looks at the path read or written, those the command may touch (`possiblePaths`, `resolvedPaths`), the folder it runs in and its text. Under a fence, it doesn’t offer “For this session”: a session rule would let the next requests through without its seeing them.

If the plugin starts again, Escouade gives it each agent’s fence and read-only again: the refusal still holds.

What the fence doesn’t hold: a command that reaches a path without naming it, the Bypass mode, where Copilot asks nothing, and the Copilot CLI view, where its requests don’t go through Escouade.

## What it cannot do

- launch `copilot` or any other program;
- open a network connection, write a file, read outside the agent’s repository;
- read an account’s GitHub sign-in;
- put Escouade’s token among the arguments: the core refuses the launch;
- accept anything beyond what is said above.
