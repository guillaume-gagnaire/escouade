# Un harness ACP décrit

Un agent qui parle **ACP** (Agent Client Protocol) devient un harness d’Escouade par un plugin **déclaratif** : son manifeste le décrit, sans une ligne de code, et le plugin ACP livré avec Escouade fait le reste.

Ce plugin (`dev.escouade.acp`) lance l’agent, lui parle et montre sa conversation, ses permissions, ses diffs et ses fins de tour, comme pour Claude Code. L’agent apparaît dans le menu « Harness » à côté de « Nouvel agent ».

## Le manifeste

Le plugin Goose (`plugins/goose/` du dépôt d’Escouade), essayé avec Goose 1.57 :

`plugins/goose/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.goose",
  "name": "Goose",
  "version": "1.0.0",
  "description": {
    "fr": "Goose, l'agent open source de Block, comme harness : décrit par ACP, sans code.",
    "en": "Goose, Block's open source agent, as a harness: described over ACP, no code."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/goose",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "harnesses": [
      {
        "id": "goose",
        "name": "Goose",
        "speaker": "Goose",
        "writer": "Goose",
        "locate": { "names": ["goose"] },
        "tested": { "oldest": "1.57.0", "newest": "1.57.0" },
        "caps": {
          "modes": [
            {
              "id": "default",
              "meaning": {
                "fr": "Goose demande avant chaque outil (son mode approve). Il écrit les fichiers lui-même, sans les faire passer par Escouade.",
                "en": "Goose asks before each tool (its approve mode). It writes files by itself, without passing them through Escouade."
              }
            },
            {
              "id": "acceptEdits",
              "meaning": {
                "fr": "Goose ne demande que pour ce qu'il juge sensible (son mode smart_approve) ; Escouade accepte ses écritures dans le dossier de l'agent.",
                "en": "Goose asks only for what it deems sensitive (its smart_approve mode); Escouade accepts its writes in the agent's folder."
              }
            },
            {
              "id": "bypassPermissions",
              "meaning": {
                "fr": "Goose ne demande rien (son mode auto) : commandes, fichiers, tout le disque. Pas de bac à sable.",
                "en": "Goose asks nothing (its auto mode): commands, files, the whole disk. No sandbox."
              }
            }
          ],
          "readOnly": true
        },
        "accountVar": "GOOSE_PATH_ROOT",
        "install": {
          "windows": "Invoke-WebRequest -Uri https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1 -OutFile download_cli.ps1; ./download_cli.ps1",
          "unix": "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash"
        },
        "updateCommand": "goose update",
        "acp": {
          "args": ["acp"],
          "fs": false,
          "modes": { "default": "approve", "acceptEdits": "smart_approve", "bypassPermissions": "auto" },
          "readOnlyMode": "chat",
          "modelOption": "model",
          "questionsInText": true,
          "tools": { "shell": "Bash", "write": "Write", "edit": "Edit", "text_editor": "Edit", "read": "Read", "tree": "Glob" }
        }
      }
    ]
  }
}
```

Chaque entrée de `contributes.harnesses` avec un objet `acp` est un harness décrit :

- `id`, `name`, `speaker` (le nom de ses agents dans les textes), `writer` (qui rédige les textes) ;
- `locate.names` : les noms de son programme, cherchés dans le PATH ; un plugin déclaratif ne donne jamais de chemin ;
- `versionArgs` (par défaut `--version`) et `tested` : la version lue et les versions essayées, comme pour les harness d’Escouade ;
- `caps` : ses modes, chacun par son id ou avec son sens (`meaning`, ce que dit le menu des modes), et ce qu’il sait faire (`images`, `pdf`, `readOnly`…) ; ce qui n’est pas dit, il ne le fait pas ;
- `models` : les modèles de son menu (`id`, `label`) ;
- `accountVar` : la variable qui donne à chaque compte son dossier de configuration ;
- `install` et `updateCommand` : sa commande d’installation (Windows, Unix) et de mise à jour.

## L’objet `acp`

- `args` : ses arguments (`["acp"]`) ; `env` et `removeEnv` : des variables à lui (jamais un secret : un manifeste se lit), et celles de l’app qu’il ne doit pas recevoir ;
- `fs` : il lit et écrit les fichiers **par Escouade** (`fs/read_text_file`, `fs/write_text_file`), vrai par défaut. Mets `false` pour un agent qui les lit et écrit lui-même, comme Goose ou Cursor CLI, même quand Escouade le lui propose ;
- `modes` : le mode ACP (`session/set_mode`) de chaque mode d’Escouade ; `readOnlyMode` : celui qui ne change rien, pour les discussions de l’onglet Docs ;
- `modelOption` : l’option de configuration du modèle (`session/set_config_option`), sinon `session/set_model` ;
- `questionsInText` : l’agent n’a pas d’outil pour poser une question ; son premier message lui dit de finir son tour sur un bloc `<escouade_question>`, qu’Escouade montre comme une carte de question ;
- `tools` : le nom d’Escouade (celui de Claude Code) de ses outils de genre `other` (`{ "shell": "Bash" }`), lus dans `_meta.<agent>.toolCall.toolName` ou dans leur titre avant « · » ; `subagentTools` : ceux qui lancent un sous-agent ;
- `extensions` : `cursor` pour les extensions `cursor/*` de Cursor CLI.

## Ce qu’Escouade tient

- **Le lancement.** Escouade trouve le programme, demande au plugin ACP comment le lancer, vérifie la description et le lance lui-même. Le serveur MCP d’Escouade est donné à la session (`mcpServers`), son jeton dans un en-tête, sur l’entrée de l’agent : jamais dans ses arguments.
- **Les permissions.** Chaque permission demandée est une carte, avec son diff pour une écriture. « Édits auto » laisse passer les écritures dans le dossier de l’agent, « Bypass » tout ce qu’il demande.
- **Les projets privés.** Escouade refuse les fichiers d’un autre projet que l’agent lit ou écrit par lui. Quand l’agent lit ou écrit un fichier lui-même pendant qu’une clôture tient, la conversation le dit : la clôture ne tient pas pour ces accès. Un harness déclaré `"fs": false` ne démarre pas dans un projet privé.
- **Ce qu’ACP ne dit pas** reste inconnu : le quota, le coût, les comptes. Les jetons d’un tour sont comptés quand l’agent les donne.

Un plugin désinstallé ou désactivé (ou le plugin ACP lui-même) : ses agents restent, marqués « Harness absent », avec leurs conversations. Ils reviennent avec lui. L’id d’un harness décrit est `plugin:<plugin>:<harness>`.

Pour l’instant, un harness décrit n’a pas de vue terminal ni de textes rédigés.

Un harness qui n’est pas décrit par ACP s’écrit en Rust : voir [écrire un harness](plugins/rust#ecrire-un-harness).

## L’essayer

Installe le dossier de ton plugin depuis la vue Plugins (« En développement »), puis choisis son harness dans le menu « Harness ». Avant l’installation, la fenêtre montre le programme que ses agents lanceront (`goose acp`). Le banc des harness d’Escouade joue le même client sur un faux agent ACP décrit par un manifeste (`tests/fixtures/plugins/fake-acp/`).
