# Use

## An agent on a described harness

Pick the harness in the “Harness” menu of “New agent”. Escouade finds its program in the `PATH`, asks the ACP plugin how to launch it, checks the description and launches it. The plugin opens the session (`session/load` of the one to resume when the agent can, else `session/new`), sets the mode and the model, then sends your messages. A message sent during a turn waits for its end: a second `session/prompt` would cancel it.

The id of a harness a third-party plugin describes is `plugin:<plugin>:<harness>`; a plugin of Escouade’s own hand, like Goose, keeps its own. For now, a described harness has no terminal view and no written texts. A turn’s tokens are counted when the agent gives them; quota and cost stay unknown.

## The modes

The modes are those the manifest declares, with their meaning as it writes it; each has its ACP mode (`session/set_mode`). What the agent asks becomes a card, but for what Escouade’s rules accept:

| Mode | What Escouade accepts without a card |
|---|---|
| Ask | Nothing. |
| Accept edits | A write or a deletion whose paths are all in the agent’s folder, outside `.git`. |
| Plan | Nothing. |
| Bypass | All it asks. |

What the agent does without asking is up to its own mode: read the meaning the manifest gives each mode.

A card has “Allow”, “For this agent” when the agent offers to keep the rule, and “Deny”, with its diff for a write.

## A discussion of the Docs tab

A discussion runs in the ACP mode that changes nothing (`readOnlyMode`), and Escouade also refuses any write, deletion, move and command the agent would ask. A harness whose manifest doesn’t name that mode holds no discussion.

## Private projects

It all depends on how the harness reads and writes files:

- **Through Escouade** (`"fs": true`, the default): it reads and writes its files through the client (`fs/read_text_file`, `fs/write_text_file`), and the host refuses those of a fenced project, whatever the plugin’s code says. If it reads or changes a file by itself while a fence holds, the conversation says so, once: the fence doesn’t hold for those accesses.
- **By itself** (`"fs": false`, like Goose): the fence cannot hold. As soon as one of your projects is private, each agent has a fence, and no agent of that harness starts, whatever its project.

A plugin of Escouade’s own hand may also say the fence holds on the agent’s requests (`fenceOnPermissions`): a request that names a fenced path is then refused without a card. Escouade believes that of its own hand alone.

## Describe a harness

An author describes an ACP harness in their plugin’s manifest, without code. Each entry of `contributes.harnesses` that has an `acp` object is a described harness.

```json
{
  "id": "dev.example.goose",
  "name": "Goose",
  "version": "1.0.0",
  "escouade": { "api": "1", "min": "2.2.0" },
  "contributes": {
    "harnesses": [{
      "id": "goose",
      "name": "Goose",
      "speaker": "Goose",
      "locate": { "names": ["goose"] },
      "tested": { "oldest": "1.57.0", "newest": "1.57.0" },
      "caps": {
        "modes": [
          { "id": "default", "meaning": { "fr": "…", "en": "…" } },
          "bypassPermissions"
        ],
        "readOnly": true
      },
      "accountVar": "GOOSE_PATH_ROOT",
      "install": { "windows": "…", "unix": "…" },
      "acp": {
        "args": ["acp"],
        "fs": false,
        "modes": { "default": "approve", "bypassPermissions": "auto" },
        "readOnlyMode": "chat"
      }
    }]
  }
}
```

The harness’s entry:

- `id`, `name`, `speaker` (its agents’ name in texts), `writer`;
- `locate.names`: its program’s names, looked for in the `PATH`. From a third-party plugin, Escouade takes neither a program path nor a name holding `/`, `\` or `:`;
- `versionArgs` (by default `--version`) and `tested`: the version read and the versions tried;
- `caps`: its modes, each by its id or with its meaning (`meaning`, what the modes menu says), and what it can do (`images`, `readOnly`…); what isn’t said, it doesn’t do;
- `models`: the models of its menu (`id`, `label`);
- `accountVar`: the variable that gives each account its configuration folder;
- `install` and `updateCommand`.

The `acp` object:

- `args`: its arguments; `env`: variables of its own, never a secret (a manifest is read by all); `removeEnv`: the app’s variables it must not get. The variables a plugin may not set (`PATH`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `ESCOUADE_*`…) are refused to a third-party plugin: the agent doesn’t start, and says why;
- `fs`: it reads and writes files through Escouade, true by default. Set `false` for an agent that reads and writes them by itself, even when Escouade offers it the client: it will then start beside no private project;
- `modes`: the ACP mode of each mode of Escouade; `readOnlyMode`: the one that changes nothing;
- `modelOption`: the configuration option of the model (`session/set_config_option`), else `session/set_model`;
- `questionsInText`: the agent has no tool to ask a question; its first message tells it to end its turn on an `<escouade_question>` block, which Escouade shows as a card;
- `tools`: Escouade’s name (Claude Code’s) for its tools of kind `other` (`{ "shell": "Bash" }`); `subagentTools`: those that start a subagent;
- `extensions`: `cursor` and `gemini`, for the extensions of Cursor CLI and Gemini CLI.

To try it, install the plugin’s folder from the Plugins view (“From a folder…”), then pick its harness. `escouade-plugin validate <folder>` reads the manifest as the app does.
