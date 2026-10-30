# Le protocole

Un plugin qui a un programme parle à Escouade en JSON-RPC 2.0, sur son entrée et sa sortie standard. Cette page décrit ce transport : quand le programme démarre, la poignée de main, les délais, les relances, la mémoire, les secrets et le journal.

## Quand le programme démarre

- **À sa première activation, jamais avant.** Escouade lance le programme de `run` quand un fait de sa liste `activation` se produit (`event:ticket.column`), ou au premier appel qu’il lui fait. Un plugin sans `activation` ne tourne pas ; un plugin sans `run` est déclaratif : rien ne s’exécute.
- **Un process par plugin.** Il sert tous les projets et tous les agents où le plugin est actif : Escouade distingue ses appels par leur `id`.
- **Dans le dossier du plugin**, avec trois variables : `ESCOUADE_PLUGIN_ID` (son id), `ESCOUADE_PLUGIN_API` (la version du protocole, `1`) et `ESCOUADE_VERSION` (celle d’Escouade).
- **Avec tout ce qu’il lance.** Sous Windows, le process est dans un Job object ; sous macOS et Linux, il mène son groupe de processus. Quand Escouade l’arrête, ce qu’il a lancé s’arrête avec lui ; sous Windows, même si Escouade plante.

## Les messages

- Du JSON-RPC 2.0, **un message par ligne** : du JSON en UTF-8, sans retour à la ligne, terminé par `\n`. La sortie standard ne sert qu’au protocole ; écris tes traces sur la sortie d’erreur, qui va au [journal](#le-journal).
- **Des lots dans les deux sens** : un tableau JSON de messages, sur une ligne. Réponds à un lot par un lot ; les réponses peuvent venir dans n’importe quel ordre, chacune avec l’`id` de sa requête.
- Une notification n’a pas d’`id` et n’attend pas de réponse.

```json
{"jsonrpc":"2.0","id":7,"method":"echo","params":{"a":1}}
[{"jsonrpc":"2.0","id":8,"method":"echo","params":1},{"jsonrpc":"2.0","method":"log","params":{"level":"info","message":"prêt"}}]
```

## La poignée de main

Le premier message d’Escouade est `initialize`. Il donne la version du protocole qu’il parle et ce qu’il sait faire :

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "escouade": { "version": "2.1.1" },
    "plugin": { "id": "dev.exemple.slack", "version": "1.2.0" },
    "capabilities": { "batch": true, "secrets": true, "settings": true, "log": true, "attach": true }
  }
}
```

Le plugin répond la version qu’il parle et ses propres capacités :

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"capabilities":{"batch":true}}}
```

- Une autre version, ou une réponse sans version, arrête le plugin, avec la raison : « it speaks version 2 of the protocol; this Escouade speaks version 1 ».
- Sans réponse en 10 secondes, le plugin est arrêté aussi.
- Rien d’autre ne lui est envoyé avant sa réponse : ce qui arrive entre-temps attend dans une file.

## Les délais

Chaque appel d’Escouade a un délai. Passé ce délai, Escouade n’attend plus : **sa réponse par défaut s’applique** (une demande d’autorisation va à l’utilisateur, comme sans plugin), et une réponse tardive est ignorée.

| Méthode | Délai |
| --- | --- |
| `initialize` | 10 s |
| `shutdown` | 1 s |
| `permissions/…` (une décision qui fait attendre un agent) | 500 ms |
| `tickets/…`, `forge/…`, `engine/oneShot` (un import, un appel à un service) | 60 s |
| Les autres | 5 s |

Chaque délai dépassé est noté au journal : « test/do: no answer within 300 ms ».

## Ce qu’Escouade sert au plugin

| Méthode | Paramètres | Réponse |
| --- | --- | --- |
| `secrets/get` | `{ "key": "webhook" }` | Le secret, ou `null` si aucun n’est gardé sous cette clé. |
| `settings/get` | `{}`, ou `{ "project": "<id>" }` | Ses réglages, tels que l’utilisateur les a choisis dans l’onglet Réglages de sa page : un objet par clé de `contributes.settings`, sans celles laissées à leur valeur par défaut. Avec un projet (le `project.id` d’un fait), ceux que son [`.escouade.yml`](escouade-yml) lui donne sous `plugins.<id>` passent devant, s’il est activé dans ce projet : seulement les clés qu’il déclare, d’une valeur de leur type, jamais un secret. |
| `log` (notification) | `{ "level": "info", "message": "…" }` | Aucune : la ligne va au journal du plugin. |

Une autre méthode répond l’erreur `-32601` et le note au journal. Les erreurs propres à Escouade : `-32001` pour un secret qui n’est pas celui du plugin, `-32002` pour un trousseau injoignable, `-32602` pour des paramètres qui ne tiennent pas.

## Les secrets

Les secrets d’un plugin sont dans le trousseau du système, sous un nom à lui : `plugin:<id>:<clé>`. Il les demande par `secrets/get`, au moment où il en a besoin.

- **Seulement les siens.** La clé doit être l’une de ses `permissions.secrets` ; un motif finissant par `*` en prend plusieurs (`account:*`). Tout autre clé est refusée par `-32001` et notée au journal, même celle d’un autre plugin écrite en entier : `plugin:dev.autre:token` n’est pas une clé de ses permissions.
- Un plugin n’obtient jamais le secret d’un compte, d’un harness ou d’une intégration d’Escouade.
- Un secret donné au plugin est masqué (`••••`) partout dans son journal, sa sortie d’erreur comprise.

## Les plantages et les relances

- **Un plantage relance le plugin**, trois fois en cinq minutes. Au quatrième, il est arrêté : « stopped after 4 crashes within 5 minutes ». Il ne redémarre plus seul ; le bouton « Relancer » de son journal, dans la vue Plugins, le relance, ses plantages oubliés.
- **Les appels en cours** quand il plante échouent : leur réponse par défaut s’applique.
- **Pendant la relance**, ce qu’Escouade lui envoie attend dans une file de 256 messages, puis part dans l’ordre une fois `initialize` répondu. Une file pleine perd son plus vieux message, et le journal le dit.
- **Ce que chaque agent lui avait attaché** (pour un harness, `agent/attach` : sa session, son mode, son modèle) est redonné au nouveau process avant tout autre message.

## La mémoire

Escouade lit la mémoire du plugin et de tout ce qu’il a lancé toutes les deux secondes, comme celle des agents. Au-delà de 1 Go, le plugin est arrêté, sans relance : « stopped: 1100 MB of memory, past the limit of 1024 MB ».

## La fermeture d’Escouade

Quand Escouade se ferme, il envoie `shutdown` à chaque plugin qui tourne, attend sa réponse une seconde au plus pour tous, puis arrête son process et tout ce qu’il a lancé. Réponds à `shutdown`, puis termine-toi : ce que tu n’as pas écrit avant est perdu.

## Le journal

Chaque plugin a son journal, dans `~/.escouade/plugin-logs/<id>.log` ; au-delà de 1 Mo, il devient `<id>.log.1` et repart de zéro. Ses lignes sont en anglais, comme le journal de l’app, et notent :

- ses démarrages, sa réponse à `initialize`, ses arrêts et leur raison ;
- ses plantages, avec leur code de sortie et ses dernières lignes de sortie d’erreur ;
- chaque ligne de sa sortie d’erreur ;
- ses messages : ses `log`, les secrets qu’il a demandés (jamais leur valeur) ;
- les appels restés sans réponse, ce qu’il a demandé et qu’on lui a refusé, les messages perdus d’une file pleine.

Les 500 dernières lignes se lisent dans l’onglet Journal de sa page, dans la vue Plugins ([installer un plugin](plugins/installer#le-journal)).

Ses méthodes, leurs paramètres, leurs réponses et leurs délais sont décrits par un schéma, [`plugin-protocol-1.json`](/schemas/plugin-protocol-1.json), d’où viennent les types du [SDK en TypeScript](plugins/sdk). Pour vérifier qu’un plugin tient ce protocole sans lancer l’app, joue [le banc de conformité](plugins/testing) : `escouade-plugin test <dossier>`.

## Ce que l’utilisateur change

Un plugin désactivé, une permission retirée ou rendue, une mise à jour installée : son process est arrêté, et le suivant reçoit le manifeste et les permissions de maintenant. Une permission de secret retirée fait répondre `-32001` à `secrets/get` pour cette clé, comme pour un secret qui n’est pas le sien.
