---
number: null
id: "en-2-2-aussi-le-tour-d-un-plugin-wasm-en"
title: "En 2.2 aussi : le tour d'un plugin WASM en temps processeur avec un plafond réel, le banc qui attend l'hôte, un fichier remplacé un à la fois, un hook tardif qui ne rouvre pas son tour"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-690"
pages: ["plugins/programmes-js-ts-en-wasm","plugins/contrat-du-manifeste","introduction/donnees-et-coupures","harness/claude-code"]
tickets: ["ESC-690","ESC-650"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":5,"removed":0},{"page":"plugins/contrat-du-manifeste","added":2,"removed":0},{"page":"introduction/donnees-et-coupures","added":3,"removed":0},{"page":"harness/claude-code","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-650 a corrigé sur release/2.3 quatre courses du produit, vues sous forte charge (décision 341 de la 2.3). Les utilisateurs de la 2.2 les ont aussi : un plugin WASM arrêté (« Crashed ») quand la machine compile à côté, le banc qui jugeait un fait trop tôt pendant que l'hôte lançait encore un programme, sous Windows une écriture de l'état ou des réglages refusée en « Accès refusé » quand deux écritures du même fichier se croisent, et dans la vue Claude Code un agent laissé « En cours » pour de bon par un hook UserPromptSubmit lu après la fin de son tour. ESC-690 les porte sur main pour une 2.2.x.

## Décision

Les mêmes règles que la décision 341 de la 2.3, sur main : 1) le tour d'un plugin WASM se compte en temps processeur de son fil, ce que l'hôte fait pour lui sur ce fil compris ; un plafond en temps réel de 10 s (TURN_MAX), vérifié par l'epoch et au retour de chaque appel à l'hôte, arrête toujours un plugin qui dort ou attend sans fin ; 2) après un fait, le banc attend 300 ms sans rien de nouveau et sans appel du plugin en cours ni programme du plugin qui tourne, 10 s au plus en temps réel ; 3) les remplacements d'un même fichier dans l'app passent un à la fois (un verrou par emplacement, sur le seul renommage) ; 4) un hook UserPromptSubmit arrivé avant la fin du dernier tour de l'interface ne rouvre pas ce tour. Les durcissements de tests de la 2.3 restent en 2.3.

## Conséquences

Un plugin n'est plus arrêté parce que la machine est chargée ; un plugin bloqué l'est toujours, au plus tard après 10 s. Le banc peut prendre jusqu'à 10 s pour un plugin qui garde un programme ouvert. Deux écritures d'un même fichier dans l'app s'attendent le temps d'un renommage. Coût : deux lectures du temps du fil par appel à l'hôte d'un plugin WASM. Au reversement dans release/2.3, ces changements y sont déjà (16809ce29).
