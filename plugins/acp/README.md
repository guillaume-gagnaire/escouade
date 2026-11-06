# ACP

`dev.escouade.acp`, a plugin of [Escouade](https://escouade.dev)'s own hand: a WASM module, with a version of its own, that
the app runs in its host.

Le client ACP générique d'Escouade : les harness qui parlent ACP, décrits par un plugin sans code.

It serves the harnesses that other plugins only **describe**: an agent that speaks ACP (the
[Agent Client Protocol](https://agentclientprotocol.com), v1), named in the `contributes.harnesses[]` of a manifest with
an `acp` object, its program and arguments, its modes and their meaning, its models. Such a plugin has no program of
its own: the core finds the agent on the machine (`locate`), asks this plugin how to launch it (`engine/launch`),
checks the launch and starts it; what the agent says comes here and goes back as the conversation (`agent/frames`),
on the contract of the harnesses in process (INT-9). The client is the crate `crates/escouade-acp`, drawn from the one
Cursor CLI is driven with; what is Cursor's own (`cursor/*`) is the extension `cursor` there.

## Describe a harness

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
      "locate": { "names": ["goose"] },
      "tested": { "oldest": "1.13.0", "newest": "1.13.0" },
      "caps": { "modes": [{ "id": "default", "meaning": { "fr": "…", "en": "…" } }, "bypassPermissions"] },
      "accountVar": "GOOSE_PATH_ROOT",
      "acp": { "args": ["acp"], "fs": false },
      "install": { "windows": "…", "unix": "…" }
    }]
  }
}
```

- `acp.args`: its arguments; `acp.env` and `acp.removeEnv`: variables of its own, and those of the app it must not get.
- `acp.fs`: it reads and writes files through Escouade (`fs/read_text_file`, `fs/write_text_file`; true by default).
  The fence of private projects is held there, by the host; false, no agent of a private project starts on it. It reads in
  the agent's folder and its repository, writes in the agent's folder, 4 MiB a file at most; anything else is refused.
- `acp.modes`: the ACP mode (`session/set_mode`) of each mode of Escouade; `acp.readOnlyMode` the one that changes
  nothing (a discussion of the Docs tab needs it); `acp.modelOption`: the configuration option of the model, else
  `session/set_model`.
- `acp.extensions`: `cursor` for Cursor CLI's `cursor/*`.

Its agent gets the id `plugin:<plugin>:<harness>`: uninstalled or turned off, its agents stay, « harness absent », with
their conversations, and come back with it.

## What it may do

- describe the launch of a harness a manifest describes, and translate what it says: the core launches it, never
  this plugin;
- read what the agent asks it to in the agent's folder and its repository, and write in the agent's folder
  (`permissions.files`, the roots `agent` and `agentRepo`). The host judges each path, its links followed, and refuses a
  private project's, whatever the code here says;
- nothing else: no network, no secret, no account, no command to run.

## Run it alone

Its code is this folder's crate, three lines over `escouade_acp::plugin`; `sh scripts/plugins-wasm.sh acp` compiles it. Its
tests are those of `crates/escouade-acp`, `src-tauri/src/plugins_acp_tests.rs` (the bench of the harnesses on a fake ACP agent
described by `tests/fixtures/plugins/fake-acp/escouade-plugin.json`, played on the module) and the parity with the native
plugin, line by line on the recorded streams (`src-tauri/src/plugins_claude_acp_parity_tests.rs`).
