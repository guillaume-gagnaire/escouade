# Développer isola

`dev.escouade.isola` est un plugin de la main d’Escouade, dans `plugins/isola/` de son dépôt. C’est un module WASM (`run.wasm`) écrit en Rust sur `escouade-plugin-kit`, compilé depuis ce dossier et lancé dans l’hôte d’Escouade. Sa version est la sienne.

C’est le premier **environnement de lancement** (`contributes.launchEnvironments`).

## Ce que déclare son manifeste

| Champ | Valeur | Ce qu’en fait l’app |
| --- | --- | --- |
| `detect` | `.isola.toml` | le fichier qui dit qu’un worktree est celui d’isola ; lu en entier pour l’utilisateur avant `up` |
| `program` | `isola` | le programme, cherché sur le PATH par l’app |
| `up` | `isola up` | la commande qu’un lancement de test lance dans un terminal (son entrée n’en est pas un : pas de mot de passe) |

Le module répond à `launch/ls`, `launch/down` et `launch/destroy`. L’app lui prête le worktree le temps d’un appel, sous le nom qu’elle donne dans `agent`, avec sa branche (`branch`) et la langue de ses messages (`lang`) : le module ne tient aucun chemin. Il lance `isola` par `proc/spawn` de l’hôte, et lit sa sortie par `proc/out` et `proc/exit`.

## Son dossier

| Fichier | Ce que c’est |
| --- | --- |
| `escouade-plugin.json` | le manifeste |
| `Cargo.toml` | sa crate, sur `escouade-plugin-kit` |
| `Cargo.lock` | les crates de crates.io que le store compile (`scripts/plugin-lock.sh`) |
| `src/lib.rs` | son code et ses tests unitaires |
| `README.fr.md`, `README.en.md` | sa page, sur le store et dans l’app |
| `docs/fr/`, `docs/en/` | cette documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | ses versions, sa licence (GPL-3.0-or-later) |

Le côté de l’app est `src-tauri/src/launchenv.rs` (et `src-tauri/src/isola.rs`, qui trouve le programme). Les tests des lancements (`src-tauri/src/core_tests.rs`, « isola ») et `src-tauri/src/plugins_isola_tests.rs` font tourner le module dans l’hôte de l’app contre le double de test `tests/fixtures/fake-isola.mjs`, qui tient lieu du programme `isola`.

## Le tester seul

Depuis `plugins/`, contre le faux Escouade du banc des plugins, comme chaque plugin de l’app :

```sh
cargo test -p escouade-plugin-isola
escouade-plugin test isola --first-party --escouade 2.2.0
```

Le banc ne lance pas d’`isola` : `launch/*` est appelé par l’app, avec un worktree qu’elle prête.

`node plugins/ci.mjs` (avec `ESCOUADE_PLUGIN_BIN` qui nomme le `escouade-plugin` de l’app) fait tout cela pour chaque plugin du dossier : sa LICENSE et son CHANGELOG, `cargo test`, `escouade-plugin validate` et le banc.

`--first-party` : son id `dev.escouade.*` est réservé aux plugins d’Escouade. Le store le prend de ce dossier du dépôt d’Escouade seulement ; ses étiquettes sont `plugins/isola/v1.2.3`.
