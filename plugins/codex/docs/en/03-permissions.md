# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds `codex` and launches `codex app-server`. The plugin describes the launch (its `-c` options, the token’s variable, the first message), and the core checks that description before it launches anything: Escouade’s token never among the arguments, the fence of a private project in the launch, a discussion held read-only. A description that doesn’t hold is refused, and the agent doesn’t start.

## What it reads

Through Escouade’s host, which judges each path and refuses those of a fenced project:

- **the agent’s repository** (`agentRepo`, read): to know whether its configuration and, for a commit, its hooks are those Codex protects (`fs/gitProtected`). The plugin doesn’t read `.git` itself: the host answers yes or no;
- **the account’s folder** (`agentHome`, read): only `config.toml`, and only when Escouade’s server is refused to the agent, to turn off an entry of yours of the same name.

It also gets what Codex writes and, for the Codex view, the lines of the session’s transcript.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin puts the token in a variable of the launch (`ESCOUADE_MCP_TOKEN`), which Codex reads (`bearer_token_env_var`): never among the arguments. It has no network to send it anywhere.
- **No account sign-in.** It never reads `auth.json`, though it is a file of a folder it may read.

## What it accepts alone, and what goes to a card

The plugin accepts one thing without a card: **`git add` and `git commit` of the agent’s worktree, in Accept edits and Auto**. All of this must hold:

- the request comes from the agent’s process, not from the Codex view;
- the command runs in the agent’s folder;
- it is nothing but `git add` and `git commit`, read as the shell would read it, on the command Codex will run and not on its summary: refused at the least doubt (an operator other than `&&` between two accepted commands, `$`, a backquote, a line break, an option of git before its subcommand);
- the repository’s configuration is protected, and for a commit its hooks are those of its `.git`;
- for a fenced agent, the commit takes nothing from a file (`-F`, `--file`, `--template`…).

These commands run outside Codex’s sandbox, which never writes in `.git`: that is why any other form asks.

Any other request of Codex (a command, a change of files, more permissions) is a card. What Codex does without asking depends on its sandbox:

| Mode | Without a card |
|---|---|
| Plan | Reads, and commands that only read, in its read-only sandbox. |
| Ask | What its sandbox allows: writes in the agent’s folder, its commands without network. |
| Accept edits | The same, and `git add` and `git commit` of the worktree, accepted by the plugin. |
| Auto | The same; its automatic reviewer decides the other requests, and the conversation says so. |
| Bypass | Everything: Codex asks nothing and has no sandbox. |

## The fence of a private project

The plugin declares to Codex, by its options, two permission profiles where each fenced path is refused (`deny`): one for the Plan mode and discussions, the other for the modes that write. Codex’s sandbox applies them to all its commands. In Bypass there is no profile: Codex has no sandbox any more, and the fence no longer holds for its commands.

If the plugin starts again, Escouade gives it each agent’s fence and read-only again: they still hold. Codex’s process goes on with its profiles.

## What it cannot do

- launch `codex` or any other program;
- open a network connection;
- write a file, or read outside the agent’s repository and its account’s folder;
- read a file of a fenced project: the host refuses it, whatever its code says;
- put Escouade’s token among the arguments: the core refuses the launch;
- accept anything but `git add` and `git commit`, as above.
