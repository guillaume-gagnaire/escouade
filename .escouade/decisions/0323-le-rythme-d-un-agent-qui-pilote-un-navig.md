---
number: 323
id: "le-rythme-d-un-agent-qui-pilote-un-navig"
title: "Le rythme d'un agent qui pilote un navigateur : il choisit, geste par geste"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-663"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-663"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":2,"removed":1}]
accepted_at: "2026-11-26T00:48:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Dans le cas phare d'ESC-663, l'agent ouvre son app à côté de sa conversation et la pilote en expliquant. Il fallait choisir s'il avance seul, s'il attend « Continuer » à chaque geste, ou s'il choisit lui-même où attendre. La question a été posée à l'utilisateur dans « À toi » (a-37), avec la maquette du cas phare.

## Décision

Le choix C de l'utilisateur (2026-11-25) : l'agent choisit, geste par geste.

Chaque `drive_browser` lui rend la main quand le geste est fini et que la page s'est posée, 1,5 s au plus. L'agent avance seul pour faire défiler et montrer. Là où l'utilisateur doit avoir vu (avant et après, avant un choix), il passe `wait_user: true` : la bande de pilotage montre « Continuer » et l'outil attend ce clic, 5 minutes au plus, puis répond `continued`, `took_hand` ou `timeout`. « Reprendre la main » reste toujours possible.

## Conséquences

La démonstration reste fluide là où rien ne presse, et sûre là où ça compte. AB-7 (`wait_user` dans `drive_browser`) et AB-8 (« Continuer » dans la bande) partent sur ce rythme. La consigne du serveur MCP dit à l'agent d'attendre avant et après un changement qu'il montre.
