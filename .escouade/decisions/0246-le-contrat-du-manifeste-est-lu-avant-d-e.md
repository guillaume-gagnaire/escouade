---
number: 246
id: "le-contrat-du-manifeste-est-lu-avant-d-e"
title: "Le contrat du manifeste est lu avant d'être servi, et ce qui n'est pas servi coupe le plugin"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-525"
pages: ["plugins/contrat-du-manifeste","plugins/vue-d-ensemble"]
tickets: ["ESC-525"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/contrat-du-manifeste","added":47,"removed":0},{"page":"plugins/vue-d-ensemble","added":2,"removed":0}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les issues #38, #39, #40 et #42 d'un auteur de plugin tiers demandent des profils, une portée « projet », des groupes, des champs sur plusieurs lignes, l'import d'un fichier, des jokers typés dans les commandes et une limite de programmes déclarée. L'étude ESC-501 les a conçus et découpés en tickets (T1 à T9). Le premier, ESC-525, livre le contrat lu et refusé par les deux lecteurs du manifeste, avant que la fenêtre et l'hôte ne servent chaque point. Une app qui lit un point sans le servir ne doit pas se tromper en silence.

## Décision

Le lecteur de l'app, celui du store et le schéma plugin-1.json lisent les ajouts de l'étude, par ajouts au schéma : scope, repo, description d'un secret, multiline, lines, import, groups, profiles, type « profile » et of, profile:<genre>, limits.processes et reason. Ils refusent ce qui sort de la grammaire avec six codes nouveaux (misplaced, repo, bounds, extension, field, profile). La grammaire des jokers du §5.3 est appliquée à la lettre (code command), sur un découpage des mots identique dans les deux lecteurs. Un plugin qui déclare des profils, un secret de portée project, un joker ou plus de 4 programmes à la fois est chargé coupé, la raison dite, et la fenêtre d'installation le dit avant l'installation ; escouade-plugin validate l'avertit sans échouer. repo: false et import sont déjà tenus : la valeur de .escouade.yml n'est pas prise pour ces réglages.

## Conséquences

Un manifeste d'aujourd'hui se lit pareil. Une accolade dans un mot littéral de permissions.commands, jusqu'ici admise, est refusée : aucun plugin de la première main n'en a. Chaque ticket suivant retire de Manifest::unserved le point qu'il sert : T4 les profils et les secrets d'un projet, T5 les jokers, T6 la limite. Le store doit déployer schemas/plugin-manifest.mjs (contrôle n° 1) et store/server/utils/checks.ts (contrôle n° 9 : profile:<genre>) pour lire ces manifestes.
