# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `agy` et le lance, puis le relance quand il le faut (`agy` ne prend son modèle, son effort et son mode qu’au lancement, et n’a pas de message qui arrête un tour). Le plugin décrit chaque lancement (`-p=`, le flux en stream-json dans les deux sens, le modèle, l’effort, le mode, la conversation, le dossier propre à l’agent comme `USERPROFILE` et `HOME`, et son sous-dossier `tmp` comme `TEMP`, `TMP` et `TMPDIR`) et les fichiers à écrire dans ce dossier. `agy` 1.5.3 écrit sans règle ni question dans son dossier temporaire : c’est donc celui de l’agent, jamais ton `%TEMP%`, où une écriture est refusée et revient au plugin. Le cœur vérifie chaque description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé dans le lancement, une discussion tenue en lecture seule, un fichier à écrire seulement dans le dossier de l’agent.

## Ce qu’il lit

Par l’hôte d’Escouade, dans le dossier propre à l’agent (`agentHome`, en lecture), et rien d’autre :

- `settings.json` et `mcp_config.json`, pour garder ce qu’`agy` y a mis lui-même (la confiance de son interface) : ses règles, `agentMode` et tout autre serveur MCP que celui d’Escouade sont remplacés, pas repris, et `toolPermission` y est toujours `request-review` (ce qu’aucune règle ne permet demande un accord, refusé en `-p`) ;
- les 4 derniers Mio du journal de la conversation : ses événements ne disent pas tout (les arguments d’un appel, le texte d’une écriture, son raisonnement) ;
- le fichier du plan, au chemin d’écriture que le modèle a donné, seulement s’il est dans ce dossier : il est lu par la racine `agentHome`, jamais dans le dépôt.

Il ne demande plus rien du worktree de l’agent : `agentRepo` n’est plus dans ses permissions (2.2.2), puisqu’il n’accepte plus seul ni `git add` ni `git commit`. Pour une écriture qu’il accepterait seul, il demande au cœur (`fs/fileOnly`, 2.2.4) si elle n’atteint qu’un fichier : un fichier qui est là, ou un chemin absent sans sous-module de l’index de git dessous. Le cœur ne lui rend qu’un oui ou un non, et seulement pour un chemin du dossier de l’agent.

Ton `~/.gemini` n’est jamais lu ni écrit. La connexion Google est dans le trousseau du système, que le plugin ne lit jamais.

## Ce qu’il fait écrire

Il n’écrit rien lui-même. Il décrit au cœur ce qu’il faut écrire dans le dossier de l’agent, et le cœur vérifie que c’est bien là : les règles de permission du mode, celles que tu as permises pour la reprise d’un tour, et le seul serveur MCP d’Escouade avec son jeton. « Pour cet agent » n’est écrit nulle part : le plugin le garde tant qu’il tourne.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les fait écrire dans `mcp_config.json`, dans le dossier de l’agent : jamais dans les arguments. Le cœur retire le jeton de ce dossier quand le process s’arrête. Le plugin n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune clé de compte.** La clé d’API d’un compte est donnée à `agy` par le cœur (`GEMINI_API_KEY`) ; le plugin sait seulement que le compte passe par une clé.

## Ce qu’il accepte seul, et ce qui va à la carte

Pendant un tour, rien ne va à la carte : `agy` fait ce que ses règles permettent et refuse le reste. L’essai réel d’`agy` 1.5.3 (2.2.3, refait en 2.2.4) a mesuré comment il juge ses règles :

- une règle `command(…)` tient la commande entière, pas son début : `;`, `&&`, `&`, `|`, une redirection, `$( )`, un retour à la ligne ou des mots en plus après la commande permise sont refusés ;
- une règle `write_file(<chemin>)` couvre ce qui est sous ce chemin, composant par composant : `<dossier>2` n’est pas couvert, `<fichier>\..\.git\x` non plus, mais `<dossier>\.git` l’est, que le dossier existe ou non ;
- un refus (`deny`) l’emporte sur une règle permise, mais seulement pour le nom qu’il dit : `<dossier>\GIT~1` (nom court) ou `<dossier>\.git.` (point final) passent un refus de `<dossier>\.git` ;
- `*` seul vaut tout ; ni `*` au milieu d’un texte, ni crochets, ni accolades ne sont des jokers.

Le plugin juge lui-même chaque refus, et n’accepte d’office que ce que la règle de l’appel tient sans ouvrir plus que l’appel jugé. Il reste prudent avec les commandes : aucune n’est acceptée d’office, même si l’essai montre qu’une règle tient la commande entière. Hors de Bypass, il ne fait écrire aucune règle `allow` à lui, et `agy` lit sans demander (son mode `plan` ne change rien). `agy` 1.5.3 dit parfois fini (`DONE`, sans erreur) un appel qu’il a refusé : le plugin le retrouve par les refus que liste la fin du tour (`denied_actions`) et par le transcript, qui ne dit jamais fini un appel refusé ; sa ligne dit le refus, et il a sa carte comme les autres. Un refus listé qu’aucun appel ne rattache n’a pas de carte, mais la conversation dit qu’un appel a été refusé :

| Mode | Ce que les règles permettent |
|---|---|
| Demander, Édits auto, Plan | Rien. |
| Bypass | Tout : `command(*)`, `write_file(*)`, `read_file(*)`, `mcp(*)`, `read_url(*)`, `execute_url(*)`. Rien pour un projet privé. |

À la fin d’un tour, le plugin juge chaque refus :

- refusé d’office, sans carte : un appel qui nomme un chemin clôturé, par son texte ou par son chemin résolu contre le dossier de l’agent (`..` compris), un mot de sa commande compris (`type ..\prive\secret.txt`) ; tout, dans une discussion ;
- accepté sans carte, si le chemin qu’il nomme est déjà absolu et normal (ni relatif, ni `.`, ni `..`, ni séparateur final, ni séparateurs mêlés) : en Bypass, tout sauf une commande ; une lecture dans un dossier des fichiers joints ; en Demander et Édits auto, l’écriture strictement dans le dossier de l’agent d’un fichier qui y est, ou d’un chemin absent sans sous-module dessous (ce que dit le cœur d’Escouade, sans que le plugin lise le worktree), sauf dans `.git` (toute casse, nom court, le fichier `.git` d’un worktree), `.husky`, `.githooks`, `.agents` et `.gemini`, sauf le dossier lui-même (`C:\wt`, `C:\wt\`, `C:\wt\.`, `.`, une chaîne vide) et sauf un nom par lequel ces dossiers commencent (`.g`) ; exactement un appel permis « Pour cet agent » ;
- une carte pour le reste, et pour toute commande : `git add` et `git commit` en Édits auto, chaque commande en Bypass avec un projet privé. L’écriture d’un dossier (`C:\wt\sub`), d’un sous-module même jamais initialisé, de ce qui en tient un, ou d’un lien va à la carte : sa règle couvrirait le `.git` qui est dessous, à toute profondeur (2.2.4). Un cœur qui ne sait pas le dire (avant la 2.2) met chaque écriture en carte. « Pour cet agent » n’est offert ni à l’agent d’un projet privé, ni pour une écriture dans les dossiers ci-dessus, du dossier lui-même ou d’un dossier, ni pour un chemin qui n’est pas absolu et normal.

Ce qui est accepté, d’office ou sur la carte, l’est par la règle de l’appel, pour la reprise du tour seulement : la carte dit que cette règle tient pour le tour suivant, et la conversation le dit d’une acceptation d’office. Un « Pour cet agent » rejoué redonne cette règle à chaque fois. Pendant une reprise qui porte une écriture, le plugin fait aussi refuser (`deny`) l’écriture dans chaque dossier gardé à la racine du dossier de l’agent, et dans chacun de ces noms juste sous chaque chemin accordé (`<chemin>\.git`, `<chemin>\.husky`…, 2.2.4), sauf celui où tu as permis une écriture : avec `agy` 1.5.3, un refus l’emporte sur la règle accordée. Ces refus ne tiennent pas contre un nom court ou un point final : c’est pourquoi rien n’est accepté d’office qui puisse avoir quelque chose dessous, et pourquoi la carte d’un dossier dit qu’elle couvre ce qui est dessous. Il ne refuse plus `<chemin accordé>\..` : `agy` 1.5.3 le lit comme le dossier du chemin, ce qui refuserait l’écriture même que tu as permise, et il refuse `<fichier>\..\.git\x` sans cela (2.2.3). `agy` est relancé avant la reprise, la règle est retirée à la fin du tour, et `agy` relancé avant le message suivant. Un appel refusé à nouveau pendant la reprise qui l’autorisait n’est pas redemandé. Une commande, un chemin ou un outil avec `*`, `?`, une parenthèse, un crochet, une accolade ou un retour à la ligne ne devient jamais une règle : son refus tient.

Un changement de mode, un « Stop » et un redémarrage du plugin font réécrire les règles, sans ce qui avait été accordé, et relancer `agy` avant le message suivant.

## La clôture d’un projet privé

Les règles de l’agent refusent la lecture de chaque chemin clôturé (`read_file(<chemin>)` en `deny`), ce qui refuse aussi son écriture : c’est la règle d’`agy`, et un refus passe avant tout, même en Bypass. Le plugin refuse d’office ce qu’`agy` a voulu faire sur ces chemins.

La clôture est dans les règles du dossier de l’agent, réécrites quand le plugin redémarre : elle tient. Une carte montrée avant ce redémarrage ne peut pas reprendre : sa réponse finit le tour.

Ce que la clôture ne tient pas d’office : une commande qui lirait ces chemins sans les nommer ou par un chemin que fait le shell (`$HOME`, `%CD%`), un nom court ou une jonction vers eux. Une telle commande va à une carte, comme toute commande d’un agent clôturé.

La « Vue Antigravity CLI » a les mêmes règles : aucune règle `allow` sauf en Bypass sans projet privé ni discussion, les refus de la clôture, et Édits auto y est le mode par défaut d’`agy`. Ce qu’aucune règle ne permet, son interface le demande. Le fichier `escouade-always.json` de la 2.2.0 n’est plus lu nulle part, et le cœur l’efface.

## Ce qu’il ne peut pas faire

- lancer `agy` ou tout autre programme ;
- ouvrir une connexion réseau ;
- lire ou faire écrire hors du dossier propre à l’agent ;
- lire ton `~/.gemini`, ta connexion Google ou la clé d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement.
