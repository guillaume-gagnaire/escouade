# Develop

For whoever wants to read or change the plugin. It lives in `plugins/antigravity/` of Escouade’s repository, id `dev.escouade.antigravity-cli`, licensed GPL-3.0-or-later.

## What it is

A WASM module (`"run": { "wasm": "Cargo.toml" }`), compiled from this folder alone, on the crates `escouade-plugin-protocol` and `escouade-acp`. It serves the harness its manifest declares (`contributes.harnesses[0]`, id `agy`): `agy -p=` driven in stream-json, one message a line on its input, one event a line on its output. It is no ACP harness.

- The core finds `agy`, asks the plugin how to launch it (`engine/launch`: the stream both ways, its model and effort, its mode, its conversation, the agent’s own folder as `USERPROFILE` and `HOME`), checks the launch and starts it.
- It starts it again when it must (`relaunches`): `agy` takes its model, effort and mode at its launch alone, and has no message that stops a turn. A stopped turn pauses its program (`Item::Pause`), the next message starts another on the same conversation (`--conversation`, `Item::Relaunch`), each description checked as the first.
- What `agy` writes comes by `agent/frames` and goes back as items of the conversation, in the form the ACP client reads (`escouade-acp`, extension `agy`): texts, calls under Claude Code’s names with their diffs, subagents, tokens, each turn’s end. What its events leave out is read from the conversation’s transcript; when `agy` writes it late, what follows is held in its order, the transcript is read again on a timer of the host (600 ms at most for a batch), and what was held goes out as `agent/items`.
- `-p` asks nothing: what needs a permission is refused on the spot, and `ask_question` skipped. At the end of a turn, the plugin judges each refusal (`words::judge`), shows the cards, has the exact rules allowed written and has the turn go on with a message saying what was allowed and answered. No prefix rule of `agy` guards anything: docs/PROTOCOL.md, “Règles et jugement”.
- In Plan, the plan written in its conversation comes on the plan’s card.

What stays in the core: Antigravity’s place in the registry, its accounts, an account’s models (`agy models`), the written texts (`agy -p`), the terminal view (its PTY, its transcript, its hooks), Escouade’s MCP server, and taking its token out of the agent’s folder once the process is over.

## The folder

| File | What it is |
|---|---|
| `escouade-plugin.json` | The manifest: the harness, its modes, its folder and key |
| `Cargo.toml`, `Cargo.lock` | The crate `escouade-plugin-agy` and the crates of crates.io the store compiles |
| `src/lib.rs` | Its `run`: the methods of the contract, the timer |
| `src/client.rs` | The client of `agy -p`, a state machine without a process |
| `src/translate.rs` | Its events as items of the conversation |
| `src/words.rs` | Its launch, rules, folder, calls and questions |
| `src/host.rs` | What it asks of the host: `fs/read`, a timer, `agent/items` |
| `src/view.rs` | The transcript read for the “Antigravity CLI view” |
| `src/tests.rs` | The client on its own |
| `src/rules_tests.rs` | The rules and the judging of refusals, and the real run of `agy`’s rules (`real_agy_rules`, ignored) |
| `README.fr.md`, `README.en.md`, `docs/` | Its page and its documentation |

## Its tests

- `src/tests.rs` and `src/rules_tests.rs`, with `cargo test` in `plugins/antigravity`;
- the bench of the harnesses on the fake `agy` (`src-tauri/src/engine/agy/bench.rs`, its transcript late too);
- the parity with the former native plugin on the streams of `agy` 1.4.9 (`src-tauri/src/plugins_wasm_agy_tests.rs`) and the recorded streams (`src-tauri/src/agent/agy_tests.rs`, `tests/fixtures/replays/agy/`);
- the core’s tests that go through it (`src-tauri/src/agy_tests.rs`, `agy_tui_tests.rs`) and the e2e `tests/e2e/agy.spec.ts`;
- the real runs, started by hand: `real_agy`, `real_agy_writes_a_text`, `tests/e2e/real-harnesses.spec.ts`, its view included; and `real_agy_rules`, which tells case by case how `agy` judges its rules (`ESCOUADE_TEST_REAL_AGY=<agy> cargo test real_agy_rules -- --ignored --nocapture` in `plugins/antigravity`).

`node plugins/ci.mjs` checks it as it checks the app’s other plugins: its version against the crate’s, `cargo test`, `escouade-plugin validate`, then the bench on the compiled module.

## Run it alone

From `plugins/`, with the app’s `escouade-plugin` (`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin build antigravity --first-party --out ../packs
escouade-plugin test antigravity --first-party --wasm ../packs/dev.escouade.antigravity-cli-<version>.wasm
```

`<version>` is the manifest’s. In the repository, `sh scripts/plugins-wasm.sh antigravity` compiles it against `crates/`, and `sh scripts/plugin-lock.sh antigravity` writes its `Cargo.lock` again after a change of the crates.
