---
number: 123
id: "reglages-r4-comptes-et-harness-reunit-le"
title: "Réglages R4 : « Comptes et harness » réunit les harness, leurs comptes et ce qui se passe à la limite"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-382"
pages: ["fenetre/reglages","harness/comptes","kanban/pilote-auto","fenetre/barre-de-statut","harness/claude-code-wsl"]
tickets: ["ESC-382"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":6,"removed":7},{"page":"harness/comptes","added":22,"removed":18},{"page":"kanban/pilote-auto","added":4,"removed":3},{"page":"fenetre/barre-de-statut","added":5,"removed":5},{"page":"harness/claude-code-wsl","added":4,"removed":4}]
accepted_at: "2026-11-08T07:29:51+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte



## Décision



## Conséquences

Une seule réponse à « que se passe-t-il à la limite ? », les deux seuils lus l'un à côté de l'autre avec des descriptions courtes (le long détail du choix du compte reste dans la spec Comptes). La section HarnessesSection de R3, qui gardait les harness de l'ancien onglet en attendant R4, est retirée : AccountsTab porte chaque harness avec ses comptes. Le chemin de l'exécutable et la mise à jour de Claude Code demandent un clic de plus (Avancé, qui se déplie seul quand Claude Code est introuvable). Un harness introuvable donne sa commande d'installation sur la ligne de son nom. La ligne d'aide sur l'ordre des comptes, qui citait les deux seuils, est retirée. Les docs des plugins de harness (plugins/*/docs) et le message du plugin Copilot nomment encore « Réglages › Harness » ou « Réglages › Comptes » : à refaire avec leur prochaine version, comme la documentation du site après R8.
