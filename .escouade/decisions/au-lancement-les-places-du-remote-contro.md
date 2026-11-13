---
number: null
id: "au-lancement-les-places-du-remote-contro"
title: "Au lancement, les places du remote control sous le plafond se prennent à tour de rôle, dans l'ordre de la dernière activité"
date: "2026-11-13"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-431"
pages: ["harness/process-des-agents"]
tickets: ["ESC-431"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/process-des-agents","added":1,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-417 triait les agents par dernière activité au lancement, mais chaque tâche appliquait d'abord ses réglages (apply_remote) avant de demander sa place : l'ordre d'arrivée au plafond dépendait du temps de chacune. Avec un plafond de 1, un agent plus ancien démarrait parfois à la place du plus récent (test du plafond rouge sur la CI de main).

## Décision

Dans start_remote, chaque agent reçoit un tour (RemoteTurn) chaîné dans l'ordre du tri : il ne prend le sémaphore des démarrages et ne demande sa place (remote_may_start) qu'après que le précédent a eu sa réponse ou a renoncé (tour lâché). Un agent actif garde son droit de démarrer hors plafond. Vaut pour les agents qui tenaient le remote control et pour ceux qui y entrent par le réglage.

## Conséquences

Démarrage déterministe des plus récents d'abord. La demande de place est sérialisée (rapide, sans attente réseau) ; les démarrages eux-mêmes restent trois à la fois. Un test force la course par un apply_remote retardé pour le plus récent.
