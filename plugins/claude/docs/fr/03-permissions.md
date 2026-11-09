# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau, ni accès au disque. Son manifeste ne déclare aucune permission, et l’hôte n’accorde à un module que ce que son manifeste déclare.

C’est le cœur d’Escouade qui trouve `claude` sur la machine et le lance. Le plugin lui décrit seulement le lancement (les arguments, le premier message), et le cœur vérifie cette description avant de lancer quoi que ce soit :

- le jeton d’Escouade n’est jamais dans les arguments, que tout le monde peut lire sur la machine ;
- pour un projet privé, la clôture est dans le lancement ;
- pour une discussion, le harness est tenu en lecture seule ;
- un fichier à écrire ne peut l’être que dans le dossier propre à l’agent.

Une description qui ne tient pas est refusée et l’agent ne démarre pas.

## Ce qu’il lit

Seulement ce qu’Escouade lui passe : les lignes que `claude` écrit, et pour la vue Claude Code les lignes du journal de la session. Il ne lit aucun fichier lui-même.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin ne met que le chemin du fichier privé qui les contient dans les arguments (`--mcp-config`) : c’est `claude` qui lit le fichier. Le plugin n’a pas de réseau pour envoyer ce jeton ailleurs.
- **Aucune connexion de compte.** La connexion à Claude (`.credentials.json`, ou le trousseau sous macOS) n’est lue que par le cœur. Le plugin n’en voit rien ; il sait seulement reconnaître les mots d’un tour qui l’a perdue.

## Ce qu’il accepte seul, et ce qui va à la carte

Le plugin n’accepte aucune permission. Chaque demande de `claude` (`can_use_tool`) devient une carte, avec ses boutons, que tu réponds dans Escouade, ou qu’un agent d’Escouade peut répondre par le serveur MCP.

Il répond seul à deux choses où Escouade n’a pas de rôle : une demande de formulaire d’un serveur MCP (`elicitation`) est déclinée, une demande qu’il ne connaît pas est refusée.

Le cœur, lui, accepte sans carte une seule chose : la lecture d’un fichier que tu as joint par son chemin, ce fichier seul.

Tout le reste dépend de Claude Code. Ce qu’il ne demande pas, il le fait sans carte :

| Mode | Sans carte |
|---|---|
| Demander | Les lectures du dossier de l’agent et des dossiers donnés par `--add-dir` (fichiers joints, fichiers du ticket), et ce que tes règles de Claude Code autorisent. |
| Édits auto | Pareil, et les écritures dans le dossier de l’agent. |
| Plan | Les lectures : Claude Code ne modifie rien avant que tu approuves le plan. |
| Auto | Ce que le classifieur de Claude Code juge sûr. |
| Bypass | Tout, sauf ce que ses règles de refus interdisent. |

## La clôture d’un projet privé

Pour un projet privé, le cœur écrit un fichier de réglages propre à l’agent, avec les règles de refus des chemins des autres projets, et, là où le bac à sable de Claude Code tourne, les mêmes chemins pour ce bac à sable. Le plugin doit nommer ce fichier dans le lancement (`--settings`) ; sans lui, le cœur dit que le plugin n’applique pas la clôture et ne lance pas l’agent.

La clôture est dans le lancement de `claude`, pas dans le plugin : si le plugin redémarre, `claude` continue avec les mêmes règles. Quand tu changes « Projet privé », l’agent reprend ses nouvelles règles avant son tour suivant : son process repart, sa conversation reste.

Ce que la clôture ne tient pas : un script qui ouvre un fichier lui-même, là où le bac à sable de Claude Code ne tourne pas (Windows sans WSL) ou n’est pas allumé.

## Ce qu’il ne peut pas faire

- lancer `claude` ou tout autre programme ;
- lire ou écrire un fichier, ouvrir une connexion réseau ;
- lire la connexion d’un compte ou un secret du trousseau ;
- mettre le jeton d’Escouade dans les arguments, ou faire écrire un fichier hors du dossier de l’agent : le cœur refuse le lancement ;
- accepter une permission à ta place.
