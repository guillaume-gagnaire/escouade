# Permissions

This page says what the plugin does on your machine, and what it cannot do.

## What it launches

Nothing. The plugin is a WASM module Escouade runs in its host: it has no process, no command, no network.

Escouade’s core finds `gemini` and launches `gemini --acp`, with your rights: that is what the install window shows. The core also writes, in the agent’s own folder, its settings and, for a private project or a discussion, its rules, then adds `--policy` to the launch. The plugin describes the launch (its arguments, `GEMINI_CLI_TRUST_WORKSPACE=true`, the first message), and the core checks that description before it launches anything: Escouade’s token never among the arguments, the fence of a private project held, a discussion held read-only.

**What `GEMINI_CLI_TRUST_WORKSPACE` costs.** Without it, Gemini starts no MCP server, Escouade’s included, and says nothing. With it, Gemini runs the hooks and reads the settings of the repository open in the agent’s folder, in every project.

## What it reads

- **Gemini’s plan**, through Escouade’s host, in the agent’s own folder (`agentHome`, read): the file its `exit_plan_mode` names, only a Markdown file of a `plans` folder, without `..`. Nothing else of that folder.
- The ACP messages Gemini writes. Gemini reads and writes its files by itself (`"fs": false`).

## The secrets it gets

- **The agent’s MCP token.** With each launch it describes, Escouade gives it the address and the token of its MCP server for that agent. The plugin hands them to Gemini’s session (`session/new`, the token in a header), on Gemini’s input: never among the arguments. It has no network to send it anywhere.
- **No account key nor sign-in.** The core gives them to Gemini (`GEMINI_API_KEY`, `GOOGLE_APPLICATION_CREDENTIALS`).

## What it accepts alone, and what goes to a card

The plugin’s ACP client answers these requests of Gemini alone, and only these:

- **refused at once**: a request that names a fenced path, in every mode; in a discussion, a write, a deletion, a move or a command;
- **accepted in Ask and Accept edits**: a write or a deletion whose paths are all in the agent’s folder, and none in `.git` (whatever its case or its Windows name; a path with `..` that leaves it isn’t in it);
- **accepted in Bypass**: all the rest.

The plan of its `exit_plan_mode` always gets its card. Everything else is a card, with its diff for a write, and “Outside the agent’s folder” when a path leaves it.

| Mode | Without a card |
|---|---|
| Ask | Its reads (Gemini doesn’t ask them); its writes submitted in the agent’s folder, outside `.git`. |
| Accept edits | Its writes (its `autoEdit` mode doesn’t ask them), and the same as above. |
| Plan | Its reads. Its plan comes on a card. |
| Bypass | Everything (its `yolo` mode), but what Escouade’s rules refuse. |

`git add` and `git commit` aren’t accepted at once: they are commands, and they get their card in Ask.

A text of the model can only lower the agent’s mode, never raise it: “[MODE_UPDATE] yolo” in a reply is ignored.

## The fence of a private project

It holds in two places:

- **Gemini’s rules** (`--policy`), written by the core in the agent’s folder: the fenced paths refused to its file tools and to the commands that name them, in every mode, `yolo` included, with “Refused by Escouade: a path of a private project.”;
- **the plugin**, which refuses without a card any request that names a fenced path (`fenceOnPermissions`, which Escouade believes only of a plugin of its own hand).

If the plugin starts again, Escouade gives it each agent’s fence and read-only again: they still hold, and the agent keeps its session, its turn and the requests waiting.

What the fence doesn’t hold: a script that opens a file by itself, without the command naming it.

## What it cannot do

- launch `gemini` or any other program;
- open a network connection, write a file;
- read anything in the agent’s folder but a Markdown plan of a `plans` folder;
- see an account’s key or sign-in;
- put Escouade’s token among the arguments: the core refuses the launch;
- let a text of the model raise the agent’s mode.
