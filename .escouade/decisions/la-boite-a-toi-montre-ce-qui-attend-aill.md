---
number: null
id: "la-boite-a-toi-montre-ce-qui-attend-aill"
title: "La boîte « À toi » montre ce qui attend ailleurs, et la prévient par un résumé"
date: "2026-11-17"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-513"
pages: ["harness/boite-a-toi","fenetre/notifications","harness/serveur-mcp"]
tickets: ["ESC-513"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":33,"removed":0},{"page":"fenetre/notifications","added":4,"removed":0},{"page":"harness/serveur-mcp","added":3,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La boîte « À toi » (IN-1, ESC-509) recevait les demandes asynchrones des agents, mais ce qui attend déjà l'utilisateur ailleurs (permissions, questions et plans d'agents arrêtés, tickets à valider ou bloqués, décisions proposées, docs à partager, relectures demandées) restait dispersé, et rien ne prévenait de l'arrivée d'une demande. docs/INBOX.md § 5, § 11 et § 13 en ont fait la conception ; l'utilisateur a voulu la boîte dans la ligne 2.2 (2026-11-17).

## Décision

IN-3 (ESC-513), sur main : la boîte montre des vues sur l'état réel de ces sources, jamais des copies, avec les gestes de leur source ; un geste se fait sur la vue lue et est refusé si elle a été réglée ailleurs ; « Plus tard » marche sur une vue sauf une bloquante. Les demandes asynchrones ne notifient que par un résumé (2 minutes de regroupement, puis une notification au plus toutes les 30 minutes ou par heure, jamais le titre d'une demande, ni carillon ni clignotement), réglé dans Réglages › Notifications et inexistant serveur MCP éteint. Une seule règle juge qui peut répondre à la place de l'utilisateur, pour la boîte et pour les demandes bloquantes. La consigne aux agents (quand déposer une demande plutôt que l'écrire dans son bilan) est dans les instructions du serveur MCP et dans la mémoire partagée.

## Conséquences

Un seul endroit pour ce qui attend l'utilisateur, sans changer les places d'aujourd'hui. Nouvelles commandes inbox_views, inbox_view_act, inbox_view_snooze ; réglage inboxSummary ; événement inboxOpen ; cinq événements de statistiques (ask_created, ask_withdrawn, ask_expired, inbox_view_settled, inbox_summary_shown). L'Overseer, l'API distante, les réglages imposés, l'invitation d'équipe et la station de l'onboarding sont laissés au reversement dans release/2.3 (docs/INBOX.md § 18). Sur main, un hôte provisoire de la boîte attend l'écran d'IN-2, qui le remplace à la fusion.
