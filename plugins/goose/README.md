# Goose

`dev.escouade.goose`: [Goose](https://github.com/aaif-goose/goose), the open source agent of Block, as a harness of
[Escouade](https://escouade.dev). A declarative plugin: its manifest describes Goose over ACP (`goose acp`), and the
ACP plugin Escouade gives (`dev.escouade.acp`) launches it, speaks to it and shows its conversation. No code.

Goose comme harness d'Escouade, décrit par ACP, sans code.

## Use it

1. Install Goose (`goose --version` answers 1.57.0 or later) and configure its provider (`goose configure`): Escouade
   reads none of its keys.
2. Install this plugin from its folder (Plugins view › “In development”), or from the store.
3. Pick “Goose” in the “Harness” menu beside “New agent”.

Each account of Goose in Escouade is a configuration folder of its own (`GOOSE_PATH_ROOT`); the machine's account uses
Goose's own configuration.

## What it does, and does not

- Modes: « Demander » is Goose's `approve` (it asks before each tool), « Édits auto » its `smart_approve` (Escouade
  accepts its writes in the agent's folder), « Bypass » its `auto`; a discussion of the Docs tab runs in its `chat`
  mode, which calls no tool.
- Its tools `shell`, `write`, `edit`, `read` are shown as Claude Code's `Bash`, `Write`, `Edit`, `Read`, each write
  with its diff.
- Goose has no tool to ask a question in ACP: it asks in text, which Escouade shows as a question's card.
- Goose writes files by itself, without passing them through Escouade: no agent on Goose starts in a private project.
- No quota nor cost (ACP tells neither); the tokens of each turn are counted.

## What it may do

Its agents run the program `goose acp` of your machine, with your rights. Nothing else: no network, no secret, no
account of its own.

## Test it

`node --test` in this folder checks its manifest. Escouade's own tests play the same client on a fake ACP agent
(`src-tauri/src/plugins_acp_tests.rs`), and its real trial on Goose is `real_goose` there.
