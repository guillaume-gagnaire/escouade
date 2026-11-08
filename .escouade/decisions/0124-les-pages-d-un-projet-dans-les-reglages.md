---
number: 124
id: "les-pages-d-un-projet-dans-les-reglages"
title: "Les pages d'un projet dans les réglages, et des renvois qui mènent à leur page"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-385"
pages: ["fenetre/reglages","projets/fermer-un-projet"]
tickets: ["ESC-385"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":27,"removed":13},{"page":"projets/fermer-un-projet","added":3,"removed":3}]
accepted_at: "2026-11-08T08:56:24+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le lot R6 de la refonte des réglages (direction A, design/settings-2-2/PROPOSITION.md §2, §5, annexes A et B). L'onglet Projets de la 2.0 tenait 9 écrans : identité, remote control, confidentialité, workflow, git, worktrees, lancement, .escouade.yml, mémoire et Kanban partagés, zone sensible. Depuis R2, les pages Git et worktrees, Lancement, Équipe et Confidentialité montraient encore tout l'onglet, ouvert sur leur partie. Les renvois de l'app passaient par d'anciens ids d'onglet (legacyTab) et atterrissaient au milieu de ce défilement.

## Décision

L'onglet est découpé en pages, chacune avec son seul contenu : Projet (identité, harness, compte, assistant ; Avancé › « Fermer le projet… », l'ancienne « Zone sensible »), Git et worktrees (avec les dépôts), Lancement, Équipe (Kanban partagé, « Réglages partagés » avec .escouade.yml en sous-titre, mémoire partagée), Confidentialité (projet privé, ses fichiers et comptes, remote control du projet, et nouveau : « Plugins dans ce projet », un interrupteur par plugin). Kanban reçoit le workflow du projet. Chaque renvoi de l'app vers un projet ouvre sa page (et, au besoin, un groupe : Workflow, Réglages partagés) ; legacyTab est retiré, un test refuse un ancien id d'onglet, et un autre un texte « Réglages › X » où X n'est pas une page.

## Conséquences

Aucune clé enregistrée ne change. Les textes de l'app et du cœur qui disaient « Réglages › Projets » nomment la page : « Réglages › Projets › Équipe » pour les commandes de .escouade.yml, « › Projet » pour les plugins proposés, « › Kanban » pour « Pas de workflow », « › Confidentialité » pour un compte réservé, « Réglages › Tous les projets » pour l'historique d'un projet fermé. Activer un plugin dans un projet se fait aussi depuis ses réglages, plus seulement depuis la vue Plugins. Les textes des plugins publiés à part attendent leur prochaine version.
