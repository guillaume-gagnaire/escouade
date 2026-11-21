---
number: null
id: "reseau-prive-r5-la-liste-d-un-projet-est"
title: "Réseau privé, R5 : la liste d'un projet est changée par l'utilisateur, proposée par le dépôt, jamais par un agent"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-551"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-551", "ESC-587"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: ""
accepted_by: ""
---

## Contexte

La liste d'hôtes privés d'un projet ouvre une partie du réseau d'une entreprise à ses agents. Qui peut la changer (docs/PRIVATE-NETWORK.md § 4.4, R5) ?

## Décision

L'utilisateur, dans Réglages du projet › Réseau privé ; le .escouade.yml du dépôt peut la proposer, accordée par son empreinte comme ses commandes ; une stratégie imposée ou le .escouade.yml d'équipe peuvent la fixer ; aucun outil MCP ne la lit ni ne l'écrit. Le plafond du connecteur, lui, n'est changé que par l'entreprise.

## Conséquences

Un dépôt piégé n'ouvre pas seul le réseau d'une entreprise, un agent détourné non plus. Un projet d'équipe peut arriver avec sa liste, accordée une fois.
