# Permissions

This page says what the plugin has run on your machine, and what it cannot do.

## What it launches

This plugin has no program: nothing of it runs, its manifest is read. It declares no permission: no network, no secret, no file.

What it has run: its agents launch `goose acp`, your machine’s program, with your rights. That is what the install window shows before installing. Escouade finds `goose` in the `PATH` (its manifest gives that name alone), and Escouade’s ACP plugin describes the launch; the core checks it, then launches Goose.

## What is read, and the secrets

The plugin itself reads nothing and gets nothing. The ACP plugin that serves Goose gets each agent’s MCP token, and hands it to Goose’s session in a header, on its input, never among the arguments (see the ACP plugin’s documentation). Goose’s configuration and keys stay its own: Escouade reads none of them.

## What goes through without a card, and what goes to a card

The ACP client that serves Goose answers these requests alone, and only these:

- **refused at once**: in a discussion, a write, a deletion, a move or a command;
- **accepted in Accept edits**: a write or a deletion whose paths are all in the agent’s folder, and none in `.git` (whatever its case or its Windows name; a path with `..` that leaves it isn’t in it);
- **accepted in Bypass**: everything.

Everything else is a card. What Goose does without asking depends on its own mode:

| Mode | Without a card |
|---|---|
| Ask (`approve`) | Nothing: Goose asks before each tool. |
| Accept edits (`smart_approve`) | What Goose doesn’t deem sensitive, and its writes submitted in the agent’s folder, outside `.git`. |
| Bypass (`auto`) | Everything: Goose asks nothing, on the whole disk. |

`git add` and `git commit` aren’t accepted at once: they are commands.

## The fence of a private project

It cannot hold: Goose reads and writes files by itself (`"fs": false`), and its manifest doesn’t say the fence holds on its requests. So Escouade refuses to launch a Goose agent as soon as it has a fence, that is as soon as one of your projects is private, whatever the agent’s project.

## What it cannot do

- run anything itself: it has no program;
- launch anything but what its manifest describes, `goose acp`, which the install window showed you;
- run an agent beside a private project.
