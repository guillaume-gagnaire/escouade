---
number: 350
id: "cloisonnement-windows-w2-tout-l-arbre-du"
title: "Cloisonnement Windows, W2 : tout l'arbre du harness tourne sous la barrière, pas ses seules commandes"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-591"
pages: ["projets/cloisonnement-windows"]
tickets: ["ESC-591"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:58:33+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-591 (docs/WINDOWS-ISOLATION.md, § 5 et § 6). La barrière d'un jeton restreint peut porter sur le harness et tout ce qu'il lance, ou seulement sur les commandes de ses outils, par un shell de remplacement. Le harness lit lui-même des fichiers qu'il tient pour tous les projets (.claude.json, history.jsonl), que la barrière ne peut donc pas refuser.

## Décision

Choisi par l'utilisateur le 2026-11-27 (demande a-44, option a) : le lanceur d'Escouade met tout l'arbre du harness sous le jeton. Ce que le harness tient lui-même pour tous les projets sort de la liste de la barrière et reste un garde-fou, par les règles deny d'aujourd'hui. Pourquoi : les seules commandes dépendraient de la façon dont chaque harness lance son shell (Git Bash, PowerShell, son propre lanceur) ; l'arbre entier ne dépend que d'Escouade.

## Conséquences

Ne dépend que d'Escouade, pas de la façon dont chaque harness lance son shell (Git Bash, PowerShell, son propre lanceur). La liste de la barrière est la clôture moins les fichiers mêlés du harness.
