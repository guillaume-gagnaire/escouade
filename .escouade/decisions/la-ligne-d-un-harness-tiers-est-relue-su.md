---
number: null
id: "la-ligne-d-un-harness-tiers-est-relue-su"
title: "La ligne d'un harness tiers est relue sur le plugin installé à l'instant du lancement"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-697"
pages: ["plugins/harness-acp-decrits"]
tickets: ["ESC-697","ESC-589"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":5,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La CI de main (45992705126, Windows) a montré une course : un démarrage commencé avant une mise à jour du plugin (le chauffage qui suit « Autoriser ») tient l'objet harness d'alors, dont l'ancienne ligne reste parmi les quatre lignes gardées (KEPT). Un test l'établit (a_start_begun_before_an_update_does_not_run_the_old_line_after_it) : sans correctif, l'ancienne ligne tournait après la mise à jour, sans nouvel accord. Le registre des harness n'était en outre remis à jour qu'à la relecture de la fenêtre.

## Décision

Juste avant que le programme d'un harness décrit par un tiers tourne (lancement, relance par son plugin, question ponctuelle), le cœur refait la ligne que décrit le plugin installé à cet instant (registre) pour ce programme : changée, elle redemande l'accord, ou, déjà acceptée, le démarrage commencé sur l'ancienne est refusé (« relance l'agent ») ; un harness sorti du registre ne lance rien. L'installation d'un plugin remet aussitôt le registre à jour. Un agent vivant à la mise à jour garde son process sur la ligne d'alors ; son démarrage suivant redemande l'accord si la ligne a changé.

## Conséquences

Aucune ancienne ligne ne tourne après une mise à jour qui la change. Seul changement pour un agent vivant : une relance de son programme par son plugin (`relaunches`) après une mise à jour qui change la ligne est refusée. Les tests qui fabriquent un harness décrit le mettent au registre, comme l'app.
