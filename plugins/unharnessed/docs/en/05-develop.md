# Develop

For whoever wants to read or change the plugin. It lives in `plugins/unharnessed/` of Escouade's repository, id `dev.escouade.unharnessed`, MIT license.

## What it is

A declarative plugin: its manifest alone, no program. The entry of `contributes.harnesses` that carries an `acp` object describes Unharnessed; Escouade's ACP plugin (`dev.escouade.acp`) serves it. Its harness id is `unharnessed`.

What the manifest says:

- `locate.names: ["unharnessed"]`, `tested` 0.9.8;
- one mode, `bypassPermissions`, with its meaning in French and in English; no `readOnly`;
- `accountVar: "UNHARNESSED_CODING_AGENT_DIR"`: each account's folder (credentials, settings, sessions);
- neither `install` nor `updateCommand`: nothing official installs it alone;
- `acp`: `args: ["--mode", "acp"]`, `fs: false` (it reads and writes by itself), no `modes` (it has none over ACP), `modelOption: "model"`, `questionsInText: true`, and `tools: { "Python cell": "Python" }` (its cell, of kind `execute`, shown as "Python" with its `rawInput.code`, instead of a `Bash` with no command);
- no `models`: they are each user's own providers', and their ACP values are pairs `["<provider>","<model>"]`.

## What its ACP says (Unharnessed 0.9.8)

Read on the wire in a sandbox, with a local model:

- `initialize`: `loadSession`, `promptCapabilities` (images, context), `mcpCapabilities.http`, `sessionCapabilities` `close` and `list`; no `terminal` asked, no `fs` used.
- `session/new`: an ACP id, and in `_meta.unharnessed.saved` the id and the file of the saved session; a `model` option (values as pairs), `thought_level` for a model that reasons, `fast_mode` for a model that has it. Neither `modes` nor `session/set_mode`, nor `session/set_model`.
- `session/update`: `agent_message_chunk`, `agent_thought_chunk` (a model that reasons), `tool_call` / `tool_call_update` (title "Python cell", kind `execute`, `rawInput.code`, the output as text, `failed` "Tool execution aborted" when stopped), `available_commands_update`, `session_info_update` (in its `_meta`: tokens, window, cost, start and end of the turns, turns of its own `origin: "wake"`); notifications of its own, `_unharnessed/kernel/changed` and `_unharnessed/kernel/restored`.
- Never a `session/request_permission`.
- `session/cancel`: the cell `failed`, the turn `cancelled`.
- `session/load`: refused but for the session the process holds, the one given by `--resume <file|id>` when it started; then the history is replayed and the kernel's variables come back. `session/list` gives the saved ids.
- `mcpServers` of `session/new`: taken, called from the kernel (`mcp.call_tool`), the `Authorization` header passed on.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest |
| `test/manifest.test.mjs` | Its test: `node --test` |
| `CHANGELOG.md`, `LICENSE` | Its versions, its MIT license |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- `node --test` in this folder checks the manifest: a harness described over ACP with no program; one mode, Bypass, said in both languages; neither read-only nor installer; `fs` at `false`; the table of its tool.
- In `src-tauri/src/plugins_acp_tests.rs`, on the fake Unharnessed (`tests/fixtures/fake-unharnessed.mjs`) and the recorded real streams (`tests/fixtures/unharnessed/`): it answers, its cell is a Python tool with its code, a turn stops, its session is not resumed by a new process, neither discussion nor private project, a turn of its own; and `the_gap_…`, the "Ask" mode not refused without Bypass.
- `tests/e2e/unharnessed.spec.ts`: the same in the app, with the resume by `--resume`, a turn of its own, and the refusal without Bypass allowed.

`node plugins/ci.mjs` checks it with the others: its `node --test`, then `escouade-plugin validate --first-party` and the bench.

## Try it

A folder installed "From a folder…" cannot carry a `dev.escouade.*` id: copy the manifest and change its `id` (as the e2e does), or, from `plugins/`, with the app's `escouade-plugin`:

```sh
escouade-plugin validate unharnessed --first-party
escouade-plugin test unharnessed --first-party
```
