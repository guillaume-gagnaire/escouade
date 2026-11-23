---
number: 290
id: "le-chef-d-escouade-prend-en-charge-et-co"
title: "Le chef d'escouade prend en charge et coche comme réglé ce qu'il traite dans la boîte « À toi »"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-557"
pages: ["harness/boite-a-toi","harness/serveur-mcp"]
tickets: ["ESC-557"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":23,"removed":0},{"page":"harness/serveur-mcp","added":8,"removed":0}]
accepted_at: "2026-11-21T20:25:46+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La boîte « À toi » lit ses vues dans l'état (ESC-513) : une vue réglée à sa source disparaît. Mais un ticket « À tester » que le chef d'escouade est en train de vérifier (sa chaîne de tests, parfois une heure) apparaissait à l'utilisateur comme sa tâche : le 2026-11-20, il en a validé trois depuis la boîte avant la fin de la vérification. Et ce que le chef règle par un autre chemin (un ticket de la ligne 2.2 fusionné à la main, qui reste « À tester » jusqu'à ce que sa carte soit fermée ; la décision d'un tel ticket, proposée jusqu'au reversement ; une demande tranchée dans sa conversation) restait affiché comme à faire. L'utilisateur, le 2026-11-20 : « quand toi, l'agent qui valide et merge un ticket, le fait, de pouvoir cocher comme terminé la tâche dans "à toi", afin d'être toujours à jour ».

## Décision

Le chef pose une marque sur une vue, gardée dans l'état durable avec l'état de sa source (`sign`), par deux outils MCP. `claim_inbox_view` (« Je m'en occupe », avec une note) : la vue ne compte plus comme attendant l'utilisateur et se lit dans un groupe « Le chef s'en occupe » ; l'utilisateur peut toujours agir dessus, après une confirmation qui dit que le chef la vérifie. `settle_inbox_view` (« Réglé », avec une note qui dit comment) : seulement un ticket à valider ou bloqué, ou la décision d'un ticket ; la vue passe dans la trace « Réglé par … », d'où l'utilisateur la remet (`inbox_view_reopen`), sa source intacte. Une marque tombe d'elle-même quand sa source change (le ticket quitte « À tester » et y revient, son blocage change, la décision est révisée) ; le chef rend une prise en charge par `release`. Jamais ce qui reste le geste de l'utilisateur : docs à partager, relecture demandée par l'hébergeur, feu vert, tâche, demande bloquante cochée, décision sans ticket, son propre ticket. Qui : le même réglage « Les agents peuvent répondre aux demandes des autres agents ». La raison de `withdraw_ask` devient une note visible dans la trace (jamais un secret), et la consigne du serveur MCP le dit au chef.

## Conséquences

Le compteur de la tuile ne compte plus ce que le chef traite ; « N en attente » de la barre d'état continue de compter les agents arrêtés. Les marques sont des données de plus dans state.json (`inboxMarks`). Une marque n'agit jamais sur la source : un ticket coché comme réglé reste « À tester » jusqu'à ce que sa carte soit fermée. Au reversement dans release/2.3, `inbox_view_reopen` est à classer dans la liste de l'API distante (servie), et l'Overseer reçoit les marques avec la délégation.
