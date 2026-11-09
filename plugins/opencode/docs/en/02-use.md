# Use

## Start an agent

Choose “OpenCode” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects › Project, “Default harness”).

A single `opencode serve` serves all the account’s OpenCode agents: each agent is a session on it, in its folder. Escouade starts the server with the first agent and stops it with the last.

The “Model” menu lists the models of the connected providers, with a search, their provider and favorites (★) first. The effort is the model’s variant when it has one. Escouade always gives the model, so that OpenCode doesn’t pick one by itself.

A message sent during a turn waits for its end: OpenCode would lose the first one’s request. Attached images go through; PDFs don’t. Only the `/compact` command is offered.

## The modes

OpenCode has no sandbox: the permission rules the plugin gives the session follow the mode, and an accepted command can do anything.

| Mode | In OpenCode |
|---|---|
| Ask | The files of the agent’s folder change without asking; commands (`git add` and `git commit` included), the web and paths outside the folder ask. |
| Accept edits | The same, and `git add` and `git commit` go through without asking, in their plain forms only. |
| Plan | OpenCode’s `plan` agent studies without changing anything, then puts its plan to you for approval. All it asks is a card. |
| Bypass | OpenCode asks nothing. |

In every mode but Bypass, a change of what git or OpenCode run or take as rules stays a card: `.git`, `.opencode`, `opencode.json`, `.husky`, `.githooks`, at any depth.

An OpenCode card has “Allow”, “Always allow” (the card says the pattern OpenCode will keep for the session, like `git add *`) and “Deny”.

## The OpenCode view

The “OpenCode view” plugs OpenCode’s interface (`opencode attach`) into the agent’s session, in a terminal of the window, even mid-turn. It doesn’t replace the agent’s process: it adds to it, and Escouade keeps the thread. A permission answered in the interface closes in Escouade, and the other way round.

## A discussion of the Docs tab

A discussion changes nothing: its rules refuse every change and every command, whatever the mode. The tools of Escouade’s MCP server stay.

## Private projects

For a private project, the session’s rules refuse the other projects’ paths to the tools that take a path (`read`, `edit`, `list`, and leaving the agent’s folder) and to the commands that name them, whatever the mode. “Always allow” is no longer offered: what OpenCode keeps would come before those refusals. A `git commit` that reads a file (`-F`…) is refused, even in Bypass.

Its limits:

- it is a guard, not a barrier: a command that reaches a path without naming it gets through;
- a command you accepted runs with your rights, and can read the server’s password (see the Permissions tab).
