# Utiliser

## Lancer un agent

Choisis « Goose » dans le menu « Harness » de « Nouvel agent ». Chaque agent a son `goose acp`. Son modèle est celui que sa session propose, par son option de configuration `model`.

Un message envoyé pendant un tour attend sa fin. Goose n’a pas d’outil pour poser une question en ACP : son premier message lui dit de finir son tour sur sa question, qui arrive en carte, et ta réponse lui est envoyée. Ses outils `shell`, `write`, `edit`, `read` et `tree` apparaissent sous les noms `Bash`, `Write`, `Edit`, `Read` et `Glob`, chaque écriture avec son diff.

ACP ne dit ni le quota ni le coût : les tokens de chaque tour sont comptés quand Goose les donne. Pour l’instant, un harness décrit n’a pas de vue terminal ni de textes rédigés.

## Les modes

Goose a trois modes dans Escouade ; pas de Plan, pas d’Auto. Goose lit et écrit les fichiers lui-même : ce qu’il demande arrive en carte, sauf ce que les règles d’Escouade acceptent.

| Mode | Chez Goose |
|---|---|
| Demander | Son mode `approve` : il demande avant chaque outil, et chaque demande est une carte. |
| Édits auto | Son mode `smart_approve` : il ne demande que pour ce qu’il juge sensible ; Escouade accepte ses écritures dans le dossier de l’agent, hors `.git`. |
| Bypass | Son mode `auto` : il ne demande rien. Commandes, fichiers, tout le disque. Pas de bac à sable. |

## Une discussion de l’onglet Docs

Une discussion tourne dans son mode `chat`, qui n’appelle aucun outil, et Escouade refuse en plus toute écriture et toute commande qu’il demanderait.

## Les projets privés

Goose écrit les fichiers lui-même, sans les faire passer par Escouade (son manifeste le dit : `"fs": false`). La clôture des projets privés ne tiendrait pas : **dès qu’un de tes projets est privé, aucun agent Goose ne démarre**, ni dans ce projet ni dans les autres, car chaque agent reçoit alors les chemins du projet privé à refuser.
