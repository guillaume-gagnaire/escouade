# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds `agy` and launches it, then launches it again when it must (`agy` takes its model, effort and mode at its launch alone, and has no message that stops a turn). The plugin describes each launch (`-p=`, stream-json both ways, the model, the effort, the mode, the conversation, the agent’s own folder as `USERPROFILE` and `HOME`) and the files to write in that folder. The core checks each description before it launches anything: Escouade’s token never among the arguments, the fence of a private project in the launch, a discussion held read-only, a file to write only in the agent’s folder.

## What it reads

Through Escouade’s host, in the agent’s own folder (`agentHome`, read), and nothing else:

- the rules kept by “For this agent”, `settings.json` and `mcp_config.json`;
- the last 4 MiB of the conversation’s transcript: its events don’t tell everything (a call’s arguments, a write’s text, its reasoning);
- the plan file `agy` names, in that folder.

Your `~/.gemini` is never read nor written. The Google sign-in is in the system keychain, which the plugin never reads.

## What it has written

It writes nothing itself. It describes to the core what to write in the agent’s folder, and the core checks it is there: the mode’s permission rules, those you allowed, and the entry of Escouade’s MCP server with its token.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin has them written in `mcp_config.json`, in the agent’s folder: never among the arguments. The core takes the token out of that folder when the process stops. The plugin has no network to send it anywhere.
- **No account key.** An account’s API key is given to `agy` by the core (`GEMINI_API_KEY`); the plugin only knows the account goes by a key.

## What it accepts alone, and what goes to a card

During a turn nothing goes to a card: `agy` does what its rules allow and refuses the rest. So the rules the plugin has written decide what goes through without a card:

| Mode | What the rules allow |
|---|---|
| Ask | Writing in the agent’s folder (`write_file(<folder>)`). |
| Accept edits | Its `accept-edits` mode, and the commands `git add` and `git commit` (`command(git add)`, `command(git commit)`). |
| Plan | Nothing more: its `plan` mode changes nothing. |
| Bypass | Everything: `command(*)`, `write_file(*)`, `read_file(*)`, `mcp(*)`, `read_url(*)`, `execute_url(*)`. |

In every mode, the folders of the files attached to the agent are allowed for reading, and `agy` reads without asking.

At the end of a turn, each refusal becomes a card, except:

- a call that names a fenced path: refused at once, without a card;
- in a discussion: no refusal is asked;
- what Escouade’s rules already accept in the mode (in Accept edits, a write in the agent’s folder outside `.git`; in Bypass, everything): accepted without a card.

“Allow” allows the call for the turn’s resumption; “For this agent” keeps it in the agent’s folder.

## The fence of a private project

The agent’s rules refuse reading each fenced path (`read_file(<path>)` as `deny`), which refuses writing it too: that is `agy`’s rule, and a refusal comes before everything, even in Bypass. The plugin refuses at once what `agy` wanted to do on those paths.

The fence is in the rules of the agent’s folder: if the plugin starts again, it holds. A card shown before that restart can’t resume: its answer ends the turn.

What the fence doesn’t hold: a command that would read those paths.

## What it cannot do

- launch `agy` or any other program;
- open a network connection;
- read or have written anything outside the agent’s own folder;
- read your `~/.gemini`, your Google sign-in or an account’s key;
- put Escouade’s token among the arguments: the core refuses the launch.
