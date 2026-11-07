# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `gemini` et lance `gemini --acp`, avec tes droits : c’est ce que la fenêtre d’installation montre. Le cœur écrit aussi, dans le dossier propre à l’agent, ses réglages et, pour un projet privé ou une discussion, ses règles, puis ajoute `--policy` au lancement. Le plugin décrit le lancement (ses arguments, `GEMINI_CLI_TRUST_WORKSPACE=true`, le premier message), et le cœur vérifie cette description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé tenue, une discussion tenue en lecture seule.

**Ce que coûte `GEMINI_CLI_TRUST_WORKSPACE`.** Sans lui, Gemini ne lance aucun serveur MCP, celui d’Escouade compris, et ne le dit pas. Avec lui, Gemini exécute les hooks et lit les réglages du dépôt ouvert dans le dossier de l’agent, dans tous les projets.

## Ce qu’il lit

- **Le plan de Gemini**, par l’hôte d’Escouade, dans le dossier propre à l’agent (`agentHome`, en lecture) : le fichier que nomme son `exit_plan_mode`, seulement un fichier Markdown d’un dossier `plans`, sans `..`. Rien d’autre de ce dossier.
- Les messages ACP que Gemini écrit. Gemini lit et écrit ses fichiers lui-même (`"fs": false`).

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les donne à la session de Gemini (`session/new`, le jeton dans un en-tête), sur l’entrée de Gemini : jamais dans les arguments. Il n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune clé ni connexion de compte.** Elles sont données à Gemini par le cœur (`GEMINI_API_KEY`, `GOOGLE_APPLICATION_CREDENTIALS`).

## Ce qu’il accepte seul, et ce qui va à la carte

Le client ACP du plugin répond seul à ces demandes de Gemini, et à elles seules :

- **refusées aussitôt** : une demande qui nomme un chemin clôturé, dans tous les modes ; dans une discussion, une écriture, une suppression, un déplacement ou une commande ;
- **acceptées en Demander et en Édits auto** : une écriture ou une suppression dont tous les chemins sont dans le dossier de l’agent, et aucun dans `.git` (quelle que soit sa casse ou son nom Windows ; un chemin avec `..` qui en sort n’est pas dedans) ;
- **acceptées en Bypass** : tout le reste.

Le plan de son `exit_plan_mode` a toujours sa carte. Tout le reste est une carte, avec son diff pour une écriture, « Hors du dossier de l’agent » quand un chemin en sort.

| Mode | Sans carte |
|---|---|
| Demander | Ses lectures (Gemini ne les demande pas) ; ses écritures soumises dans le dossier de l’agent, hors `.git`. |
| Édits auto | Ses écritures (son mode `autoEdit` ne les demande pas), et les mêmes que ci-dessus. |
| Plan | Ses lectures. Son plan arrive sur une carte. |
| Bypass | Tout (son mode `yolo`), sauf ce que les règles d’Escouade refusent. |

`git add` et `git commit` ne sont pas acceptés d’office : ce sont des commandes, et elles ont leur carte en Demander.

Un texte du modèle ne peut que baisser le mode de l’agent, jamais le monter : « [MODE_UPDATE] yolo » dans une réponse est ignoré.

## La clôture d’un projet privé

Elle tient à deux endroits :

- **les règles de Gemini** (`--policy`), écrites par le cœur dans le dossier de l’agent : les chemins clôturés refusés à ses outils de fichiers et aux commandes qui les nomment, dans tous les modes, `yolo` compris, avec « Refused by Escouade: a path of a private project. » ;
- **le plugin**, qui refuse sans carte toute demande qui nomme un chemin clôturé (`fenceOnPermissions`, qu’Escouade ne croit que d’un plugin de sa propre main).

Si le plugin redémarre, Escouade lui redonne la clôture et la lecture seule de chaque agent : elles tiennent toujours, et l’agent garde sa session, son tour et les demandes qui attendent.

Ce que la clôture ne tient pas : un script qui ouvre un fichier lui-même, sans que la commande le nomme.

## Ce qu’il ne peut pas faire

- lancer `gemini` ou tout autre programme ;
- ouvrir une connexion réseau, écrire un fichier ;
- lire autre chose, dans le dossier de l’agent, qu’un plan Markdown d’un dossier `plans` ;
- voir la clé ou la connexion d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- laisser un texte du modèle monter le mode de l’agent.
