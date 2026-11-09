# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds a described harness’s program and launches it, with your rights: the install window of the plugin that describes it shows that program and its arguments. The ACP plugin describes the launch (the arguments, the variables, the first message, `initialize`), and the core checks that description before it launches anything: Escouade’s token never among the arguments, the fence of a private project held, a discussion held read-only, and for a third-party plugin no forbidden variable.

## What it reads and writes

Only for a harness that passes its files through Escouade (`"fs": true`), and only what the agent asks of it (`fs/read_text_file`, `fs/write_text_file`), through Escouade’s host:

- **read** in the agent’s folder (`agent`) and in its repository (`agentRepo`);
- **write** in the agent’s folder (`agent`), but in a discussion;
- 4 MiB a file at most; a path that isn’t absolute, outside those folders or in a fenced project is refused. The host judges each path, its links followed, whatever the plugin’s code says. Each write and each refusal goes to the plugin’s journal.

For a harness that reads and writes by itself (`"fs": false`), the plugin reads and writes nothing. It keeps nothing on the disk.

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin hands them to the session (`session/new`, in `mcpServers`, the token in a header), on the agent’s input: never among its arguments. It has no network to send it anywhere.
- **No account secret.** It has no account of its own; a harness’s keys stay in its configuration.

## What it accepts alone, and what goes to a card

The client answers these requests of the agent (`session/request_permission`) alone, and only these:

- **refused at once**: in a discussion, a write, a deletion, a move or a command; for a harness of Escouade’s own hand that declares `fenceOnPermissions`, a request that names a fenced path;
- **accepted in Accept edits**: a write or a deletion whose paths are all in the agent’s folder, and none in `.git` (whatever its case, a worktree’s `.git` file, or its Windows names `.git.`, `.git::$INDEX_ALLOCATION`, `git~1`; a path with `..` that leaves it isn’t in it);
- **accepted in Bypass**: everything.

Everything else is a card, with its diff for a write, “Outside the agent’s folder” when a path leaves it, the server and the tool for an MCP tool.

| Mode | Without a card |
|---|---|
| Ask | Nothing the agent asks. |
| Accept edits | Writes and deletions in the agent’s folder, outside `.git`. |
| Plan | Nothing the agent asks. |
| Bypass | All it asks. |
| another declared mode | Nothing the agent asks. |

What the agent does without asking depends on its own mode: that is what each mode’s meaning, in its manifest, must say. `git add` and `git commit` are never accepted at once: they are commands.

It also answers alone, without a card, what Escouade can’t do: an unknown request of the agent is refused (`unsupported`).

## The fence of a private project

- **`"fs": true`**: the fence holds on the files the agent reads and writes through the client, refused by the host with “Refused by Escouade: {path} is in a private project, out of this agent's reach.”. What it reads or changes by itself, the conversation says, once: the fence doesn’t hold for those accesses.
- **`"fs": false`**: without `fenceOnPermissions`, the agent doesn’t start as soon as it has a fence, that is as soon as one of your projects is private. `fenceOnPermissions` is believed only of a plugin of Escouade’s own hand: it then refuses any request that names a fenced path, without holding what the agent does without asking.

If the plugin starts again, Escouade gives it each agent’s fence and read-only again: they still hold, and the agent keeps its session, its turn and the requests waiting.

## What Escouade doesn’t take from a third-party plugin

For a harness described by a plugin that isn’t of its own hand, Escouade takes out of the manifest the program’s path (`locate.program`), the names that aren’t a bare name, and `fenceOnPermissions`; it refuses the variables a plugin may not set.

## What it cannot do

- launch a program: the core launches the agent;
- open a network connection;
- read outside the agent’s folder and its repository, write outside the agent’s folder, touch a file of a fenced project;
- put Escouade’s token among the arguments: the core refuses the launch;
- accept a command on your behalf, but in Bypass.
