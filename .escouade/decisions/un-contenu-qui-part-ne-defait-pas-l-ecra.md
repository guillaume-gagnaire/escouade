---
number: null
id: "un-contenu-qui-part-ne-defait-pas-l-ecra"
title: "Un contenu qui part ne défait pas l'écran partagé"
date: "2026-11-09"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-399"
pages: ["fenetre/ecran-partage"]
tickets: ["ESC-399"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/ecran-partage","added":12,"removed":3}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Retour en bêta 2.2 (2026-11-09) : avec un agent en haut et une console en bas, archiver l'agent faisait disparaître la console. L'écran partagé (ESC-392, décision 128) retirait l'agent archivé de son volet ; avec un seul volet restant, la disposition tombait et la zone montrait seul l'agent sélectionné (l'archivé), sans le terminal.

## Décision

Le volet d'un agent archivé ou supprimé (par son menu, le Kanban, le serveur MCP ou un autre écran, projet affiché ou non) prend l'agent que l'app montrerait seul à sa place : le premier agent non archivé du projet, par date de création, qui n'est pas déjà dans un volet ; même place, même part, actif si l'agent parti l'était. Sans autre agent, et pour un terminal fermé ou un lancement retiré, le volet se ferme et son voisin prend sa place ; les autres volets gardent place et parts. Le dernier volet restant occupe seul la zone et c'est lui qu'on voit. La disposition ainsi remise en ordre est écrite (elle revient au redémarrage).

## Conséquences

La disposition qu'on s'est faite (un terminal en bas, par exemple) survit au départ d'un agent. La sélection suit le volet actif dans chaque projet, même hors écran. Fonction pure `settle` dans lib/panes.ts, appelée par l'état à l'archivage, à la suppression, au retrait d'un lancement et à la fermeture d'un terminal.
