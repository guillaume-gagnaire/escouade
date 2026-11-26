---
number: 341
id: "le-tour-d-un-plugin-wasm-en-temps-proces"
title: "Le tour d'un plugin WASM en temps processeur, avec un plafond réel ; le banc attend l'hôte ; un fichier remplacé un à la fois"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-650"
pages: ["plugins/programmes-js-ts-en-wasm","plugins/contrat-du-manifeste","introduction/donnees-et-coupures","harness/claude-code"]
tickets: ["ESC-650"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":7,"removed":0},{"page":"plugins/contrat-du-manifeste","added":2,"removed":0},{"page":"introduction/donnees-et-coupures","added":3,"removed":0},{"page":"harness/claude-code","added":2,"removed":0}]
accepted_at: "2026-11-26T17:22:42+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-650 (tests fragiles sous charge, lot 3). Sous une forte charge (un nextest complet à côté, une grosse compilation chez l'utilisateur), trois budgets ou ordres du produit tombaient sans faute du code qu'ils jugent : le tour de 2 s d'un plugin WASM, compté en temps réel, arrêtait le plugin (« Crashed ») ; le banc de conformité jugeait un fait après 300 ms de silence, alors que l'hôte lançait encore un programme du plugin ; sous Windows, deux remplacements d'un même fichier de données au même instant échouaient en « Accès refusé ». Et dans la vue Claude Code, un hook `UserPromptSubmit` lu après la fin de son tour remettait l'agent « En cours » pour de bon.

## Décision

1) Le tour d'un plugin WASM se compte en temps processeur de son fil, ce que l'hôte fait pour lui sur ce fil compris (escouade.call, WASI, le lancement d'un programme). Un plafond en temps réel de 10 s (Limits::ceiling, TURN_MAX par défaut), vérifié par l'epoch et au retour de chaque appel à l'hôte, arrête toujours un plugin qui dort ou attend sans fin. 2) Après un fait, le banc attend 300 ms sans rien de nouveau ET sans appel du plugin en cours ni programme du plugin qui tourne, 10 s au plus en temps réel. 3) Les remplacements d'un même fichier dans l'app passent un à la fois (un verrou par emplacement, sur le seul renommage). 4) Un hook UserPromptSubmit arrivé avant la fin du dernier tour de l'interface ne rouvre pas ce tour. Choix 1 et 2 de l'overseer, le 2026-11-26 (« Temps CPU + plafond », « Attendre l'hôte »).

## Conséquences

Un plugin n'est plus arrêté parce que la machine est chargée. Un plugin qui ne fait que dormir dans un appel à l'hôte reste coupé par le temps réel de cet appel, et dans tous les cas par le plafond de 10 s. Le banc prend jusqu'à 10 s pour un plugin qui garde un programme ouvert. Deux écritures d'un même fichier dans l'app attendent l'une l'autre le temps d'un renommage. Le coût : deux lectures du temps du fil par appel à l'hôte d'un plugin WASM.
