# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds `agy` and launches it, then launches it again when it must (`agy` takes its model, effort and mode at its launch alone, and has no message that stops a turn). The plugin describes each launch (`-p=`, stream-json both ways, the model, the effort, the mode, the conversation, the agent’s own folder as `USERPROFILE` and `HOME`, and its `tmp` subfolder as `TEMP`, `TMP` and `TMPDIR`) and the files to write in that folder. `agy` 1.5.3 writes in its temporary folder with no rule nor question: so it is the agent’s, never your `%TEMP%`, where a write is refused and comes back to the plugin. The core checks each description before it launches anything: Escouade’s token never among the arguments, the fence of a private project in the launch, a discussion held read-only, a file to write only in the agent’s folder.

## What it reads

Through Escouade’s host, in the agent’s own folder (`agentHome`, read), and nothing else:

- `settings.json` and `mcp_config.json`, to keep what `agy` put there itself (its interface’s trust): their rules, `agentMode` and any MCP server but Escouade’s are replaced, not taken over, and `toolPermission` is always `request-review` there (what no rule allows needs a review, refused in `-p`);
- the last 4 MiB of the conversation’s transcript: its events don’t tell everything (a call’s arguments, a write’s text, its reasoning);
- the plan file, at the write path the model gave, only when it is in that folder: it is read by the root `agentHome`, never in the repository.

It asks nothing of the agent’s worktree any more: `agentRepo` is no longer in its permissions (2.2.2), as it no longer accepts a `git add` or a `git commit` alone. For a write it would accept alone, it asks the core (`fs/fileOnly`, 2.2.4) whether it reaches a file alone: a file that is there, or a path not there with no submodule of git’s index under it. The core gives it a yes or a no alone, and only for a path of the agent’s folder.

Your `~/.gemini` is never read nor written. The Google sign-in is in the system keychain, which the plugin never reads.

## What it has written

It writes nothing itself. It describes to the core what to write in the agent’s folder, and the core checks it is there: the mode’s permission rules, those you allowed for a turn’s resumption, and Escouade’s MCP server alone with its token. “For this agent” is written nowhere: the plugin keeps it while it runs.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin has them written in `mcp_config.json`, in the agent’s folder: never among the arguments. The core takes the token out of that folder when the process stops. The plugin has no network to send it anywhere.
- **No account key.** An account’s API key is given to `agy` by the core (`GEMINI_API_KEY`); the plugin only knows the account goes by a key.

## What it accepts alone, and what goes to a card

During a turn nothing goes to a card: `agy` does what its rules allow and refuses the rest. The real trial of `agy` 1.5.3 (2.2.3, run again for 2.2.4) measured how it judges its rules:

- a `command(…)` rule holds the whole command, not its start: `;`, `&&`, `&`, `|`, a redirection, `$( )`, a line break or more words after the allowed command are refused;
- a `write_file(<path>)` rule covers what is under that path, part by part: `<folder>2` is not covered, nor is `<file>\..\.git\x`, but `<folder>\.git` is, whether the folder exists or not;
- a refusal (`deny`) wins over an allowed rule, but only for the name it says: `<folder>\GIT~1` (short name) or `<folder>\.git.` (final dot) get past a refusal of `<folder>\.git`;
- `*` alone is anything; a `*` inside a text, brackets and braces are no jokers.

The plugin judges each refusal itself, and accepts by its rules only what the call’s rule holds without opening more than the call judged. It stays careful with commands: none is accepted by its rules, even though the trial shows a rule holds the whole command. Outside Bypass, it has no `allow` rule of its own written, and `agy` reads without asking (its `plan` mode changes nothing). `agy` 1.5.3 sometimes tells done (`DONE`, no error) a call it refused: the plugin finds it by the refusals the end of the turn lists (`denied_actions`) and by the transcript, which never tells a refused call done; its row says it was refused, and it has its card as the others do. A refusal listed that no call matches has no card, but the conversation says a call was refused:

| Mode | What the rules allow |
|---|---|
| Ask, Accept edits, Plan | Nothing. |
| Bypass | Everything: `command(*)`, `write_file(*)`, `read_file(*)`, `mcp(*)`, `read_url(*)`, `execute_url(*)`. Nothing for a private project. |

At the end of a turn, the plugin judges each refusal:

- refused at once, without a card: a call that names a fenced path, by its text or by its path resolved against the agent’s folder (`..` included), a word of its command included (`type ..\prive\secret.txt`); everything, in a discussion;
- accepted without a card, when the path it names is already absolute and normal (not relative, no `.`, no `..`, no final separator, no mixed separators): in Bypass, everything but a command; a read in a folder of the attached files; in Ask and Accept edits, the write strictly in the agent’s folder of a file that is there, or of a path not there with no submodule under it (what Escouade’s core says, without the plugin reading the worktree), except in `.git` (any case, short name, a worktree’s `.git` file), `.husky`, `.githooks`, `.agents` and `.gemini`, but the folder itself (`C:\wt`, `C:\wt\`, `C:\wt\.`, `.`, an empty string) and but a name those folders start with (`.g`); exactly a call allowed “For this agent”;
- a card for the rest, and for every command: `git add` and `git commit` in Accept edits, each command in Bypass with a private project. The write of a folder (`C:\wt\sub`), of a submodule even never initialized, of what holds one, or of a link goes to a card: its rule would cover the `.git` under it, at any depth (2.2.4). A core that cannot tell (before 2.2) has each write go to a card. “For this agent” is offered neither to a private project’s agent, nor for a write in the folders above, of the folder itself or of a folder, nor for a path that isn’t absolute and normal.

What is accepted, by its rules or on the card, is so by the call’s rule, for the turn’s resumption only: the card says that rule holds for the next turn, and the conversation says so of what the rules accepted. A “For this agent” replayed gives that rule again each time. During a resumption that carries a write, the plugin also has refused (`deny`) the write in each kept folder at the root of the agent’s folder, and in each of those names right under each path allowed (`<path>\.git`, `<path>\.husky`…, 2.2.4), but the one you allowed a write in: `agy` 1.5.3 has a refusal win over the rule allowed. Those refusals don’t hold against a short name or a final dot: that is why nothing is accepted by the rules that could have anything under it, and why the card of a folder says it covers what is under it. It refuses no `<allowed path>\..` any more: `agy` 1.5.3 reads it as the path’s folder, which would refuse the very write you allowed, and it refuses `<file>\..\.git\x` without it (2.2.3). `agy` is launched again before the resumption, the rule is taken back at the end of the turn, and `agy` launched again before the next message. A call refused again during the resumption that allowed it is not asked again. A command, a path or a tool with `*`, `?`, a parenthesis, a bracket, a brace or a line break never becomes a rule: its refusal holds.

A change of mode, a “Stop” and a restart of the plugin have the rules written again, without what had been allowed, and `agy` launched again before the next message.

## The fence of a private project

The agent’s rules refuse reading each fenced path (`read_file(<path>)` as `deny`), which refuses writing it too: that is `agy`’s rule, and a refusal comes before everything, even in Bypass. The plugin refuses at once what `agy` wanted to do on those paths.

The fence is in the rules of the agent’s folder, written again when the plugin starts again: it holds. A card shown before that restart can’t resume: its answer ends the turn.

What the fence doesn’t hold at once: a command that would read those paths without naming them or by a path the shell makes (`$HOME`, `%CD%`), a short name or a junction to them. Such a command goes to a card, as every command of a fenced agent.

The “Antigravity CLI view” has the same rules: no `allow` rule but in Bypass without a private project nor a discussion, the refusals of the fence, and Accept edits is `agy`’s default mode there. What no rule allows, its interface asks. The `escouade-always.json` file of 2.2.0 is read nowhere any more, and the core erases it.

## What it cannot do

- launch `agy` or any other program;
- open a network connection;
- read or have written anything outside the agent’s own folder;
- read your `~/.gemini`, your Google sign-in or an account’s key;
- put Escouade’s token among the arguments: the core refuses the launch.
