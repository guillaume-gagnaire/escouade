# Développer

Cette page est pour qui veut lire, modifier ou tester le plugin. Son code est sous GPL-3.0-or-later, dans le dépôt d'Escouade.

## Son dossier

`plugins/github/` du dépôt d'Escouade :

| Fichier | Ce que c'est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : son id `dev.escouade.github`, sa version, ses permissions, son compte, sa source et son hébergeur |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-github`, et le verrou des crates publiées qu'utilise sa compilation |
| `src/lib.rs` | Ses deux points : l'hébergeur et la source, et l'adresse de l'API d'une instance |
| `src/pulls.rs`, `src/pulls_tests.rs` | L'hébergeur : PR, merge et auto-merge, checks, journaux et relances d'Actions, relectures, déploiements, limites ; et ses tests |
| `src/issues.rs` | La source GitHub Issues |
| `README.fr.md`, `README.en.md` | Sa page sur le store et dans l'app |
| `docs/fr/`, `docs/en/` | Cette documentation |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence |

Son programme est un module (`run.wasm`), compilé par `escouade-plugin build` depuis ce dossier et son `Cargo.lock`. Il s'appuie sur le kit des plugins Rust d'Escouade (`crates/escouade-plugin-kit`), qui répond au protocole, demande à Escouade le jeton de chaque compte et garde un client par compte.

## Ses tests

- Les tests de sa crate, dans l'espace de travail de `plugins/` : `cd plugins && cargo test -p escouade-plugin-github`.
- Ceux de l'app, qui jouent son module dans l'hôte d'Escouade contre un faux GitHub : `src-tauri/src/forge/github_tests.rs` (les PR, les checks, les relectures, les déploiements, les limites) et `src-tauri/src/integrations/github_tests.rs` (GitHub Issues). Ils veulent le module compilé d'abord : `sh scripts/plugins-wasm.sh github`.
- Tous les plugins de la première main, comme la CI : `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (sous Windows, `escouade-plugin.exe`). Il vérifie sa licence et son changelog, que la version du manifeste est celle de la crate, lance `cargo test`, `escouade-plugin validate` et le banc de conformité.

## Le lancer seul

Contre le faux Escouade du banc des plugins, avec le `escouade-plugin` de l'app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin test plugins/github --first-party --escouade 2.2.0
```

Contre le vrai github.com, en lecture seule, à travers le plugin tel que l'app le lance (il prend le jeton de `gh` et n'écrit rien) :

```sh
cd src-tauri && ESCOUADE_TEST_REAL_GITHUB=1 cargo test --lib real_github_reads -- --ignored --nocapture
```

Son id `dev.escouade.github` est réservé à la première main, et Escouade refuse un autre plugin du même id. Pour en faire le tien, copie le dossier, donne-lui un id à toi (`io.github.<ton-login>.github`), puis charge-le dans la vue Plugins › « En développement » › « Charger un dossier… ».
