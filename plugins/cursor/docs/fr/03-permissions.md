# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau, ni accès au disque. Son manifeste ne déclare aucune permission de fichier ni de réseau.

C’est le cœur d’Escouade qui trouve `agent` et lance `agent acp`, avec tes droits : c’est ce que la fenêtre d’installation montre. Le plugin décrit le lancement (ses arguments, les variables de l’app à retirer, `MSYSTEM` et `EXEPATH`, le premier message), et le cœur vérifie cette description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé tenue. Une description qui ne tient pas est refusée et l’agent ne démarre pas.

## Ce qu’il lit

Seulement ce qu’Escouade lui passe : les messages ACP que Cursor écrit. Il ne lit aucun fichier : Cursor lit et écrit les siens lui-même (`"fs": false`).

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les donne à la session de Cursor (`session/new`, le jeton dans un en-tête), sur l’entrée de Cursor : jamais dans les arguments. Il n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune clé de compte.** La clé d’API d’un compte est donnée à Cursor par le cœur (`CURSOR_API_KEY`). Le plugin sait seulement que le compte passe par une clé, pour dire le bon message quand Cursor la refuse.

## Ce qu’il accepte seul, et ce qui va à la carte

Le client ACP du plugin répond seul à ces demandes de Cursor, et à elles seules :

- **refusées aussitôt** : une demande qui nomme un chemin clôturé, dans tous les modes ;
- **acceptées en Édits auto** : une écriture ou une suppression dont tous les chemins sont dans le dossier de l’agent, et aucun dans `.git` (quelle que soit sa casse ou son nom Windows ; un chemin avec `..` qui en sort n’est pas dedans) ;
- **acceptées en Bypass** : tout le reste.

Tout le reste est une carte, avec son diff pour une écriture, et « Hors du dossier de l’agent » quand un chemin en sort.

| Mode | Sans carte |
|---|---|
| Demander | Rien de ce que Cursor demande. Ce qu’il fait sans demander (ses lectures et écritures de fichiers) échappe à Escouade. |
| Édits auto | Les écritures et suppressions soumises dans le dossier de l’agent, hors `.git`. |
| Plan | Rien de ce que Cursor demande ; son plan arrive sur une carte. |
| Bypass | Tout ce qu’il demande, sauf ce qui nomme un chemin clôturé. |

`git add` et `git commit` ne sont pas acceptés d’office : ce sont des commandes, et elles ont leur carte.

## La clôture d’un projet privé

Le manifeste dit que Cursor lit et écrit lui-même (`"fs": false`) et que la clôture tient sur ses demandes (`fenceOnPermissions`). Escouade ne croit ce second point que d’un plugin de sa propre main. Le plugin refuse alors, sans carte, toute demande qui nomme un chemin clôturé : son titre, ses arguments, les chemins qu’elle touche.

Si le plugin redémarre, Escouade lui redonne la clôture de chaque agent : le refus tient toujours, et l’agent garde sa session, son tour et les demandes qui attendent.

Ce que la clôture ne tient pas : les lectures et écritures que Cursor fait sans demander, et une commande qui atteint un chemin sans le nommer.

## Ce qu’il ne peut pas faire

- lancer `agent` ou tout autre programme ;
- lire ou écrire un fichier, ouvrir une connexion réseau ;
- voir la clé d’API ou la connexion d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- empêcher Cursor de lire ou d’écrire un fichier qu’il ne demande pas.
