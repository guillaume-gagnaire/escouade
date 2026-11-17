# Développer

Pour qui veut lire ou changer le plugin. Il vit dans `plugins/unharnessed/` du dépôt d’Escouade, id `dev.escouade.unharnessed`, licence MIT.

## Ce qu’il est

Un plugin déclaratif : son manifeste seul, sans programme. L’entrée de `contributes.harnesses` qui porte un objet `acp` décrit Unharnessed ; le plugin ACP d’Escouade (`dev.escouade.acp`) le sert. Son id de harness est `unharnessed`.

Ce que dit le manifeste :

- `locate.names: ["unharnessed"]`, `tested` 0.9.8 ;
- un seul mode, `bypassPermissions`, avec son sens en français et en anglais ; pas de `readOnly` ;
- `accountVar: "UNHARNESSED_CODING_AGENT_DIR"` : le dossier de chaque compte (identifiants, réglages, sessions) ;
- ni `install` ni `updateCommand` : rien d’officiel ne l’installe seul ;
- `acp` : `args: ["--mode", "acp"]`, `fs: false` (il lit et écrit lui-même), pas de `modes` (il n’en a pas en ACP), `modelOption: "model"`, `questionsInText: true`, et `tools: { "Python cell": "Python" }` (sa cellule, de genre `execute`, montrée « Python » avec son `rawInput.code`, au lieu d’un `Bash` sans commande) ;
- pas de `models` : ce sont ceux des fournisseurs de chacun, et leurs valeurs ACP sont des paires `["<fournisseur>","<modèle>"]`.

## Ce que son ACP dit (Unharnessed 0.9.8)

Relevé sur le fil dans un bac à sable, avec un modèle local :

- `initialize` : `loadSession`, `promptCapabilities` (images, contexte), `mcpCapabilities.http`, `sessionCapabilities` `close` et `list` ; pas de `terminal` demandé, pas de `fs` utilisé.
- `session/new` : un id ACP, et dans `_meta.unharnessed.saved` l’id et le fichier de la session enregistrée ; une option `model` (valeurs en paire), `thought_level` pour un modèle qui raisonne, `fast_mode` pour un modèle qui l’a. Ni `modes` ni `session/set_mode`, ni `session/set_model`.
- `session/update` : `agent_message_chunk`, `agent_thought_chunk` (modèle qui raisonne), `tool_call` / `tool_call_update` (titre « Python cell », genre `execute`, `rawInput.code`, la sortie en texte, `failed` « Tool execution aborted » à l’arrêt), `available_commands_update`, `session_info_update` (dans son `_meta` : jetons, fenêtre, coût, début et fin des tours, tours à lui `origin: "wake"`) ; des notifications à lui, `_unharnessed/kernel/changed` et `_unharnessed/kernel/restored`.
- Jamais de `session/request_permission`.
- `session/cancel` : la cellule `failed`, le tour `cancelled`.
- `session/load` : refusé sauf pour la session que le processus tient, celle qu’on lui a donnée par `--resume <fichier|id>` au lancement ; alors l’historique est rejoué et les variables du noyau reviennent. `session/list` rend les ids enregistrés.
- `mcpServers` de `session/new` : pris, appelés depuis le noyau (`mcp.call_tool`), l’en-tête `Authorization` transmis.

## Le dossier

| Fichier | Ce que c’est |
|---|---|
| `escouade-plugin.json` | Le manifeste |
| `test/manifest.test.mjs` | Son test : `node --test` |
| `CHANGELOG.md`, `LICENSE` | Ses versions, sa licence MIT |
| `README.fr.md`, `README.en.md`, `docs/` | Sa page et sa documentation |

## Ses tests

- `node --test` dans ce dossier vérifie le manifeste : un harness décrit par ACP sans programme ; un seul mode, Bypass, dit dans les deux langues ; ni lecture seule, ni installeur ; `fs` à `false` ; la table de son outil.
- Dans `src-tauri/src/plugins_acp_tests.rs`, sur le faux Unharnessed (`tests/fixtures/fake-unharnessed.mjs`) et les flux réels enregistrés (`tests/fixtures/unharnessed/`) : il répond, sa cellule est un outil Python avec son code, un tour s’arrête, sa session n’est pas reprise par un nouveau processus, ni discussion ni projet privé, un tour à lui ; et `the_gap_…`, le mode « Demander » qui n’est pas refusé sans Bypass.
- `tests/e2e/unharnessed.spec.ts` : le même dans l’app, dont la reprise par `--resume`, un tour à lui, et le refus sans Bypass permis.

`node plugins/ci.mjs` le vérifie avec les autres : son `node --test`, puis `escouade-plugin validate --first-party` et le banc.

## L’essayer

Un dossier installé « Depuis un dossier… » ne peut pas porter un id `dev.escouade.*` : copie le manifeste et change son `id` (comme le fait l’e2e), ou, depuis `plugins/`, avec l’`escouade-plugin` de l’app :

```sh
escouade-plugin validate unharnessed --first-party
escouade-plugin test unharnessed --first-party
```
