# Développer

Pour qui veut lire, tester ou modifier ce plugin, dans le dépôt d’Escouade.

## Son dossier

`plugins/elevenlabs/` :

| Fichier | Ce que c’est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : `run.wasm`, l’hôte `api.elevenlabs.io`, l’activation `voice:elevenlabs`, `store.docs` |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-elevenlabs`, compilée en module WASM (`cdylib`) ; son `Cargo.lock` fixe les crates publiées sur crates.io, celles que le store compile |
| `src/lib.rs` | Une ligne : `escouade_voice::serve::run("elevenlabs", …)`. Tout le service est dans la crate `escouade-voice` |
| `README.fr.md`, `README.en.md` | Sa page sur le store, où la vue Plugins de l’app renvoie |
| `docs/fr/`, `docs/en/` | Ces pages |
| `CHANGELOG.md`, `LICENSE` | Ses versions ; GPL-3.0-or-later |

Le code d’ElevenLabs est dans `crates/escouade-voice` : `online.rs` (la dictée), `live.rs` (le temps réel), `speech.rs` (la voix), `catalog.rs` (les modèles et les voix), `serve.rs` (les appels `voice/*`), `wire.rs` (leur forme). Dans le dépôt, `plugins/Cargo.toml` les remplace par ceux de `crates/` (`[patch.crates-io]`).

## Ses tests

- `cargo test -p escouade-voice`, dans `src-tauri/` : le service sans module, ElevenLabs compris.
- `cargo test -p escouade-plugin-elevenlabs`, dans `plugins/`.
- Dans `src-tauri/` (`cargo test`) : le module contre de faux serveurs (`src/voice/*_tests.rs`), et `src/voice/parity_tests.rs`, qui le compare à ce que faisait le plugin natif (`tests/fixtures/plugins/parity/voice-elevenlabs.json`).
- De bout en bout : `tests/e2e/voice.spec.ts`.

## Le vérifier comme la CI

Lance cargo **depuis `src-tauri/`** : sa `rust-toolchain.toml` fixe la version de Rust. Construis l’outil de l’app, puis lance le contrôle des plugins :

```sh
cd src-tauri && cargo build --bin escouade-plugin && cd ..
ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs --changed
```

(sous Windows, `escouade-plugin.exe`). Pour ce plugin, `ci.mjs` lit sa LICENSE et son CHANGELOG, compare la version du manifeste à celle de la crate, lance `cargo test`, puis `escouade-plugin validate`, `build` (le module, comme le store le compile) et `test` (le banc de conformité, dans l’hôte de l’app), avec `--first-party` pour son identifiant réservé.

## Le lancer seul

Dans `plugins/`, avec l’outil construit ci-dessus :

```sh
../src-tauri/target/debug/escouade-plugin validate elevenlabs --first-party --escouade 2.2.0
../src-tauri/target/debug/escouade-plugin test elevenlabs --first-party --escouade 2.2.0
```

`--escouade` dit la version d’Escouade pour laquelle le juger : sans lui, l’outil prend la sienne, et un outil plus ancien que la 2.2.0 refuse le plugin. `test` compile le module et le joue contre le faux Escouade du banc. Pour l’essayer dans l’app d’un build de développement, `sh scripts/plugins-wasm.sh elevenlabs` écrit son module dans `src-tauri/target/plugins-wasm/elevenlabs/plugin.wasm`, que l’app prend tant que son amorce ne le donne pas.

Une version se publie par le chef d’Escouade : une étiquette `plugins/elevenlabs/vX.Y.Z`, puis sa soumission au store. Ne monte pas la version dans une contribution.
