# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve le programme d’un harness décrit et le lance, avec tes droits : la fenêtre d’installation du plugin qui le décrit montre ce programme et ses arguments. Le plugin ACP décrit le lancement (les arguments, les variables, le premier message, `initialize`), et le cœur vérifie cette description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé tenue, une discussion tenue en lecture seule, et pour un plugin tiers aucune variable interdite.

## Ce qu’il lit et écrit

Seulement pour un harness qui passe ses fichiers par Escouade (`"fs": true`), et seulement ce que l’agent lui demande (`fs/read_text_file`, `fs/write_text_file`), par l’hôte d’Escouade :

- **lire** dans le dossier de l’agent (`agent`) et dans son dépôt (`agentRepo`) ;
- **écrire** dans le dossier de l’agent (`agent`), sauf dans une discussion ;
- 4 Mio par fichier au plus ; un chemin qui n’est pas absolu, hors de ces dossiers ou dans un projet clôturé est refusé. L’hôte juge chaque chemin, ses liens suivis, quoi que dise le code du plugin. Chaque écriture et chaque refus va au journal du plugin.

Pour un harness qui lit et écrit lui-même (`"fs": false`), le plugin ne lit ni n’écrit rien. Il ne garde rien sur le disque.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les donne à la session (`session/new`, dans `mcpServers`, le jeton dans un en-tête), sur l’entrée de l’agent : jamais dans ses arguments. Il n’a pas de réseau pour l’envoyer ailleurs.
- **Aucun secret de compte.** Il n’a pas de compte à lui ; les clés d’un harness restent dans sa configuration.

## Ce qu’il accepte seul, et ce qui va à la carte

Le client répond seul à ces demandes de l’agent (`session/request_permission`), et à elles seules :

- **refusées aussitôt** : dans une discussion, une écriture, une suppression, un déplacement ou une commande ; pour un harness de la main d’Escouade qui déclare `fenceOnPermissions`, une demande qui nomme un chemin clôturé ;
- **acceptées en Édits auto** : une écriture ou une suppression dont tous les chemins sont dans le dossier de l’agent, et aucun dans `.git` (quelle que soit sa casse, le fichier `.git` d’un worktree, ou ses noms Windows `.git.`, `.git::$INDEX_ALLOCATION`, `git~1` ; un chemin avec `..` qui en sort n’est pas dedans) ;
- **acceptées en Bypass** : tout.

Tout le reste est une carte, avec son diff pour une écriture, « Hors du dossier de l’agent » quand un chemin en sort, le serveur et l’outil pour un outil MCP.

| Mode | Sans carte |
|---|---|
| Demander | Rien de ce que l’agent demande. |
| Édits auto | Les écritures et suppressions dans le dossier de l’agent, hors `.git`. |
| Plan | Rien de ce que l’agent demande. |
| Bypass | Tout ce qu’il demande. |
| un autre mode déclaré | Rien de ce que l’agent demande. |

Ce que l’agent fait sans demander dépend de son propre mode : c’est ce que le sens de chaque mode, dans son manifeste, doit dire. `git add` et `git commit` ne sont jamais acceptés d’office : ce sont des commandes.

Il répond seul aussi, sans carte, à ce qu’Escouade ne sait pas faire : une demande inconnue de l’agent est refusée (`unsupported`).

## La clôture d’un projet privé

- **`"fs": true`** : la clôture tient sur les fichiers que l’agent lit et écrit par le client, refusés par l’hôte avec « Refused by Escouade: {chemin} is in a private project, out of this agent's reach. ». Ce qu’il lit ou modifie lui-même, la conversation le dit, une fois : la clôture ne tient pas pour ces accès.
- **`"fs": false`** : sans `fenceOnPermissions`, l’agent ne démarre pas dès qu’il a une clôture, c’est-à-dire dès qu’un de tes projets est privé. `fenceOnPermissions` n’est cru que d’un plugin de la main d’Escouade : il refuse alors toute demande qui nomme un chemin clôturé, sans tenir ce que l’agent fait sans demander.

Si le plugin redémarre, Escouade lui redonne la clôture et la lecture seule de chaque agent : elles tiennent toujours, et l’agent garde sa session, son tour et les demandes qui attendent.

## Ce qu’Escouade ne prend pas d’un plugin tiers

Pour un harness décrit par un plugin qui n’est pas de sa main, Escouade retire du manifeste le chemin du programme (`locate.program`), les noms qui ne sont pas un nom seul, et `fenceOnPermissions` ; il refuse les variables qu’un plugin ne peut pas poser.

## Ce qu’il ne peut pas faire

- lancer un programme : c’est le cœur qui lance l’agent ;
- ouvrir une connexion réseau ;
- lire hors du dossier de l’agent et de son dépôt, écrire hors du dossier de l’agent, toucher un fichier d’un projet clôturé ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- accepter une commande à ta place, hors Bypass.
