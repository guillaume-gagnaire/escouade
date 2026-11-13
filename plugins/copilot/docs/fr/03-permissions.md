# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `copilot` et lance `copilot --server --stdio`. Le plugin décrit le lancement (les options, les variables à poser et à retirer, le premier message), et le cœur vérifie cette description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé tenue, une discussion tenue en lecture seule. Une description qui ne tient pas est refusée et l’agent ne démarre pas.

## Ce qu’il lit

- **Le dépôt de l’agent** (`agentRepo`, en lecture), par l’hôte d’Escouade : pour savoir si sa configuration et, pour un commit, ses hooks sont protégés (`fs/gitProtected`). Le plugin ne lit pas `.git` lui-même : l’hôte répond oui ou non, et refuse un chemin d’un projet clôturé.
- Ce que Copilot écrit et, pour la vue Copilot CLI, les lignes du journal de la session.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. L’entrée d’Escouade est un fichier privé que le cœur écrit pour l’agent (`--additional-mcp-config`) et retire quand son process ou sa vue s’arrête. Pour le process, le plugin met aussi le jeton dans une variable du lancement (`ESCOUADE_MCP_TOKEN`), que l’entrée nommait avant Escouade 2.3. Pour la vue Copilot CLI, le jeton n’est que dans ce fichier : aucune variable ne le donne à ses commandes, et `ESCOUADE_MCP_TOKEN` est parmi celles que Copilot retire de leur sortie (`--secret-env-vars`). Le jeton n’est jamais dans les arguments, et le plugin n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune connexion de compte.** La connexion à GitHub reste dans le trousseau du système. Le plugin n’appelle jamais ce qui donne un jeton en clair (`account.getAllUsers`, `account.getCurrentAuth`) : `auth.getStatus` lui dit seulement comment Copilot est connecté.

## Ce qu’il accepte seul, et ce qui va à la carte

Le plugin répond sans carte, pour le process de l’agent (pas pour la vue Copilot CLI), à ces demandes seules :

- **une lecture** dont tous les chemins sont dans le dossier de l’agent, dans tous les modes, discussion comprise ;
- **une écriture** dont tous les chemins sont dans le dossier de l’agent, aucun dans `.git` ni dans la configuration de Copilot (`.copilot`, `.github/copilot`, `.github/hooks`), quelle que soit leur casse ou leur nom Windows, sauf en Plan et dans une discussion ;
- **`git add` et `git commit`**, en Édits auto et en Auto, quand la commande tourne dans le worktree de l’agent, n’écrit dans aucun fichier par redirection, et n’est que cela, lue comme son shell (PowerShell ou un shell POSIX) la lit : `;`, `|`, un `&` seul, une redirection, `$`, un accent grave ou un saut de ligne vont à la carte. La configuration du dépôt doit être protégée, et pour un commit ses hooks ceux de son `.git` ; pour un agent clôturé, un commit qui lit un fichier (`-F`, `--file`) va à la carte.

Chaque chemin est pris tel que demandé et tel que Copilot l’a résolu, liens suivis. Ce que la politique de Copilot réserve à un humain (`managedApprovalRequired`) et une sortie de son bac à sable (`requestSandboxBypass`) ont toujours leur carte.

Tout le reste est une carte : les autres commandes, le web, les outils MCP, les chemins hors du dossier.

| Mode | Sans carte |
|---|---|
| Demander | Les lectures et les écritures du dossier de l’agent, hors `.git` et la configuration de Copilot. |
| Édits auto | Pareil, et `git add` et `git commit` du worktree. |
| Plan | Les lectures du dossier de l’agent. |
| Auto | Comme Édits auto ; Copilot enchaîne seul les étapes de sa tâche. |
| Bypass | Tout : Copilot ne demande rien (`allow-all`). |

Dans une discussion, seules les lectures du dossier passent ; Escouade refuse les écritures et les commandes.

## La vue Copilot CLI

Le plugin décrit aussi le lancement de l’interface de Copilot (`terminal/launch`), que le cœur vérifie comme celui du process. Ses demandes s’y répondent dans l’interface, par toi ; le plugin n’y donne que ces permissions et ces refus :

| Mode | Permis sans question | Refusé |
|---|---|---|
| Demander, Édits auto | Rien | Toujours : l’écriture dans `.git`, `.copilot`, `.github/copilot`, `.github/hooks` (par leur nom et par le chemin du dossier de l’agent) et dans le dossier des fichiers joints |
| Plan | Rien (`--mode plan`) | Pareil |
| Auto | `git add` et `git commit` (`--mode autopilot`) | Pareil |
| Bypass | Tout (`--allow-all`) | Pareil : un refus l’emporte sur toute permission |
| Discussion | Rien | En plus, toute écriture et toute commande |

Aucune écriture ne passe sans question en Édits auto ni en Auto : `--allow-tool=write` couvrirait le `.git` du worktree, que le process envoie à la carte, et l’aide de Copilot CLI 1.0.98 ne dit pas si un refus par chemin (`write(<dossier>)`) couvre les fichiers sous ce dossier. Les refus restent posés en plus.

Un agent clôturé n’a pas de vue : rien ne tient la clôture dans l’interface de Copilot (ses règles n’ont pas de sorte pour une lecture, ses hooks ne font que regarder), et le plugin refuse son lancement en le disant.

## La clôture d’un projet privé

Le plugin refuse aussitôt, sans carte, une demande qui nomme un chemin clôturé, avec ce mot pour Copilot : « Refusé par Escouade : ce chemin est celui d’un autre projet, et l’un des deux est privé. » Il regarde le chemin lu ou écrit, ceux que la commande peut toucher (`possiblePaths`, `resolvedPaths`), le dossier où elle tourne et son texte. Sous une clôture, il n’offre pas « Pour cette session » : une règle de session ferait passer les demandes suivantes sans qu’il les voie.

Si le plugin redémarre, Escouade lui redonne la clôture et la lecture seule de chaque agent : le refus tient toujours.

Ce que la clôture ne tient pas : une commande qui atteint un chemin sans le nommer, et le mode Bypass, où Copilot ne demande rien. La vue Copilot CLI est refusée à un agent clôturé.

## Ce qu’il ne peut pas faire

- lancer `copilot` ou tout autre programme ;
- ouvrir une connexion réseau, écrire un fichier, lire hors du dépôt de l’agent ;
- lire la connexion GitHub d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- accepter autre chose que ce qui est dit plus haut.
