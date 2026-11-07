# Développer

Cette page est pour qui veut lire, modifier ou tester le plugin. Son code est sous GPL-3.0-or-later, dans le dépôt d'Escouade.

## Son dossier

`plugins/bitbucket/` du dépôt d'Escouade :

| Fichier | Ce que c'est |
| --- | --- |
| `escouade-plugin.json` | Le manifeste : son id `dev.escouade.bitbucket`, sa version, ses permissions, son compte et son hébergeur |
| `Cargo.toml`, `Cargo.lock` | La crate `escouade-plugin-bitbucket`, et le verrou des crates publiées qu'utilise sa compilation |
| `src/lib.rs` | L'hébergeur et l'adresse de l'API |
| `src/pulls.rs`, `src/pulls_tests.rs` | Le client de Bitbucket Cloud (API 2.0) : PR, merge, statuts, pipelines et journaux, commentaires et relectures, déploiements, limite par heure ; et ses tests |
| `README.fr.md`, `README.en.md` | Sa page sur le store et dans l'app |
| `docs/fr/`, `docs/en/` | Cette documentation |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence |

Son programme est un module (`run.wasm`), compilé par `escouade-plugin build` depuis ce dossier et son `Cargo.lock`. Il s'appuie sur le kit des plugins Rust d'Escouade (`crates/escouade-plugin-kit`), qui répond au protocole, demande à Escouade le jeton de chaque compte et garde un client par compte.

## Ses tests

- Les tests de sa crate, dans l'espace de travail de `plugins/` : `cd plugins && cargo test -p escouade-plugin-bitbucket`.
- Ceux de l'app, qui jouent son module dans l'hôte d'Escouade contre un faux serveur nourri de réponses à la forme que donne la documentation (`tests/fixtures/forges/bitbucket/`) : `src-tauri/src/forge/bitbucket_tests.rs`. Ils veulent le module compilé d'abord : `sh scripts/plugins-wasm.sh bitbucket`.
- Tous les plugins de la première main, comme la CI : `ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs` (sous Windows, `escouade-plugin.exe`). Il vérifie sa licence et son changelog, que la version du manifeste est celle de la crate, lance `cargo test`, `escouade-plugin validate` et le banc de conformité.

Personne ne l'a encore essayé sur bitbucket.org : ce qui reste à essayer est dans `docs/TODO-2.2.md`.

## Le lancer seul

Contre le faux Escouade du banc des plugins, avec le `escouade-plugin` de l'app (`cargo build --bin escouade-plugin` dans `src-tauri/`) :

```sh
escouade-plugin test plugins/bitbucket --first-party --escouade 2.2.0
```

Son id `dev.escouade.bitbucket` est réservé à la première main, et Escouade refuse un autre plugin du même id. Pour en faire le tien, copie le dossier, donne-lui un id à toi (`io.github.<ton-login>.bitbucket`), puis charge-le dans la vue Plugins › « En développement » › « Charger un dossier… ».
