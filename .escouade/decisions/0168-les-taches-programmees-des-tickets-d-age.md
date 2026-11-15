---
number: 168
id: "les-taches-programmees-des-tickets-d-age"
title: "Les tâches programmées : des tickets d'agent lancés à heure fixe, sur la machine"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-448"
pages: ["kanban/taches-programmees","fenetre/barre-laterale","harness/serveur-mcp"]
tickets: ["ESC-448"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/taches-programmees","added":31,"removed":0},{"page":"fenetre/barre-laterale","added":3,"removed":0},{"page":"harness/serveur-mcp","added":8,"removed":0}]
accepted_at: "2026-11-14T23:23:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

docs/OVERSEER.md (2026-10-28) décide les tâches programmées et dit de les faire avant l'Overseer. Il restait au constructeur : la lecture du cron, l'heure d'été, la reprise au démarrage, la façon de ne jamais lancer deux fois.

## Décision

Une tâche programmée (consigne, critères, boucles, harness, compte, modèle, effort, rythme chaque jour / semaine / mois ou cron à cinq champs, heure de la machine) crée et lance un ticket à chaque échéance, comme « Lancer » : places et quotas du Kanban, même pilote auto éteint ; le ticket porte « ⏱ programmée » et un lien vers sa tâche. Les expressions et les rythmes simples (écrits en cron) sont lus par la crate croner, heure d'été comprise. Seule la dernière échéance depuis la précédente traitée compte : plusieurs manquées donnent une exécution au plus ; elle est écrite et enregistrée avant la création du ticket (un arrêt entre les deux perd une exécution, n'en fait jamais deux). Une échéance passe son tour tant que le ticket précédent est « À faire » ou « En cours ». Une échéance tombée avant le démarrage d'Escouade part au retour si « Lancer au retour d'Escouade, une seule fois » est coché (par défaut), sinon elle passe. Les tâches restent dans l'état local, jamais partagées, leurs tickets non plus. Section « Programmées » au-dessus de « Lancement » (place A), formulaire du ticket en mode « Programmer » (rythme sous le titre, puis consigne et le reste ; ni branche, ni Après, ni fichiers, ni partage). Cinq outils MCP (list/create/update/pause/delete_scheduled_task), aux règles de projet privé des tickets.

## Conséquences

Nouvelle page de spec Kanban › Tâches programmées ; Barre latérale et Serveur MCP complétées. Une dépendance de plus (croner, MIT). Les échéances sont regardées toutes les 15 s ; l'horloge des tâches est injectable (tests Rust, et en debug ESCOUADE_SCHEDULE_SPEED / ESCOUADE_SCHEDULE_TICK_MS pour l'e2e). L'Overseer les pilotera par les outils MCP.
