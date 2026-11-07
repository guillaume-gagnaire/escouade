---
number: 115
id: "les-validations-d-un-projet-numerotent-l"
title: "Les validations d'un projet numérotent leurs décisions l'une après l'autre"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-377"
pages: ["docs/decisions"]
tickets: ["ESC-377"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/decisions","added":4,"removed":2}]
accepted_at: "2026-11-07T21:24:36+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-373 et ESC-374, validés à quelques secondes d'écart le 2026-11-07, ont reçu tous deux la décision 0113 : chaque validation lisait le registre de main avant que l'autre y soit mergée, sans verrou entre elles. La vue Décisions plantait aussi sur deux fichiers du même numéro (clé en double dans le registre).

## Décision

Un verrou par projet entoure la lecture des registres et la numérotation des décisions d'un ticket validé ; les numéros donnés restent retenus jusqu'à la fin de la validation (mergés, ils sont sur la branche ; arrêtée, ils redeviennent libres), et la validation suivante comme « Accepter » dans la fenêtre numérotent après eux. Un doublon déjà présent n'est pas corrigé par l'app : le registre montre les deux, chacun avec son détail.

## Conséquences

Deux validations simultanées d'un projet donnent deux numéros qui se suivent. Elles s'attendent quelques instants le temps de numéroter (pas pendant les tests ni le merge). Restent hors de portée : deux machines qui valident en même temps, et une validation par PR qui libère son numéro avant la fusion de la PR.
