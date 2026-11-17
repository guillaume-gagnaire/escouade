# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/prime-agent/` du dépôt d’Escouade, id `dev.escouade.prime-agent`, licence MIT.

## Ce qu’il est

Un plugin déclaratif : son manifeste seul, sans programme. L’entrée de `contributes.harnesses` qui porte un objet `acp` décrit Prime Agent ; le plugin ACP d’Escouade (`dev.escouade.acp`) le sert. Son id de harness est `prime-agent` : un plugin de la première main garde l’id qu’il déclare (un plugin tiers aurait `plugin:<plugin>:<harness>`).

Ce que dit le manifeste :

- `locate.names: ["prime-agent"]`, `tested` 0.10.3 ;
- un seul mode, `bypassPermissions`, avec son sens en français et en anglais ; pas de `readOnly` ;
- pas d’`accountVar` : un seul compte (voir l’onglet Installer) ;
- `install` (ses deux scripts officiels) et `updateCommand` (`prime-agent update`) ;
- `acp` : `args: ["--mode", "acp", "--no-session"]`, `fs: false` (il lit et écrit lui-même), pas de `modes` (il n’en a pas en ACP), `modelOption: "model"`, `questionsInText: true`, et `tools: { "Python cell": "Python" }`, le nom que la conversation donne à son unique outil.

## Ce que dit son ACP (Prime Agent 0.10.3, relevé sur le fil)

- `initialize` : `loadSession: false`, `promptCapabilities` (images, contexte embarqué), `mcpCapabilities.http`, `sessionCapabilities.close` ; ni `session/load`, ni `session/list` (annoncés pour une version suivante).
- `session/new` : un id de session, et une option de configuration `model` dont les valeurs sont `["<fournisseur>","<modèle>"]` ; pour un modèle qui raisonne, une seconde option, `thought_level` (`off` à `high`). Le `cwd` demandé est ignoré : la session travaille dans le dossier du process (Escouade le lance dans celui de l’agent).
- `session/set_mode` et `session/set_model` : « Method not found ». `session/set_config_option` change le modèle et le niveau de réflexion.
- Un tour : du texte en `agent_message_chunk` ; chaque cellule en `tool_call` (titre « Python cell », `kind: execute`, son code dans `rawInput.code`) puis `tool_call_update` (sa sortie) ; des `session_info_update` à lui (`_meta.ai.primeintellect.prime-agent` : phases du tour, sous-agents en cours) que le client ignore. Pas de pensée vue avec le modèle de l’essai, pas de plan, ni jetons ni coût.
- `session/cancel` : le tour finit en `cancelled`. `session/close` répond `{}`.
- Jamais de `session/request_permission`, jamais de `fs/*`.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste |
| `test/manifest.test.mjs` | Son test : `node --test` |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence MIT |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- `node --test` dans ce dossier vérifie le manifeste ;
- les tests d’Escouade jouent le client ACP sur un faux Prime Agent (`tests/fixtures/fake-prime-agent.mjs`) et rejouent les flux réels de l’essai (`tests/fixtures/prime-agent/`) : `src-tauri/src/plugins_prime_agent_tests.rs` ; l’e2e `tests/e2e/prime-agent.spec.ts` fait de même dans l’app.

`node plugins/ci.mjs` le vérifie avec les autres : son `node --test`, puis `escouade-plugin validate --first-party` et le banc.

## L’essayer

Installe ce dossier depuis la vue Plugins (« Depuis un dossier… ») et choisis son harness dans le menu « Harness ». Ou, depuis `plugins/`, avec l’`escouade-plugin` de l’app :

```sh
escouade-plugin validate prime-agent --first-party
escouade-plugin test prime-agent --first-party
```
