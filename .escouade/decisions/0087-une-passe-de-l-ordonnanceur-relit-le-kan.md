---
number: 87
id: "une-passe-de-l-ordonnanceur-relit-le-kan"
title: "Une passe de l'ordonnanceur relit le Kanban avant chaque démarrage du pilote auto"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-357"
pages: ["kanban/ci-des-branches"]
tickets: ["ESC-357"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/ci-des-branches","added":2,"removed":0}]
accepted_at: "2026-11-05T14:49:16+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Une passe prenait une copie de tous les projets à son début et décidait dessus. Chaque démarrage prend quelques secondes. Un pilote auto éteint, une pause allumée ou une cible vue rouge pendant la passe ne retenait donc pas les tickets qu'elle n'avait pas encore démarrés. ESC-357 l'a montré : l'instrumentation de 9 échecs sur 9 a montré une passe sur « pause non », alors que le Kanban disait déjà « pause oui » et que la cible était rouge.

## Décision

Chaque projet est relu à son tour dans la passe. Avant chaque démarrage du pilote auto, le Kanban et la CI de la cible sont relus : s'il est éteint ou retenu par la cible rouge, le ticket reste « À faire ». Un ticket lancé à la main part quand même, comme avant.

## Conséquences

Le pilote auto éteint et la pause s'appliquent aussi aux démarrages restants de la passe en cours, comme la pause au-delà du quota. Un ticket déjà « En cours » continue. Non traité : au démarrage de l'app, la CI de la cible reste inconnue jusqu'à sa première lecture, et la pause ne retient rien pendant ce temps (écrit dans les limites de la page).
