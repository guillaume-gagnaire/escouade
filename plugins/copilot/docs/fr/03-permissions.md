# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `copilot` et lance `copilot --server --stdio`. Le plugin décrit le lancement (les options, les variables à poser et à retirer, le premier message), et le cœur vérifie cette description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé tenue, une discussion tenue en lecture seule. Une description qui ne tient pas est refusée et l’agent ne démarre pas.

## Ce qu’il lit

- **Le dépôt de l’agent** (`agentRepo`, en lecture), par l’hôte d’Escouade : pour savoir si sa configuration et, pour un commit, ses hooks sont protégés (`fs/gitProtected`). Le plugin ne lit pas `.git` lui-même : l’hôte répond oui ou non, et refuse un chemin d’un projet clôturé.
- Ce que Copilot écrit et, pour la vue Copilot CLI, les lignes du journal de la session.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin met le jeton dans une variable du lancement (`ESCOUADE_MCP_TOKEN`), que l’entrée d’Escouade nomme dans son en-tête ; l’entrée elle-même est un fichier privé de l’agent (`--additional-mcp-config`). Le jeton n’est jamais dans les arguments, et le plugin n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune connexion de compte.** La connexion à GitHub reste dans le trousseau du système. Le plugin n’appelle jamais ce qui donne un jeton en clair (`account.getAllUsers`, `account.getCurrentAuth`) : `auth.getStatus` lui dit seulement comment Copilot est connecté.

## Ce qu’il accepte seul, et ce qui va à la carte

Le plugin répond sans carte, pour le process de l’agent (pas pour la vue Copilot CLI), à ces demandes seules :

- **une lecture** dont tous les chemins sont dans le dossier de l’agent, dans tous les modes, discussion comprise ;
- **une écriture** dont tous les chemins sont dans le dossier de l’agent et aucun dans `.git` (quelle que soit sa casse ou son nom Windows), sauf en Plan et dans une discussion ;
- **`git add` et `git commit`**, en Édits auto et en Auto, quand la commande tourne dans le worktree de l’agent, n’écrit dans aucun fichier par redirection, et n’est que cela, lue comme son shell (PowerShell ou un shell POSIX) la lit : `;`, `|`, un `&` seul, une redirection, `$`, un accent grave ou un saut de ligne vont à la carte. La configuration du dépôt doit être protégée, et pour un commit ses hooks ceux de son `.git` ; pour un agent clôturé, un commit qui lit un fichier (`-F`, `--file`) va à la carte.

Chaque chemin est pris tel que demandé et tel que Copilot l’a résolu, liens suivis. Ce que la politique de Copilot réserve à un humain (`managedApprovalRequired`) et une sortie de son bac à sable (`requestSandboxBypass`) ont toujours leur carte.

Tout le reste est une carte : les autres commandes, le web, les outils MCP, les chemins hors du dossier.

| Mode | Sans carte |
|---|---|
| Demander | Les lectures et les écritures du dossier de l’agent, hors `.git`. |
| Édits auto | Pareil, et `git add` et `git commit` du worktree. |
| Plan | Les lectures du dossier de l’agent. |
| Auto | Comme Édits auto ; Copilot enchaîne seul les étapes de sa tâche. |
| Bypass | Tout : Copilot ne demande rien (`allow-all`). |

Dans une discussion, seules les lectures du dossier passent ; Escouade refuse les écritures et les commandes.

## La clôture d’un projet privé

Le plugin refuse aussitôt, sans carte, une demande qui nomme un chemin clôturé, avec ce mot pour Copilot : « Refusé par Escouade : ce chemin est celui d’un autre projet, et l’un des deux est privé. » Il regarde le chemin lu ou écrit, ceux que la commande peut toucher (`possiblePaths`, `resolvedPaths`), le dossier où elle tourne et son texte. Sous une clôture, il n’offre pas « Pour cette session » : une règle de session ferait passer les demandes suivantes sans qu’il les voie.

Si le plugin redémarre, Escouade lui redonne la clôture et la lecture seule de chaque agent : le refus tient toujours.

Ce que la clôture ne tient pas : une commande qui atteint un chemin sans le nommer, le mode Bypass, où Copilot ne demande rien, et la vue Copilot CLI, où ses demandes ne passent pas par Escouade.

## Ce qu’il ne peut pas faire

- lancer `copilot` ou tout autre programme ;
- ouvrir une connexion réseau, écrire un fichier, lire hors du dépôt de l’agent ;
- lire la connexion GitHub d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement ;
- accepter autre chose que ce qui est dit plus haut.
