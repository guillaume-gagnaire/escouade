# Développer

Cette page est pour qui veut lire, modifier ou tester le plugin. Son code est sous GPL-3.0-or-later, dans le dépôt d'Escouade.

## Son dossier

`plugins/gitea/` du dépôt d'Escouade :

| Fichier | Ce que c'est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : son id `dev.escouade.gitea`, sa version, ses permissions, son compte et son hébergeur |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-gitea`, et le verrou des crates publiées qu'utilise sa compilation |
| `src/lib.rs` | L'hébergeur, et l'adresse de l'API d'une instance (`<instance>/api/v1`) |
| `src/pulls.rs` | Le client de Gitea et de Forgejo : PR, merge et auto-merge, statuts, Actions, relectures et fils, et ce qui dépend de la version ; avec ses tests |
| `README.fr.md`, `README.en.md` | Sa page sur le store et dans l'app |
| `docs/fr/`, `docs/en/` | Cette documentation |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence |

Son programme est un module (`run.wasm`), compilé par `escouade-plugin build` depuis ce dossier et son `Cargo.lock`. Il s'appuie sur le kit des plugins Rust d'Escouade (`crates/escouade-plugin-kit`), qui répond au protocole, demande à Escouade le jeton de chaque compte et garde un client par compte.

## Ses tests

- Les tests de sa crate, dans l'espace de travail de `plugins/` : `cd plugins && cargo test -p escouade-plugin-gitea`.
- Ceux de l'app, qui jouent son module dans l'hôte d'Escouade contre un faux serveur nourri de réponses écrites d'après les documents d'API (`tests/fixtures/forges/gitea/`, dont le README dit comment), une fois comme un Gitea, une fois comme un Forgejo : `src-tauri/src/forge/gitea_tests.rs`. Ils veulent le module compilé d'abord : `sh scripts/plugins-wasm.sh gitea`.
- Tous les plugins de la première main, comme la CI : `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (sous Windows, `escouade-plugin.exe`). Il vérifie sa licence et son changelog, que la version du manifeste est celle de la crate, lance `cargo test`, `escouade-plugin validate` et le banc de conformité.

Ce qui reste à essayer sur un vrai Codeberg et un vrai Gitea, les routes d'Actions avant tout, est dans `docs/TODO-2.2.md` (« Hébergeurs de code »).

## Le lancer seul

Contre le faux Escouade du banc des plugins, avec le `escouade-plugin` de l'app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin test plugins/gitea --first-party --escouade 2.2.0
```

Son id `dev.escouade.gitea` est réservé à la première main, et Escouade refuse un autre plugin du même id. Pour en faire le tien, copie le dossier, donne-lui un id à toi (`io.github.<ton-login>.gitea`), puis charge-le dans la vue Plugins › « En développement » › « Charger un dossier… ».
