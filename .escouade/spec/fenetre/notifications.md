---
title: "Notifications"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Escouade prévient quand un agent a besoin de l'utilisateur : une question, une autorisation, un plan à approuver, un tour fini, une erreur, un ticket prêt ou bloqué. Dans l'app, par des pastilles et un carillon ; hors de l'app, par une notification du système.

## Comportement

### Déclencheurs

- Question d'un agent (question à choix, autorisation, plan à approuver).
- Fin de tour.
- Erreur : tour en erreur, limite d'usage, process arrêté.
- Tickets : « ATL-42 prêt à tester », « ATL-42 bloqué : <raison> » ; un clic ouvre le Kanban du projet. La fin de tour d'un agent de ticket « En cours » ne notifie pas ; ses questions, si.

### Texte

- Titre : « <projet> · <agent> ».
- Corps : 120 caractères au plus, sur une ligne, « … » à la coupe. Il suit la langue de l'interface (« Autoriser Bash : npm test ? » / « Allow Bash: npm test? »).
  - Autorisation : « Autoriser <Outil> : <argument> ? », avec le premier argument utile (la commande Bash, le fichier depuis le dossier de l'agent, l'URL, la requête, le motif, la description…). Sans argument : « Autoriser <Outil> ? ».
  - Plan à approuver : « Approuver le plan : <première ligne du plan, sans markdown> ? », ou « Approuver le plan ? » sans texte.
  - Question : le texte de la première question, sans markdown ; à défaut, « Claude attend ta réponse ».
  - Fin de tour : la première ligne de la réponse finale (le texte après le dernier outil du tour), sans markdown ; sinon « Tâche terminée ».
  - Erreur : « Erreur : <raison> » ; pour un process mort, « Claude Code s'est arrêté (code N) ».
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
- Son : activé, avec « ▶ Tester ».

## Limites

- Un outil MCP s'affiche sous son nom brut (`mcp__serveur__outil`).
