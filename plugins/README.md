# Escouade's own plugins

The plugins of [Escouade](https://escouade.dev)'s own hand, each a folder of its own: its manifest, its README, its
LICENSE, its CHANGELOG and its tests, readable alone. Their ids are `dev.escouade.*`, which the store takes from this
repository alone. The folder's short name is only a folder's: the id is the manifest's, and the accounts, the settings
and the keychain go by it (docs/PLUGINS.md §8.6.1).

## On the store

Published on the store, submitted from their folder like anyone's plugin (docs/STORE.md §3.6).

| Folder           | Id                           | License | What it shows                                                                             | Kind                        |
| ---------------- | ---------------------------- | ------- | ----------------------------------------------------------------------------------------- | --------------------------- |
| `high-contrast/` | `dev.escouade.high-contrast` | MIT     | A theme, dark and light, and a palette of project colors (`contributes.themes`)           | Declarative                 |
| `discord/`       | `dev.escouade.discord`       | MIT     | The tickets to test and the agents waiting, in a Discord channel (`contributes.webhooks`) | Declarative                 |
| `slack/`         | `dev.escouade.slack`         | MIT     | The same in Slack, in each project's channel, the bot's token in the keychain             | TypeScript program, in WASM |
| `team-rules/`    | `dev.escouade.team-rules`    | MIT     | The agents' commands allowed or denied by `rules.json` (`permissions/decide`)             | TypeScript program, in WASM |
| `goose/`         | `dev.escouade.goose`         | MIT     | Goose as a harness, described over ACP (`contributes.harnesses[].acp`)                    | Declarative                 |

A version is a tag `plugins/<folder>/v1.2.3` (never `v1.2.3`, the app's releases), then its submission on the store
from its folder, its review and its signature: `npm run plugins:publish -- --changed` goes through them, run by the
chief alone (docs/PLUGINS.md §8.6.2). A manifest says its `store.categories`, which its first version needs (not Cursor CLI's, Gemini CLI's
nor Mistral's: their module embeds their manifest, and the tool holds theirs).

They are checked as an author checks a plugin before a tag: `node ci.mjs` looks for each one's LICENSE and CHANGELOG, runs
its tests (the TypeScript ones' typecheck too, and checks that their `dist/` is the build of `src/`), then `escouade-plugin
validate` and the conformance bench of Escouade (a `run.js` plugin is compiled to WASM and played in the app's host, with
no Node), with `--first-party` for their reserved ids. The repository's CI runs it on each change: all of them on `main`,
those a change touches on a pull request (`--changed`).

```sh
npx tsc -p ../sdk/plugin/tsconfig.json                       # the SDK they use (file:../../sdk/plugin)
ESCOUADE_PLUGIN_BIN=/path/to/escouade-plugin node ci.mjs     # all of them, as the CI does
ESCOUADE_PLUGIN_BIN=/path/to/escouade-plugin node ci.mjs --changed   # those this branch touches, since main
cd slack && npm install && npm test && npm run bench         # one of them
```

To try one in Escouade: Plugins view › “In development”, then its folder. To make one yours, copy its folder and give it
an id of your own (`io.github.<your-login>.<plugin>`). The documentation shows each one whole:
https://escouade.dev/en/docs/plugins/examples

## `seed.json`: the app's seed

`seed.json` names the versions of the plugins of the first hand the installer carries: `id`, `version` and the SHA-256 of
their `version.json`, on the store already. The release downloads them and checks every signature (`store/scripts/seed.ts`);
the app installs them with no network, by the path of an install from the store (docs/PLUGINS.md §8.7). It is filled once they
are published; `src-tauri/src/plugins/builtin.rs` lists the ids it may hold.

## With the app

The plugins shipped **with** the app, on their way to the store (docs/PLUGINS.md §8): for now each is embedded in the
app and listed in its registry (`src-tauri/src/plugins/builtin.rs`); a test of the app checks that each folder has its
manifest, its README, its LICENSE and its CHANGELOG, and is in the registry. Their program is `escouade-plugins <name>`,
signed with the app, and their version is Escouade's, or, once ported, a module (`run.wasm`) with a version of its own.
All are under **GPL-3.0-or-later**: their code comes from the app's. `ci.mjs` checks them too: their LICENSE and CHANGELOG, `escouade-plugin validate --builtin`, and for those with a crate
`cargo test` and the crate's version against the manifest's (a manifest at `0.0.0`, stamped by the app, passes). They have
no bench until they are `run.wasm`: then `ci.mjs` builds the module and plays it.

| Folder          | Id                             | License          | What it serves                                           |
| --------------- | ------------------------------ | ---------------- | -------------------------------------------------------- |
| `claude/`       | `dev.escouade.claude-code`     | GPL-3.0-or-later | Claude Code as a harness                                 |
| `codex/`        | `dev.escouade.codex`           | GPL-3.0-or-later | Codex as a harness                                       |
| `copilot/`      | `dev.escouade.copilot-cli`     | GPL-3.0-or-later | GitHub Copilot CLI as a harness                          |
| `cursor/`       | `dev.escouade.cursor-cli`      | GPL-3.0-or-later | Cursor CLI as a harness                                  |
| `gemini/`       | `dev.escouade.gemini-cli`      | GPL-3.0-or-later | Gemini CLI as a harness                                  |
| `antigravity/`  | `dev.escouade.antigravity-cli` | GPL-3.0-or-later | Antigravity CLI as a harness                             |
| `opencode/`     | `dev.escouade.opencode`        | GPL-3.0-or-later | OpenCode as a harness                                    |
| `acp/`          | `dev.escouade.acp`             | GPL-3.0-or-later | The harnesses a manifest describes over ACP, Goose's too |
| `mistral/`      | `dev.escouade.mistral`         | GPL-3.0-or-later | Mistral Vibe as a harness, and Mistral's voice           |
| `openai/`       | `dev.escouade.openai`          | GPL-3.0-or-later | OpenAI's voice                                           |
| `groq/`         | `dev.escouade.groq`            | GPL-3.0-or-later | Groq's voice                                             |
| `elevenlabs/`   | `dev.escouade.elevenlabs`      | GPL-3.0-or-later | ElevenLabs' voice                                        |
| `deepgram/`     | `dev.escouade.deepgram`        | GPL-3.0-or-later | Deepgram's voice                                         |
| `github/`       | `dev.escouade.github`          | GPL-3.0-or-later | GitHub's code host and GitHub Issues                     |
| `gitlab/`       | `dev.escouade.gitlab`          | GPL-3.0-or-later | GitLab's code host and GitLab Issues                     |
| `bitbucket/`    | `dev.escouade.bitbucket`       | GPL-3.0-or-later | Bitbucket Cloud's code host                              |
| `gitea/`        | `dev.escouade.gitea`           | GPL-3.0-or-later | The code host of Gitea and Forgejo                       |
| `azure-devops/` | `dev.escouade.azure-devops`    | GPL-3.0-or-later | Azure DevOps' code host and Azure Boards                 |
| `linear/`       | `dev.escouade.linear`          | GPL-3.0-or-later | Linear in the Kanban                                     |
| `jira/`         | `dev.escouade.jira`            | GPL-3.0-or-later | Jira in the Kanban                                       |
| `trello/`       | `dev.escouade.trello`          | GPL-3.0-or-later | Trello in the Kanban                                     |
| `clickup/`      | `dev.escouade.clickup`         | GPL-3.0-or-later | ClickUp in the Kanban                                    |
| `isola/`        | `dev.escouade.isola`           | GPL-3.0-or-later | isola as a launch environment                            |

Those with a crate (`Cargo.toml`, `src/`) depend by version on the crates that go to crates.io, and `[patch.crates-io]` of `src-tauri/Cargo.toml` brings them back to `crates/` here.
A plugin in `run.wasm` (Linear, Trello, Jira, ClickUp, isola, GitHub, GitLab, Bitbucket, Gitea, Azure DevOps, ACP, Claude Code) has left that workspace for the one of this folder, `Cargo.toml`, with the same patch: `sh scripts/plugins-wasm.sh` compiles its module from there against `crates/`, for the tests of the app and a build of development. Its folder also has a `Cargo.lock` of its own, which fixes the crates published: cargo does not read it here, `escouade-plugin build` and the store do, from the folder alone. To port a plugin: docs/PLUGINS.md §8.19. They stand on the kit of the Rust plugins (`crates/escouade-plugin-kit`) or on the
ACP client (`crates/escouade-acp`). The demonstration, `dev.escouade.minimal` (in development builds only), is a fixture:
`tests/fixtures/plugins/minimal/`.
