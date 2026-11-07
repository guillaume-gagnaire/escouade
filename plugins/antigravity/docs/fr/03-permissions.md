# Permissions

Cette page dit ce que le plugin fait sur ta machine, et ce qu’il ne peut pas faire.

## Ce qu’il lance

Rien. Le plugin est un module WASM qu’Escouade fait tourner dans son hôte : il n’a ni process, ni commande, ni réseau.

C’est le cœur d’Escouade qui trouve `agy` et le lance, puis le relance quand il le faut (`agy` ne prend son modèle, son effort et son mode qu’au lancement, et n’a pas de message qui arrête un tour). Le plugin décrit chaque lancement (`-p=`, le flux en stream-json dans les deux sens, le modèle, l’effort, le mode, la conversation, le dossier propre à l’agent comme `USERPROFILE` et `HOME`) et les fichiers à écrire dans ce dossier. Le cœur vérifie chaque description avant de lancer quoi que ce soit : le jeton d’Escouade jamais dans les arguments, la clôture d’un projet privé dans le lancement, une discussion tenue en lecture seule, un fichier à écrire seulement dans le dossier de l’agent.

## Ce qu’il lit

Par l’hôte d’Escouade, dans le dossier propre à l’agent (`agentHome`, en lecture), et rien d’autre :

- les règles gardées par « Pour cet agent », `settings.json` et `mcp_config.json` ;
- les 4 derniers Mio du journal de la conversation : ses événements ne disent pas tout (les arguments d’un appel, le texte d’une écriture, son raisonnement) ;
- le fichier du plan qu’`agy` nomme, dans ce dossier.

Ton `~/.gemini` n’est jamais lu ni écrit. La connexion Google est dans le trousseau du système, que le plugin ne lit jamais.

## Ce qu’il fait écrire

Il n’écrit rien lui-même. Il décrit au cœur ce qu’il faut écrire dans le dossier de l’agent, et le cœur vérifie que c’est bien là : les règles de permission du mode, celles que tu as permises, et l’entrée du serveur MCP d’Escouade avec son jeton.

## Les secrets qu’il reçoit

- **Le jeton MCP de l’agent.** À chaque lancement qu’il décrit, Escouade lui donne l’adresse et le jeton de son serveur MCP pour cet agent. Le plugin les fait écrire dans `mcp_config.json`, dans le dossier de l’agent : jamais dans les arguments. Le cœur retire le jeton de ce dossier quand le process s’arrête. Le plugin n’a pas de réseau pour l’envoyer ailleurs.
- **Aucune clé de compte.** La clé d’API d’un compte est donnée à `agy` par le cœur (`GEMINI_API_KEY`) ; le plugin sait seulement que le compte passe par une clé.

## Ce qu’il accepte seul, et ce qui va à la carte

Pendant un tour, rien ne va à la carte : `agy` fait ce que ses règles permettent et refuse le reste. Ce sont donc les règles que le plugin fait écrire qui décident de ce qui passe sans carte :

| Mode | Ce que les règles permettent |
|---|---|
| Demander | Écrire dans le dossier de l’agent (`write_file(<dossier>)`). |
| Édits auto | Son mode `accept-edits`, et les commandes `git add` et `git commit` (`command(git add)`, `command(git commit)`). |
| Plan | Rien de plus : son mode `plan` ne change rien. |
| Bypass | Tout : `command(*)`, `write_file(*)`, `read_file(*)`, `mcp(*)`, `read_url(*)`, `execute_url(*)`. |

Dans tous les modes, les dossiers des fichiers joints à l’agent sont permis en lecture, et `agy` lit sans demander.

À la fin d’un tour, chaque refus devient une carte, sauf :

- un appel qui nomme un chemin clôturé : refusé d’office, sans carte ;
- dans une discussion : aucun refus n’est demandé ;
- ce que les règles d’Escouade acceptent déjà dans le mode (en Édits auto, une écriture dans le dossier de l’agent hors `.git` ; en Bypass, tout) : accepté sans carte.

« Autoriser » permet l’appel pour la reprise du tour ; « Pour cet agent » le garde dans le dossier de l’agent.

## La clôture d’un projet privé

Les règles de l’agent refusent la lecture de chaque chemin clôturé (`read_file(<chemin>)` en `deny`), ce qui refuse aussi son écriture : c’est la règle d’`agy`, et un refus passe avant tout, même en Bypass. Le plugin refuse d’office ce qu’`agy` a voulu faire sur ces chemins.

La clôture est dans les règles du dossier de l’agent : si le plugin redémarre, elle tient. Une carte montrée avant ce redémarrage ne peut pas reprendre : sa réponse finit le tour.

Ce que la clôture ne tient pas : une commande qui lirait ces chemins.

## Ce qu’il ne peut pas faire

- lancer `agy` ou tout autre programme ;
- ouvrir une connexion réseau ;
- lire ou faire écrire hors du dossier propre à l’agent ;
- lire ton `~/.gemini`, ta connexion Google ou la clé d’un compte ;
- mettre le jeton d’Escouade dans les arguments : le cœur refuse le lancement.
