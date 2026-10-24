# Prise en main

Une première session pas à pas : ajouter un projet, créer un agent, choisir son modèle, son effort et son mode de permission, lui confier une demande, répondre à ses questions, relire ce qu’il a changé et commiter.

## Avant de commencer

Il te faut Claude Code installé et connecté à ton compte Claude, et git (voir l’[installation](installation)). Sous macOS, `Ctrl` devient `⌘` dans les raccourcis de cette page.

Au premier lancement, Escouade n’a aucun compte : l’écran d’accueil dit « Aucun compte ». Clique sur « Ajouter un compte… » et garde « Le compte de la machine », ton Claude Code déjà connecté, puis « Ajouter ce compte » (voir [plusieurs comptes](accounts#aucun-compte-au-depart)). Sans compte, aucun agent ne démarre.

## Ajouter un projet

Un projet est un dossier de code. Sur l’écran d’accueil, clique sur « + Ajouter un projet ». Plus tard, le bouton `+` à droite des onglets de la barre de titre fait la même chose. La fenêtre « Nouveau projet » s’ouvre : c’est un assistant en étapes, listées à gauche, qui règle tout ce qui fait marcher la méthode de travail d’Escouade (agents en worktree, tickets du Kanban, lancements, validation) dès le premier ticket. Chaque étape a « Suivant » et, sauf la première et le récapitulatif, « Passer », qui garde ses valeurs telles qu’elles sont ; un clic sur une étape de la liste y va directement.

**1. Départ.**

| Champ | Ce qu’il fait |
|---|---|
| « Dossier » | Le chemin du projet. « Parcourir… » ouvre le sélecteur de dossier. Sous le champ, une ligne dit ce qu’Escouade sait du dossier : « Dépôt git détecté · branche main · propre » (ou le nombre de fichiers modifiés), « Aucun dépôt git · un dépôt sera initialisé » ou « Dossier introuvable ». |
| « Nom » | Rempli avec le nom du dossier, que tu peux changer. « Aperçu de l’onglet » montre l’onglet tel qu’il sera. |
| « Couleur » | Une des 14 couleurs proposées ; la première qu’aucun projet n’utilise encore est choisie d’office. Elle teinte l’onglet et l’interface quand le projet est affiché. |
| « Point de départ » | « Copier la configuration d’un projet » (avec la liste des projets ouverts, celui affiché d’abord, sans les projets privés), « Configuration recommandée » (choisie par défaut) ou « Partir de zéro » (les valeurs par défaut d’Escouade). |

La configuration recommandée : un worktree par agent, validation par merge squash dans la branche courante, worktree supprimé une fois validé, commits conventionnels, l’agent résout les conflits, commits rédigés par l’agent.

**2. Réglages copiés** (seulement en copie). Les réglages de l’autre projet sont listés par groupe (« Worktrees », « Lancement », « Kanban », « Git et compte »), chacun avec sa case, et vérifiés dans le nouveau dossier : tout est coché sauf ce qui ne marcherait pas ici, avec la raison à côté. Une commande dont le dossier n’existe pas dans le nouveau dépôt (« dossier « web » introuvable ici ») et un dossier des worktrees écrit en chemin complet (celui de l’autre projet) ne sont pas repris ; une branche cible introuvable est remplacée par la branche courante (« branche introuvable : main à la place ») ; un compte préféré inactif, réservé à d’autres projets ou supprimé retombe sur « Automatique ». Les sources liées (Jira, Trello, GitHub, Linear), les tickets, « Projet privé » et les comptes réservés ne sont jamais copiés : un projet privé n’est jamais proposé, et ses comptes réservés restent les siens. Ce qui est repris se retrouve dans les étapes suivantes, où tu peux le changer.

**3. Worktrees** : un worktree par agent, dossier des worktrees, préfixes des branches, fichiers copiés, commandes de préparation et de démontage, avec « ✦ Remplir automatiquement », où Claude lit le dossier pour les proposer (voir [worktrees](worktrees)). **4. Lancement** : les commandes qui lancent le projet, avec « ✦ Remplir automatiquement » aussi (voir [lancement](launch)). **5. Kanban** : action à la validation, branche cible, stratégie, suppression du worktree, message de commit conventionnel, conflits, commande de tests et « Relancer les tests avant », agents en parallèle, pilote auto, modèle, effort et mode (voir [Réglages : Kanban](settings-board)). **6. Compte et commit** : harness par défaut (quand plusieurs sont installés), compte préféré, qui écrit les commits, remote control, puis « Projet privé » (allumé d’office si « Nouveaux projets privés » l’est) et, pour un projet privé, les comptes à lui réserver (ceux qui ne servent aucun autre projet) et « Comptes partagés à la limite » (voir [Réglages : Projets](settings-projects)). Les comptes cochés lui sont réservés à la création. **7. Intégrations** (facultatif) : une source à lier pour chaque service dont le compte est connecté.

**8. Récapitulatif.** Ce qui empêcherait la méthode de travail de marcher, chaque point avec sa correction, puis ce qui sera réglé et « Créer un premier agent » (avec son modèle) :

| Contrôle | Correction |
|---|---|
| « Le dépôt n’a aucun commit » : aucun ticket ne démarre, un worktree part d’un commit | « Faire un premier commit » : un commit vide, aucun fichier n’est ajouté |
| « La branche cible … est introuvable » | « Prendre la branche courante » |
| « Les worktrees iraient dans un dossier suivi par git » | « Utiliser .claude/worktrees » |
| « Aucun compte Claude ne sert ce projet » | Activer un compte, ou retirer sa réservation, dans Réglages › Comptes |
| « Claude Code est introuvable » | L’installer, ou indiquer son chemin dans Réglages › Harness |

Chaque contrôle est aussi dit dans l’étape qu’il concerne (le premier dès « Départ »). Clique sur « Créer le projet » : le projet s’ouvre dans un nouvel onglet avec ses réglages, puis son premier agent si tu l’as demandé. « Créer sans l’assistant », en bas à gauche, crée le projet tout de suite avec les valeurs par défaut d’Escouade et un premier agent, comme avant l’assistant. Plus tard, Réglages › Projets › « Ouvrir l’assistant… » le rouvre sur un projet existant (voir [Réglages : Projets](settings-projects#assistant-de-configuration)).

Si le dossier est celui d’un projet que tu as fermé, l’historique passe avant l’assistant : la fenêtre ajoute un encart, par exemple « Historique retrouvé : 1 agent, 3 tickets, dernière activité le 14 octobre 2026 », et son bouton devient « Rouvrir le projet » : le projet revient avec son historique (agents, conversations, tickets, réglages), sans étape. L’interrupteur « Partir d’un projet vierge » en crée un neuf à la place, par l’assistant. Voir [Réglages : Projets](settings-projects#projets-fermes).

> [!NOTE]
> Si le dossier n’est pas dans un dépôt git, Escouade lance `git init` pour en créer un : sans git, il n’y aurait ni diff ni commit à relire.

## Créer un agent

Un agent est une conversation avec Claude Code dans ce projet. Pour en créer un, appuie sur `Ctrl+N` ou clique sur « + Nouvel agent » en haut de la barre latérale : l’agent est créé sans rien te demander, et sa conversation s’affiche. Il s’appelle `agent-1`, `agent-2`… jusqu’à sa première demande ; Escouade lui donne alors un nom court tiré de ce que tu as écrit. Pour le renommer toi-même, double-clique sur son nom.

Son statut est « Prêt », et la conversation vide dit « Agent prêt », avec le dossier où il travaille. La flèche ▾ à droite de « + Nouvel agent » propose « Nouvel agent sur une branche… » (voir les [branches](branches)).

## Choisir le modèle, l’effort et le mode de permission

Sous le champ de message, trois menus règlent l’agent (un quatrième, « Compte », apparaît quand tu as plusieurs comptes Claude). Tu peux les changer en cours de conversation.

| Menu | Choix |
|---|---|
| « Modèle » | « Fable », « Opus », « Sonnet » et « Haiku ». Chacun lance le modèle le plus récent de sa famille que connaît Claude Code, et s’affiche avec sa version (par exemple « Sonnet 5.5 »). |
| « Effort » | « Bas » (réponses rapides, peu de réflexion), « Moyen » (équilibré), « Élevé » (réflexion approfondie), « Très élevé » (réflexion très approfondie), « Max » (réflexion maximale, plus de tokens). Haiku ne gère pas l’effort. |
| « Mode » | Le mode de permission, détaillé ci-dessous. |

Les modes de permission décident quand Claude te demande l’autorisation d’agir :

| Mode | Ce qui se passe |
|---|---|
| « Auto » | Un classifieur approuve les actions sûres et demande pour le reste. Indisponible avec un modèle sans ce mode, comme Haiku 4.5 (Claude Code 2.1.292 et avant). |
| « Demander » | Claude demande ton accord avant chaque action sensible. |
| « Plan » | Claude analyse et propose un plan sans rien modifier. |
| « Édits auto » | Les modifications de fichiers sont acceptées sans demander. |
| « Bypass » | Aucune demande de permission. |

Un nouvel agent démarre avec le modèle, l’effort et le mode de son compte, réglés dans sa fiche de Réglages › Comptes, « Nouveaux agents » (par défaut ceux du harness : Sonnet, Moyen et Auto pour Claude Code). Voir les [réglages des comptes](settings-accounts) et [les agents](managing-agents).

> [!WARNING]
> En mode « Bypass », Claude agit sans aucune demande de permission : Escouade te le rappelle quand tu le choisis. Réserve-le aux environnements sûrs.

## Envoyer une première demande

Clique dans le champ de message, décris la tâche (« Décris la tâche à confier à Claude… »), puis appuie sur `Entrée` pour envoyer. `Maj+Entrée` va à la ligne.

- Tape `@` suivi d’un bout de nom de fichier pour choisir un fichier du projet, ou `/` pour choisir une commande.
- Colle ou glisse n’importe quel fichier (une image, un PDF, un texte, mais aussi une archive ou un tableur) dans le champ, ou clique sur le trombone, pour le joindre au message.

Dès l’envoi, le statut de l’agent passe à « En cours » et la conversation dit « Claude travaille… ». Ses appels d’outils apparaissent en lignes compactes que tu peux déplier. Tu peux lui écrire pendant qu’il travaille : il en tient compte dès sa prochaine étape. Pour l’arrêter, appuie sur `Échap` dans le champ de message ou clique sur « ■ Stop ».

## Répondre à une question ou à une autorisation

Quand Claude a besoin de toi, une carte à bordure ambre apparaît dans la conversation, le statut de l’agent devient « Question », et son onglet et sa carte clignotent si tu regardes ailleurs. Il y a deux cartes.

**« Claude demande une autorisation »** nomme l’outil et ce qu’il va faire. Trois boutons :

- « Autoriser » (`Ctrl+Entrée`) ;
- « Toujours autoriser » (`Ctrl+Maj+Entrée`), quand Claude Code propose de retenir ce choix ;
- « Refuser ».

Pour refuser en expliquant quoi faire à la place, écris-le dans le champ de message et envoie-le. Quand Claude Code refuserait la demande par défaut, « Refuser » est le bouton principal et seul un clic sur « Autoriser » l’accorde : les touches ne répondent pas. Quand Claude propose un plan, la carte s’appelle « Claude propose un plan » et les boutons sont « Approuver le plan », « Approuver et accepter les édits » et « Continuer à planifier ».

**« Claude attend ta réponse »** pose une question avec des options. Clique sur une option, ou appuie sur `Alt+1` à `Alt+9`, ou réponds librement dans le champ de message. Avec une seule question à choix unique, la réponse part dès ce clic ; avec plusieurs questions ou un choix multiple, « Valider » (`Ctrl+Entrée`) envoie les réponses.

> [!TIP]
> `Ctrl+J` ouvre le prochain agent qui attend ta réponse, le curseur dans son champ de message. Une carte qui vient d’arriver ignore les touches de réponse pendant une demi-seconde, pour qu’une frappe destinée à ton message ne réponde pas à sa place.

## Voir ce que l’agent a changé

Quand Claude a fini, le statut devient « Terminé » et une carte « Tâche terminée » donne la durée, les tokens, le coût et la liste des fichiers modifiés ; un clic sur un fichier l’ouvre dans l’éditeur.

Pour relire les changements, ouvre le panneau « Non commités » : clique sur « Fichiers ▸ » dans l’en-tête de la conversation, ou appuie sur `Ctrl+Maj+B`.

- Chaque ligne est un fichier, avec sa lettre (M pour modifié, A pour ajouté, D pour supprimé) et ses lignes ajoutées et retirées.
- Un clic sur un fichier ouvre son diff dans une grande fenêtre. « Unifié » et « Côte à côte » changent la présentation, `Échap` ferme la fenêtre. Quand il y a plusieurs fichiers, une colonne à gauche permet de passer de l’un à l’autre.
- « Voir le diff » ouvre le diff de tous les fichiers de la liste.
- « Cet agent » liste les fichiers de l’agent ; « Tout le projet » liste ceux de tous les agents du projet.

Pour garder la conversation et le diff sous les yeux ensemble, passe à la disposition côte à côte avec `Ctrl+Maj+L` : le panneau reste affiché à droite et montre le diff du fichier choisi sous la liste.

![Un agent dont le panneau « Non commités » liste trois fichiers, avec le diff du premier à droite de la conversation](/images/git.jpg)

## Commiter

Le bouton « Commit… » du panneau commite les changements de l’agent (« Commit tout… » quand « Tout le projet » est choisi). Ce qui se passe dépend du réglage « Commit » du projet, dans Réglages › Projets :

- **« Rédigé par l’agent »** (par défaut) : Escouade envoie à l’agent un message qui lui demande de commiter ses changements avec un message clair au format Conventional Commits. Un message éphémère confirme « Demande de commit envoyée à » suivi du nom de l’agent, puis l’agent commite comme pour n’importe quelle demande.
- **« Direct, avec un message proposé »** : la fenêtre « Commit » liste les fichiers, Haiku rédige un message que tu relis et modifies, « Régénérer » en demande un autre, et « Commiter » commite. Rien n’est commité sans ce clic.

Si l’agent travaille dans un worktree, le panneau propose aussi « Merger … → … », qui fusionne sa branche dans celle du projet. Voir [Git et commits](git) et [les worktrees](worktrees).

## Et ensuite

- [Les agents](managing-agents) et [la conversation](chat) : modes de permission, messages, pièces jointes, recherche, historique.
- [Notifications](notifications) et [suivre ses agents](overview) : être prévenu, répondre sans ouvrir chaque conversation.
- [Git et commits](git), [les worktrees](worktrees) et [les branches](branches) : relire, commiter, isoler les agents.
- [L’éditeur](editor), [les terminaux](terminals) et [lancer le projet](launch).
- [Le Kanban](board) et [le pilote auto](autopilot) : confier des tickets à des agents.
- [Les comptes Claude](accounts) et [les quotas et le coût](usage).
- [Les raccourcis clavier](shortcuts) et [l’interface](interface) pour faire le tour de la fenêtre.
