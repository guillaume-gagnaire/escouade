# Développer

Pour qui veut lire ou changer le plugin, ou décrire un autre harness ACP sur son modèle. Il vit dans `plugins/goose/` du dépôt d’Escouade, id `dev.escouade.goose`, licence MIT.

## Ce qu’il est

Un plugin déclaratif : son manifeste seul, sans programme. L’entrée de `contributes.harnesses` qui porte un objet `acp` décrit Goose ; le plugin ACP d’Escouade (`dev.escouade.acp`) le sert. Son id de harness est `goose` : un plugin de la première main garde l’id qu’il déclare (un plugin tiers aurait `plugin:<plugin>:<harness>`).

Ce que dit le manifeste :

- `locate.names: ["goose"]`, `tested` 1.57.0 ;
- ses trois modes, chacun avec son sens en français et en anglais, et `readOnly` ;
- `accountVar: "GOOSE_PATH_ROOT"` : le dossier de configuration de chaque compte ;
- `install` et `updateCommand` ;
- `acp` : `args: ["acp"]`, `fs: false` (il lit et écrit lui-même), `modes` (`approve`, `smart_approve`, `auto`), `readOnlyMode: "chat"`, `modelOption: "model"`, `questionsInText: true`, et `tools`, le nom d’Escouade de ses outils de genre `other` (`shell` → `Bash`, `write` → `Write`, `edit` et `text_editor` → `Edit`, `read` → `Read`, `tree` → `Glob`).

La documentation du plugin ACP dit chaque clé de l’objet `acp`, et ce qu’Escouade tient pour un harness décrit.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste |
| `test/manifest.test.mjs` | Son test : `node --test` |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence MIT |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- `node --test` dans ce dossier vérifie le manifeste : un harness décrit par ACP, sans programme à lui ; chaque mode avec son mode de Goose et son sens dans les deux langues ; la lecture seule par `chat` ; `fs` à `false` ;
- les tests d’Escouade jouent le même client sur un faux agent ACP (`src-tauri/src/plugins_acp_tests.rs`), et son essai réel sur Goose est `real_goose`, au même endroit, à lancer à la main.

`node plugins/ci.mjs` le vérifie avec les autres : son `node --test`, puis `escouade-plugin validate --first-party` et le banc.

## L’essayer

Installe ce dossier depuis la vue Plugins (« Depuis un dossier… ») et choisis son harness dans le menu « Harness ». Ou, depuis `plugins/`, avec l’`escouade-plugin` de l’app :

```sh
escouade-plugin validate goose --first-party
escouade-plugin test goose --first-party
```
