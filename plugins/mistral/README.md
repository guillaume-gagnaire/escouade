# Mistral

`dev.escouade.mistral`, a plugin [Escouade](https://escouade.dev) gives: shipped with the app, signed with it, updated with it
(its version is Escouade's; the `0.0.0` of its manifest is replaced by the app's when it is read).

Mistral dans Escouade, un seul plugin et un seul compte : le service de voix de Réglages › Voix (Voxtral : dictée d'un fichier, transcription en direct, voix de l'agent, catalogue des modèles et des voix ; INT-13) et Mistral Vibe comme harness, par ACP (INT-10.6). Ses agents conversent comme avant, par un programme à part qu'Escouade lance et surveille.

One process serves both (`docs/PLUGINS.md` §4.13): `escouade-plugins mistral` answers `voice/*` and the contract of the harnesses.

## Use it

Nothing to do: it is in the Plugins view under « Livrés avec Escouade », on. Add your key in Réglages › Voix, as before, and your
Mistral Vibe accounts in Réglages › Comptes. Turn the plugin off and both go with it: the dictation falls back to local, and an
agent of Mistral Vibe says at its start « Le plugin Mistral (dev.escouade.mistral) est coupé ou n'est pas chargé : rallume-le dans
la vue Plugins… » and keeps its session. It can never be removed, and a plugin of the store with the same id is refused: this one
is the app's. A plugin of the store that serves the same voice service id replaces that service.

## The Mistral account

The plugin declares one kind of account, `mistral` (`contributes.accounts`, the rule of `docs/PLUGINS.md` §4.4): its points are its
voice and the harness `vibe`.

- **The account `mistral`** is the key of Réglages › Voix, in the keychain under `plugin:dev.escouade.mistral:account:mistral`.
  Its former name, `dictation-mistral`, stays beside it and is read first (the reversible migration of INT-6).
- **A Mistral Vibe account with a key of its own** keeps it, under `plugin:dev.escouade.mistral:account:vibe-<id>`, its former name
  `vibe:<id>` beside it, read first: two Mistral organizations stay two accounts, each billed its own.
- **Each side falls back on the other.** A new Vibe account may leave its key empty and go by the Mistral account's; without a
  key in Réglages › Voix, the voice goes by the first active Vibe account's, and says whose.
- **The Principal of Vibe** stays the machine's: the key `vibe --setup` kept, which Escouade never reads nor shares.

Nobody gives a key again. The plugin reads no secret (no `permissions.secrets`): the key goes in each voice call, to this plugin
alone, which keeps nothing, and to Vibe by `MISTRAL_API_KEY`, given by the core, never to this plugin.

## What it does for the voice

Default model: `voxtral-mini-latest`; live model: `voxtral-mini-transcribe-realtime-2602`. Its module answers `voice/*` with the
crate `crates/escouade-voice`, the sound going to `api.mistral.ai`.

## What it does for Mistral Vibe

It serves the harness its manifest declares (`contributes.harnesses[0]`, id `vibe`): Mistral Vibe's `vibe-acp`, driven
over ACP v1 by the client of `crates/escouade-acp` with its extension `vibe`. On the contract of the harnesses in
process (INT-9):

- the core finds `vibe-acp` (on the PATH, else where `uv tool install mistral-vibe` leaves it), writes in the agent's
  own `VIBE_HOME` the trust of its folder and, for a private project, the `denylist` of its file tools, asks this
  plugin how to launch it (`engine/launch`: `vibe-acp`, no telemetry), checks the launch and starts it;
- what Vibe writes comes here (`agent/frames`) and goes back as the items of the conversation and the lines to write
  to it: the session (new, resumed or copied by `session/fork`), its agent (`ask`, `accept-edits`, `auto-approve`), its
  model and thinking (`off` for « Bas », `high` otherwise), texts, reasonings, calls under Claude Code's names (its
  `agents.spawn` a `Task`, its MCP tools `mcp__<server>__<tool>`), permissions named by their call alone, its questions
  (`elicitation/create`, a form), its task list, the end of each turn with what the session's cost grew by, and its
  rate limit (« limité », resumed a minute later);
- Escouade's mode Plan, which Vibe does not have, is played here: each prompt says to plan without changing anything,
  its writes are refused, and the turn's last words are the plan to approve on its card; approved, it is carried out in
  the mode chosen;
- a refusal of the user's cuts the turn (`session/cancel`): Vibe would otherwise try the command again under other
  forms, each asking again; « Toujours » is Vibe's `allow_always_permanent`, kept in the agent's own `VIBE_HOME`.

What stays in the core, as the contract says: the terminal view (`vibe` in a PTY, its journal and its hooks followed),
the accounts, the models read for an account's menu, the texts it writes (`vibe -p`, no tool, within a price), the files of
its `VIBE_HOME`, Escouade's MCP server.

A discussion of the Docs tab never goes to it: it has no mode that changes nothing (`readOnlyMode`).

## What it may do

- reach `api.mistral.ai` (permission `network`), for its voice, nothing else;
- hear the sound the user dictates, and the text the agent reads, for this service alone;
- receive the key of the service in each voice call (the plugin keeps nothing and writes nothing);
- describe the launch of Mistral Vibe and translate what it says: the core launches it, never this plugin;
- refuse at once a permission that names a path of a private project (`fenceOnPermissions`: Vibe reads and writes by
  itself; the core also refuses those paths to its file tools in its `config.toml`);
- nothing else: no secret, no file, no command to run. The plugin is only told a Vibe account goes by a key (`byKey`).

## What is in this folder

| File                   | What it is                                                             |
| ---------------------- | ---------------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: its account, the harness, its modes, models, efforts, `acp` |
| `Cargo.toml`           | The crate `escouade-plugin-mistral`                                    |
| `src/lib.rs`           | The module: `escouade-voice`, and the service of `escouade-acp` beside it |
| `README.md`            | This page                                                              |

It is a module: its first turn serves `voice/*` by `crates/escouade-voice`, and the rest by the service of
`escouade_acp::plugin` and the client of `crates/escouade-acp` (`vibe.rs`), in one instance. Its tests: the service in
`cargo test -p escouade-voice`, the module against fake servers (`src-tauri/src/voice/*_tests.rs`, `voice/parity_tests.rs`), the account (`src-tauri/src/plugins_accounts_tests.rs`, on a fake keychain), the bench of the harnesses on
the fake Vibe (`src-tauri/src/engine/vibe/bench.rs`), the recorded streams of Mistral Vibe 3.0.3 replayed by its client
(`src-tauri/src/plugins_acp_tests.rs`, `agent/vibe_tests.rs`), the tests of the core that go through it (`src-tauri/src/vibe_tests.rs`,
`vibe_tui_tests.rs`), the e2e `tests/e2e/vibe.spec.ts` and `voice.spec.ts`, and the real runs (`real_vibe`, `real_vibe_writes_a_text`,
`real-harnesses.spec.ts`).

## Run it alone

Against the fake Escouade of the plugins' bench, with the app's own `escouade-plugin` and `escouade-plugins` (built with
`cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`, side by side in `target/debug`):

```sh
escouade-plugin test plugins/mistral --builtin
```
