# Develop

For whoever wants to read or change the plugin, or describe another ACP harness on its model. It lives in `plugins/goose/` of Escouade’s repository, id `dev.escouade.goose`, licensed MIT.

## What it is

A declarative plugin: its manifest alone, without a program. The entry of `contributes.harnesses` that holds an `acp` object describes Goose; Escouade’s ACP plugin (`dev.escouade.acp`) serves it. Its harness id is `goose`: a plugin of Escouade’s own hand keeps the id it declares (a third party’s would get `plugin:<plugin>:<harness>`).

What the manifest says:

- `locate.names: ["goose"]`, `tested` 1.57.0;
- its three modes, each with its meaning in French and English, and `readOnly`;
- `accountVar: "GOOSE_PATH_ROOT"`: each account’s configuration folder;
- `install` and `updateCommand`;
- `acp`: `args: ["acp"]`, `fs: false` (it reads and writes by itself), `modes` (`approve`, `smart_approve`, `auto`), `readOnlyMode: "chat"`, `modelOption: "model"`, `questionsInText: true`, and `tools`, Escouade’s name for its tools of kind `other` (`shell` → `Bash`, `write` → `Write`, `edit` and `text_editor` → `Edit`, `read` → `Read`, `tree` → `Glob`).

The ACP plugin’s documentation says each key of the `acp` object, and what Escouade holds for a described harness.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest |
| `test/manifest.test.mjs` | Its test: `node --test` |
| `CHANGELOG.md`, `LICENSE` | Its versions, its MIT licence |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- `node --test` in this folder checks the manifest: one harness described over ACP, without a program of its own; each mode with its Goose mode and its meaning in both languages; read-only by `chat`; `fs` set to `false`;
- Escouade’s tests play the same client on a fake ACP agent (`src-tauri/src/plugins_acp_tests.rs`), and its real run on Goose is `real_goose`, in the same place, started by hand.

`node plugins/ci.mjs` checks it with the others: its `node --test`, then `escouade-plugin validate --first-party` and the bench.

## Try it

Install this folder from the Plugins view (“From a folder…”) and pick its harness in the “Harness” menu. Or, from `plugins/`, with the app’s `escouade-plugin`:

```sh
escouade-plugin validate goose --first-party
escouade-plugin test goose --first-party
```
