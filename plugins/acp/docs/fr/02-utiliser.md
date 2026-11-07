# Utiliser

## Un agent sur un harness décrit

Choisis le harness dans le menu « Harness » de « Nouvel agent ». Escouade trouve son programme dans le `PATH`, demande au plugin ACP comment le lancer, vérifie la description et le lance. Le plugin ouvre la session (`session/load` de celle à reprendre quand l’agent le sait, sinon `session/new`), règle le mode et le modèle, puis envoie tes messages. Un message envoyé pendant un tour attend sa fin : un second `session/prompt` l’annulerait.

L’id d’un harness décrit par un plugin tiers est `plugin:<plugin>:<harness>` ; un plugin de la main d’Escouade, comme Goose, garde le sien. Pour l’instant, un harness décrit n’a pas de vue terminal ni de textes rédigés. Les tokens d’un tour sont comptés quand l’agent les donne ; le quota et le coût restent inconnus.

## Les modes

Les modes sont ceux que le manifeste déclare, avec leur sens tel qu’il l’écrit ; chacun a son mode ACP (`session/set_mode`). Ce que l’agent demande devient une carte, sauf ce que les règles d’Escouade acceptent :

| Mode | Ce qu’Escouade accepte sans carte |
|---|---|
| Demander | Rien. |
| Édits auto | Une écriture ou une suppression dont tous les chemins sont dans le dossier de l’agent, hors `.git`. |
| Plan | Rien. |
| Bypass | Tout ce qu’il demande. |

Ce que l’agent fait sans demander, c’est son mode à lui qui le décide : lis le sens que le manifeste donne à chaque mode.

Une carte a « Autoriser », « Pour cet agent » quand l’agent propose de retenir la règle, et « Refuser », avec son diff pour une écriture.

## Une discussion de l’onglet Docs

Une discussion tourne dans le mode ACP qui ne change rien (`readOnlyMode`), et Escouade refuse en plus toute écriture, suppression, déplacement et commande que l’agent demanderait. Un harness dont le manifeste ne dit pas ce mode ne tient pas de discussion.

## Les projets privés

Tout dépend de la façon dont le harness lit et écrit les fichiers :

- **Par Escouade** (`"fs": true`, le défaut) : il lit et écrit ses fichiers par le client (`fs/read_text_file`, `fs/write_text_file`), et l’hôte refuse ceux d’un projet clôturé, quoi que dise le code du plugin. S’il lit ou modifie un fichier lui-même pendant qu’une clôture tient, la conversation le dit, une fois : la clôture ne tient pas pour ces accès.
- **Lui-même** (`"fs": false`, comme Goose) : la clôture ne peut pas tenir. Dès qu’un de tes projets est privé, chaque agent a une clôture, et aucun agent de ce harness ne démarre, quel que soit son projet.

Un plugin de la main d’Escouade peut aussi dire que la clôture tient sur les demandes de l’agent (`fenceOnPermissions`) : une demande qui nomme un chemin clôturé est alors refusée sans carte. Escouade ne croit cela que de sa propre main.

## Décrire un harness

Un auteur décrit un harness ACP dans le manifeste de son plugin, sans code. Chaque entrée de `contributes.harnesses` qui a un objet `acp` est un harness décrit.

```json
{
  "id": "dev.exemple.goose",
  "name": "Goose",
  "version": "1.0.0",
  "escouade": { "api": "1", "min": "2.2.0" },
  "contributes": {
    "harnesses": [{
      "id": "goose",
      "name": "Goose",
      "speaker": "Goose",
      "locate": { "names": ["goose"] },
      "tested": { "oldest": "1.57.0", "newest": "1.57.0" },
      "caps": {
        "modes": [
          { "id": "default", "meaning": { "fr": "…", "en": "…" } },
          "bypassPermissions"
        ],
        "readOnly": true
      },
      "accountVar": "GOOSE_PATH_ROOT",
      "install": { "windows": "…", "unix": "…" },
      "acp": {
        "args": ["acp"],
        "fs": false,
        "modes": { "default": "approve", "bypassPermissions": "auto" },
        "readOnlyMode": "chat"
      }
    }]
  }
}
```

L’entrée du harness :

- `id`, `name`, `speaker` (le nom de ses agents dans les textes), `writer` ;
- `locate.names` : les noms de son programme, cherchés dans le `PATH`. Escouade ne prend d’un plugin tiers ni chemin de programme ni nom qui contient `/`, `\` ou `:` ;
- `versionArgs` (par défaut `--version`) et `tested` : la version lue et les versions essayées ;
- `caps` : ses modes, chacun par son id ou avec son sens (`meaning`, ce que dit le menu des modes), et ce qu’il sait faire (`images`, `readOnly`…) ; ce qui n’est pas dit, il ne le fait pas ;
- `models` : les modèles de son menu (`id`, `label`) ;
- `accountVar` : la variable qui donne à chaque compte son dossier de configuration ;
- `install` et `updateCommand`.

L’objet `acp` :

- `args` : ses arguments ; `env` : des variables à lui, jamais un secret (un manifeste se lit) ; `removeEnv` : celles de l’app qu’il ne doit pas recevoir. Les variables qu’un plugin ne peut pas poser (`PATH`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `ESCOUADE_*`…) sont refusées à un plugin tiers : l’agent ne démarre pas, et dit pourquoi ;
- `fs` : il lit et écrit les fichiers par Escouade, vrai par défaut. Mets `false` pour un agent qui les lit et écrit lui-même, même quand Escouade le lui propose : il ne démarrera alors à côté d’aucun projet privé ;
- `modes` : le mode ACP de chaque mode d’Escouade ; `readOnlyMode` : celui qui ne change rien ;
- `modelOption` : l’option de configuration du modèle (`session/set_config_option`), sinon `session/set_model` ;
- `questionsInText` : l’agent n’a pas d’outil pour poser une question ; son premier message lui dit de finir son tour sur un bloc `<escouade_question>`, qu’Escouade montre comme une carte ;
- `tools` : le nom d’Escouade (celui de Claude Code) de ses outils de genre `other` (`{ "shell": "Bash" }`) ; `subagentTools` : ceux qui lancent un sous-agent ;
- `extensions` : `cursor` et `gemini`, pour les extensions de Cursor CLI et de Gemini CLI.

Pour l’essayer, installe le dossier du plugin depuis la vue Plugins (« Depuis un dossier… »), puis choisis son harness. `escouade-plugin validate <dossier>` lit le manifeste comme l’app.
