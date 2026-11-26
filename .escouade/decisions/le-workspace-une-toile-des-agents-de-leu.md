---
number: null
id: "le-workspace-une-toile-des-agents-de-leu"
title: "Le Workspace : une toile des agents, de leurs outils et de leurs livrables, et un résumé de toile par agent dans le cœur"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: ""
pages: ["fenetre/workspace","fenetre/barre-des-projets","fenetre/vue-d-ensemble","conversation/avancee-d-un-plan","kanban/environnements"]
tickets: []
documents: ["docs/superpowers/specs/2026-11-25-vue-workspace-design.md"]
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/workspace","added":74,"removed":0},{"page":"fenetre/barre-des-projets","added":1,"removed":1},{"page":"fenetre/vue-d-ensemble","added":1,"removed":1},{"page":"conversation/avancee-d-un-plan","added":1,"removed":0},{"page":"kanban/environnements","added":4,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Ce que font les agents d'un projet est éparpillé : la barre latérale en détaille un à la fois, le plan est au-dessus de la conversation, les sous-agents sont une liste, la CI une pastille. Trois compositions ont été maquettées avec l'utilisateur : la boussole (l'agent au centre, un sens par côté), les couloirs (une colonne par agent, en bandes) et la scène (un seul agent déplié). Les skills, les plugins, les artefacts et les commandes de tests se lisent dans les appels d'outils, or la fenêtre ne charge qu'une conversation à la fois.

## Décision

Une vue « Workspace », ouverte par le menu du bouton « Éditeur » et depuis la « Vue d'ensemble » : une toile où chaque agent non archivé est une boussole, son plan au-dessus, ses outils, son équipe à droite, ses tests sous sa carte et ses livrables (artefacts, CI distante, déploiements) dans une zone d'atterrissage en dessous. Deux altitudes : sous 60 % de zoom, les côtés en piles ; au-delà, tout déplié ; un agent qui attend toujours déplié. On y répond aux questions et on y écrit à un agent par le chemin d'envoi de la conversation, sorti de `Composer` et partagé.

Le cœur tient pour chaque agent un résumé de toile (`AgentMeta::canvas`, crate `escouade-plan`), envoyé avec lui et mis à jour à chaque appel d'outil : skills et plugins de l'agent et de chaque sous-agent, ses dernières commandes de terminal, ses artefacts. Il dit aussi les déploiements d'un commit de la branche de chaque agent. La toile ne charge la conversation d'un agent que le temps de sa question.

## Conséquences

La toile ne coûte ni mémoire ni appels de plus par agent. Le résumé est borné (40 outils par fil, 30 sous-agents, 20 artefacts, 5 commandes) et n'est jamais remis à zéro : il dit ce que l'agent a utilisé dans sa vie. Pas de bibliothèque de graphe : un `<svg>` sous des cartes en HTML. Les cartes ne se déplacent pas à la main ; la toile ne s'ouvre pas dans un volet de l'écran partagé.
