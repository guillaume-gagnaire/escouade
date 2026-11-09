# Les harness

Un harness est le programme qui fait tourner un agent : Claude Code, Codex, OpenCode, GitHub Copilot CLI, Cursor CLI, Mistral Vibe, Antigravity CLI ou Gemini CLI. Cette page les compare, puis dit pour chacun son installation, son compte, ses modèles et ses modes, son coût, sa vue terminal et ce qu’il ne fait pas.

> [!NOTE]
> Codex, OpenCode et Copilot CLI sont là depuis Escouade 1.8, Cursor CLI depuis la 2.0.1, Mistral Vibe, Antigravity CLI et Gemini CLI depuis la version qui suit la 2.0.5 ; Claude Code depuis le début.

## Choisir un harness

Escouade lance le harness de ta machine, avec ton propre compte : installe-en au moins un et connecte-le une fois, comme pour t’en servir dans un terminal. Réglages › Comptes et harness liste chacun avec son exécutable, sa version et son état (voir [les réglages des harness](settings-claude-code)).

Puis ajoute-lui un compte dans Réglages › Comptes et harness : aucun harness n’en a au départ, et rien ne démarre sur un harness sans compte. « Ajouter un compte… » propose le compte déjà connecté sur la machine, ou un nouveau (voir [plusieurs comptes](accounts#aucun-compte-au-depart)). Le modèle, l’effort et le mode des nouveaux agents se règlent dans la fiche de chaque compte, avec ceux de son harness.

Dès que deux harness sont installés :

- chaque projet a son « Harness par défaut » (Réglages › Projets › Projet), Claude Code sans réglage ;
- la flèche de « Nouvel agent », ou `Ctrl+Maj+N`, ouvre le menu « Harness », qui les numérote (voir [l’interface](interface#la-liste-des-agents)) ;
- la fenêtre d’un ticket a son champ « Harness », celui du projet par défaut, que le pilote auto suit ;
- la carte de chaque agent porte la marque de son harness, dès que des agents tournent sur deux harness.

Un agent garde son harness. Quand plus aucun compte du sien ne peut le prendre, il peut [continuer sur un autre harness](accounts#continuer-sur-un-autre-harness), avec sa conversation, son worktree et sa branche.

Quel que soit le harness, l’agent a la même conversation : la réponse et le raisonnement en direct, ses outils sous les mêmes noms (`Bash`, `Edit` avec son diff, `Read`, `Task`…), les mêmes cartes de permission, de question et de plan à approuver, le bandeau « Plan » avec ses tâches et ses sous-agents, la reprise au redémarrage et « Dupliquer la conversation ». Ce qu’un harness ne sait pas faire n’apparaît pas pour ses agents.

## Comparaison

| | Claude Code | Codex | OpenCode | Copilot CLI | Cursor CLI | Mistral Vibe | Antigravity CLI | Gemini CLI |
|---|---|---|---|---|---| --- | --- | --- |
| Installation | script de Claude Code | `npm install -g @openai/codex` | `npm install -g opencode-ai` | `npm install -g @github/copilot` | script de Cursor | `uv tool install mistral-vibe` | script d’Antigravity | `npm install -g @google/gemini-cli` |
| Testé avec | 2.1.283 à 2.1.301 | 0.163.0 | 1.19.3 | 1.0.98 | 2026.10.22 | 3.0.3 | 1.4.9 | 0.66.0, sans compte (pas encore essayé en vrai) |
| Compte | abonnement Claude ou clé API | compte ChatGPT | les fournisseurs connectés à OpenCode, modèles locaux compris | compte GitHub avec Copilot, Free compris | compte Cursor (`agent login`, ou une clé d’API), Free compris | clé d’API Mistral (`vibe --setup`) | compte Google (connexion de son interface) | compte Google (connexion de son interface, dans le dossier du compte) ou clé d’API Gemini |
| Plusieurs comptes | oui | oui | non | oui | oui, par clé d’API | oui, par clé d’API | non | oui |
| Modèles | ceux que Claude Code liste pour chaque compte | ceux que Codex liste pour ton compte | ceux de tes fournisseurs, avec recherche et favoris | ceux que Copilot liste, avec leur multiplicateur | ceux de ton compte, Auto en tête | ceux de sa session | ceux qu’`agy models` liste, l’effort de Gemini à part | ceux de sa session, Auto en tête |
| Modes | les cinq | les cinq | quatre (pas d’Auto) | les cinq | quatre (pas d’Auto) | quatre (pas d’Auto ; Plan joué par Escouade) | quatre (pas d’Auto) | quatre (pas d’Auto) |
| Ce qui protège | ses permissions | son bac à sable | ses règles de permission | les règles d’Escouade | ses permissions, pour ses commandes | ses permissions ; les règles d’Escouade | ses règles, écrites par Escouade ; demandes une fois le tour fini | ses permissions ; ses règles pour un projet privé |
| Message pendant un tour | attend la fin du tour | entre dans le tour | attend la fin du tour | entre dans le tour | attend la fin du tour | attend la fin du tour | attend la fin du tour | attend la fin du tour |
| Coût | exact | estimé | exact (0 en local) | exact, en crédits d’IA | non donné | exact | non donné (tokens) | non donné (tokens) |
| Quota | fenêtres 5 h et 7 jours | fenêtres 5 h et 7 jours (30 jours en offre gratuite) | aucun : budget par jour | crédits du mois | lien vers son tableau de bord | aucun : budget par jour, lien vers la console de Mistral | non lu | non lu : lien vers sa page d’usage |
| Vue terminal | « Vue Claude Code » | « Vue Codex » | « Vue OpenCode » | « Vue Copilot CLI » | « Vue Cursor CLI », suivi limité | « Vue Mistral Vibe » | « Vue Antigravity CLI » | « Vue Gemini CLI » |
| PDF joints | oui | non | non | non | non | non | non | non |
| Remote control, tâches de fond | oui | non | non | non | non | non | non | non |

## Claude Code

### Installation et compte

Installe Claude Code avec son script (`irm https://claude.ai/install.ps1 | iex` sous Windows, `curl -fsSL https://claude.ai/install.sh | bash` sous macOS et Linux), puis connecte-le avec ton abonnement Claude ou une clé API. Escouade le trouve dans le `PATH` ou aux endroits habituels, même dans WSL (voir [l’installation](installation)). Ses comptes ont chacun leur dossier (`CLAUDE_CONFIG_DIR`) : voir [plusieurs comptes](accounts).

### Modèles et modes

Les modèles sont ceux que le Claude Code de chaque compte liste dans son `/model`, avec leur nom, leurs efforts et le mode « Auto » de chacun : un modèle qu’un compte a en plus (Mythos, par exemple) apparaît dans le Composer de ses agents, un modèle qu’il n’a pas n’y est pas. Escouade les lit au démarrage et à l’ajout d’un compte, sans rien consommer, puis à chaque démarrage d’agent. Le Kanban et les tickets proposent ceux des comptes actifs. Un agent qui passe sur un compte qui n’a pas son modèle prend le modèle par défaut, et sa conversation le dit. Avant la première lecture, la liste est Fable, Opus, Sonnet et Haiku. Les cinq modes ont leur sens d’origine : « Auto » (un classifieur accepte les actions sûres), « Demander », « Plan », « Édits auto » et « Bypass » (voir [les modes de permission](managing-agents#modes-de-permission)). Ce sont les permissions de Claude Code qui protègent.

### Coût, quota et vue

Le coût d’un tour est celui que donne Claude Code. Le quota, ce sont la fenêtre de 5 heures et la fenêtre hebdomadaire de ton compte, dans la barre d’état. La [vue Claude Code](claude-code-view) fait tourner l’agent dans l’interface de Claude Code elle-même. Il est seul à avoir le [remote control](remote-control), les tâches de fond et la lecture des PDF.

Claude Code est un plugin livré avec Escouade (`dev.escouade.claude-code`) : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, un agent de Claude Code dit au lancement qu’il faut le rallumer, et garde sa session.

## Codex

Codex est un plugin livré avec Escouade (`dev.escouade.codex`) : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, un agent de Codex dit au lancement qu’il faut le rallumer, et garde sa session.

### Installation et compte

`npm install -g @openai/codex`, puis `codex login` avec ton compte ChatGPT. Escouade lance le binaire natif du paquet npm (sous Windows, le `codex.exe` du paquet, pas son lanceur) : l’app de bureau de Codex installe souvent une version plus ancienne, que le `PATH` peut trouver d’abord. Chaque agent a son `codex app-server`. Ses comptes ont chacun leur `CODEX_HOME` : voir [les comptes Codex](accounts#comptes-codex).

### Modèles et modes

Les modèles et leurs efforts sont ceux que Codex liste pour chaque compte : Escouade les lit au démarrage et à l’ajout d’un compte, sans lancer d’agent ni rien consommer, les garde d’un démarrage à l’autre et les relit quand le menu « Modèle » s’ouvre sur une liste de plus d’un jour. Avant la première lecture, le menu ne propose que GPT-6-Luna, marqué « liste en cours de lecture ». Le modèle que choisit la `config.toml` du compte (`model = …`) y est aussi, marqué « de config.toml », même quand Codex ne le liste pas. Les modes passent par le bac à sable de Codex :

| Mode | Chez Codex |
|---|---|
| Plan | Le mode Plan de Codex : il étudie, peut te poser des questions, puis propose son plan, sans rien modifier (bac à sable en lecture seule). |
| Demander | Bac à sable : écriture dans le dossier de l’agent, réseau coupé. Une commande qui en sort demande, `git add` et `git commit` compris (Codex protège tout le dossier `.git`). |
| Édits auto | Pareil, et Escouade accepte d’office `git add` et `git commit` dans le worktree de l’agent, quand la commande n’est que cela et que ses hooks sont ceux du dépôt ; le réseau et le reste demandent. |
| Auto | Le relecteur automatique de Codex décide des demandes à ta place, et la conversation dit ce qu’il a approuvé. |
| Bypass | Codex ne demande rien et sort de son bac à sable. |

Une demande de Codex a trois boutons : « Autoriser », « Pour cette session » et « Refuser ». Un message envoyé pendant qu’il travaille entre dans son tour au lieu d’attendre.

### Coût, quota et vue

Codex ne donne pas le coût : Escouade l’estime d’après les tokens du tour et le tarif public d’OpenAI, et l’écrit « ≈ … estimé ». Le quota, ce sont les fenêtres de ton compte (5 heures et 7 jours, ou une seule de 30 jours avec l’offre gratuite). La « Vue Codex » fait tourner l’agent dans l’interface de Codex (voir [la vue terminal](claude-code-view#avec-codex)).

### Ce que Codex ne fait pas

Il ne lit pas les PDF (le bouton les refuse en le disant) et ne propose dans Escouade que la commande `/compact`, qui compacte le contexte de sa conversation. Un agent Codex ne reprend pas sur un autre compte : il attend la remise à zéro, ou continue sur un autre harness. Pas de remote control ni de tâches de fond.

## OpenCode

OpenCode est un plugin livré avec Escouade (`dev.escouade.opencode`) : ses agents conversent par un programme à part qu’Escouade lance et surveille, sur le serveur de leur compte. Il se voit dans la vue Plugins, sous « Livrés avec Escouade ». S’il plante, le serveur et le tour continuent et la conversation le dit ; coupé, un agent d’OpenCode dit qu’il faut le rallumer, et garde sa session.

### Installation et compte

`npm install -g opencode-ai` installe OpenCode 1, le seul que pilote Escouade : OpenCode 2 (`@opencode/cli`) est reconnu et dit « Non pris en charge ». Connecte un fournisseur avec `opencode auth login` (« Se connecter… » dans Réglages › Comptes et harness le lance) ; les modèles locaux d’Ollama marchent aussi. OpenCode a un seul compte dans Escouade, sa config habituelle. Un seul `opencode serve` sert tous les agents OpenCode, protégé par un mot de passe tiré au hasard qui ne quitte pas Escouade ; chaque agent y est une session.

### Modèles et modes

Ses modèles, comme ceux de Codex, Copilot CLI et Cursor CLI, sont lus au démarrage et à l’ajout d’un compte, sans lancer d’agent, gardés d’un démarrage à l’autre et relus quand le menu s’ouvre sur une liste de plus d’un jour. Le menu « Modèle » liste les modèles des fournisseurs connectés, avec une recherche, leur fournisseur et des favoris (★) en tête, aussi réglables dans Réglages › Comptes et harness › OpenCode › « Modèles ». L’effort est la variante du modèle quand il en a. Escouade donne toujours le modèle, pour qu’OpenCode n’en choisisse pas un de lui-même.

OpenCode n’a pas de bac à sable : ce sont ses règles de permission qui suivent le mode, et une commande acceptée peut tout faire.

| Mode | Chez OpenCode |
|---|---|
| Demander | Les fichiers du dossier de l’agent changent sans demander ; les commandes (`git add` et `git commit` compris), le web et les chemins hors du dossier demandent. |
| Édits auto | Pareil, et `git add` et `git commit` passent sans demander. |
| Plan | L’agent `plan` d’OpenCode étudie sans rien modifier, puis te propose son plan à approuver. |
| Bypass | OpenCode ne demande rien. |

Une demande d’OpenCode a « Autoriser », « Toujours autoriser » (la carte dit le motif qu’OpenCode gardera pour la session, comme `git add *`) et « Refuser ». Un message envoyé pendant un tour attend sa fin.

### Coût, budget et vue

Le coût d’un tour est celui qu’OpenCode donne, au prix du fournisseur : 0 avec un modèle local. OpenCode n’a pas de quota : la barre d’état montre le coût du jour de son compte, et « Budget par jour ($) », dans Réglages › Comptes et harness, met le pilote auto en pause jusqu’à minuit une fois atteint (voir [le compte OpenCode](accounts#compte-opencode)). La « Vue OpenCode » branche l’interface d’OpenCode sur la session de l’agent sans arrêter le suivi d’Escouade (voir [la vue terminal](claude-code-view#avec-opencode)).

### Ce qu’OpenCode ne fait pas

Pas de mode « Auto » (OpenCode n’a pas de relecteur), pas de bac à sable, un seul compte, pas de PDF joints, pas d’autre commande `/` que `/compact` dans Escouade, pas de remote control ni de tâches de fond.

## Copilot CLI

Copilot CLI est un plugin livré avec Escouade (`dev.escouade.copilot-cli`) : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, un agent de Copilot dit au lancement qu’il faut le rallumer, et garde sa session.

### Installation et compte

`npm install -g @github/copilot`, puis `copilot login` avec un compte GitHub qui a Copilot (Free compris). Chaque agent a son `copilot --server --stdio`, sans le serveur MCP GitHub que Copilot branche de lui-même, sans accès au dossier temporaire du système et sans les jetons GitHub dans l’environnement de ses commandes. Ses comptes ont chacun leur `COPILOT_HOME` : voir [les comptes Copilot CLI](accounts#comptes-copilot-cli).

### Modèles et modes

Le menu des modèles écrit le multiplicateur de chacun (`×3`), ou son coût relatif sur un compte Free ; `auto` n’est jamais proposé. Le modèle et le mode changent tout de suite, même pendant un tour.

Copilot n’a pas de bac à sable quand Escouade le pilote : ce sont les règles d’Escouade qui répondent sans carte aux demandes sûres.

| Mode | Chez Copilot CLI |
|---|---|
| Demander | Les lectures et écritures du dossier de l’agent passent ; les commandes (`git add` et `git commit` compris), le web, les outils MCP et les chemins hors du dossier demandent. |
| Édits auto | Pareil, et `git add` et `git commit` de son worktree passent. |
| Plan | Copilot étudie et propose un plan sans rien modifier ; ses lectures du dossier passent. |
| Auto | Copilot enchaîne seul jusqu’au bout de sa tâche (autopilot), avec les règles d’« Édits auto ». |
| Bypass | Copilot ne demande rien. |

Une demande de Copilot a « Autoriser », « Pour cette session » et « Refuser » ; pas de « Toujours », qui écrirait une règle durable dans la config de Copilot. Un formulaire d’un serveur MCP devient une carte à champs. Un message envoyé pendant un tour y entre aussitôt.

### Coût, crédits et vue

Le coût d’un tour vient des crédits d’IA de ses appels (un crédit vaut 0,01 $). La barre « crédits » dit ce qui est utilisé ce mois-ci, jusqu’à leur retour le 1ᵉʳ du mois ; le plafond de crédits d’une session atteint devient une carte. La « Vue Copilot CLI » fait tourner l’agent dans l’interface de Copilot (voir [la vue terminal](claude-code-view#avec-copilot-cli)).

### Ce que Copilot CLI ne fait pas

Pas de « Toujours autoriser », pas de mode `assisted` (expérimental), pas de PDF joints, pas d’autre commande `/` que `/compact` dans Escouade (suivie, si tu veux, de ce qu’il faut garder), pas de remote control ni de tâches de fond. Un agent Copilot ne reprend pas sur un autre compte.

## Cursor CLI

Cursor CLI est un plugin livré avec Escouade (`dev.escouade.cursor-cli`) : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, un agent de Cursor dit au lancement qu’il faut le rallumer, et garde sa session.

### Installation et compte

Installe Cursor CLI avec son script (`irm 'https://cursor.com/install?win32=true' | iex` sous Windows, `curl https://cursor.com/install -fsS | bash` sous macOS et Linux), puis connecte-le avec `agent login`. Escouade le trouve dans `%LOCALAPPDATA%\cursor-agent` sous Windows, même absent du `PATH`, et lance sa dernière version installée. Chaque agent a son `agent acp` et son propre dossier de configuration, retiré avec lui : ses règles, son modèle et ses sessions y sont gardés, jamais dans ta config de Cursor. La connexion de la machine (`agent login`) est « le compte de la machine ». Un autre compte Cursor s’ajoute par sa clé d’API (cursor.com, Settings › API Keys) dans Réglages › Comptes et harness : elle est gardée dans le trousseau du système et donnée à ses agents par `CURSOR_API_KEY`, jamais dans un fichier ni sur une ligne de commande ; « Changer la clé… » la redonne.

### Modèles et modes

Le menu « Modèle » liste les modèles de ton compte, Auto en tête, avec leurs efforts. Une offre Free ne fait tourner qu’Auto : un autre modèle finit le tour en erreur, en le disant. Le modèle et le mode changent tout de suite.

Cursor n’a pas de bac à sable sous Windows. Il lit et modifie les fichiers sans demander, même hors du dossier de l’agent, et sans passer par Escouade, qui ne peut donc pas l’en empêcher ; ses commandes, le web et les outils MCP demandent. Pour un projet privé, Escouade refuse d’office ses commandes qui nomment les chemins des autres projets, mais pas ses lectures ni ses écritures.

| Mode | Chez Cursor CLI |
|---|---|
| Demander | Le mode Agent de Cursor : ses commandes, le web et les outils MCP demandent ; une écriture qu’il soumet passe dans le dossier de l’agent, et demande ailleurs, avec son diff. |
| Édits auto | Pareil, et `git add` et `git commit` du dossier de l’agent passent. |
| Plan | Le mode Plan de Cursor : il étudie et propose son plan à approuver, sans rien modifier. |
| Bypass | Escouade accepte tout ce que Cursor demande. |

Une demande de Cursor a « Autoriser », « Pour cet agent » et « Refuser » : « Pour cet agent » garde la règle (`Shell(git status)`, par exemple) dans le dossier de l’agent, pour lui seul et ses reprises. Ses questions, son plan, ses tâches et ses sous-agents passent dans la conversation et le bandeau « Plan ». Un message envoyé pendant un tour attend sa fin : Cursor arrêterait le tour en cours pour lui.

Les skills, commandes, agents et hooks de Claude Code de ton compte et du projet tournent aussi sous Cursor, comme pour les agents Claude Code ; Escouade les laisse comme tu les as réglés. Un hook qui refuse une écriture la bloque : l’outil finit en échec dans la conversation, « Cursor n’a rien écrit », Cursor n’en donnant pas la raison.

### Coût, quota et ce que Cursor ne fait pas

Cursor ne dit ni les tokens ni le coût d’un tour quand Escouade le pilote : la conversation n’en montre pas, et les statistiques comptent ses tours et leur durée. Il ne dit pas non plus son usage du mois : pas de barre dans la barre d’état, mais « Usage sur cursor.com » dans Réglages › Comptes et harness et dans le panneau des quotas, qui ouvre son tableau de bord. Une limite atteinte (« Upgrade your plan to continue ») arrête le tour sur une carte ambre qui le dit, avec, s’il y en a, le compte ou le harness où continuer. Pas de « Dupliquer la conversation » (Cursor ne copie pas une session), pas de PDF joints, pas de commandes `/` dans Escouade (pour compacter son contexte, tape `/summarize` dans sa vue terminal), ni remote control ni tâches de fond. Il reçoit le serveur MCP d’Escouade par sa session ; l’interrupteur ne le déclare pas encore dans la config de Cursor. La « Vue Cursor CLI » fait tourner l’agent dans l’interface de Cursor, avec un suivi limité (voir [la vue terminal](claude-code-view#avec-cursor-cli)).

## Mistral Vibe

Mistral Vibe est servi par le plugin « Mistral », livré avec Escouade (`dev.escouade.mistral`), qui porte aussi sa voix : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, la voix Mistral s’en va aussi, et un agent de Vibe dit au lancement qu’il faut le rallumer, et garde sa session.

### Installation et compte

Installe Mistral Vibe avec `uv tool install mistral-vibe`, puis donne-lui ta clé d’API Mistral avec `vibe --setup` : elle est gardée dans le trousseau du système, et Escouade ne la lit jamais. Escouade trouve son `vibe-acp` dans le `PATH`, ou dans `~/.local/bin` où `uv` le pose. Chaque agent a son `vibe-acp` et son propre dossier `VIBE_HOME`, retiré avec lui : ses règles, ses sessions et la confiance de son dossier y sont gardées, jamais dans ton `~/.vibe`. Ses mises à jour et sa télémétrie y sont coupées.

La clé de `vibe --setup` est « le compte de la machine ». Un autre compte Mistral s’ajoute par sa clé d’API (console.mistral.ai, API Keys) dans Réglages › Comptes et harness, ou par la clé Mistral de Réglages › Voix, en laissant le champ vide : la voix et Vibe partagent le compte Mistral. La clé est gardée dans le trousseau du système et donnée à ses agents par `MISTRAL_API_KEY`, jamais dans un fichier ni sur une ligne de commande ; « Changer la clé… » la redonne. Un compte sans clé ne démarre rien. « Reprendre sur <compte> » garde la session de l’agent : seule la clé change.

### Modèles et modes

Le menu « Modèle » liste les modèles que sa session propose. Sa réflexion est l’effort : « Bas » la coupe, « Élevé » la garde. Le modèle, l’effort et le mode changent tout de suite.

Vibe n’a pas de bac à sable. Il lit sans demander ; ses commandes hors de sa liste (`cat`, `ls`, `git status`… passent), ses écritures et les outils MCP demandent.

| Mode | Chez Mistral Vibe |
|---|---|
| Demander | Son agent `ask` : ses écritures passent dans le dossier de l’agent et demandent ailleurs, avec leur diff ; ses commandes hors de sa liste et les outils MCP demandent. |
| Édits auto | Son agent `accept-edits` : ses écritures passent sans demander, même hors du dossier. |
| Plan | Vibe n’en a pas : Escouade lui demande d’étudier et de proposer un plan, refuse ses écritures, et ses derniers mots arrivent en plan à approuver ; approuvé, Vibe le met en œuvre dans le même tour. |
| Bypass | Son agent `auto-approve` : il fait tout sans demander. |

Une demande de Vibe a « Autoriser », « Pour cet agent » et « Refuser » : « Pour cet agent » garde la règle dans le `VIBE_HOME` de l’agent, pour lui seul et ses reprises. Un refus arrête le tour : Vibe réessaierait sinon la même commande sous d’autres formes. Ses questions, ses tâches et ses sous-agents passent dans la conversation et le bandeau « Plan ». Un message envoyé pendant un tour attend sa fin. « Dupliquer la conversation » copie toute sa session.

Pour un projet privé, les chemins des autres projets sont refusés à ses outils de lecture et d’écriture par la config de l’agent, et Escouade refuse d’office ce qu’il demande sur eux. En Bypass, Vibe reste alors dans son agent `ask`, dont Escouade accepte le reste, pour que ces refus tiennent : son `auto-approve` passerait outre.

### Coût, quota et ce que Vibe ne fait pas

Vibe donne le coût et les tokens de chaque tour : la conversation, la carte de l’agent, la barre d’état et les statistiques les montrent, comme pour OpenCode. Il ne dit pas de quota (une clé d’API Mistral est facturée à l’usage) : le coût du jour et un « Budget par jour ($) » en tiennent lieu, et « Usage sur admin.mistral.ai », dans Réglages › Comptes et harness et dans le panneau des quotas, ouvre la console de Mistral. Sa limite de débit (« Rate limit exceeded », après cinq minutes de réessais de Vibe) arrête le tour sur une carte ambre ; l’agent reprend seul une minute plus tard, ou, pour un agent de ticket, continue sur un autre compte Vibe si « Reprendre sur un autre compte un agent de ticket arrêté par la limite » est allumé. Les textes rédigés, quand « Modèle des textes rédigés » est « Mistral Vibe », passent par `vibe -p`, sans aucun outil, en un tour et 0,05 $ au plus. Pas de mode Auto, de PDF joints, de commandes `/` dans Escouade (`/compact` part comme un message), ni de remote control ni de tâches de fond. Il reçoit le serveur MCP d’Escouade par sa session ; l’interrupteur ne le déclare pas encore dans la config de Vibe. La « Vue Mistral Vibe » fait tourner l’agent dans l’interface de Vibe, sur la même session (voir [la vue terminal](claude-code-view#avec-mistral-vibe)).

## Antigravity CLI

Antigravity CLI est un plugin livré avec Escouade (`dev.escouade.antigravity-cli`) : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, un agent d’Antigravity CLI dit au lancement qu’il faut le rallumer, et garde sa conversation.

### Installation et compte

Installe Antigravity CLI avec son script (`irm https://antigravity.google/cli/install.ps1 | iex` sous Windows, `curl -fsSL https://antigravity.google/cli/install.sh | bash` ailleurs), puis ajoute son compte dans Réglages › Comptes et harness (voir ci-dessous). Escouade trouve `agy` dans le `PATH`, ou là où son script le pose. Chaque agent a son propre dossier, retiré avec lui, qui tient lieu de dossier personnel à `agy` : ses règles, ses conversations et le serveur MCP d’Escouade y sont gardés, jamais dans ton `~/.gemini`, que Gemini CLI partage. Ses mises à jour sont coupées.

Son compte de la machine est ta connexion Google : « Ajouter un compte Antigravity CLI… » le propose, connecté ou non. Pas encore connecté, « Ajouter ce compte » l’ajoute puis lance `agy` dans un terminal de la fenêtre, qui ouvre ton navigateur ; Escouade ne voit jamais tes identifiants, seulement que le trousseau du système garde la connexion. « Se connecter… », sur sa fiche, refait la même chose. Une clé `GEMINI_API_KEY` de ta machine ne lui est pas passée.

Un second compte Google n’est pas possible : la connexion d’`agy` est celle de la machine, quel que soit le dossier. « Un nouveau compte » est donc une clé d’API Gemini (sur aistudio.google.com, « Get API key ») : gardée dans le trousseau du système, donnée à ses agents par `GEMINI_API_KEY`, jamais écrite dans un fichier ni sur une ligne de commande. Ses agents passent alors par l’API Gemini de cette clé, avec ses modèles seulement. « Changer la clé… » la redonne. Chaque compte a ses défauts (modèle, effort, mode), comme ceux des autres harness.

### Modèles et modes

Le menu « Modèle » liste les modèles de ton compte (`agy models`) ; pour Gemini, l’effort (Bas, Moyen, Élevé) est son menu « Effort ». Un changement de modèle, d’effort ou de mode prend effet au message suivant : Escouade relance `agy` sur la même conversation.

Antigravity ne demande rien pendant un tour quand Escouade le pilote : ce que ses règles ne permettent pas est refusé sur le moment. Escouade te le demande une fois le tour fini (« Autoriser », « Pour cet agent », « Refuser »), écrit la règle, puis l’agent reprend et le refait, dans le même tour. Ses questions arrivent de même, à la fin du tour, et ta réponse lui est envoyée. Il lit sans demander. Pas de bac à sable.

| Mode | Chez Antigravity CLI |
|---|---|
| Demander | Ses écritures passent dans le dossier de l’agent ; ses commandes, ses écritures ailleurs et les outils MCP sont demandés à la fin du tour. |
| Édits auto | Son mode `accept-edits` ; `git add` et `git commit` passent aussi. |
| Plan | Son mode `plan` : il étudie et écrit un plan sans rien changer, qui arrive en plan à approuver ; approuvé, Antigravity le met en œuvre. |
| Bypass | Les règles d’Escouade lui permettent tout, sans rien demander. |

« Pour cet agent » garde la règle pour lui seul et ses reprises. Ses sous-agents passent dans la conversation et le bandeau « Plan ». Un message envoyé pendant un tour attend sa fin. « Stop » arrête le tour ; le message suivant reprend la conversation.

Pour un projet privé, la lecture des chemins des autres projets lui est refusée par ses règles (ce qui refuse aussi l’écriture), même en Bypass, et ce qu’il demande sur eux est refusé d’office. Ses commandes qui les liraient ne sont pas gardées.

### Coût, quota et ce qu’Antigravity ne fait pas

Antigravity donne les tokens de chaque tour (sur sa carte, dans les statistiques), pas leur coût ni son quota : son `/usage` (par groupe de modèles, hebdomadaire) n’est que dans son interface ; « Usage sur antigravity.google », dans Réglages › Comptes et harness et le panneau des quotas, ouvre la page de ses forfaits. Quand son quota est épuisé, le tour finit « limité » sur une carte qui le dit ; l’agent réessaie seul à la remise à zéro qu’Antigravity annonce, sinon une heure plus tard (une minute si c’est la capacité du modèle qui manque), ou passe sur un autre compte selon « Changer de compte à » et « Continuer sur le harness suivant ». Pas de mode Auto, d’images ni de PDF joints, de « Dupliquer la conversation », de commandes `/` dans Escouade, ni de remote control ni de tâches de fond. Il reçoit le serveur MCP d’Escouade par la config de son dossier ; l’interrupteur ne le déclare pas encore dans la tienne. La « Vue Antigravity CLI » fait tourner l’agent dans l’interface d’Antigravity, sur la même conversation (voir [la vue terminal](claude-code-view#avec-antigravity-cli)).

## Gemini CLI

Gemini CLI est un plugin livré avec Escouade (`dev.escouade.gemini-cli`) : ses agents conversent par un programme à part qu’Escouade lance et surveille. Il se voit dans la vue Plugins, sous « Livrés avec Escouade » ; coupé, un agent de Gemini dit au lancement qu’il faut le rallumer, et garde sa session.

> [!WARNING]
> Gemini CLI n’a pas encore été essayé avec un vrai compte : son adaptateur suit sa documentation et ce que son programme fait contre un faux serveur. Les points à voir en vrai : la taille de ses réponses en direct, ses questions, ses quotas et leurs messages, sa connexion Google donnée à chaque agent, sa vue terminal et ses hooks.

### Installation et compte

Installe Gemini CLI par npm (`npm install -g @google/gemini-cli`, Node 20 ou plus) ou Homebrew (`brew install gemini-cli`), puis ajoute son compte dans Réglages › Comptes et harness (voir ci-dessous). Escouade trouve `gemini` dans le `PATH`, ou dans le dossier global de npm. Chaque agent a son propre dossier de configuration (`GEMINI_CLI_HOME`), retiré avec lui : ses réglages, ses sessions et ses règles y sont gardés, jamais dans ton `~/.gemini`, qu’Antigravity CLI partage. Escouade y coupe ses mises à jour et ses statistiques d’usage, et approuve le dossier de l’agent (sans quoi Gemini ignorerait le serveur MCP d’Escouade).

Un compte Gemini est l’un ou l’autre, au choix quand tu l’ajoutes :

- **Un compte Google.** Escouade lance l’interface de Gemini (`gemini`) dans un terminal de la fenêtre, avec le dossier du compte pour `GEMINI_CLI_HOME` : elle ouvre ton navigateur sur ton compte Google, et garde sa connexion dans ce dossier. Escouade ne la lit jamais : il voit seulement qu’elle est là, avec l’adresse du compte. Ses agents la reçoivent par `GOOGLE_APPLICATION_CREDENTIALS`, depuis leur propre dossier. « Se reconnecter… » la refait.
- **Une clé d’API Gemini** (sur aistudio.google.com, « Get API key ») : gardée dans le trousseau du système, donnée à ses agents par `GEMINI_API_KEY`, jamais écrite dans un fichier ni sur une ligne de commande. « Changer la clé… » la redonne.

Une clé ou Vertex AI de la machine (`GEMINI_API_KEY`, `GOOGLE_API_KEY`, `GOOGLE_GENAI_USE_VERTEXAI`) ne passe jamais à un compte ajouté. Le compte de la machine n’est proposé que si la machine a une telle clé : la connexion Google de ton `~/.gemini` n’est jamais donnée aux agents. Chaque compte a ses défauts (modèle, mode), comme ceux des autres harness.

### Modèles et modes

Le menu « Modèle » liste les modèles de sa session, Auto (son routeur, qui choisit Pro ou Flash à chaque tour) en tête. Il n’a pas de menu « Effort » : sa réflexion ne se règle pas en cours de route.

| Mode | Chez Gemini CLI |
|---|---|
| Demander | Son mode `default` : il lit sans demander ; ses écritures dans le dossier de l’agent passent par les règles d’Escouade, ailleurs elles sont demandées avec leur diff ; ses commandes et les outils MCP sont demandés ; ses sous-agents sont refusés. |
| Édits auto | Son mode `autoEdit` : ses écritures passent sans demander. |
| Plan | Son mode `plan` : il étudie sans rien changer et écrit son plan, qui arrive en plan à approuver ; approuvé, Gemini le met en œuvre. |
| Bypass | Son mode `yolo` : il fait tout sans rien demander, sous-agents compris. |

« Pour cet agent » vaut pour sa session. Refuser arrête le tour ; ton mot, s’il y en a un, lui est envoyé ensuite. Gemini n’a pas d’outil pour poser une question quand Escouade le pilote : Escouade lui dit de finir son tour sur sa question, qui arrive en carte (dans la fenêtre comme dans la liste), et ta réponse lui est envoyée. Il n’a pas de liste de tâches. Un message envoyé pendant un tour attend sa fin. « Stop » arrête le tour ; la conversation reprend au redémarrage. Pas de bac à sable.

Pour un projet privé, ses outils de fichiers et ses commandes qui nomment les chemins des autres projets sont refusés par ses règles (`--policy`), même en Bypass, et ce qu’il demande sur eux est refusé d’office. Un script qui ouvre un fichier lui-même n’est pas gardé.

### Coût, quota et ce que Gemini ne fait pas

Gemini donne les tokens de chaque tour, pas leur coût ni son quota : son usage se lit par `/stats` dans son interface, et « Usage sur aistudio.google.com », dans Réglages › Comptes et harness et le panneau des quotas, ouvre la page d’usage des clés d’API. Sa limite arrête le tour « limité » sur une carte qui le dit ; l’agent reprend seul quand Gemini le dit (« Please retry in 30s »), sinon une minute plus tard, ou une heure plus tard pour son quota du jour, ou passe sur un autre compte selon « Changer de compte à » et « Continuer sur le harness suivant ». Les textes qu’il rédige (nom d’un agent, message de commit…) passent par `gemini -p`, tous ses outils refusés. La « Vue Gemini CLI » fait tourner l’agent dans l’interface de Gemini, sur la même session (voir [la vue terminal](claude-code-view#avec-gemini-cli)). Pas de mode Auto, de PDF joints, de « Dupliquer la conversation », de commandes `/` dans Escouade (`/compact` : son `/compress`, dans son interface), ni de remote control ni de tâches de fond. Il reçoit le serveur MCP d’Escouade par sa session ; l’interrupteur ne le déclare pas encore dans ta config.

## D’autres harness, par un plugin

Un agent qui parle ACP (Goose, par exemple) se décrit dans le manifeste d’un plugin, sans code : voir [un harness ACP décrit](plugins/harnesses). Le plugin ACP, livré avec Escouade (`dev.escouade.acp`), le lance et lui parle ; il se coupe dans la vue Plugins, et les agents de ces harness restent alors, marqués « harness absent », avec leurs conversations. Les harness de cette page, eux, font encore partie de l’app.

## Le serveur MCP

Quel que soit leur harness, les agents d’Escouade reçoivent son serveur MCP, chacun avec son propre jeton, quand « Les agents peuvent piloter Escouade » est allumé. Le même interrupteur le déclare dans chaque harness installé, pour que Claude Code, Codex, OpenCode ou Copilot CLI lancés hors d’Escouade, dans un terminal ou leur app, pilotent Escouade : voir [le serveur MCP](mcp).

## Textes rédigés

Le nom d’un nouvel agent, les messages de commit et les descriptions de pull request proposés sont écrits par le harness de Réglages › Général › « Modèle des textes rédigés » : « Claude Haiku », « Codex (GPT-6-Luna) », un modèle d’OpenCode , « Copilot (GPT-5 mini) », « Cursor (Auto) », « Mistral Vibe », « Gemini 3.8 Flash » (Antigravity CLI) ou « Gemini CLI », parmi les harness installés (voir [les textes rédigés](settings-application#textes-rediges)).
