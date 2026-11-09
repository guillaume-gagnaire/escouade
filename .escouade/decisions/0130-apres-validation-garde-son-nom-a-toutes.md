---
number: 130
id: "apres-validation-garde-son-nom-a-toutes"
title: "« Après validation » garde son nom à toutes les largeurs du Kanban"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-393"
pages: ["kanban/kanban"]
tickets: ["ESC-393","ESC-390"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/kanban","added":1,"removed":1}]
accepted_at: "2026-11-09T07:18:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-390, dans une zone de travail de 1000 px ou moins (la fenêtre de 1024 × 728 de la CI), le libellé « Après validation : » du bouton des réglages du Kanban est masqué par CSS. Le bouton restait visible et cliquable, mais son nom accessible devenait le seul résumé (« merge squash → main »). Un lecteur d'écran ne le trouvait plus sous ce nom, pas plus que les e2e environments.spec.ts et settings-links.spec.ts.

## Décision

Le bouton porte un nom fixe, « Après validation : <résumé> », à toutes les largeurs. Le libellé visible reste masqué sous 1000 px, comme décidé par ESC-390 (décision 129).

## Conséquences

Rien ne change à l'écran. Le nom du bouton ne dépend plus de la largeur.
