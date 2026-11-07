# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds `agy` and launches it, then launches it again when it must (`agy` takes its model, effort and mode at its launch alone, and has no message that stops a turn). The plugin describes each launch (`-p=`, stream-json both ways, the model, the effort, the mode, the conversation, the agent’s own folder as `USERPROFILE` and `HOME`) and the files to write in that folder. The core checks each description before it launches anything: Escouade’s token never among the arguments, the fence of a private project in the launch, a discussion held read-only, a file to write only in the agent’s folder.

## What it reads

Through Escouade’s host, in the agent’s own folder (`agentHome`, read), and nothing else:

- `settings.json` and `mcp_config.json`, to keep what `agy` put there itself (its interface’s trust): their rules, `toolPermission`, `agentMode` and any MCP server but Escouade’s are replaced, not taken over;
- the last 4 MiB of the conversation’s transcript: its events don’t tell everything (a call’s arguments, a write’s text, its reasoning);
- the plan file, at the write path the model gave, in that folder.

It also asks the host whether the git of the agent’s worktree is protected (`agentRepo`, read: `fs/gitProtected`), before it accepts a `git add` or a `git commit` alone. It reads no file there.

Your `~/.gemini` is never read nor written. The Google sign-in is in the system keychain, which the plugin never reads.

## What it has written

It writes nothing itself. It describes to the core what to write in the agent’s folder, and the core checks it is there: the mode’s permission rules, those you allowed for a turn’s resumption, and Escouade’s MCP server alone with its token. “For this agent” is written nowhere: the plugin keeps it while it runs.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin has them written in `mcp_config.json`, in the agent’s folder: never among the arguments. The core takes the token out of that folder when the process stops. The plugin has no network to send it anywhere.
- **No account key.** An account’s API key is given to `agy` by the core (`GEMINI_API_KEY`); the plugin only knows the account goes by a key.

## What it accepts alone, and what goes to a card

During a turn nothing goes to a card: `agy` does what its rules allow and refuses the rest. `agy` reads a rule as the start of a text (“commands that start with”), and how it judges them was never tried: so the plugin counts on no rule to guard anything. Outside Bypass, it has none of its own written, and `agy` reads without asking (its `plan` mode changes nothing):

| Mode | What the rules allow |
|---|---|
| Ask, Accept edits, Plan | Nothing. |
| Bypass | Everything: `command(*)`, `write_file(*)`, `read_file(*)`, `mcp(*)`, `read_url(*)`, `execute_url(*)`. Nothing for a private project. |

At the end of a turn, the plugin judges each refusal:

- refused at once, without a card: a call that names a fenced path, by its text or by its resolved path (`..` included); everything, in a discussion;
- accepted without a card: in Bypass, everything; a read in a folder of the attached files; in Ask and Accept edits, a write in the agent’s folder (resolved path), except in `.git` (any case, short name, a worktree’s `.git` file), `.husky`, `.githooks`, `.agents` and `.gemini`; in Accept edits, a command that is nothing but the worktree’s `git add` and `git commit`, read as Codex and Copilot read it (refused at the least doubt: `;`, `|`, `$`, a redirection, a line break, an option before the subcommand), when the host says its git protected; exactly a call allowed “For this agent”;
- a card for the rest. “For this agent” is offered neither to a private project’s agent, nor for a write in the folders above.

What is accepted is so by the call’s exact rule, for the turn’s resumption only: `agy` is launched again before the resumption, the rule is taken back at the end of the turn, and `agy` launched again before the next message. A call refused again during the resumption that allowed it is not asked again. A command, a path or a tool with `*`, `?`, a parenthesis or a line break never becomes a rule: its refusal holds.

A change of mode, a “Stop” and a restart of the plugin have the rules written again, without what had been allowed, and `agy` launched again before the next message.

## The fence of a private project

The agent’s rules refuse reading each fenced path (`read_file(<path>)` as `deny`), which refuses writing it too: that is `agy`’s rule, and a refusal comes before everything, even in Bypass. The plugin refuses at once what `agy` wanted to do on those paths.

The fence is in the rules of the agent’s folder, written again when the plugin starts again: it holds. A card shown before that restart can’t resume: its answer ends the turn.

What the fence doesn’t hold: a command that would read those paths without naming them, a short name or a junction to them.

## What it cannot do

- launch `agy` or any other program;
- open a network connection;
- read or have written anything outside the agent’s own folder (of its worktree, it knows only what the host says of its git);
- read your `~/.gemini`, your Google sign-in or an account’s key;
- put Escouade’s token among the arguments: the core refuses the launch.
