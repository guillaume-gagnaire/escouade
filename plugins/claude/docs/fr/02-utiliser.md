# Utiliser

## Lancer un agent

Claude Code est le harness par défaut d’un projet tant que tu n’en choisis pas un autre (Réglages › Projets › Projet, « Harness par défaut »). « Nouvel agent » le lance ; dès que deux harness sont installés, la flèche de « Nouvel agent » ouvre le menu « Harness ».

L’agent démarre sur le premier compte actif de Claude Code qui a encore du quota. Il garde sa conversation d’un redémarrage à l’autre, et « Dupliquer la conversation » la copie sous une nouvelle session.

Les modèles sont ceux que le `claude` de chaque compte liste : avant la première lecture, Fable, Opus, Sonnet et Haiku. L’effort va de Bas à Max, sauf pour Haiku, qui n’en a pas. Le modèle, l’effort et le mode changent pendant que l’agent tourne.

Un message envoyé pendant un tour attend la fin du tour. Les images et les PDF joints passent, les commandes `/` de Claude Code aussi. Claude Code est le seul harness à avoir le remote control et les tâches de fond.

## Les modes

Escouade donne le mode à Claude Code (`--permission-mode`), et ce sont les permissions de Claude Code qui décident. Ce que Claude Code demande arrive en carte dans Escouade ; ce qu’il ne demande pas, il le fait.

| Mode | Ce que Claude Code fait sans carte |
|---|---|
| Demander (`default`) | Il lit dans le dossier de l’agent sans demander. Les écritures et les commandes demandent, sauf ce que tes règles de Claude Code autorisent. |
| Édits auto (`acceptEdits`) | Il écrit aussi les fichiers du dossier de l’agent sans demander. Les commandes demandent. |
| Plan (`plan`) | Il étudie sans rien modifier, puis propose son plan sur une carte : « Approuver le plan », « Approuver et accepter les édits » ou « Continuer à planifier ». Approuvé, le travail reprend dans le mode que la carte dit. |
| Auto (`auto`) | Le classifieur de Claude Code accepte les actions qu’il juge sûres et demande le reste. |
| Bypass (`bypassPermissions`) | Il ne demande rien. Ses règles de refus (`deny`) tiennent encore. |

Tes règles comptent : celles du `settings.json` du compte et celles du projet (`.claude/settings.json`, `.claude/settings.local.json`) sont lues par Claude Code, comme dans un terminal.

En plus, dans tous les modes :

- un fichier que tu as joint par son chemin est lu sans carte ;
- les dossiers des fichiers joints à l’agent et aux tickets lui sont donnés en lecture (`--add-dir`).

Une carte de permission a « Autoriser », « Refuser » et, quand Claude Code propose une règle, « Toujours autoriser » : la règle est alors celle que Claude Code a proposée.

## La vue Claude Code

La vue Claude Code fait tourner l’agent dans l’interface de Claude Code elle-même, dans un terminal de la fenêtre, sur la même conversation : Escouade arrête l’une et reprend la session dans l’autre. Ses permissions s’y répondent dans cette interface. Un agent de ticket reste dans la vue d’Escouade.

## Une discussion de l’onglet Docs

Une discussion ne change rien : Claude Code y est lancé sans ses outils qui écrivent ou lancent une commande (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash`, `PowerShell`, `Monitor`, `EnterWorktree`), quel que soit le mode.

## Les projets privés

Pour un projet privé, Escouade écrit dans un fichier de réglages à l’agent des règles de refus (`Read` et `Edit`) pour les chemins des autres projets, et pour les siens chez les agents des autres projets. Claude Code les applique à ses outils de fichiers et aux commandes dont il lit les chemins (`cat`, `sed`, `Get-Content`…), même en Bypass.

Là où le bac à sable de Claude Code tourne (macOS, Linux, WSL2), les mêmes chemins lui sont refusés aussi (`sandbox.filesystem`) : une fois ce bac à sable allumé, ils tiennent pour toutes ses commandes.

Ses limites :

- sous Windows sans WSL, pas de bac à sable : un script qui ouvre un fichier lui-même passe ;
- les sessions, la mémoire et l’historique de Claude Code sont ceux du compte : pour qu’un projet privé ne les partage pas, donne-lui un compte réservé à lui seul.
