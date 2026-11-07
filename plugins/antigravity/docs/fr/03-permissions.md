# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `agy` et le lance, puis le relance quand il le faut (`agy` ne prend son modèle, son effort et son mode qu’au lancement, et n’a pas de message qui arrête un tour). Le plugin décrit chaque lancement (`-p=`, le flux en stream-json dans les deux sens, le modèle, l’effort, le mode, la conversation, le dossier propre à l’agent comme `USERPROFILE` et `HOME`) et les fichiers à écrire dans ce dossier. Le cœur vérifie chaque description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé dans le lancement, une discussion tenue en lecture seule, un fichier à écrire seulement dans le dossier de l’agent.

## Ce qu’il lit

Par l’hôte d’Escouade, dans le dossier propre à l’agent (`agentHome`, en lecture), et rien d’autre :

- `settings.json` et `mcp_config.json`, pour garder ce qu’`agy` y a mis lui-même (la confiance de son interface) : ses règles, `toolPermission`, `agentMode` et tout autre serveur MCP que celui d’Escouade sont remplacés, pas repris ;
- les 4 derniers Mio du journal de la conversation : ses événements ne disent pas tout (les arguments d’un appel, le texte d’une écriture, son raisonnement) ;
- le fichier du plan, au chemin d’écriture que le modèle a donné, dans ce dossier.

Il demande aussi à l’hôte si le git du worktree de l’agent est protégé (`agentRepo`, en lecture : `fs/gitProtected`), avant d’accepter seul un `git add` ou un `git commit`. Il n’y lit aucun fichier.

Ton `~/.gemini` n’est jamais lu ni écrit. La connexion Google est dans le trousseau du système, que le plugin ne lit jamais.

## Ce qu’il fait écrire

Il n’écrit rien lui-même. Il décrit au cœur ce qu’il faut écrire dans le dossier de l’agent, et le cœur vérifie que c’est bien là : les règles de permission du mode, celles que tu as permises pour la reprise d’un tour, et le seul serveur MCP d’Escouade avec son jeton. « Pour cet agent » n’est écrit nulle part : le plugin le garde tant qu’il tourne.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les fait écrire dans `mcp_config.json`, dans le dossier de l’agent : jamais dans les arguments. Le cœur retire le jeton de ce dossier quand le process s’arrête. Le plugin n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune clé de compte.** La clé d’API d’un compte est donnée à `agy` par le cœur (`GEMINI_API_KEY`) ; le plugin sait seulement que le compte passe par une clé.

## Ce qu’il accepte seul, et ce qui va à la carte

Pendant un tour, rien ne va à la carte : `agy` fait ce que ses règles permettent et refuse le reste. `agy` lit une règle comme un début de texte (« commands that start with »), et la façon dont il les juge n’a pas été essayée : le plugin ne compte donc sur aucune règle pour garder quelque chose. Hors de Bypass, il n’en fait écrire aucune à lui, et `agy` lit sans demander (son mode `plan` ne change rien) :

| Mode | Ce que les règles permettent |
|---|---|
| Demander, Édits auto, Plan | Rien. |
| Bypass | Tout : `command(*)`, `write_file(*)`, `read_file(*)`, `mcp(*)`, `read_url(*)`, `execute_url(*)`. Rien pour un projet privé. |

À la fin d’un tour, le plugin juge chaque refus :

- refusé d’office, sans carte : un appel qui nomme un chemin clôturé, par son texte ou par son chemin résolu (`..` compris) ; tout, dans une discussion ;
- accepté sans carte : en Bypass, tout ; une lecture dans un dossier des fichiers joints ; en Demander et Édits auto, une écriture dans le dossier de l’agent (chemin résolu), sauf dans `.git` (toute casse, nom court, le fichier `.git` d’un worktree), `.husky`, `.githooks`, `.agents` et `.gemini` ; en Édits auto, une commande qui n’est que `git add` et `git commit` du worktree, lue comme le font Codex et Copilot (refusée au moindre doute : `;`, `|`, `$`, redirection, retour à la ligne, option avant la sous-commande), quand l’hôte dit son git protégé ; exactement un appel permis « Pour cet agent » ;
- une carte pour le reste. « Pour cet agent » n’est offert ni à l’agent d’un projet privé, ni pour une écriture dans les dossiers ci-dessus.

Ce qui est accepté l’est par la règle exacte de l’appel, pour la reprise du tour seulement : `agy` est relancé avant la reprise, la règle est retirée à la fin du tour, et `agy` relancé avant le message suivant. Un appel refusé à nouveau pendant la reprise qui l’autorisait n’est pas redemandé. Une commande, un chemin ou un outil avec `*`, `?`, une parenthèse ou un retour à la ligne ne devient jamais une règle : son refus tient.

Un changement de mode, un « Stop » et un redémarrage du plugin font réécrire les règles, sans ce qui avait été accordé, et relancer `agy` avant le message suivant.

## La clôture d’un projet privé

Les règles de l’agent refusent la lecture de chaque chemin clôturé (`read_file(<chemin>)` en `deny`), ce qui refuse aussi son écriture : c’est la règle d’`agy`, et un refus passe avant tout, même en Bypass. Le plugin refuse d’office ce qu’`agy` a voulu faire sur ces chemins.

La clôture est dans les règles du dossier de l’agent, réécrites quand le plugin redémarre : elle tient. Une carte montrée avant ce redémarrage ne peut pas reprendre : sa réponse finit le tour.

Ce que la clôture ne tient pas : une commande qui lirait ces chemins sans les nommer, un nom court ou une jonction vers eux.

## Ce qu’il ne peut pas faire

- lancer `agy` ou tout autre programme ;
- ouvrir une connexion réseau ;
- lire ou faire écrire hors du dossier propre à l’agent (de son worktree, il ne sait que ce que l’hôte dit de son git) ;
- lire ton `~/.gemini`, ta connexion Google ou la clé d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement.
