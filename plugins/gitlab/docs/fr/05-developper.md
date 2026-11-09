# Développer

Cette page est pour qui veut lire, modifier ou tester le plugin. Son code est sous GPL-3.0-or-later, dans le dépôt d'Escouade.

## Son dossier

`plugins/gitlab/` du dépôt d'Escouade :

| Fichier | Ce que c'est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : son id `dev.escouade.gitlab`, sa version, ses permissions, son compte, sa source et son hébergeur |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-gitlab`, et le verrou des crates publiées qu'utilise sa compilation |
| `src/lib.rs` | Ses deux points : l'hébergeur et la source, et l'adresse de l'API d'une instance (`<instance>/api/v4`) |
| `src/merges.rs`, `src/merges_tests.rs` | L'hébergeur : MR, merge et auto-merge, pipelines et jobs, relectures et discussions, déploiements, limites ; et ses tests |
| `src/issues.rs` | La source GitLab Issues |
| `README.fr.md`, `README.en.md` | Sa page sur le store et dans l'app |
| `docs/fr/`, `docs/en/` | Cette documentation |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence |

Son programme est un module (`run.wasm`), compilé par `escouade-plugin build` depuis ce dossier et son `Cargo.lock`. Il s'appuie sur le kit des plugins Rust d'Escouade (`crates/escouade-plugin-kit`), qui répond au protocole, demande à Escouade le jeton de chaque compte et garde un client par compte.

## Ses tests

- Les tests de sa crate, dans l'espace de travail de `plugins/` : `cd plugins && cargo test -p escouade-plugin-gitlab`.
- Ceux de l'app, qui jouent son module dans l'hôte d'Escouade contre des réponses enregistrées sur gitlab.com (`tests/fixtures/forges/gitlab/`) : `src-tauri/src/forge/gitlab_tests.rs` (les MR) et `src-tauri/src/integrations/gitlab_tests.rs` (GitLab Issues). Ils veulent le module compilé d'abord : `sh scripts/plugins-wasm.sh gitlab`.
- Tous les plugins de la première main, comme la CI : `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (sous Windows, `escouade-plugin.exe`). Il vérifie sa licence et son changelog, que la version du manifeste est celle de la crate, lance `cargo test`, `escouade-plugin validate` et le banc de conformité.

Ce qui reste à essayer sur une vraie instance est dans `docs/TODO-2.2.md`.

## Le lancer seul

Contre le faux Escouade du banc des plugins, avec le `escouade-plugin` de l'app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin test plugins/gitlab --first-party --escouade 2.2.0
```

Son id `dev.escouade.gitlab` est réservé à la première main, et Escouade refuse un autre plugin du même id. Pour en faire le tien, copie le dossier, donne-lui un id à toi (`io.github.<ton-login>.gitlab`), puis charge-le dans la vue Plugins › « En développement » › « Charger un dossier… ».
