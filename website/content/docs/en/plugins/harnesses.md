# A described ACP harness

An agent that speaks **ACP** (Agent Client Protocol) becomes a harness of Escouade through a **declarative** plugin: its manifest describes it, without a line of code, and Escouade’s ACP plugin does the rest.

That plugin (`dev.escouade.acp`) launches the agent, speaks to it and shows its conversation, its permissions, its diffs and the end of its turns, as for Claude Code. The agent shows in the “Harness” menu beside “New agent”.

## The manifest

The Goose plugin (`plugins/goose/` of Escouade’s repository), tried with Goose 1.57:

`plugins/goose/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.goose",
  "name": "Goose",
  "version": "1.0.2",
  "description": {
    "fr": "Goose, l'agent open source de Block, comme harness : décrit par ACP, sans code.",
    "en": "Goose, Block's open source agent, as a harness: described over ACP, no code."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/goose",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "harnesses": [
      {
        "id": "goose",
        "name": "Goose",
        "speaker": "Goose",
        "writer": "Goose",
        "locate": { "names": ["goose"] },
        "tested": { "oldest": "1.57.0", "newest": "1.57.0" },
        "caps": {
          "modes": [
            {
              "id": "default",
              "meaning": {
                "fr": "Goose demande avant chaque outil (son mode approve). Il écrit les fichiers lui-même, sans les faire passer par Escouade.",
                "en": "Goose asks before each tool (its approve mode). It writes files by itself, without passing them through Escouade."
              }
            },
            {
              "id": "acceptEdits",
              "meaning": {
                "fr": "Goose ne demande que pour ce qu'il juge sensible (son mode smart_approve) ; Escouade accepte ses écritures dans le dossier de l'agent.",
                "en": "Goose asks only for what it deems sensitive (its smart_approve mode); Escouade accepts its writes in the agent's folder."
              }
            },
            {
              "id": "bypassPermissions",
              "meaning": {
                "fr": "Goose ne demande rien (son mode auto) : commandes, fichiers, tout le disque. Pas de bac à sable.",
                "en": "Goose asks nothing (its auto mode): commands, files, the whole disk. No sandbox."
              }
            }
          ],
          "readOnly": true
        },
        "accountVar": "GOOSE_PATH_ROOT",
        "install": {
          "windows": "Invoke-WebRequest -Uri https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1 -OutFile download_cli.ps1; ./download_cli.ps1",
          "unix": "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash"
        },
        "updateCommand": "goose update",
        "acp": {
          "args": ["acp"],
          "fs": false,
          "modes": { "default": "approve", "acceptEdits": "smart_approve", "bypassPermissions": "auto" },
          "readOnlyMode": "chat",
          "modelOption": "model",
          "questionsInText": true,
          "tools": { "shell": "Bash", "write": "Write", "edit": "Edit", "text_editor": "Edit", "read": "Read", "tree": "Glob" }
        }
      }
    ]
  },
  "store": { "categories": ["harness"], "docs": "docs" }
}
```

Each entry of `contributes.harnesses` with an `acp` object is a described harness:

- `id`, `name`, `speaker` (the name of its agents in the texts), `writer` (who drafts the texts);
- `locate.names`: the names of its program, looked for on the PATH; a declarative plugin never gives a path, nor a shell or an interpreter (`sh`, `bash`, `cmd`, `powershell`, `pwsh`, `python*`, `node`, `ruby`, `perl`, `osascript`, `wscript`…): the manifest is refused (code `interpreter`). That is only a first filter: the harness of a third-party plugin asks for the user’s agreement at its first launch, its whole command said (the program found, `acp.args`, `acp.resume.args`, `versionArgs`, `acp.env`, `acp.removeEnv`), and again when a version of your plugin changes it;
- `versionArgs` (`--version` by default) and `tested`: the version read and the versions tried, as for Escouade’s harnesses. The version of a harness described by anyone but Escouade is read only after a first gesture of the user’s on it (an agent or a terminal started), never at the install nor at a start;
- `caps`: its modes, each by its id or with its meaning (`meaning`, what the menu of modes says), and what it can do (`images`, `pdf`, `readOnly`…); what is not said, it does not do; only Escouade’s modes count (`plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions`): a harness that declares none is held as Bypass, and one that has a single mode runs its agents in it (if that is Bypass, they start only with “Allow Bypass mode”, and the installation says so);
- `models`: the models of its menu (`id`, `label`);
- `accountVar`: the variable that gives each account its configuration folder;
- `install` and `updateCommand`: its install command (Windows, Unix) and its update command.

## The `acp` object

- `args`: its arguments (`["acp"]`); `env` and `removeEnv`: variables of its own (never a secret: a manifest is read by all), and those of the app it must not get. they count as declared (the install window shows them). The families a plugin never sets ([the variables](plugins/capabilities#the-variables-env): `PATH`, `HOME`, `GIT_*`, `NODE_*`, `*_PROXY`, `ESCOUADE_*`…) are refused, here as in `accountVar` and `keyVar`: the manifest does not pass, and the agent doesn’t start, saying why;
- `fs`: it reads and writes files **through Escouade** (`fs/read_text_file`, `fs/write_text_file`), true by default. Set `false` for an agent that reads and writes them itself, as Goose and Cursor CLI do, even when Escouade offers it;
- `modes`: the ACP mode (`session/set_mode`) of each mode of Escouade; `readOnlyMode`: the one that changes nothing, for the discussions of the Docs tab;
- `modelOption`: the configuration option of the model (`session/set_config_option`), else `session/set_model`;
- `questionsInText`: the agent has no tool to ask a question; its first message tells it to end its turn on a block `<escouade_question>`, which Escouade shows as a question’s card;
- `tools`: Escouade’s name (Claude Code’s) of its tools of the kind `other` (`{ "shell": "Bash" }`), read from `_meta.<agent>.toolCall.toolName` or from their title before “·”; `subagentTools`: those that start a subagent;
- `extensions`: `cursor` for the extensions `cursor/*` of Cursor CLI;
- `resume`: how it takes back a session it does not load as it is, since Escouade 2.2.3: `args`, the arguments added to its launch (`["--resume", "{session}"]`, `{session}` the session’s id, which can be no option), and `session`, a JSON pointer to the id it keeps the session under in the answer to `session/new` (`/_meta/<agent>/saved/sessionId`), when it is not the session’s;
- `turnEnd`: what ends a turn it begins by itself (no `session/prompt`), and makes that turn one of the conversation: a list of entries, each of them JSON pointers into the `update` of a `session/update` and the value each must have (`{ "/_meta/<agent>/phase": "terminalQuiescence" }`);
- `efforts`: the value of its option of reasoning for each effort of Escouade (`{ "low": "low", "high": "high" }`), as `modes` for its modes; `effortOption`: the id of that option, else the one of the category `thought_level`. The option is set only when the session offers it (a model that reasons) with that value; declare the efforts of the menu too (the harness’s `efforts`);
- `usage`: where its `session/update`s tell what its context holds (`context`) and the window of its model (`window`), JSON pointers into their `update`. Without it, Escouade reads ACP’s `usage_update` (`used`, `size`).

An option’s value made of parts (`["ollama","qwen3"]`, as a JSON text) is found by its last part or its parts joined by `/`: the model `qwen3` or `ollama/qwen3`. A manifest written before these fields works as it is: a version of Escouade that does not know them ignores them.

## What Escouade holds

- **The launch.** Escouade finds the program, asks the ACP plugin how to launch it, checks the description and launches it itself. Escouade’s MCP server goes to the session (`mcpServers`), its token in a header, on the agent’s input: never among its arguments.
- **The permissions.** Each permission asked is a card, with its diff for a write. “Accept edits” lets the writes in the agent’s folder through, “Bypass” all it asks.
- **Private projects.** Escouade refuses the files of another project the agent reads or writes through it. When the agent reads or writes a file by itself while a fence holds, the conversation says so: the fence does not hold for these. A harness declared `"fs": false` does not start in a private project.
- **What ACP does not tell** stays unknown: the quota, the cost, the accounts. A turn’s tokens are counted when the agent gives them; nothing is estimated.
- **The resume.** After a restart, Escouade loads the agent’s session again (`session/load`) when the harness can (`loadSession`, or `resume`). Otherwise, or when the session is lost, the agent starts again on a new session, the conversation says so, and its next message gives its instructions again (its ticket’s protocol) with a summary of what it did.
- **The turns it begins by itself.** When its manifest says how they end (`turnEnd`), what the agent says with no message asking it is a turn of its own: the agent is at work, its text stays in that turn, and it is back at rest at that end, or at the next message, or when stopped. Without `turnEnd`, nothing changes.

A plugin uninstalled or turned off (or the ACP plugin itself): its agents stay, marked “Harness missing”, with their conversations. They come back with it. The id of a described harness is `plugin:<plugin>:<harness>`.

For now, a described harness has no terminal view and drafts no texts.

A harness that ACP does not describe is written in Rust: see [writing a harness](plugins/rust#writing-a-harness).

## Trying it

Install your plugin’s folder from the Plugins view (“In development”), then pick its harness in the “Harness” menu. Before the install, the window shows the program its agents will run (`goose acp`). Escouade’s bench of the harnesses plays the same client on a fake ACP agent described by a manifest (`tests/fixtures/plugins/fake-acp/`).
