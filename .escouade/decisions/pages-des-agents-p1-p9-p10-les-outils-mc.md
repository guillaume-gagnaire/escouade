---
number: null
id: "pages-des-agents-p1-p9-p10-les-outils-mc"
title: "Pages des agents P1, P9, P10 : les outils MCP, qui montre, durée de vie (la cible du pilotage fixée)"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-666"
pages: ["harness/pages-des-agents"]
tickets: ["ESC-666","ESC-661","ESC-663"]
documents: []
supersedes: [316]
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":24,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La décision 316 (ESC-661, choix de l'utilisateur a-36) fixait les outils des pages et laissait à AP-0 la forme de la cible du pilotage (`page: "p-12"` ou `browser: "page:p-12"`). ESC-666 (AP-0) a tranché le pilotage de toutes les lignes de « Navigateurs » dans la décision *Un seul pilotage pour toutes les lignes de « Navigateurs »* (un-seul-pilotage-pour-toutes-les-lignes). Il porte aussi les pages en 2.2, sur `main`, qui n'a pas l'Overseer. Cette décision-ci ne garde que ce qui est propre aux pages, pour qu'il n'y ait plus qu'une décision sur le pilotage.

## Décision

Choix de l'utilisateur du 2026-11-25 (a-36), inchangés sur le fond.

**P1, les outils** :
- `show_page` : modèle, données ou HTML / fichier du worktree, versions, `ask` ;
- `get_page` ;
- `close_page`.

Une page se pilote par `drive_browser`, visée par `browser: "page:p-12"`, selon la décision *Un seul pilotage pour toutes les lignes de « Navigateurs »*. Sur une page, il n'y a que les gestes de montrer, `tab` et `version`. Écarté : un seul outil `page` à actions.

**P9, qui montre** : les agents et les discussions ; l'Overseer en 2.3 (`main` ne l'a pas) ; Claude hors Escouade seulement avec une demande.

**P10, la durée de vie** :
- la page vit avec son agent, puis le délai des fichiers de la boîte (30 j par défaut) ;
- 20 pages par agent, 10 versions.

Écarté : le temps du lancement d'Escouade seulement.

## Conséquences

- Un registre `p-<n>` dans l'état durable et des copies dans `<données>/pages/`, construits par AP-2 en 2.2, de `main`.
- Le pilotage d'une page est construit par AB-7 et AB-8, où AP-6 est fondu.
- Les outils se coupent par l'interrupteur « Pages des agents » de Réglages › Agents en 2.2. En 2.3, ce sont Fonctionnalités et la partie « Lancement et navigateurs », par AB-R.
