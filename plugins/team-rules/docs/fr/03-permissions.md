# Permissions de Règles de l’équipe

Ce que le plugin voit, ce qu’il décide, et ce qu’il ne peut pas faire.

## Ce qu’il est

Un programme en TypeScript (`dist/plugin.mjs`), qu’Escouade compile en WASM et fait tourner isolé, dans Escouade : il n’a pas de processus à lui, pas d’accès à ton disque ni au réseau, et pas de Node à installer.

## Ce qu’il demande

| Permission | Ce que dit la fenêtre d’installation |
| --- | --- |
| `run` : `dist/plugin.mjs` | « Tourne dans Escouade, isolé » |
| `decide` | « Décide des autorisations de tes agents », en rouge |

`decide` est la permission la plus forte d’un plugin : une autorisation qu’il donne n’arrive pas jusqu’à toi. Elle se retire dans l’onglet **Permissions** de sa page : il n’est alors plus interrogé. Aucun projet ne peut la donner : un `.escouade.yml` n’active jamais un plugin.

## Les demandes qu’il reçoit

Pour chaque autorisation qu’un agent d’un projet où il est actif demande, Escouade lui envoie :

- la demande : l’outil (`Bash`, `Edit`, `WebFetch`…), sa famille (`command`, `edit`, `read`, `fetch`, `mcp`, `tool`) et ce que l’outil a reçu, ses textes longs coupés (pour une commande : la ligne et sa description) ;
- le projet (son id, son nom, s’il est privé), le ticket (son id, sa clé, son titre) et l’agent (son id, son nom).

Jamais la conversation de l’agent. Un projet privé ne lui envoie rien tant que tu ne l’y as pas activé. Un plan à approuver et une question de l’agent ne lui sont jamais envoyés.

## Les décisions qu’il prend

Il ne juge que les commandes (`kind` à `command`) ; pour tout le reste il répond « à toi ». Pour une commande, il répond selon son `rules.json` : autorisée, refusée avec un message, ou à toi (l’onglet Utiliser dit comment).

Escouade tient le reste :

- il a 500 ms pour répondre (3 secondes, une seule fois, s’il n’avait pas encore démarré) ; au-delà, ou s’il plante, la demande va à toi ;
- un refus, de lui ou d’un autre plugin, l’emporte ; sinon une autorisation suffit ; sinon c’est toi ;
- une autorisation vaut pour cette demande seulement ;
- chaque décision est écrite à son journal.

## Ce qu’il lit et écrit

Il lit son `rules.json`, qu’Escouade lui donne au démarrage, et rien d’autre. Il n’écrit rien.

## Les secrets

Aucun.

## Ce qui part ailleurs

Rien. Il n’a pas de réseau : ses règles et ses décisions restent sur ta machine.

## Ce qu’il ne peut pas faire

- Lancer une commande lui-même : il répond à une demande, il n’en fait pas.
- Autoriser autre chose qu’une commande de ses règles `allow` : une modification de fichier, une page web, un outil MCP te reviennent toujours.
- Passer outre un refus d’un autre plugin.
- Répondre à un plan ou à une question de l’agent.
- Lire tes fichiers, joindre le réseau, garder un secret.
- Agir dans un projet où il est coupé.
