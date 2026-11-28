---
number: 365
id: "pas-de-bulle-de-la-decouverte-par-dessus"
title: "Pas de bulle de la découverte par-dessus une fenêtre ouverte"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-522"
pages: ["fenetre/a-decouvrir"]
tickets: ["ESC-522"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/a-decouvrir","added":3,"removed":0}]
accepted_at: "2026-11-21T18:00:57+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La bulle du premier ticket (le pilote auto, ESC-506) attend un moment calme jusqu'à six secondes. Sur un poste chargé, elle arrivait alors que la fenêtre du ticket suivant était ouverte, s'affichait par-dessus en montrant un contrôle caché dessous, et prenait l'Échap de la fenêtre : « Abandonner les modifications ? » n'était jamais demandé (app.spec « a ticket is written in a large window », 2 fois sur 2 sous charge en CI locale, reproduit 1 fois sur 8).

## Décision

Une fenêtre ouverte (app.modal) compte comme un moment occupé pour les bulles au bon moment, comme la frappe ou une permission : le moment attend qu'elle se ferme (six secondes au plus), sinon il passe. Une bulle au bon moment déjà à l'écran se ferme quand une fenêtre s'ouvre ; celle posée par « Essayer » reste, son contrôle pouvant être dans la fenêtre.

## Conséquences

Échap revient toujours à la fenêtre ouverte. Une bulle peut se perdre si une fenêtre reste ouverte plus de six secondes après son moment, comme pendant une frappe.
