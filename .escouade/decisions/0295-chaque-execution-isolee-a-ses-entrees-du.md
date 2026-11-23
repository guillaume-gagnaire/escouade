---
number: 295
id: "chaque-execution-isolee-a-ses-entrees-du"
title: "Chaque exécution isolée a ses entrées du trousseau à elle, reprises à sa fin et balayées quand elle est morte"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-584"
pages: ["kanban/integrations"]
tickets: ["ESC-584"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/integrations","added":3,"removed":0}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les e2e et les tests écrivaient leurs secrets sous le service commun dev.gagnaire.escouade.sandbox et ne les retiraient jamais : le 2026-11-21, 512 entrées « …escouade.sandbox » remplissaient le gestionnaire d'identifiants de Windows, qui refusait un secret long (Windows error code 8). Effacer tout le service aurait pu retirer le secret d'un e2e en cours dans un autre worktree.

## Décision

Sous Windows, une exécution isolée (ESCOUADE_DATA_DIR) écrit ses entrées sous `<entrée>.run-<étiquette>.dev.gagnaire.escouade.sandbox`, l'étiquette (FNV-1a sur 64 bits) tirée de son dossier de données, ce dossier noté dans le commentaire de l'entrée. Une build de debug répond à `--sandbox-keychain clean <dossier>… | sweep | list <dossier>…`. La fixture des e2e balaie au début de chaque test les restes des exécutions mortes (dossier disparu, ou entrée de plus de 24 h), et à sa fin, réussi ou non, reprend ce que les apps du test ont écrit. Les tests Rust du vrai trousseau passent par une exécution isolée de leur dossier, et la reprennent dans un Drop. Rien d'autre n'est jamais touché : seulement un nom qui finit par `.escouade.sandbox` et porte l'étiquette d'une exécution.

## Conséquences

Le trousseau ne se remplit plus des restes des tests. Les entrées écrites avant ESC-584 (sans étiquette) restent : c'est à l'utilisateur de les effacer (demande a-18). Après restartAs vers un autre dossier, une app ne retrouve plus les secrets du premier dossier. Sous macOS, rien ne change (pas d'e2e là-bas). Rien ne change pour l'utilisateur de l'app installée.
