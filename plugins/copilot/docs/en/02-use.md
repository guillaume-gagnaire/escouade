# Use

## Start an agent

Choose “Copilot CLI” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects › Project, “Default harness”). Each agent has its own `copilot --server --stdio`, without the GitHub MCP server Copilot plugs in by itself, without access to the system’s temporary folder, and without the GitHub tokens in its commands’ environment.

The model menu writes each model’s multiplier (`×3`), or its relative cost on a Free account; `auto` is never offered. Model and mode change at once, even during a turn.

A message sent during a turn goes into it at once. Attached images go through; PDFs don’t. Only the `/compact` command is offered, followed if you like by what to keep. A turn’s cost comes from the AI credits of its calls. A Copilot agent doesn’t resume on another account.

## The modes

Copilot has no sandbox when Escouade drives it: it asks, and the plugin’s rules answer the safe requests without a card. The rest comes as a card.

| Mode | In Copilot CLI |
|---|---|
| Ask | Reads and writes in the agent’s folder go through (but a write in `.git`); commands, `git add` and `git commit` included, the web, MCP tools and paths outside the folder ask. |
| Accept edits | The same, and `git add` and `git commit` of the worktree go through when the command is nothing more. |
| Plan | Copilot studies and proposes a plan without changing anything: its reads of the folder go through, a write asks. |
| Auto | Copilot goes on by itself to the end of its task (its autopilot mode), with the rules of Accept edits. |
| Bypass | Copilot asks nothing (its `allow-all` mode). |

A Copilot card has “Allow”, “For this session” and “Deny”; no “Always”, which would write a lasting rule in Copilot’s config. A form of an MCP server becomes a card with fields. A session’s ceiling of credits, once reached, becomes a card that offers to add some, lift it or stop.

## The Copilot CLI view

The “Copilot CLI view” runs the agent in Copilot’s interface, in a terminal of the window, on the same session. Its hooks go through a plugin of the agent’s own, given to that interface alone: nothing is written in your Copilot folder. In this view, its requests are answered in Copilot’s interface: the plugin’s rules don’t answer them, and the fence of a private project doesn’t reach them.

## A discussion of the Docs tab

A discussion changes nothing: its reads of the folder go through, and Escouade refuses its writes and commands.

## Private projects

For a private project, the plugin refuses at once, without a card, any request of Copilot that names a path of another project: the path read or written, those its command may touch, the folder it runs in, the command’s text. “For this session” is no longer offered, so that each request goes past that refusal.

Its limits:

- it is a guard, not a barrier: a command that reaches a path without naming it gets through;
- in Bypass, Copilot asks nothing: the fence no longer holds;
- in the Copilot CLI view, its requests don’t go through Escouade;
- Copilot’s sessions are the account’s: so that a private project doesn’t share them, give it an account reserved to it alone.
