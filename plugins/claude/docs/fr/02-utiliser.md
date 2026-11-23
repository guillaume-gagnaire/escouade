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

## Les fonctionnalités expérimentales

Dans la page du plugin, onglet Réglages, le groupe « Fonctionnalités expérimentales » allume des drapeaux de Claude Code pour tous les projets ; la page Plugins d’un projet les règle autrement pour lui (« Comme pour tous les projets », « Activé », « Désactivé »). Le `.escouade.yml` d’un dépôt ne les allume jamais.

- **Code Mode** (`CLAUDE_CODE_EXPERIMENTAL_CODE_MODE`, Claude Code 2.1.323 ou plus, 2.1.326 avec les Agent Teams) : Claude écrit du JavaScript qui fait ses appels d’outils (l’outil `REPL`). Il faut un abonnement Claude ou un compte Console, et Opus 5.5, Sonnet 5.5, Haiku 5.5 ou Fable 5.2.
- **Agent Teams** (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS`, 2.1.32 ou plus) : une session mène des coéquipiers.
- **Autres drapeaux** : des noms de `CLAUDE_CODE_EXPERIMENTAL_*` seulement, chacun posé à 1. Tout autre nom est laissé de côté, et la conversation le dit.

Ils ne vont qu’à la vue Claude Code, dans le fichier de réglages privé de son interface : jamais à la conversation d’Escouade, à une discussion ni à un agent de ticket, que Claude Code lance en stream-json, où ces drapeaux ne font rien. Le Code Mode n’est pas passé, et la conversation dit pourquoi, avec le remote control, un compte qui passe par une passerelle ou un fournisseur cloud, un Claude Code trop ancien ou un modèle qui ne le prend pas. La première fois, Claude Code demande dans le terminal de confirmer (« Yes, turn on Code Mode »).

Un agent dont la session commence en Code Mode le garde : Claude Code ne reprend une telle session que dans son interface. Sa conversation le dit (« En Code Mode : cet agent se reprend dans son terminal »), « Vue Escouade » reste grisé, et Escouade ne le relance jamais en stream-json, ni à la reprise automatique ni par un ticket. Son interface le reprend avec le Code Mode, même éteint depuis, et sans le remote control.

## Une discussion de l’onglet Docs

Une discussion ne change rien : Claude Code y est lancé sans ses outils qui écrivent ou lancent une commande (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash`, `PowerShell`, `Monitor`, `EnterWorktree`), quel que soit le mode.

## Les projets privés

Pour un projet privé, Escouade écrit dans un fichier de réglages à l’agent des règles de refus (`Read` et `Edit`) pour les chemins des autres projets, et pour les siens chez les agents des autres projets. Claude Code les applique à ses outils de fichiers et aux commandes dont il lit les chemins (`cat`, `sed`, `Get-Content`…), même en Bypass.

Là où le bac à sable de Claude Code tourne (macOS, Linux, WSL2), les mêmes chemins lui sont refusés aussi (`sandbox.filesystem`) : une fois ce bac à sable allumé, ils tiennent pour toutes ses commandes.

Ses limites :

- sous Windows sans WSL, pas de bac à sable : un script qui ouvre un fichier lui-même passe ;
- les sessions, la mémoire et l’historique de Claude Code sont ceux du compte : pour qu’un projet privé ne les partage pas, donne-lui un compte réservé à lui seul.
