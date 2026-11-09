# Développer

Cette page est pour qui veut lire, modifier ou tester le plugin. Son code est sous GPL-3.0-or-later, dans le dépôt d'Escouade.

## Son dossier

`plugins/azure-devops/` du dépôt d'Escouade :

| Fichier | Ce que c'est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : son id `dev.escouade.azure-devops`, sa version, ses permissions, son compte, sa source et son hébergeur |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-azure-devops`, et le verrou des crates publiées qu'utilise sa compilation |
| `src/lib.rs` | Ses deux points : l'hébergeur et la source, et ce qu'une adresse d'organisation veut dire |
| `src/pulls.rs`, `src/pulls_tests.rs` | L'hébergeur (API REST 7.1 d'Azure Repos et d'Azure Pipelines) : PR, merge et auto-complete, checks, journaux et relances, fils et relectures ; et ses tests |
| `src/boards.rs`, `src/boards_tests.rs` | La source Azure Boards : types et états, requête WIQL, critères, état et commentaires ; et ses tests |
| `src/html.rs` | Les champs HTML d'Azure Boards lus en Markdown |
| `src/joined.rs` | Plusieurs appels à la fois |
| `README.fr.md`, `README.en.md` | Sa page sur le store et dans l'app |
| `docs/fr/`, `docs/en/` | Cette documentation |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence |

Son programme est un module (`run.wasm`), compilé par `escouade-plugin build` depuis ce dossier et son `Cargo.lock`. Il s'appuie sur le kit des plugins Rust d'Escouade (`crates/escouade-plugin-kit`), qui répond au protocole, demande à Escouade le jeton de chaque compte et garde un client par compte.

## Ses tests

- Les tests de sa crate, dans l'espace de travail de `plugins/` : `cd plugins && cargo test -p escouade-plugin-azure-devops`.
- Ceux de l'app, qui jouent son module dans l'hôte d'Escouade contre un faux serveur (`tests/fixtures/forges/azure/`) : `src-tauri/src/forge/azure_tests.rs` (les PR) et `src-tauri/src/integrations/azure_tests.rs` (Azure Boards). Ils veulent le module compilé d'abord : `sh scripts/plugins-wasm.sh azure-devops`.
- Tous les plugins de la première main, comme la CI : `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (sous Windows, `escouade-plugin.exe`). Il vérifie sa licence et son changelog, que la version du manifeste est celle de la crate, lance `cargo test`, `escouade-plugin validate` et le banc de conformité.

Deux essais réels sont écrits dans les tests de la crate, et ignorés : `real_azure_devops` (`src/pulls_tests.rs` : ouvre une PR en brouillon, la relit, commente, répond, résout, publie une relecture sans vote) et `real_azure_boards` (`src/boards_tests.rs` : crée une Task, la trouve, la déplace, la commente, la met à la corbeille). Ils veulent une organisation et un jeton, avec les variables que leurs commentaires nomment (`ESCOUADE_TEST_AZURE_URL`, `ESCOUADE_TEST_AZURE_PAT`, et `ESCOUADE_TEST_AZURE_REPO` et `ESCOUADE_TEST_AZURE_BRANCH`, ou `ESCOUADE_TEST_AZURE_PROJECT`) :

```sh
cd plugins
cargo test -p escouade-plugin-azure-devops real_azure_devops -- --ignored --nocapture
cargo test -p escouade-plugin-azure-devops real_azure_boards -- --ignored --nocapture
```

Ce qui reste à essayer pour de vrai est dans `docs/TODO-2.2.md` (« Hébergeurs de code »).

## Le lancer seul

Contre le faux Escouade du banc des plugins, avec le `escouade-plugin` de l'app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin test plugins/azure-devops --first-party --escouade 2.2.0
```

Son id `dev.escouade.azure-devops` est réservé à la première main, et Escouade refuse un autre plugin du même id. Pour en faire le tien, copie le dossier, donne-lui un id à toi (`io.github.<ton-login>.azure-devops`), puis charge-le dans la vue Plugins › « En développement » › « Charger un dossier… ».
