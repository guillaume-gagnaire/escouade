# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/antigravity/` du dépôt d’Escouade, id `dev.escouade.antigravity-cli`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul, sur les crates `escouade-plugin-protocol` et `escouade-acp`. Il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `agy`) : `agy -p=` piloté en stream-json, un message par ligne sur son entrée, un événement par ligne sur sa sortie. Ce n’est pas un harness ACP.

- Le cœur trouve `agy`, demande au plugin comment le lancer (`engine/launch` : le flux dans les deux sens, son modèle et son effort, son mode, sa conversation, le dossier propre à l’agent comme `USERPROFILE` et `HOME`), vérifie le lancement et le démarre.
- Il le relance quand il le faut (`relaunches`) : `agy` ne prend son modèle, son effort et son mode qu’au lancement, et n’a pas de message qui arrête un tour. Un tour arrêté met son programme en pause (`Item::Pause`), le message suivant en relance un sur la même conversation (`--conversation`, `Item::Relaunch`), chaque description vérifiée comme la première.
- Ce que `agy` écrit arrive par `agent/frames` et repart en éléments de la conversation, sous la forme que lit le client ACP (`escouade-acp`, extension `agy`) : textes, appels sous les noms de Claude Code avec leurs diffs, sous-agents, tokens, fin de chaque tour. Ce que ses événements laissent de côté est lu dans le journal de la conversation ; quand `agy` l’écrit en retard, la suite attend dans son ordre, le journal est relu sur une minuterie de l’hôte (600 ms au plus pour un lot), et ce qui attendait part en `agent/items`.
- `-p` ne demande rien : ce qui demande une permission est refusé sur le moment, et `ask_question` sauté. À la fin d’un tour, le plugin juge chaque refus (`words::judge`), montre les cartes, fait écrire les règles exactes permises et fait reprendre le tour par un message qui dit ce qui a été permis et répondu. Aucune règle-préfixe d’`agy` ne garde quelque chose : docs/PROTOCOL.md, « Règles et jugement ».
- En Plan, le plan écrit dans sa conversation arrive sur la carte du plan.

Reste au cœur : la place d’Antigravity au registre, ses comptes, les modèles d’un compte (`agy models`), les textes rédigés (`agy -p`), la vue terminal (son PTY, son journal, ses hooks), le serveur MCP d’Escouade, et le retrait de son jeton du dossier de l’agent une fois le process fini.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, son dossier et sa clé |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-agy` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son `run` : les méthodes du contrat, la minuterie |
| `src/client.rs` | Le client de `agy -p`, une machine à états sans process |
| `src/translate.rs` | Ses événements en éléments de la conversation |
| `src/words.rs` | Son lancement, ses règles, son dossier, ses appels et questions |
| `src/host.rs` | Ce qu’il demande à l’hôte : `fs/read`, une minuterie, `agent/items` |
| `src/view.rs` | Le journal lu pour la « Vue Antigravity CLI » |
| `src/tests.rs` | Le client seul |
| `src/rules_tests.rs` | Les règles et le jugement des refus, et l’essai réel des règles d’`agy` (`real_agy_rules`, ignoré) |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- `src/tests.rs` et `src/rules_tests.rs`, avec `cargo test` dans `plugins/antigravity` ;
- le banc des harness sur le faux `agy` (`src-tauri/src/engine/agy/bench.rs`, son journal en retard compris) ;
- la parité avec l’ancien plugin natif sur les flux d’`agy` 1.4.9 (`src-tauri/src/plugins_wasm_agy_tests.rs`) et les flux enregistrés (`src-tauri/src/agent/agy_tests.rs`, `tests/fixtures/replays/agy/`) ;
- les tests du cœur qui passent par lui (`src-tauri/src/agy_tests.rs`, `agy_tui_tests.rs`) et l’e2e `tests/e2e/agy.spec.ts` ;
- les essais réels, à lancer à la main : `real_agy`, `real_agy_writes_a_text`, `tests/e2e/real-harnesses.spec.ts`, sa vue comprise ; et `real_agy_rules`, qui dit cas par cas comment `agy` juge ses règles (`ESCOUADE_TEST_REAL_AGY=<agy> cargo test real_agy_rules -- --ignored --nocapture` dans `plugins/antigravity`).

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build antigravity --first-party --out ../packs
escouade-plugin test antigravity --first-party --wasm ../packs/dev.escouade.antigravity-cli-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh antigravity` le compile contre `crates/`, et `sh scripts/plugin-lock.sh antigravity` réécrit son `Cargo.lock` après un changement des crates.
