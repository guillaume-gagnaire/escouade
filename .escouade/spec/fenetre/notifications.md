---
title: "Notifications"
subject: "Fenêtre"
sources: ["docs/SPEC.md","docs/TODO-1.8.md (chapitre 2, « Notifications, vue d'ensemble, serveur MCP »)","src-tauri/src/prnews.rs"]
tickets: ["ESC-68"]
decisions: []
compared: null
---

## Ce que c'est

Escouade prévient quand un agent a besoin de l'utilisateur : une question, une autorisation, un plan à approuver, un tour fini, une erreur, un ticket prêt ou bloqué, et quand un hébergeur de code a du nouveau sur une pull request ou un déploiement. Dans l'app, par des pastilles et un carillon ; hors de l'app, par une notification du système.

## Comportement

### Déclencheurs

- Question d'un agent (question à choix, autorisation, plan à approuver).
- Fin de tour.
- Erreur : tour en erreur, limite d'usage, process arrêté.
- Tickets : « ATL-42 prêt à tester », « ATL-42 bloqué : <raison> » ; un clic ouvre le Kanban du projet. La fin de tour d'un agent de ticket « En cours » ne notifie pas ; ses questions, si.
- Hébergeurs de code : voir plus bas.

### Les nouvelles des hébergeurs

Huit nouvelles, chacune son réglage :

- CI en échec, changements demandés, PR approuvée, PR mergée, conflit : ce qui devient vrai entre deux lectures d'une PR d'un ticket « En revue » ou d'un agent. Une CI repassée au vert puis de nouveau en échec est redite. Une PR mergée ne dit que cela. Une PR fermée sans merge ne dit ni sa CI ni ses relectures (son ticket se bloque, voir *Tickets*).
- Relecture demandée : une relecture nouvellement demandée au compte avec lequel l'hébergeur est lu, sur une PR qui n'est pas la sienne ; un clic ouvre la vue « Pull requests » du projet.
- Déploiement en production réussi : un nouveau déploiement réussi dans un environnement de production, à un autre commit ; un clic ouvre le Kanban du projet. Un succès plus ancien que ce qui était connu n'en est pas un nouveau.
- Déploiement en échec : voir *Environnements et déploiements* ; dans tout environnement.

Elles ne demandent rien à l'hébergeur : une notification est l'écart entre deux lectures déjà faites. Ce qui était déjà vrai à la première lecture ne se dit pas. Une PR tenue par un ticket n'est dite que par le suivi de ce ticket, pas par la lecture de son agent. Quand des changements demandés arrivent avec leurs retours de relecture, seule la nouvelle « changements demandés » part ; les retours qui arrivent seuls gardent « DEM-1 : un retour de relecture sur la PR #12 », réglé avec les tickets.

### Texte

- Titre : « <projet> · <agent> » ; « <projet> » pour un ticket, une relecture demandée ou un déploiement.
- Corps : 120 caractères au plus, sur une ligne, « … » à la coupe. Il suit la langue de l'interface (« Autoriser Bash : npm test ? » / « Allow Bash: npm test? »).
  - Autorisation : « Autoriser <Outil> : <argument> ? », avec le premier argument utile (la commande Bash, le fichier depuis le dossier de l'agent, l'URL, la requête, le motif, la description…). Sans argument : « Autoriser <Outil> ? ».
  - Plan à approuver : « Approuver le plan : <première ligne du plan, sans markdown> ? », ou « Approuver le plan ? » sans texte.
  - Question : le texte de la première question, sans markdown ; à défaut, « Claude attend ta réponse ».
  - Fin de tour : la première ligne de la réponse finale (le texte après le dernier outil du tour), sans markdown ; sinon « Tâche terminée ».
  - Erreur : « Erreur : <raison> » ; pour un process mort, « Claude Code s'est arrêté (code N) ».
  - Pull request : « PR #12 : CI en échec », « DEM-1 · PR #12 : approuvée » pour celle d'un ticket (« MR » sur GitLab). Jamais son titre : il est écrit par un tiers et part hors de l'app.
  - Déploiement : « Déployé : production (<commit>) », « Déploiement en échec : production (<commit>) ».
- Le texte entre accents graves est repris tel quel (`*.log`, `__init__.py`).

### Dans l'app

- Pastilles qui pulsent sur la tuile du projet, la carte de l'agent et la barre de statut.
- Carillon : deux notes synthétisées. « ♪ » de la barre de statut le coupe ou le rend.

### Hors de l'app

- App en arrière-plan ou dans la zone de notification, sous Windows : un toast (un clic ouvre l'agent), la barre des tâches qui clignote, un badge sur l'icône de la zone de notification.
- Sous macOS : une notification du système, l'icône du Dock qui rebondit, un badge dans la barre des menus.

### Redémarrage pour mise à jour

- « Escouade redémarre pour se mettre à jour », 30 s avant, avec un bouton « Plus tard » sous Windows. Elle est envoyée quel que soit le réglage des notifications (voir *Livraison et mises à jour*).

## Réglages

- Réglages › Notifications › « Me prévenir pour » : « Questions et autorisations », « Tâches terminées », « Erreurs », « Tickets (prêt à tester, bloqué) », tous activés par défaut.
- Un type désactivé n'a ni notification du système ni carillon : « Désactivé : ni notification système ni carillon pour ce type ; l'onglet, la carte et la barre des tâches signalent toujours l'agent. » (« le Dock » sous macOS). La tuile et la carte de l'agent clignotent quand même, et la barre des tâches (ou le Dock) signale toujours l'agent.
- Réglages › Notifications › « Pull requests et déploiements » : « CI en échec », « Changements demandés », « PR approuvée », « PR mergée », « Conflit », « Relecture demandée », « Déploiement en production réussi », « Déploiement en échec », tous activés par défaut. Désactivée, une nouvelle ne fait plus rien du tout, barre des tâches comprise : aucun agent n'attend derrière elle.
- Son : activé, avec « ▶ Tester ».

## Limites

- Un outil MCP s'affiche sous son nom brut (`mcp__serveur__outil`).
- Une nouvelle n'arrive qu'avec la lecture qui la voit : toutes les minutes fenêtre devant et toutes les 5 minutes sinon pour une PR de ticket et un déploiement suivi ; seulement quand l'agent est affiché pour la PR d'un agent ordinaire ; seulement quand la liste du projet est lue (vue « Pull requests », vue d'ensemble ouverte, outil `list_pull_requests`) pour une relecture demandée.
- Ce qui change pendant que l'app est fermée est dit à la prochaine lecture pour une PR suivie, mais pas pour une relecture demandée : la première lecture d'une liste après le démarrage ne dit rien.
- Essayé contre le faux GitHub seulement. Essai réel à faire sur GitHub ; GitLab, Bitbucket Cloud, Gitea/Forgejo et Azure DevOps suivent leur documentation.
