---
number: null
id: "escouade-n-ouvre-aucun-fichier-que-le-sy"
title: "Escouade n'ouvre aucun fichier que le système lancerait, installerait, monterait ou suivrait, et met en quarantaine ce qu'elle copie sous macOS"
date: "2026-11-22"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-628"
pages: ["fenetre/fenetre","harness/boite-a-toi"]
tickets: ["ESC-628"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/fenetre","added":9,"removed":1},{"page":"harness/boite-a-toi","added":3,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La revue de sécurité de la 2.3 (constat SR1-06) a montré que la liste des types refusés à « Ouvrir » (sentfiles::RUNNABLE) laissait passer des fichiers qui exécutent du code, montent un disque ou joignent un autre ordinateur en s'ouvrant (.terminal, .msc, .appinstaller, .iso, .library-ms…). Un agent détourné par le contenu d'un dépôt pouvait en écrire un dans le projet ; un clic sur « Ouvrir » l'aurait lancé. Le défaut est aussi dans la 2.2 publiée.

## Décision

1) RUNNABLE couvre quatre familles sur les trois systèmes à la fois : programmes et scripts, installeurs, images de disque, raccourcis (90 extensions, chacune justifiée dans le code d'après la liste des pièces jointes bloquées d'Outlook et les détournements connus de macOS et Linux), le nom jugé quelle que soit sa casse, ses points et espaces finaux et son flux NTFS. 2) Le refus dit sa famille et propose « Afficher dans le dossier » sur son toast (refus marqué `not-opened|`). 3) Sous macOS, Escouade pose com.apple.quarantine sur ce qu'elle copie ou reçoit (pièces jointes des demandes, images copiées d'un agent, fichier envoyé ouvert là où il est), sans écraser une quarantaine existante ; sous Windows elle garde la Zone.Identifier de la source, sans en inventer.

## Conséquences

Certains fichiers de développement (.py, .bin, .img, .psm1) ne s'ouvrent plus par le système depuis Escouade : ils se montrent dans leur dossier, comme .js déjà. La liste reste une liste de noms ; la quarantaine fait que Gatekeeper juge sous macOS aussi ce qu'elle ne connaît pas. La fenêtre garde une copie de la liste (src/lib/runnable.ts), tenue égale par un test Vitest.
