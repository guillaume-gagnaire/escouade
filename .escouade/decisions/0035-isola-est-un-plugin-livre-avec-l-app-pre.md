---
number: 35
id: "isola-est-un-plugin-livre-avec-l-app-pre"
title: "isola est un plugin livré avec l'app, premier « environnement de lancement »"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-286"
pages: ["kanban/lancement-de-test"]
tickets: ["ESC-286"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/lancement-de-test","added":1,"removed":0}]
accepted_at: "2026-11-01T16:12:55+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

isola était codé en dur dans le cœur (isola.rs). Les décisions du 2026-10-26 veulent tout l'externe en plugins, en process, les plugins de première main livrés avec l'app et désactivables (INT-14, ESC-286).

## Décision

Un point d'extension `contributes.launchEnvironments` (detect, program, up) ; le cœur décide qu'un worktree est celui d'un environnement et lance `up` dans un terminal après approbation du fichier lu ; le plugin, un process (`escouade-plugins isola`), répond à `launch/ls`, `launch/down` et `launch/destroy`. Seuls les plugins livrés avec l'app servent ce point.

## Conséquences

isola se coupe pour l'app ou un projet ; coupé, un ticket part comme sans isola. Un environnement tiers (Docker Compose, devcontainers) demandera son propre contrat de confiance pour `up`.
