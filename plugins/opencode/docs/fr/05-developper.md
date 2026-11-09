# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/opencode/` du dépôt d’Escouade, id `dev.escouade.opencode`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul. Il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `opencode`) : OpenCode 1 (`opencode-ai`, 1.19) piloté par `opencode serve`. OpenCode n’est pas un process par agent : un serveur sert les agents d’un compte (0,7 Go par serveur), chacun dans une session.

- Le cœur trouve `opencode`, demande au plugin comment le lancer (`engine/launch` : `opencode serve`, sa config, un mot de passe tiré au hasard, marqué `shared`), vérifie le lancement et démarre le serveur dans son objet Job, une fois pour les agents du même compte ; il l’arrête avec le dernier agent.
- Le plugin est le client de ce serveur (`agent/open`) : il ouvre, reprend ou copie la session de l’agent, pose les règles de son mode avant de l’ouvrir (refusées, pas de session), donne au dossier de l’agent le serveur MCP d’Escouade avec son jeton, envoie ses messages, répond à ses permissions et questions.
- Il lit seul le flux d’événements du serveur, et traduit chaque événement en éléments de la conversation, envoyés au cœur au fil de l’eau (`agent/items`) : textes, raisonnements, appels sous les noms de Claude Code avec leurs diffs, permissions, questions, plan, tâches, sous-agents, contexte, coût, fin de chaque tour.
- Un message envoyé pendant un tour attend sa fin.

Reste au cœur : le lancement et l’arrêt du serveur, la vue terminal (`opencode attach` dans le PTY du cœur, d’après ce que décrit `agent/open`), les comptes, les modèles lus pour le menu d’un compte, les textes rédigés (`opencode run`), le serveur MCP d’Escouade.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, efforts, dossier de compte, son installation |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-opencode` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son tour, les méthodes du contrat, la description du lancement |
| `src/server.rs` | Le client d’`opencode serve` : ses requêtes, son flux d’événements |
| `src/session.rs` | La session d’un agent : messages retenus, permissions, questions, modes |
| `src/translate.rs` | Ses événements en éléments de la conversation |
| `src/rules.rs` | Les règles de chaque mode, la clôture, la config, les modèles tels qu’OpenCode les liste |
| `src/rules_verdicts.json` | Ce que font ces règles, cas par cas, tel que le vrai OpenCode l’a fait |
| `src/tests.rs` | Le plugin sur ses lignes, sans module |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- ceux du crate, joués sur ses lignes sans module (`src/tests.rs`, `src/rules.rs`), avec `cargo test` dans `plugins/` ;
- la parité avec l’ancien plugin natif (`src-tauri/src/plugins_wasm_parity_tests.rs`, `tests/fixtures/plugins/parity/opencode*.json`) ;
- le banc des harness sur le faux `opencode serve` (`src-tauri/src/engine/opencode/bench.rs`) et les flux enregistrés d’OpenCode 1.19.3 (`tests/fixtures/replays/opencode/`) ;
- les tests du cœur qui passent par lui (`src-tauri/src/opencode_tests.rs`, `src-tauri/src/agent/opencode_tests.rs`) et l’e2e `tests/e2e/opencode.spec.ts` ;
- les essais réels, à lancer à la main : `real_opencode`, `real_opencode_view`, `real_fence`, `tests/e2e/real-harnesses.spec.ts`. `node scripts/opencode-real-rules.mjs` joue les règles sur un vrai `opencode serve` avec un faux modèle, sans compte.

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build opencode --first-party --out ../packs
escouade-plugin test opencode --first-party --wasm ../packs/dev.escouade.opencode-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh opencode` le compile contre `crates/`, et `sh scripts/plugin-lock.sh opencode` réécrit son `Cargo.lock` après un changement des crates.
