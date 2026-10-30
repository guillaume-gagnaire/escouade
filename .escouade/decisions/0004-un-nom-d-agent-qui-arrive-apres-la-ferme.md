---
number: 4
title: "Un nom d'agent qui arrive après la fermeture de son projet est perdu"
date: "2026-10-30"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-300"
pages: ["projets/fermer-un-projet"]
tickets: ["ESC-300"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/fermer-un-projet","added":1,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le nom donné après le premier message (ESC-156) est demandé au modèle des textes en arrière-plan. Fermé puis rouvert pendant ce temps, le projet rend l'agent sous le même id : le nom en route s'écrivait dans l'agent rouvert (et pouvait renommer sa branche) alors que l'historique gardait l'ancien. Test Linux intermittent closed_tests (ESC-300).

## Décision

Le nom s'applique à l'agent qui l'a demandé, tant qu'il est dans l'app. La fermeture attend un nommage déjà commencé (verrou de nommage du projet) : le nom et la branche renommée sont gardés dans l'historique. Un nom qui arrive après est perdu : rien n'est renommé, ni l'agent rouvert ni sa branche. Même règle pour un renommage à la main qui attendait.

## Conséquences

L'historique d'un projet fermé est toujours ce que l'agent rouvert montre ; un agent fermé trop tôt garde agent-N (renommable à la main). La fermeture peut attendre la fin d'un renommage de branche en cours.
