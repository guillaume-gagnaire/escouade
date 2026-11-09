---
number: 117
id: "l-amorce-de-la-2-2-les-26-plugins-signes"
title: "L'amorce de la 2.2 : les 26 plugins signés, Goose et ceux sur demande compris ; Antigravity CLI et isola restent donnés par l'app"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-354"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-354"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":9,"removed":0}]
accepted_at: "2026-11-08T00:13:20+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Z2 (ESC-354) remplit plugins/seed.json d'après le catalogue du store. Au 2026-11-07, 26 plugins de première main ont une dernière version signée ; Antigravity CLI n'en a pas (2.2.1 non signée, 2.2.2 pas publiée) et isola n'est pas au store. Goose et les plugins sur demande (Discord, Slack, Contraste élevé, Règles de l'équipe) sont publiés et signés, mais n'étaient pas au registre de builtin.rs, seule liste des ids que l'amorce acceptait. Avec la liste remplie, un build de développement sans les fichiers de l'amorce perdait aussi tous ses harness.

## Décision

L'amorce nomme les 26 versions signées : Goose et ceux sur demande y entrent (builtin::seeds les accepte, par sort::seeds_beyond_the_registry), proposés par la vue et installés seulement si la configuration les utilise. Antigravity CLI et isola restent hors de la liste, donnés par l'app (origine builtin) jusqu'à leur signature : l'overseer l'a choisi le 2026-11-07. Un build de développement écarte les entrées dont l'amorce n'a pas les fichiers et les lance du registre ; une release les garde et dit ce qui manque. La release attend la première liste de révocation signée par l'utilisateur : seed.ts garde son refus d'une amorce sans liste (choix de l'overseer le 2026-11-07).

## Conséquences

Une première release de la 2.2 demande d'abord la liste de révocation nº 1 au store, et la version passée à 2.2.0 (les versions de l'amorce demandent escouade.min 2.2.0). Chaque nouveau plugin signé (Antigravity CLI, isola) s'ajoute d'une ligne à plugins/seed.json. L'amorce ajoute 2,5 Mo à l'installeur (17,4 Mo en clair).
