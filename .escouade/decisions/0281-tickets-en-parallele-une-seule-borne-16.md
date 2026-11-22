---
number: 281
id: "tickets-en-parallele-une-seule-borne-16"
title: "Tickets en parallèle : une seule borne, 16 sur un poste, 64 sur un serveur, un plafond imposable"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-620"
pages: ["kanban/kanban","a-distance/mode-serveur","fenetre/reglages-imposes","harness/serveur-mcp","kanban/pilote-auto"]
tickets: ["ESC-620"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/kanban","added":19,"removed":1},{"page":"a-distance/mode-serveur","added":5,"removed":0},{"page":"fenetre/reglages-imposes","added":7,"removed":0},{"page":"harness/serveur-mcp","added":5,"removed":1},{"page":"kanban/pilote-auto","added":1,"removed":1}]
accepted_at: "2026-11-22T06:16:34+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

« En parallèle » était borné à 6 en dur, à sept endroits (clamp(1, 6) du Kanban, de start_ticket, de get_board, de board_set ; 1..=6 de set_board ; les puces 1 à 6 de la fenêtre et de l'assistant). L'utilisateur demande le 2026-11-21 de monter plus haut, surtout sur un serveur, qui tourne H24 sur une machine dimensionnée pour lui. Un poste, lui, partage sa machine avec son utilisateur, et chaque agent qui compile Escouade et lance ses e2e prend plusieurs Go et plusieurs cœurs.

## Décision

Une seule borne, dans le modèle (escouade_model::parallel), que tout suit : le Kanban et le pilote auto, start_ticket et set_board (et get_board, qui dit maxParallelLimit), la fenêtre (Réglages › Kanban, l'assistant), l'API distante (board_set, board_capacity servie). De 1 à 16 sur un poste ; de 1 à 64 sur un serveur (escouade-server, et l'espace d'un serveur vu d'un client : son cœur répond). Une nouvelle stratégie, projects.maxParallel (restreindre, un plafond de 1 à 64), l'abaisse, du poste géré comme du enforced du .escouade.yml d'équipe ; le plus bas tient. Le .escouade.yml du dépôt n'en porte pas (ESC-163 : le nombre d'agents est de la machine et du quota de chacun). Au-delà de 6, la fenêtre avertit d'après la machine (cœurs, mémoire, place d'un agent mesurée sur le dépôt d'Escouade) et dit que le quota part plus vite : une indication, jamais un refus. set_board refuse hors de la borne en disant d'où elle vient ; la fenêtre ramène dans la borne.

## Conséquences

16 sur un poste : au-delà, même une grosse station (32 cœurs, 128 Go) ne tient pas tant d'agents qui compilent, et le quota d'un compte s'épuise en peu de temps ; 64 sur un serveur : de quoi remplir une grosse machine louée pour ça, sans laisser une valeur absurde. Une valeur réglée plus haut qu'un plafond arrivé ensuite est gardée, le Kanban ne démarrant pas plus que la borne. Le partage par compte ne change pas : la pause au-delà du quota du pilote auto tient quel que soit le nombre de places. Une ligne de plus dans l'ADMX, les profils de macOS, le schéma de /etc et la page « Stratégies du poste » ; escouade-policy dépend d'escouade-model pour la borne.
