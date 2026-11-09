# Permissions

Cette page dit ce que le plugin fait faire sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Ce plugin n’a pas de programme : rien de lui ne s’exécute, son manifeste est lu. Il ne déclare aucune permission : ni réseau, ni secret, ni fichier.

Ce qu’il fait faire : ses agents lancent `goose acp`, le programme de ta machine, avec tes droits. C’est ce que la fenêtre d’installation montre avant d’installer. Escouade trouve `goose` dans le `PATH` (son manifeste ne donne que ce nom), et le plugin ACP d’Escouade décrit le lancement ; le cœur le vérifie, puis lance Goose.

## Ce qui est lu, et les secrets

Le plugin lui-même ne lit rien et ne reçoit rien. Le plugin ACP qui sert Goose reçoit le jeton MCP de chaque agent, et le donne à la session de Goose dans un en-tête, sur son entrée, jamais dans les arguments (voir la documentation du plugin ACP). La configuration et les clés de Goose restent les siennes : Escouade n’en lit aucune.

## Ce qui passe sans carte, et ce qui va à la carte

Le client ACP qui sert Goose répond seul à ces demandes, et à elles seules :

- **refusées aussitôt** : dans une discussion, une écriture, une suppression, un déplacement ou une commande ;
- **acceptées en Édits auto** : une écriture ou une suppression dont tous les chemins sont dans le dossier de l’agent, et aucun dans `.git` (quelle que soit sa casse ou son nom Windows ; un chemin avec `..` qui en sort n’est pas dedans) ;
- **acceptées en Bypass** : tout.

Tout le reste est une carte. Ce que Goose fait sans demander dépend de son propre mode :

| Mode | Sans carte |
|---|---|
| Demander (`approve`) | Rien : Goose demande avant chaque outil. |
| Édits auto (`smart_approve`) | Ce que Goose ne juge pas sensible, et ses écritures soumises dans le dossier de l’agent, hors `.git`. |
| Bypass (`auto`) | Tout : Goose ne demande rien, sur tout le disque. |

`git add` et `git commit` ne sont pas acceptés d’office : ce sont des commandes.

## La clôture d’un projet privé

Elle ne peut pas tenir : Goose lit et écrit les fichiers lui-même (`"fs": false`), et son manifeste ne dit pas que la clôture tient sur ses demandes. Escouade refuse donc de lancer un agent Goose dès qu’il a une clôture, c’est-à-dire dès qu’un de tes projets est privé, quel que soit le projet de l’agent.

## Ce qu’il ne peut pas faire

- exécuter quoi que ce soit lui-même : il n’a pas de programme ;
- lancer autre chose que ce que son manifeste décrit, `goose acp`, que la fenêtre d’installation t’a montré ;
- faire tourner un agent à côté d’un projet privé.
