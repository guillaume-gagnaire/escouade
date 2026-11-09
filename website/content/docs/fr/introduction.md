# Présentation

Escouade est un environnement de développement agentique (ADE) : une application de bureau, pour Windows, macOS et Linux, où tu développes avec des agents de bout en bout, du ticket au merge. Cette page dit ce qu’elle est, à qui elle sert, les mots qu’elle emploie et où trouver le reste de la documentation.

## Ce qu’est Escouade

Un ADE réunit tout le cycle de développement au même endroit, fait avec des agents qui travaillent en parallèle, chacun dans sa conversation et, si tu veux, dans son propre worktree git :

- **planifier** : un Kanban de tickets avec leurs critères d’acceptation, leurs captures et leurs fichiers, importés au besoin de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards, que des agents prennent seuls (c’est le pilote auto) et sur lesquels ils bouclent jusqu’à atteindre les critères ;
- **coder** : un onglet par projet, autant d’agents que tu veux, chacun avec une conversation lisible (markdown, appels d’outils dépliables, questions et demandes d’autorisation en cartes) ou l’interface de Claude Code elle-même ; ses sous-agents et ses tâches de fond dans la barre latérale, l’avancée de son plan, un éditeur de fichiers et de vrais terminaux ;
- **relire et tester** : les fichiers modifiés, leurs diffs, le git graph du dépôt et ses branches, « ▶ Tester » sur chaque ticket et les commandes qui lancent le projet ;
- **livrer** : le commit, le merge, le squash ou le rebase dans la branche cible, le push ou la pull request ;
- **piloter** : des notifications quand un agent attend ta réponse ou a terminé, la vue d’ensemble de tous les agents, les quotas, le coût et des statistiques, plusieurs comptes, un serveur MCP qui laisse un agent mener Escouade, et le remote control depuis ton téléphone.

Aujourd’hui, chaque agent est une vraie session de [Claude Code](https://code.claude.com/docs/fr/overview), l’outil en ligne de commande d’Anthropic pour programmer avec Claude : Escouade lance `claude` pour lui et affiche sa conversation. La session reste aussi sur ton disque, dans les fichiers de Claude Code. L’application tourne sur ta machine : tes projets, tes conversations et tes réglages sont dans `~/.escouade/` (voir les [données locales](data)). Elle est gratuite et open source, sous licence GPL-3.0 (ou ultérieure).

## Ce qu’Escouade n’est pas

- **Un produit d’Anthropic.** C’est un projet indépendant, non affilié à Anthropic. Claude et Claude Code sont des marques d’Anthropic.
- **Un moteur d’agents.** Claude Code doit être installé et connecté à ton compte Claude sur la machine : Escouade ne contient ni modèle ni abonnement, elle utilise les tiens. Voir l’[installation](installation).
- **Un IDE de plus.** Son éditeur suffit pour relire et retoucher ce que font les agents, mais il n’a ni serveur de langage ni extensions : garde ton IDE pour le reste, chaque worktree est un vrai dossier que tu ouvres où tu veux. Voir [l’éditeur](editor).

## À qui elle s’adresse

À toute personne qui développe avec des agents et veut mener tout le cycle au même endroit, sans jongler entre des terminaux, un outil de tickets et un client Git :

- tu travailles sur plusieurs tâches ou plusieurs projets en même temps, et tu veux voir d’un coup d’œil quel agent travaille, lequel attend ta réponse et lequel a fini ;
- tu veux relire et commiter ce que chaque agent a changé, sans que les agents se marchent dessus ;
- tu écris tes tâches comme des tickets, dans Escouade ou dans Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards, et tu veux les confier à des agents qui recommencent jusqu’à atteindre les critères d’acceptation ;
- tu as plusieurs comptes Claude (un perso, un pro) et tu veux suivre leurs quotas.

## Les notions de base

Ces mots reviennent dans toute l’interface et dans cette documentation.

### Projet

Un projet est un dossier de code, ouvert dans un onglet de la barre de titre. Il a sa couleur, ses agents, son Kanban, ses terminaux et ses propres réglages. Si le dossier n’est pas un dépôt git, Escouade en initialise un à la création du projet. Voir la [prise en main](quick-start).

### Agent et statuts

Un agent est une conversation avec Claude Code dans un projet. Il a toujours l’un de ces cinq statuts, que tu lis dans la barre latérale, dans l’en-tête de sa conversation et dans la barre d’état :

| Statut | Ce qu’il veut dire | Signe |
|---|---|---|
| « Prêt » | L’agent existe et n’a pas encore reçu de demande. | Point gris |
| « En cours » | Claude travaille sur une demande. | Point vert qui pulse |
| « Question » | Claude attend ta réponse : une question ou une demande d’autorisation. | Point ambre qui pulse |
| « Terminé » | Claude a fini sa demande. | Coche verte |
| « Erreur » | La demande s’est arrêtée sur une erreur. Un nouveau message relance Claude sur la même session. | Point rouge |

Voir [les agents](managing-agents).

### Conversation

La conversation est l’échange entre toi et un agent : tes messages, les réponses de Claude, ses appels d’outils et les cartes de questions et d’autorisations. Elle est gardée d’un redémarrage à l’autre, et chaque agent reprend sa session Claude là où il l’avait laissée. Voir [la conversation](chat).

### Ticket et Kanban

Un ticket décrit une tâche : un titre, une description et des critères d’acceptation. Le Kanban d’un projet, ouvert avec le bouton « Kanban » de la barre latérale, range les tickets en quatre colonnes : « À faire », « En cours », « À tester » et « Terminé ». Un agent prend un ticket, travaille et boucle jusqu’à en atteindre les critères ; tu le testes, puis tu le valides. Tu peux aussi importer des tickets de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues et Azure Boards. Voir [le Kanban](board).

### Pilote auto

Le pilote auto est ce qui fait partir les tickets « À faire » tout seul, dès qu’une place se libère. Il est activé par défaut sur un projet : le réglage « En parallèle » fixe combien d’agents prennent des tickets en même temps (de 1 à 6, 2 par défaut), chacun dans son worktree. Il se met en pause près de la limite de tes quotas (réglage « Pause au-delà du quota »). Voir [le pilote auto](autopilot).

### Worktree

Un worktree est une copie de travail séparée de ton dépôt, sur sa propre branche. Avec l’option « Un worktree par agent », chaque nouvel agent travaille dans le sien : ses modifications ne se mélangent ni à celles des autres agents ni à ton dossier, et tu les relis et les merges ensuite. Les agents des tickets travaillent chacun dans leur worktree. Voir [les worktrees](worktrees).

### Compte Claude

Un compte Claude est un dossier de configuration de Claude Code, avec sa connexion. Il n’y en a aucun au départ : dans Réglages › Comptes et harness, ajoute ton compte habituel (« Le compte de la machine », nommé « Principal », lancé comme avant), ou un nouveau (un perso, un pro). Chaque agent reste sur son compte. Voir [les comptes Claude](accounts).

### Vue d’ensemble

La « Vue d’ensemble » est une page qui liste les agents de tous les projets, ceux qui attendent ta réponse en tête. Tu l’ouvres avec son bouton dans la barre de titre ou avec `Ctrl+Maj+A`. Voir [suivre ses agents](overview).

## Systèmes et langues pris en charge

| Système | Versions |
|---|---|
| Windows | Windows 10 ou 11 |
| macOS | macOS 11 (Big Sur) ou plus récent, sur Mac Apple Silicon ou Intel |
| Linux | x86-64, avec une glibc 2.35 ou plus récente (Ubuntu 22.04, Debian 12, Fedora 36 et suivantes) |

Le détail des prérequis est dans l’[installation](installation).

L’interface existe en français et en anglais. Par défaut, elle suit la langue du système : français si le système est en français, anglais sinon. Réglages › Général permet de choisir l’une ou l’autre sans redémarrer, et de choisir à part la langue des textes que Claude rédige (messages de commit, descriptions de pull request, commentaires publiés dans Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards, consignes données aux agents), car beaucoup d’équipes commitent en anglais avec une interface en français. Voir les [réglages de l’application](settings-application).

## Plan de la documentation

- **Démarrer** : l’[installation](installation), la [prise en main](quick-start) et un tour de [l’interface](interface).
- **Agents et conversation** : commence par [les agents](managing-agents), puis la conversation, la vue d’ensemble, les notifications, la vue Claude Code et le remote control.
- **Code et Git** : [Git et commits](git), puis les worktrees, les branches, les [hébergeurs de code](code-hosts), l’éditeur, les terminaux et le lancement du projet.
- **Kanban et automatisation** : [le Kanban](board), puis le pilote auto, les tests et la validation des tickets, les intégrations et le serveur MCP.
- **Comptes et consommation** : les [comptes Claude](accounts), puis les quotas, le coût et les statistiques.
- **Réglages** : chaque page de la fenêtre des réglages, option par option, à partir de [la fenêtre et Général](settings-application).
- **Référence** : les [raccourcis clavier](shortcuts), les données locales, les mises à jour et le [dépannage](troubleshooting).

## Soutenir Escouade

Escouade est gratuite et open source. Si elle t’est utile, tu peux [offrir un café](support:) pour aider à la faire avancer, au montant de ton choix : le bouton « Offrir un café » de [Réglages › À propos](settings-about) ouvre la même page.
