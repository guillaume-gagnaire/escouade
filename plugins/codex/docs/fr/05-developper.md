# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/codex/` du dépôt d’Escouade, id `dev.escouade.codex`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul, sur les crates `escouade-plugin-protocol` et `escouade-acp` (pour les règles de git). Il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `codex`) : `codex app-server`, piloté par son propre protocole JSON-RPC, un message par ligne. Ce n’est pas un harness ACP.

- Le cœur trouve `codex`, demande au plugin comment le lancer (`engine/launch` : `app-server`, l’outil de plan, les profils de permissions d’un projet privé, le serveur d’Escouade par `-c`), vérifie la description et lance le process, chaque compte dans son dossier (`CODEX_HOME`).
- Le plugin ouvre la conversation par le protocole (`opens` : nouvelle, reprise ou copiée), lit les modèles du compte (`model/list`, toutes les pages) et dit au cœur que l’agent est prêt (`Item::Ready`).
- Ce que Codex écrit arrive par `agent/frames` et repart en éléments de la conversation : raisonnements et textes en direct, appels sous les noms de Claude Code avec leurs diffs, cartes, questions, le plan du mode Plan, les tâches, les sous-agents, la jauge de contexte, le quota, la fin de chaque tour.
- Un message pendant un tour y entre (`turn/steer`) ; le tour suivant part avec le modèle, l’effort et le mode du moment (`turn/start`).
- Il estime le coût d’un tour d’après ses tokens : `src/prices.rs` est la grille publique d’OpenAI.
- La « Vue Codex » est lue ici aussi (`view/translate`) : le cœur lui donne les lignes du journal de la session.

Reste au cœur : la place de Codex au registre, les comptes et leur connexion, le quota et les modèles d’un compte quand aucun agent ne tourne, les textes rédigés (`codex exec`), le PTY de la vue terminal et ses hooks, le serveur MCP d’Escouade.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, modèles, efforts, sa variable de dossier |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-codex` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son `run` et les méthodes du contrat |
| `src/client.rs` | Le client de son app-server, une machine à états sans process |
| `src/translate.rs` | Ses messages en éléments de la conversation, `git add` et `git commit` acceptés |
| `src/words.rs` | Ses modes, la clôture, le serveur d’Escouade dans ses options, son quota |
| `src/prices.rs` | Les tarifs publics des modèles d’OpenAI |
| `src/view.rs` | Le journal lu pour la « Vue Codex » |
| `src/tests.rs`, `src/rules_tests.rs` | Le client seul, et ce que tiennent ses règles par le service (redémarrage, commit qui lit un fichier) |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- `src/tests.rs`, `src/rules_tests.rs` et `src/prices.rs`, avec `cargo test` dans `plugins/` ;
- le banc des harness sur le faux Codex (`src-tauri/src/engine/codex/bench.rs`) ;
- les flux enregistrés de Codex 0.163.0 (`src-tauri/src/agent/codex_tests.rs`, `tests/fixtures/replays/codex/`) et la parité avec l’ancien plugin natif, appel par appel (`src-tauri/src/plugins_harness_parity_tests.rs`, `tests/fixtures/plugins/parity/codex.jsonl`) ;
- les tests du cœur qui passent par lui (`src-tauri/src/codex_tests.rs`, `codex_tui_tests.rs`) et l’e2e `tests/e2e/codex.spec.ts` ;
- les essais réels, à lancer à la main : `real_codex_compact`, `real_codex_view`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build codex --first-party --out ../packs
escouade-plugin test codex --first-party --wasm ../packs/dev.escouade.codex-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh codex` le compile contre `crates/`, et `sh scripts/plugin-lock.sh codex` réécrit son `Cargo.lock` après un changement des crates.
