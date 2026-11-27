---
number: null
id: "la-2-2-11-a-la-2-2-19-reversees-dans-la"
title: "La 2.2.11 à la 2.2.19 reversées dans la 2.3 : les navigateurs et les pages des agents sur le cœur sans fenêtre, le menu « Éditeur » et les workflows suivent leurs interrupteurs"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-701"
pages: ["fenetre/fonctionnalites","fenetre/workspace","kanban/navigateurs-pilotes","harness/pages-des-agents"]
tickets: ["ESC-701","ESC-660","ESC-657","ESC-662","ESC-691"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/fonctionnalites","added":5,"removed":0},{"page":"fenetre/workspace","added":4,"removed":0},{"page":"kanban/navigateurs-pilotes","added":7,"removed":0},{"page":"harness/pages-des-agents","added":4,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

release/2.3 reçoit par une vraie fusion (ESC-701, second parent origin/main e3ea8e282, la 2.2.19) les 56 commits de main depuis la 2.2.10 : les navigateurs lus, ouverts et pilotés par les agents, les pages des agents, le Workspace, What's up ?, les mises à jour automatiques des plugins, la ligne d'un harness tiers relue au lancement, les plugins Copilot (protocole 4) et les sous-agents comptés sur leur modèle. Le code de main suppose un cœur qui tient l'AppHandle de Tauri (`core.app`) ; celui de la 2.3 n'a qu'un hôte (`Host`) et se construit aussi sans bureau (`escouade-server`). La 2.3 a ses propres interrupteurs de fonctionnalités (F-1) et ses parties de projet (ESC-658), que le menu « Éditeur » et les workflows déplacés dans le Workspace ne connaissaient pas. Et release/2.3 portait onze numéros de décision donnés deux fois.

## Décision

1. Le comportement des navigateurs et des pages de main, publié et vérifié en 2.2.13 à 2.2.18, vaut tel quel ; ESC-657 et son port ESC-660 étaient le même code. La 2.3 y garde en propre le relais des adresses d'un serveur (`onThisComputer`), les fenêtres des espaces dans les capacités, la partie « Lancement et navigateurs » et le bornage à la fonctionnalité `desktop`.
2. Le cœur atteint les webviews par l'hôte du poste : `Core::app()` (l'AppHandle, retrouvé par `Host::desktop`), `Core::browsers()` et `Core::shelter()` ; sans fenêtre, aucun. Sur `escouade-server`, les outils des navigateurs passent leurs contrôles puis répondent « Pas de navigateur sur un serveur », « Pages des agents » y vaut coupé, et ni WebKit, ni GTK, ni JavaScriptCore n'y sont liés.
3. Les commandes ajoutées par main entrent dans la liste unique : les gestes de la fenêtre sur ses webviews et la jauge du micro du poste seulement, celles qui touchent aussi l'état du cœur d'un navigateur ou d'une page refusées à distance (`not-served-yet`), `plugins_auto_failed` servie.
4. Les outils MCP offerts combinent les filtres des deux lignes : fonctionnalités (F-1), parties du projet de l'agent (ESC-658), pages et navigateurs allumés ; la consigne du serveur est celle de main (`get_info`).
5. Le bouton « Éditeur » ouvre le menu de main ; son entrée « Éditeur » (et son glisser) n'y sont que la fonctionnalité Éditeur allumée. « Workflows » du Workspace et l'étape d'un agent suivent l'interrupteur des Workflows. Les événements de main passent par `Core::track`, qui coche aussi les découvertes.
6. Les plugins de première main reprennent le code de main (Copilot : protocoles 3 et 4 et `turnModels` ; Codex et OpenCode : `turnModels`), écrit en JSON sur la crate publiée 0.1.2 du protocole, et montent au-dessus des deux lignes : copilot 2.2.11, codex et opencode 2.2.7.
7. Les onze numéros donnés deux fois sur release/2.3 suivent la règle de l'acceptation (le numéro qui suit le plus haut du registre de la branche où elle arrive) : celle arrivée la première sur release/2.3 garde le sien, l'autre prend le suivant libre, 0349 à 0359 dans l'ordre d'arrivée, et les pages qui la citaient suivent.

## Conséquences

Les deux lignes gardent leurs fonctionnalités ensemble ; le prochain reversement repart de ce commit. Un serveur sans fenêtre dit clairement qu'il n'a ni navigateur ni page, plutôt que d'en promettre. Les navigateurs et les pages restent du poste : le mode serveur des navigateurs (AB-R) reste à faire. Les plugins montés (copilot 2.2.11, codex et opencode 2.2.7) sont à signer et publier au store avec la 2.3. Les tickets et documents hors du dépôt qui citaient un des onze numéros déplacés (0168, 0227, 0228, 0229, 0261, 0262, 0263, 0277, 0288, 0329, 0340) gardent l'ancien.
