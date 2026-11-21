---
number: null
id: "la-limite-de-programmes-d-un-plugin-est"
title: "La limite de programmes d'un plugin est celle qu'il déclare (limits.processes, 16 au plus), une permission dite au-delà de 4"
date: "2026-11-19"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-529"
pages: ["plugins/contrat-du-manifeste"]
tickets: ["ESC-529"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/contrat-du-manifeste","added":26,"removed":8}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Un plugin en WASM ne lançait que 4 programmes à la fois (proc/spawn), une limite fixe de l'hôte. Un plugin de tunnel (issue #42) en a besoin de deux par tunnel et d'un tunnel par projet. ESC-525 lisait déjà limits.processes (1 à 16) et limits.reason, mais coupait un plugin qui demandait plus de 4. L'étude ESC-501 (§5.6) recommandait une limite déclarée plutôt qu'un compte par projet ; l'utilisateur l'a choisie le 2026-11-17 (choix n° 8).

## Décision

limits.processes (1 à 16, 4 d'office) fixe le nombre de programmes vivants d'une instance ; l'hôte ne donne jamais plus de 16, quoi que dise le manifeste, et une place se rend à la fin d'un programme. Le refus dit la limite du plugin (« N programs at once at most for this plugin (limits.processes) »), dans la réponse et au journal. Au-delà de 4, pour un plugin qui a un programme et des commandes, c'est une permission (mot processes:<N>) : la fenêtre d'installation et l'onglet Permissions disent « Jusqu'à N programmes à la fois », « 4 sans le demander » et la raison de l'auteur (limits.reason) ; une mise à jour qui l'augmente redemande l'accord, une baisse non ; la retirer rend 4. À 4 ou moins, rien ne change. La coupure d'ESC-525 pour ce point est retirée.

## Conséquences

Un plugin peut tenir jusqu'à 16 programmes : au pire une vingtaine de Mio de l'app (environ 1,3 Mio par programme : trois tuyaux, quatre tâches, 256 Kio de crédit, 1 Mio d'entrée en attente) plus les programmes eux-mêmes. « Tout mettre à jour » ne fait pas passer une hausse sans la demander. La revue du store juge la raison. Les plugins de la première main n'ont pas limits.processes et gardent 4. PROCESSES_SERVED devient PROCESSES_UNASKED dans les deux lecteurs.
