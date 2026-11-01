# A described ACP harness

An agent that speaks **ACP** (Agent Client Protocol) becomes a harness of Escouade through a **declarative** plugin: its manifest describes it, without a line of code, and the ACP plugin shipped with Escouade does the rest.

That plugin (`dev.escouade.acp`) launches the agent, speaks to it and shows its conversation, its permissions, its diffs and the end of its turns, as for Claude Code. The agent shows in the “Harness” menu beside “New agent”.

## The manifest

The Goose plugin (`plugins/goose/` of Escouade’s repository), tried with Goose 1.57:

```json
{
  "id": "dev.escouade.goose",
  "name": "Goose",
  "version": "1.0.0",
  "license": "MIT",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "harnesses": [
      {
        "id": "goose",
        "name": "Goose",
        "locate": { "names": ["goose"] },
        "tested": { "oldest": "1.57.0", "newest": "1.57.0" },
        "caps": {
          "modes": [
            { "id": "default", "meaning": { "fr": "Goose demande avant chaque outil.", "en": "Goose asks before each tool." } },
            "acceptEdits",
            "bypassPermissions"
          ],
          "readOnly": true
        },
        "accountVar": "GOOSE_PATH_ROOT",
        "install": { "unix": "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash" },
        "acp": {
          "args": ["acp"],
          "fs": false,
          "modes": { "default": "approve", "acceptEdits": "smart_approve", "bypassPermissions": "auto" },
          "readOnlyMode": "chat",
          "modelOption": "model",
          "questionsInText": true,
          "tools": { "shell": "Bash", "write": "Write", "edit": "Edit", "read": "Read" }
        }
      }
    ]
  }
}
```

Each entry of `contributes.harnesses` with an `acp` object is a described harness:

- `id`, `name`, `speaker` (the name of its agents in the texts), `writer` (who drafts the texts);
- `locate.names`: the names of its program, looked for on the PATH; a declarative plugin never gives a path;
- `versionArgs` (`--version` by default) and `tested`: the version read and the versions tried, as for Escouade’s harnesses;
- `caps`: its modes, each by its id or with its meaning (`meaning`, what the menu of modes says), and what it can do (`images`, `pdf`, `readOnly`…); what is not said, it does not do;
- `models`: the models of its menu (`id`, `label`);
- `accountVar`: the variable that gives each account its configuration folder;
- `install` and `updateCommand`: its install command (Windows, Unix) and its update command.

## The `acp` object

- `args`: its arguments (`["acp"]`); `env` and `removeEnv`: variables of its own (never a secret: a manifest is read by all), and those of the app it must not get;
- `fs`: it reads and writes files **through Escouade** (`fs/read_text_file`, `fs/write_text_file`), true by default. Set `false` for an agent that reads and writes them itself, as Goose and Cursor CLI do, even when Escouade offers it;
- `modes`: the ACP mode (`session/set_mode`) of each mode of Escouade; `readOnlyMode`: the one that changes nothing, for the discussions of the Docs tab;
- `modelOption`: the configuration option of the model (`session/set_config_option`), else `session/set_model`;
- `questionsInText`: the agent has no tool to ask a question; its first message tells it to end its turn on a block `<escouade_question>`, which Escouade shows as a question’s card;
- `tools`: Escouade’s name (Claude Code’s) of its tools of the kind `other` (`{ "shell": "Bash" }`), read from `_meta.<agent>.toolCall.toolName` or from their title before “·”; `subagentTools`: those that start a subagent;
- `extensions`: `cursor` for the extensions `cursor/*` of Cursor CLI.

## What Escouade holds

- **The launch.** Escouade finds the program, asks the ACP plugin how to launch it, checks the description and launches it itself. Escouade’s MCP server goes to the session (`mcpServers`), its token in a header, on the agent’s input: never among its arguments.
- **The permissions.** Each permission asked is a card, with its diff for a write. “Accept edits” lets the writes in the agent’s folder through, “Bypass” all it asks.
- **Private projects.** Escouade refuses the files of another project the agent reads or writes through it. When the agent reads or writes a file by itself while a fence holds, the conversation says so: the fence does not hold for these. A harness declared `"fs": false` does not start in a private project.
- **What ACP does not tell** stays unknown: the quota, the cost, the accounts. A turn’s tokens are counted when the agent gives them.

A plugin uninstalled or turned off (or the ACP plugin itself): its agents stay, marked “Harness missing”, with their conversations. They come back with it. The id of a described harness is `plugin:<plugin>:<harness>`.

For now, a described harness has no terminal view and drafts no texts.

## Trying it

Install your plugin’s folder from the Plugins view (“In development”), then pick its harness in the “Harness” menu. Before the install, the window shows the program its agents will run (`goose acp`). Escouade’s bench of the harnesses plays the same client on a fake ACP agent described by a manifest (`tests/fixtures/plugins/fake-acp/`).
