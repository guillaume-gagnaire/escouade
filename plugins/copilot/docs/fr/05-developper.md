# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/copilot/` du dépôt d’Escouade, id `dev.escouade.copilot-cli`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul, sur les crates `escouade-plugin-protocol` et `escouade-acp` (pour les règles de git). Il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `copilot`) : `copilot --server --stdio`, piloté par le protocole de son SDK (JSON-RPC 2.0, protocole 3) par un client d’Escouade. Ce n’est pas un harness ACP.

- Le cœur trouve `copilot`, demande au plugin comment le lancer (`engine/launch`), vérifie la description et le lance. Ses messages vont avec des en-têtes `Content-Length` (`framing`), et `runtime.shutdown` est écrit avant de fermer son entrée (`closeLine`), pour qu’il sauve sa session.
- Le plugin ouvre la session lui-même, étape par étape (`opens`) : le protocole vérifié, la connexion, la session nouvelle, reprise ou copiée, ses modes, le serveur d’Escouade, son modèle, les modèles du compte. Un refus arrête l’agent (`Item::Stop`) et dit pourquoi.
- Ce que Copilot écrit arrive par `agent/frames` et repart en éléments de la conversation : textes, raisonnements, appels sous les noms de Claude Code avec leurs diffs, permissions, questions, formulaires, fin d’un plan, plafond de crédits, tâches, sous-agents, crédits du mois, contexte, tokens et coût de chaque tour.
- Les images et fichiers d’un message passent tels que le cœur les a faits ; `/compact` par sa requête (`agent/compact`), un titre par `session.name.set` (`agent/rename`).
- Le journal de la vue Copilot CLI est lu par la même traduction (`view/translate`, `src/view.rs`).

Reste au cœur : le PTY de la vue terminal et ses hooks, les comptes (`copilot login` dans le dossier de chacun), le quota et les modèles d’un compte lus par un serveur à eux, les textes rédigés (`copilot -p`), le serveur MCP d’Escouade.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, modèles, efforts, framing et fermeture |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-copilot` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son `run` : les méthodes du contrat, un client par agent |
| `src/client.rs` | Le client du serveur de Copilot, une machine à états sans process |
| `src/translate.rs` | Ses événements en éléments de la conversation, les demandes sûres ; aussi ceux de la vue |
| `src/view.rs` | Le journal lu pour la vue Copilot CLI |
| `src/words.rs` | Ses arguments, modes, modèles, crédits, la clôture, ce qu’il dit |
| `src/tests.rs`, `src/rules_tests.rs` | Le client seul, et ce que tiennent ses règles par le service |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- `src/tests.rs` et `src/rules_tests.rs`, avec `cargo test` dans `plugins/` ;
- la parité avec l’ancien plugin natif (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/copilot.jsonl`) ;
- le banc des harness sur le faux Copilot (`src-tauri/src/engine/copilot/bench.rs`) et les flux enregistrés de Copilot CLI 1.0.98 (`src-tauri/src/agent/copilot_tests.rs`, `tests/fixtures/replays/copilot/`) ;
- les tests du cœur qui passent par lui (`src-tauri/src/copilot_tests.rs`, `copilot_tui_tests.rs`) et l’e2e `tests/e2e/copilot.spec.ts` ;
- les essais réels, à lancer à la main : `real_copilot`, `real_copilot_view`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build copilot --first-party --out ../packs
escouade-plugin test copilot --first-party --wasm ../packs/dev.escouade.copilot-cli-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh copilot` le compile contre `crates/`, et `sh scripts/plugin-lock.sh copilot` réécrit son `Cargo.lock` après un changement des crates.
