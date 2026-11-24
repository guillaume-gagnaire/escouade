---
number: 311
id: "les-parties-d-un-projet-s-allument-et-se"
title: "Les parties d'un projet s'allument et se coupent par projet ; le poste passe devant le .escouade.yml"
date: "2026-11-23"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-658"
pages: ["projets/parties-du-projet","projets/escouade-yml","fenetre/reglages"]
tickets: ["ESC-658"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/parties-du-projet","added":37,"removed":0},{"page":"projets/escouade-yml","added":17,"removed":0},{"page":"fenetre/reglages","added":4,"removed":0}]
accepted_at: "2026-11-24T02:38:46+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur veut (2026-11-23) activer ou désactiver, projet par projet, les Docs, la mémoire partagée, le Kanban, le lancement et les navigateurs : dans les réglages du projet sur chaque poste, avec un défaut que le dépôt peut proposer par .escouade.yml. Les fonctionnalités (ESC-550) ne se coupaient que pour toute l'app, et un dépôt cloné ne doit, par ce défaut, rien faire tourner de plus que ce que l'utilisateur a permis ni éteindre une protection.

## Décision

Quatre parties par projet (docs, memory, kanban, launch), un registre dans le cœur (escouade_model::parts) et son miroir dans la fenêtre. Une partie coupée retire, dans ce projet seul, son interface, ses outils MCP (absents de la liste de ses agents, refusés pour lui avec la raison, quel que soit l'appelant qui le nomme), ce que ses agents en entendent (spec, documents, mémoire, recette de lancement, tickets d'un brainstorm) et son travail de fond (comparaison au code, Planifier, discussions ; démarrage des tickets et workflows ; lancements et sondes), sans rien effacer. Qui décide, le premier l'emporte : l'équipe qui l'impose, ce poste (Réglages › Projets › Parties du projet, gardé dans l'état de la machine, jamais dans un fichier), le dépôt (parts.<id> du .escouade.yml, une clé du fichier seul : ni appliquée, ni exportée), l'équipe qui le recommande, sinon allumée. Une partie ne fait que retirer : allumée, elle ne lance rien que ses propres interrupteurs ne lancent, donc un dépôt qui dit true ne fait rien tourner de plus, et false ne lève aucune protection (confiance des commandes, projet privé, autorisations, stratégies). La fonctionnalité de l'app coupée emporte la partie dans tous les projets.

## Conséquences

Aucun projet existant ne change (tout allumé sans rien de dit). Une nouvelle page de réglages par projet et une commande (project_set_part) dans la liste unique ; quatre clés parts.* au schéma de .escouade.yml ; l'équipe peut les recommander ou les imposer. Les navigateurs d'ESC-657 se branchent sur la partie launch (partOn / Core::part_on). Un client MCP extérieur voit toujours la liste entière, ses appels pour un projet coupé sont refusés.
