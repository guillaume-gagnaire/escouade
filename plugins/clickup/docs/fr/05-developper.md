# Développer

Cette page est pour qui veut lire, modifier ou tester le plugin. Son code est sous GPL-3.0-or-later, dans le dépôt d'Escouade.

## Son dossier

`plugins/clickup/` du dépôt d'Escouade :

| Fichier | Ce que c'est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : son id `dev.escouade.clickup`, sa version, ses permissions, son compte et sa source |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-clickup`, et le verrou des crates publiées qu'utilise sa compilation |
| `src/lib.rs` | Le client de ClickUp (API v2), sa source du Kanban, et les statuts par défaut de chaque colonne |
| `README.fr.md`, `README.en.md` | Sa page sur le store et dans l'app |
| `docs/fr/`, `docs/en/` | Cette documentation |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence |

Son programme est un module (`run.wasm`), compilé par `escouade-plugin build` depuis ce dossier et son `Cargo.lock`. Il s'appuie sur le kit des plugins Rust d'Escouade (`crates/escouade-plugin-kit`), qui répond au protocole, demande à Escouade le secret du compte et garde un client par compte.

## Ses tests

- Les tests de sa crate, dans l'espace de travail de `plugins/` : `cd plugins && cargo test -p escouade-plugin-clickup`.
- Ceux de l'app, qui jouent son module dans l'hôte d'Escouade contre un faux ClickUp (`src-tauri/src/integrations/fake.rs`) : `src-tauri/src/plugins_clickup_tests.rs`, et les tests d'intégration du cœur (`src-tauri/src/integrations_tests.rs`), qui passent par lui. Ils veulent le module compilé d'abord : `sh scripts/plugins-wasm.sh clickup`.
- Tous les plugins de la première main, comme la CI : `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (sous Windows, `escouade-plugin.exe`). Il vérifie sa licence et son changelog, que la version du manifeste est celle de la crate, lance `cargo test`, `escouade-plugin validate` et le banc de conformité.

## Le lancer seul

Contre le faux Escouade du banc des plugins, avec le `escouade-plugin` de l'app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin test plugins/clickup --first-party --escouade 2.2.0
```

Le banc joue la poignée de main, les délais, le secret demandé, un plantage et l'arrêt, puis un appel de la source (`account/me`, `tickets/containers`…).

Son id `dev.escouade.clickup` est réservé à la première main, et Escouade refuse un autre plugin du même id. Pour en faire le tien, copie le dossier, donne-lui un id à toi (`io.github.<ton-login>.clickup`), puis charge-le dans la vue Plugins › « En développement » › « Charger un dossier… ».
