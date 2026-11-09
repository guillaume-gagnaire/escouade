# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no access to the disk.

Escouade’s core finds `opencode` and launches `opencode serve`, once for the account’s agents, in its Job object; it stops it with the last agent. The plugin describes that launch (its config, the password), and the core checks it before it launches anything: Escouade’s token never among the arguments, the fence of a private project held, a discussion held read-only.

## What it reaches

- **OpenCode’s server, and it alone** (`local:shared`): the host opens to the plugin the port the core read in the server’s output, while it runs, and no other. Its requests go by `net/fetch`, its events by `net/stream`, 16 MiB at most an answer: a copy of a session heavier than that is refused.
- It reads no file, and sees neither your OpenCode config nor your providers’ keys.

## The secrets it has

- **The server’s password.** The plugin draws it at random and gives it to the server by a variable (`OPENCODE_SERVER_PASSWORD`); it uses it to talk to it.
- **Each agent’s MCP token.** When its session opens, Escouade gives it the address and the token of its MCP server for that agent. The plugin gives them to OpenCode’s server for the agent’s folder, by a request: never among the arguments. It can reach that server alone.

### What the password protects from

The server listens on `127.0.0.1` alone, behind that password. That keeps out the machine’s other users and a browser’s pages, **not the agents**: OpenCode gives its whole environment to the commands it runs, so any command of an agent can read `OPENCODE_SERVER_PASSWORD`. With it and the port, such a command can change the rules of its session, answer its own cards, and reach the sessions of the account’s other agents on that server, whatever their folder. OpenCode has no setting that keeps the variable from its commands.

So a card and a fence hold what an agent asks through OpenCode’s tools, not what a command you accepted does on purpose.

## What it accepts alone, and what goes to a card

The plugin accepts nothing itself: it gives each session its mode’s rules, before the session opens (refused, no session), and OpenCode applies them. The last rule that matches wins.

| Mode | Without a card |
|---|---|
| Ask | Reading and changing the files of the agent’s folder. |
| Accept edits | The same, and `git add` and `git commit` in their plain forms. |
| Plan | Reading. Its `plan` agent changes only its plan; the rest is a card. |
| Bypass | Everything. |

Commands, the web (`webfetch`, `websearch`) and paths outside the agent’s folder are cards, but in Bypass.

**The plain forms of git**, the only ones accepted in Accept edits: `git add <paths>`, `git add -A`, `-u`, `--all`, `--update`, `git commit -m <message>` and `-am <message>`, with nothing else. A second option, a redirection, or what a shell would expand or unquote (`$`, a backquote, `{`, `^`, `%`…) goes to a card.

**Always a card, but in Bypass**: a change of what git or OpenCode run or take as rules, `.git` (a file in a worktree), `.opencode`, `opencode.json` and `opencode.jsonc`, `.husky`, `.githooks`, at any depth, under Windows’ short names and its streams too. A plugin of `.opencode/plugin` runs at the next start of the folder’s instance, an `opencode.json` gives a subagent its rules, a hook runs at the next `git commit`.

**A discussion**: every change and every command refused by OpenCode itself, never a card; an MCP server’s tools stay.

“Always allow” has OpenCode keep the pattern it proposed, for the rest of the session; the card says which.

## The fence of a private project

After the mode’s rules, the plugin gives the session refusals (`deny`) for each fenced path, in both spellings on Windows: to the tools that take a path (`read`, `edit`, `list`, `external_directory`) and to the commands that name it. For a fenced path inside the worktree, `edit` is refused as OpenCode spells it too, relative. `git commit -F` and the options of `git add` and `git commit` that read a file are refused, in Bypass too.

Under a fence, “Always allow” isn’t offered: what OpenCode keeps comes after the fence’s refusals and would win over them.

The rules of a resumed or copied session are set before it opens; a change of mode waits for them. If the plugin starts again, the server and the turn go on, and Escouade gives the plugin each agent’s fence and read-only again.

What the fence doesn’t hold: a command that reaches a path without naming it, and what an accepted command does with the server’s password.

## What it cannot do

- launch `opencode` or any other program;
- reach another address than the server the core launched;
- read or write a file, read a secret of the keychain;
- put Escouade’s token among the arguments: the core refuses the launch;
- stop an accepted command from reading the server’s password.
