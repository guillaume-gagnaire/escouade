# Develop

For whoever wants to read or change the plugin. It lives in `plugins/prime-agent/` of Escouade’s repository, id `dev.escouade.prime-agent`, MIT license.

## What it is

A declarative plugin: its manifest alone, without a program. The entry of `contributes.harnesses` that carries an `acp` object describes Prime Agent; Escouade’s ACP plugin (`dev.escouade.acp`) serves it. Its harness id is `prime-agent`: a plugin of the first hand keeps the id it declares (a third party’s would have `plugin:<plugin>:<harness>`).

What the manifest says:

- `locate.names: ["prime-agent"]`, `tested` 0.10.3;
- one mode, `bypassPermissions`, with its meaning in French and in English; no `readOnly`;
- no `accountVar`: one account (see the Install tab);
- `install` (its two official scripts) and `updateCommand` (`prime-agent update`);
- `acp`: `args: ["--mode", "acp", "--no-session"]`, `fs: false` (it reads and writes by itself), no `modes` (it has none over ACP), `modelOption: "model"`, `questionsInText: true`, and `tools: { "Python cell": "Python" }`, the name the conversation gives its one tool.

## What its ACP says (Prime Agent 0.10.3, read on the wire)

- `initialize`: `loadSession: false`, `promptCapabilities` (images, embedded context), `mcpCapabilities.http`, `sessionCapabilities.close`; neither `session/load` nor `session/list` (announced for a later version).
- `session/new`: a session id, and a configuration option `model` whose values are `["<provider>","<model>"]`; for a model that reasons, a second option, `thought_level` (`off` to `high`). The `cwd` asked is ignored: the session works in the process’s folder (Escouade starts it in the agent’s).
- `session/set_mode` and `session/set_model`: “Method not found”. `session/set_config_option` changes the model and the reasoning effort.
- A turn: text in `agent_message_chunk`; each cell in `tool_call` (title “Python cell”, `kind: execute`, its code in `rawInput.code`) then `tool_call_update` (its output); `session_info_update`s of its own (`_meta.ai.primeintellect.prime-agent`: phases of the turn, subagents under way) that the client ignores. No thought seen with the trial’s model, no plan, neither tokens nor cost.
- `session/cancel`: the turn ends `cancelled`. `session/close` answers `{}`.
- Never a `session/request_permission`, never an `fs/*`.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest |
| `test/manifest.test.mjs` | Its test: `node --test` |
| `CHANGELOG.md`, `LICENSE` | Its versions, its MIT license |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- `node --test` in this folder checks the manifest;
- Escouade’s tests play the ACP client on a fake Prime Agent (`tests/fixtures/fake-prime-agent.mjs`) and replay the real streams of the trial (`tests/fixtures/prime-agent/`): `src-tauri/src/plugins_prime_agent_tests.rs`; the e2e `tests/e2e/prime-agent.spec.ts` does the same in the app.

`node plugins/ci.mjs` checks it with the others: its `node --test`, then `escouade-plugin validate --first-party` and the bench.

## Try it

Install this folder from the Plugins view (“From a folder…”) and pick its harness in the “Harness” menu. Or, from `plugins/`, with the app’s `escouade-plugin`:

```sh
escouade-plugin validate prime-agent --first-party
escouade-plugin test prime-agent --first-party
```
