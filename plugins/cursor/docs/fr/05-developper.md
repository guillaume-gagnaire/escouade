# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/cursor/` du dépôt d’Escouade, id `dev.escouade.cursor-cli`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul. Son code tient en quelques lignes : il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `cursor`, son objet `acp` avec l’extension `cursor`) par le client ACP du crate `escouade-acp` (`escouade_acp::plugin::serve_as`), comme le plugin ACP sert les harness que d’autres plugins décrivent.

- Le cœur trouve `agent` (sous Windows, le lanceur de son installeur est lancé comme son `node` et son `index.js`), demande au plugin comment le lancer (`engine/launch` : `agent acp`, `MSYSTEM` et `EXEPATH` retirés), vérifie la description et le lance, chaque agent dans son dossier de configuration (`CURSOR_CONFIG_DIR`).
- Ce que Cursor écrit arrive par `agent/frames` et repart en éléments de la conversation : la session (nouvelle ou reprise), son mode, son modèle et son effort, textes, raisonnements, appels sous les noms de Claude Code avec leurs diffs, permissions, questions (`cursor/ask_question`), plan (`cursor/create_plan`), tâches (`cursor/update_todos`), sous-agents, fin de chaque tour, et les refus de Cursor (limite d’usage, modèle hors de l’offre, clé ou connexion refusée).
- Les images et fichiers texte d’un message passent tels que le cœur les a faits ; ce qui a été dit dans sa vue terminal passe avant le message suivant (`relay` d’`engine/launch`).

Reste au cœur : la vue terminal (son PTY, son journal suivi), les comptes (la connexion de la machine, un compte par sa clé, `keyVar`), les modèles lus pour le menu d’un compte, les textes rédigés, le serveur MCP d’Escouade.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, modèles, efforts, son objet `acp` |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-cursor` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son `run` : le service du plugin ACP, avec son propre harness, et le test de son manifeste |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

Le client, ses règles de permission et l’extension `cursor` sont dans `crates/escouade-acp/` (`agent.rs`, `cursor.rs`).

## Ses tests

- le test de son manifeste dans `src/lib.rs`, et ceux de `crates/escouade-acp`, avec `cargo test` ;
- le banc des harness sur le faux Cursor (`src-tauri/src/engine/cursor/bench.rs`) ;
- les flux enregistrés de Cursor CLI 2026.10.22 rejoués par son client (`src-tauri/src/plugins_acp_tests.rs`) ;
- les tests du cœur qui passent par lui (`src-tauri/src/cursor_tests.rs`, `cursor_tui_tests.rs`) et l’e2e `tests/e2e/cursor.spec.ts` ;
- les essais réels, à lancer à la main : `real_cursor`, `real_cursor_view`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build cursor --first-party --out ../packs
escouade-plugin test cursor --first-party --wasm ../packs/dev.escouade.cursor-cli-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh cursor` le compile contre `crates/`, et `sh scripts/plugin-lock.sh cursor` réécrit son `Cargo.lock` après un changement des crates.
