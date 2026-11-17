---
number: 197
id: "les-strategies-imposees-jugent-le-mode-o"
title: "Les stratégies imposées jugent le mode où le harness tourne"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-497"
pages: ["fenetre/reglages-imposes","harness/comptes"]
tickets: ["ESC-497","ESC-489","ESC-455","ESC-465"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages-imposes","added":11,"removed":0},{"page":"harness/comptes","added":2,"removed":0}]
accepted_at: "2026-11-17T15:16:48+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-489 (2.2.2) a posé que le mode jugé est celui où le harness tourne vraiment, mais seulement pour « Autoriser le mode Bypass ». Les stratégies de la 2.3 (TS-11, TS-12 : permissions.allowBypass, permissions.modes, permissions.defaultMode, du poste et d'une équipe) jugeaient encore le mode de l'agent. Le reversement de main dans release/2.3 devait les faire juger le même mode, et trancher ce que deviennent un nouvel agent, un agent dont le mode devient interdit et le mode par défaut d'une équipe face aux modes d'un harness.

## Décision

Sur chaque chemin (nouvel agent de la fenêtre, d'un outil ou du Kanban, ticket, tâche programmée, Kanban d'équipe, démarrage et reprise, vue terminal, interface gardée en fond pour Remote Control, démarrage d'avance, changement de mode, passation, Overseer), les stratégies jugent le mode où le harness fait tourner l'agent, avant « Autoriser le mode Bypass », et le refus dit ce mode et qui l'interdit. Un nouvel agent dont la stratégie refuse ce mode n'est pas créé, même depuis la fenêtre. Parti du mode par défaut, il va au plus proche des modes de son harness que la stratégie permet, jamais plus permissif. Le mode par défaut imposé par une équipe vaut pour les nouveaux agents et les tickets de ses projets. Bypass coupé par une équipe, ou absent de sa borne, retire aussi les arguments de Bypass des lancements de ses projets. Un agent dont le mode devient interdit passe au plus proche des modes de son harness. Ce qui tourne d'un agent (process, interface) dans un harness ou un mode que la stratégie ne permet plus, ou en Bypass quand « Autoriser le mode Bypass » vient d'être éteint dans l'app, ne reste pas en vie : au repos, il s'arrête aussitôt ; en plein tour, il finit son tour puis s'arrête (le tour n'est pas coupé, comme la spec le disait déjà d'un mode devenu interdit), et la conversation dit pourquoi avec la raison du refus. Le réglage de l'app éteint ne change pas le mode de l'agent. Un démarrage refusé (message, « Relancer », vue, reprise programmée, démarrage d'avance) arrête aussi ce qui tournait encore, avec la même raison.

## Conséquences

Rien ne change pour les harness d'Escouade, dont le mode demandé est celui où ils tournent. Un harness de plugin à un seul mode ou sans « Demander » peut se voir refuser par la stratégie un agent que « Autoriser le mode Bypass » seul aurait laissé créer ; la fenêtre le dit au lieu de créer un agent qui ne démarrerait jamais. Le mode par défaut d'une équipe s'applique désormais à ses projets (il n'était que lu et gardé). Éteindre « Autoriser le mode Bypass » arrête les agents qui tournent en Bypass (leur process démarré d'avance compris) ; leur conversation reprend là où elle était une fois le réglage rallumé ou le mode changé. Un tour en cours dans un mode devenu interdit va à son terme : c'est un délai accepté pour ne pas perdre le travail, comme pour un harness devenu interdit.
