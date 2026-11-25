---
number: 318
id: "pages-des-agents-p2-p3-une-ligne-de-navi"
title: "Pages des agents P2, P3 : une ligne de Navigateurs, et les choix par ask_user confirmés hors de la page"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-661"
pages: ["harness/pages-des-agents","harness/boite-a-toi","kanban/page-du-lancement"]
tickets: ["ESC-661"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-26T00:19:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-661 : où la page s'ouvre (le cas phare : à côté de la conversation, pilotée) et comment un choix cliqué dans une page qui n'est pas digne de confiance revient à l'agent.

## Décision

Choisi par l'utilisateur le 2026-11-25 (demande a-36, recommandations acceptées). P2 : une ligne de « Navigateurs » (`page:<p-n>`), ouverte à côté de la conversation quand l'utilisateur regarde l'agent, et sa demande dans « À toi » ; écartés : un onglet du panneau de droite, les deux. P3 : les choix sont une demande `ask_user` portée par la page ; un clic dans la page ne fait que proposer (canal `_/choose` au schéma fermé, validé par le cœur) et la réponse part au geste de l'utilisateur dans une bande d'Escouade hors de la webview, qui montre le libellé du cœur ; ensuite le circuit de la boîte (10 s, message, rien d'autre) ; un feu vert ne se propose jamais depuis la page. Écarté : le clic dans la page suffit.

## Conséquences

Un geste de plus pour l'utilisateur, en échange d'aucune réponse possible sans lui même si la page est piégée. AP-3, AP-4, AP-9.
