---
number: 205
id: "prime-agent-harness-decrit-par-acp-au-se"
title: "Prime Agent, harness décrit par ACP au seul mode Bypass, sans session gardée dans son démon"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-485"
pages: ["plugins/harness-acp-decrits","plugins/plugins-d-exemple"]
tickets: ["ESC-485","ESC-484","ESC-489"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":15,"removed":2},{"page":"plugins/plugins-d-exemple","added":5,"removed":3}]
accepted_at: "2026-11-18T12:14:29+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur a demandé un plugin Prime Agent (Prime Intellect) le 2026-11-16. L'essai de la 0.10.3 dans un bac à sable montre un agent qui ne demande jamais rien, n'a qu'un outil (un noyau Python), ne charge pas de session par ACP (loadSession false) et démarre un démon par utilisateur qui survit à ses clients. Sans --no-session, chaque session ACP reste dans ce démon (worker et noyau Python) après le départ de l'agent. Le démon garde aussi le dossier avec lequel il a démarré : deux comptes y seraient mêlés.

## Décision

Le plugin dev.escouade.prime-agent décrit Prime Agent par ACP, sans code. Ses agents se lancent par `prime-agent --mode acp --no-session`. Il n'a qu'un mode, bypassPermissions, dont le sens est écrit en clair, et ni lecture seule, ni `fs`, ni accountVar : un seul compte. Sa cellule Python est montrée « Python » avec son code. Il demande Escouade 2.2.2 (escouade.min), qui refuse ses agents quand le mode Bypass n'est pas permis (ESC-489). plugins/ci.mjs juge chaque plugin pour son escouade.min quand celui-ci est plus récent que wasm-since.

## Conséquences

Aucune session, aucun worker ni noyau Python n'est laissé dans le démon après un agent. Le prix : pas de sous-agents rlm.spawn, et pas de reprise de session. Le démon lui-même reste à corriger dans l'app : il survit, vide, sous Unix, et meurt avec le Job du premier agent sous Windows, en emportant les sessions des autres agents. Le correctif à faire : un socket d'Escouade par compte, posé par le cœur, et l'arrêt du démon au dernier agent. Le client ACP doit aussi dire une session neuve et redonner les consignes quand un agent ne charge pas de session. Le plugin n'est ni signé, ni soumis, ni dans l'amorce.
