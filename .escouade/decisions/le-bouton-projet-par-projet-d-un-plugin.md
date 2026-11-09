---
number: null
id: "le-bouton-projet-par-projet-d-un-plugin"
title: "Le bouton projet par projet d'un plugin s'applique à son harness, activé au premier usage dans un projet privé"
date: "2026-11-09"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-404"
pages: ["plugins/vue-d-ensemble","plugins/livres-avec-escouade"]
tickets: ["ESC-404"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/vue-d-ensemble","added":9,"removed":1},{"page":"plugins/livres-avec-escouade","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Retour de bêta 2.2 (2026-11-09) : « J'arrive à utiliser Claude Code même sans avoir activé le projet dans les paramètres du plugin. » Le choix d'un plugin par projet (`plugins::active_in`) ne valait que pour ses faits, ses dossiers et ses autorisations ; le registre des harness les inscrit pour toute l'app. Un agent tournait donc sur un harness dont le plugin était coupé pour son projet, y compris dans un projet privé, dont la page du plugin dit « coupé d'office, rien ne lui parvient », alors que le module du harness reçoit tout ce que fait l'agent.

## Décision

Le cœur applique le bouton aux harness (première main et harness ACP décrits). Coupé pour un projet : le harness n'y est plus proposé (menus « Harness » du projet, rédacteur des textes, relève), le cœur refuse d'y créer, démarrer, reprendre, faire parler un agent sur lui ou d'ouvrir sa vue terminal, et le couper arrête ses agents de ce projet (le ticket d'un de ces agents est bloqué, avec la raison). Dans un projet privé, coupé d'office, le harness reste proposé : le premier agent sur lui demande une fois l'accord (ce que le plugin recevra, ses permissions) ; « Activer » allume le plugin pour ce projet et l'agent part, « Annuler » ne fait rien partir. Un agent existant dans ce cas montre une carte « Activer ». Le Kanban, le pilote auto et les outils MCP ne posent jamais la question : le ticket est bloqué avec la raison et « Activer » sur sa carte ; `create_agent` et `start_ticket` refusent ; aucun outil n'allume un plugin pour un projet. Migration, une fois : un projet privé qui a déjà des agents non archivés sur un harness a ce plugin allumé pour lui, là où l'utilisateur n'avait rien choisi.

## Conséquences

Les projets privés qui servaient déjà ne se bloquent pas d'un coup ; un projet privé neuf, ou rendu privé ensuite, demande l'accord à son premier agent. Les tests qui font tourner des agents dans un projet privé y allument d'abord le harness, comme l'utilisateur. Aucun plugin de `plugins/` n'a changé : le cœur décide. Les docs du site qui disaient les plugins livrés « actifs partout, projets privés compris » sont corrigées.
