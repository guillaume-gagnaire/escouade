# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/gemini/` du dépôt d’Escouade, id `dev.escouade.gemini-cli`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul. Son code tient en quelques lignes : il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `gemini`, son objet `acp` avec l’extension `gemini`) par le client ACP du crate `escouade-acp` (`escouade_acp::plugin::serve_as`).

- Le cœur trouve `gemini`, écrit dans le `GEMINI_CLI_HOME` de l’agent son `settings.json` (pas de mise à jour, pas de statistiques, comment il se connecte) et, pour un projet privé ou une discussion, ses règles ; il demande au plugin comment le lancer (`engine/launch` : `--acp`, le dossier approuvé par `GEMINI_CLI_TRUST_WORKSPACE`), ajoute `--policy` quand il y a des règles, vérifie le lancement et le démarre.
- Ce que Gemini écrit arrive par `agent/frames` et repart en éléments de la conversation : la session (nouvelle, ou reprise par `session/load`, dont le rejeu de l’historique est laissé de côté), son mode, son modèle, textes, raisonnements, appels sous les noms de Claude Code (leur outil lu dans leur id, un sous-agent en `Task`, ses outils MCP en `mcp__<serveur>__<outil>`), permissions, fin de chaque tour avec ses tokens.
- Son changement de mode, un texte à lui (« [MODE_UPDATE] plan »), est lu comme tel et sorti du fil, et ne peut que baisser le mode.
- Ses questions passent par un bloc `<escouade_question>` en fin de tour ; son plan est la permission de son `exit_plan_mode`, lu dans le fichier qu’il nomme.
- Un refus de l’utilisateur coupe le tour (`session/cancel`) ; sa limite d’usage finit le tour « limité », sa reprise prévue.

Reste au cœur : la vue terminal (`gemini` dans un PTY, suivi par ses hooks), les comptes, les modèles lus pour le menu d’un compte, les textes rédigés (`gemini -p`, tous ses outils refusés), les fichiers de son `GEMINI_CLI_HOME`, le serveur MCP d’Escouade.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, son objet `acp` |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-gemini` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son `run` : le service du plugin ACP, avec son propre harness, et le test de son manifeste |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

Le client et l’extension `gemini` sont dans `crates/escouade-acp/` (`agent.rs`, `gemini.rs`).

## Ses tests

- le test de son manifeste dans `src/lib.rs`, et ceux de `crates/escouade-acp`, avec `cargo test` ;
- la parité avec l’ancien plugin natif (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/gemini.jsonl`) ;
- le banc des harness sur le faux Gemini (`src-tauri/src/engine/gemini/bench.rs`) et les flux de l’étude de Gemini CLI 0.66.0 rejoués par son client (`src-tauri/src/plugins_acp_tests.rs`, `src-tauri/src/agent/gemini_tests.rs`) ;
- les tests du cœur qui passent par lui (`gemini_tui_tests.rs`) et l’e2e `tests/e2e/gemini.spec.ts` ;
- l’essai réel `real_gemini`, écrit et ignoré : aucun compte ne l’a encore lancé.

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build gemini --first-party --out ../packs
escouade-plugin test gemini --first-party --wasm ../packs/dev.escouade.gemini-cli-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh gemini` le compile contre `crates/`, et `sh scripts/plugin-lock.sh gemini` réécrit son `Cargo.lock` après un changement des crates.
