# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/acp/` du dépôt d’Escouade, id `dev.escouade.acp`, licence GPL-3.0-or-later.

## Ce qu’il est

Un module WASM (`"run": { "wasm": "Cargo.toml" }`), compilé depuis ce dossier seul. Son code tient en trois lignes sur `escouade_acp::plugin::serve_as("acp", None)` : il n’a pas de harness à lui. La description du harness lui vient avec chaque agent (`described` d’`engine/launch` et d’`agent/attach`), sur le contrat des harness en process.

Le client est le crate `crates/escouade-acp`, le même que servent Cursor CLI, Gemini CLI et Mistral Vibe :

- **la session** : la première ligne du lancement est `initialize` ; sa réponse ouvre la session (`session/load` de celle à reprendre quand l’agent le sait, sinon `session/new`, le serveur MCP d’Escouade donné avec son jeton dans un en-tête), puis le mode et le modèle sont réglés. Un message envoyé avant que la session soit ouverte, ou pendant un tour, l’attend. Les consignes passent avant le premier message ;
- **le tour** : textes et raisonnements rassemblés, appels sous les noms de Claude Code, leur entrée au fil de l’eau, leur fin avec leur résultat (le diff d’une écriture) ; la fin du prompt finit le tour ;
- **les permissions** : à l’utilisateur, sauf celles que les règles d’Escouade répondent (voir l’onglet Permissions) ;
- **les fichiers** que l’agent lit et écrit par le client (`fs/*`), par l’hôte ; ce qu’il fait lui-même pendant une clôture est dit une fois.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste : ses permissions de fichiers et ses limites ; pas de harness |
| `Cargo.toml`, `Cargo.lock` | Le crate `escouade-plugin-acp` et les crates de crates.io que compile le store |
| `src/lib.rs` | Son `run` |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

Dans `crates/escouade-acp/src/` : `agent.rs` (un agent, ses permissions et ses fichiers), `plugin.rs` (le service), `files.rs` (les fichiers par l’hôte), `tools.rs`, `questions.rs`, `texts.rs` (ce qu’il dit), et les extensions `cursor.rs`, `gemini.rs`, `vibe.rs`.

## Ses tests

- ceux de `crates/escouade-acp`, avec `cargo test` ;
- `src-tauri/src/plugins_acp_tests.rs` : le banc des harness sur un faux agent ACP décrit par `tests/fixtures/plugins/fake-acp/escouade-plugin.json`, joué sur le module, et l’essai réel `real_goose`, à lancer à la main ;
- la parité avec l’ancien plugin natif, ligne par ligne sur les flux enregistrés (`src-tauri/src/plugins_claude_acp_parity_tests.rs`, `tests/fixtures/plugins/parity/acp.json`).

`node plugins/ci.mjs` le vérifie comme les autres plugins de l’app : sa version contre celle du crate, `cargo test`, `escouade-plugin validate`, puis le banc sur le module compilé.

## Le lancer seul

Depuis `plugins/`, avec l’`escouade-plugin` de l’app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin build acp --first-party --out ../packs
escouade-plugin test acp --first-party --wasm ../packs/dev.escouade.acp-<version>.wasm
```

`<version>` est celle du manifeste. Dans le dépôt, `sh scripts/plugins-wasm.sh acp` le compile contre `crates/`, et `sh scripts/plugin-lock.sh acp` réécrit son `Cargo.lock` après un changement des crates.
