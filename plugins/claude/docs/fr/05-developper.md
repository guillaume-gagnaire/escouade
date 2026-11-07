# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/claude/` du dépôt d’Escouade, id `dev.escouade.claude-code`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul. C’est l’exemple d’un harness écrit en Rust : une machine à états qui transforme des lignes en lignes. Ce qu’il ne fait pas reste au cœur.

Il sert le harness que déclare son manifeste (`contributes.harnesses[0]`, id `claude`) : `claude` piloté en stream-json et par son protocole de contrôle, sur le contrat des harness en process.

- Le cœur trouve `claude`, demande au plugin comment le lancer (`engine/launch` : les arguments du flux, le modèle, l’effort, le mode, les consignes ajoutées, la session à reprendre ou copier, les dossiers lus, le serveur d’Escouade par un fichier de l’agent), vérifie la description et lance le process, chaque compte dans son dossier (`CLAUDE_CONFIG_DIR`).
- Le plugin écrit `initialize` en première ligne (`opens`), dit au cœur les commandes et les modèles du compte (`Item::Told`), puis que l’agent est prêt (`Item::Ready`).
- Ce que `claude` écrit arrive par `agent/frames` et repart en éléments de la conversation : textes et raisonnements en direct, appels avec leurs diffs, sous-agents, cartes de permission et de question, fin de tour avec le coût de chaque modèle, quota d’un `rate_limit_event`, limite d’usage, connexion perdue.
- Un changement de modèle, d’effort ou de mode, une interruption, un titre, une tâche arrêtée, le remote control partent en requêtes de contrôle, dont le cœur attend la réponse (`Asked`, `Item::Answer`).
- La « Vue Claude Code » passe aussi par le module (`view/translate`) : le cœur lui donne les lignes du journal de la session.

Reste au cœur : la place de Claude Code au registre, les comptes et leur connexion, le quota lu à l’endpoint d’usage, les modèles d’un compte quand aucun agent ne tourne, les textes rédigés (`claude -p`), le PTY de la vue terminal et ses hooks, le serveur MCP d’Escouade, le plan et les demandes de chaque agent.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : le harness, ses modes, modèles, efforts, son dossier de compte |
| `Cargo.toml` | Le crate `escouade-plugin-claude`, un `cdylib` que le cœur ne lie pas |
| `Cargo.lock` | Les crates publiés sur crates.io : ce que compile le store |
| `src/lib.rs` | Son `run` et les méthodes du contrat |
| `src/client.rs` | Le client de `claude`, une machine à états sans process |
| `src/translate.rs` | Ses lignes en éléments de la conversation |
| `src/view.rs` | Le journal lu pour la « Vue Claude Code » |
| `src/localcmd.rs` | Ce qu’il écrit d’une commande locale (`/compact`, `! ls`) |
| `src/words.rs` | Ce qui se dit dans la langue de l’app, les fenêtres de son quota |
| `src/tests.rs` | Le client et la traduction seuls |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

Trois choses font du crate un module : `[lib] crate-type` dans `Cargo.toml`, `"run": { "wasm": "Cargo.toml" }` dans le manifeste, et dans `src/lib.rs` :

```rust
pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run("claude", json!({ "harnesses": ["claude"] }), move |m, params| plugin.handle(m, params))
}

escouade_plugin_protocol::module!(run);
```

`turn::run` garde le gestionnaire ; chaque tour du module lit les lignes venues et y répond. Rien n’attend dans un tour : `client.rs` et `translate.rs` gardent leur état entre deux lignes, et ce qui est demandé à `claude` revient en ligne plus tard. Pas de `tokio`, de thread, de `std::fs` ni de `std::process`.

## Ses tests

- `src/tests.rs`, avec `cargo test` dans `plugins/` ;
- la parité avec l’ancien plugin natif, ligne par ligne sur les flux enregistrés (`src-tauri/src/plugins_claude_acp_parity_tests.rs`, `tests/fixtures/plugins/parity/claude.json`) ;
- le banc des harness sur le faux `claude` (`src-tauri/src/engine/claude/bench.rs`) et les flux enregistrés (`tests/fixtures/replays/claude/`) ;
- les tests du process réel (`src-tauri/src/process_tests.rs`), ceux des agents dans `core_tests.rs`, presque tous les e2e de `tests/e2e/` ;
- les essais réels, à lancer à la main : `real_claude_models` et `real_claude_view` (`src-tauri/src/tui_tests.rs`), `real_fence`, `tests/e2e/real-harnesses.spec.ts`.

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) : le module comme le store le compile, depuis ce dossier seul et les crates de son `Cargo.lock`, puis le faux Escouade du banc.

```sh
escouade-plugin build claude --first-party --out ../packs
escouade-plugin test claude --first-party --wasm ../packs/dev.escouade.claude-code-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh claude` le compile contre `crates/`, pour les tests de l’app et une build de développement. Après un changement des crates qu’il utilise, `sh scripts/plugin-lock.sh claude` réécrit son `Cargo.lock`.
