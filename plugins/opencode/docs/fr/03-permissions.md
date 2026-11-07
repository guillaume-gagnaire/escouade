# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni accès au disque.

C’est le cœur d’Escouade qui trouve `opencode` et lance `opencode serve`, une fois pour les agents du compte, dans son objet Job ; il l’arrête avec le dernier agent. Le plugin décrit ce lancement (sa config, le mot de passe), et le cœur le vérifie avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé tenue, une discussion tenue en lecture seule.

## Ce qu’il touche

- **Le serveur d’OpenCode, et lui seul** (`local:shared`) : l’hôte ouvre au plugin le port que le cœur a lu dans la sortie du serveur, tant qu’il tourne, et aucun autre. Ses requêtes passent par `net/fetch`, ses événements par `net/stream`, 16 Mio au plus par réponse : la copie d’une session plus lourde que cela est refusée.
- Il ne lit aucun fichier, et ne voit ni ta config d’OpenCode ni les clés de tes fournisseurs.

## Les secrets qu’il a

- **Le mot de passe du serveur.** Le plugin le tire au hasard et le donne au serveur par une variable (`OPENCODE_SERVER_PASSWORD`) ; il s’en sert pour lui parler.
- **Le jeton MCP de chaque agent.** À l’ouverture de sa session, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les donne au serveur d’OpenCode pour le dossier de l’agent, par une requête : jamais dans les arguments. Il ne peut joindre que ce serveur.

### Ce que protège le mot de passe

Le serveur écoute sur `127.0.0.1` seul, derrière ce mot de passe. Cela tient à l’écart les autres utilisateurs de la machine et les pages d’un navigateur, **pas les agents** : OpenCode donne tout son environnement aux commandes qu’il lance, donc toute commande d’un agent peut lire `OPENCODE_SERVER_PASSWORD`. Avec lui et le port, une telle commande peut changer les règles de sa session, répondre à ses propres cartes, et atteindre les sessions des autres agents du compte sur ce serveur, quel que soit leur dossier. OpenCode n’a pas de réglage qui garde la variable loin de ses commandes.

Une carte et une clôture tiennent donc ce qu’un agent demande par les outils d’OpenCode, pas ce qu’une commande que tu as acceptée fait exprès.

## Ce qu’il accepte seul, et ce qui va à la carte

Le plugin n’accepte rien lui-même : il donne à chaque session les règles de son mode, avant que la session s’ouvre (refusées, pas de session), et c’est OpenCode qui les applique. La dernière règle qui correspond l’emporte.

| Mode | Sans carte |
|---|---|
| Demander | Lire et modifier les fichiers du dossier de l’agent. |
| Édits auto | Pareil, et `git add` et `git commit` dans leurs formes simples. |
| Plan | Lire. Son agent `plan` ne modifie que son plan ; le reste est une carte. |
| Bypass | Tout. |

Les commandes, le web (`webfetch`, `websearch`) et les chemins hors du dossier de l’agent sont des cartes, sauf en Bypass.

**Les formes simples de git**, les seules acceptées en Édits auto : `git add <chemins>`, `git add -A`, `-u`, `--all`, `--update`, `git commit -m <message>` et `-am <message>`, sans rien d’autre. Une seconde option, une redirection, ou ce qu’un shell développerait ou dé-quoterait (`$`, un accent grave, `{`, `^`, `%`…) va à la carte.

**Toujours à la carte, sauf en Bypass** : une modification de ce que git ou OpenCode exécutent ou prennent pour règles, `.git` (un fichier dans un worktree), `.opencode`, `opencode.json` et `opencode.jsonc`, `.husky`, `.githooks`, à toute profondeur, sous les noms courts de Windows et ses flux aussi. Un plugin de `.opencode/plugin` tourne au prochain démarrage de l’instance du dossier, un `opencode.json` donne ses règles à un sous-agent, un hook tourne au prochain `git commit`.

**Une discussion** : toute modification et toute commande refusées par OpenCode lui-même, jamais une carte ; les outils d’un serveur MCP restent.

« Toujours autoriser » fait garder par OpenCode le motif qu’il a proposé, pour le reste de la session ; la carte dit lequel.

## La clôture d’un projet privé

Après les règles du mode, le plugin donne à la session des refus (`deny`) pour chaque chemin clôturé, dans ses deux écritures sous Windows : aux outils qui prennent un chemin (`read`, `edit`, `list`, `external_directory`) et aux commandes qui le nomment. Pour un chemin clôturé dans le worktree, `edit` est aussi refusé tel qu’OpenCode l’écrit, en relatif. `git commit -F` et les options de `git add` et `git commit` qui lisent un fichier sont refusées, en Bypass aussi.

Sous une clôture, « Toujours autoriser » n’est pas proposé : ce qu’OpenCode garde passe après les refus de la clôture et l’emporterait sur eux.

Les règles d’une session reprise ou copiée sont posées avant qu’elle s’ouvre ; un changement de mode les attend. Si le plugin redémarre, le serveur et le tour continuent, et Escouade redonne au plugin la clôture et la lecture seule de chaque agent.

Ce que la clôture ne tient pas : une commande qui atteint un chemin sans le nommer, et ce qu’une commande acceptée fait avec le mot de passe du serveur.

## Ce qu’il ne peut pas faire

- lancer `opencode` ou tout autre programme ;
- joindre une autre adresse que le serveur que le cœur a lancé ;
- lire ou écrire un fichier, lire un secret du trousseau ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- empêcher une commande acceptée de lire le mot de passe du serveur.
