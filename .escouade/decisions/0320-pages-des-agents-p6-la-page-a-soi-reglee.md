---
number: 320
id: "pages-des-agents-p6-la-page-a-soi-reglee"
title: "Pages des agents P6 : la page à soi réglée sur le poste ou proposée par le dépôt"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-661"
pages: ["harness/pages-des-agents","projets/escouade-yml"]
tickets: ["ESC-661"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-26T00:19:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-661 : l'utilisateur veut une page à soi, réglée une fois, que les agents remplissent.

## Décision

Choisi par l'utilisateur le 2026-11-25 (demande a-36, recommandation acceptée). Un fichier HTML choisi dans Réglages › Projets › Lancement (gardé sur le poste) ou proposé par `pages.custom` du `.escouade.yml` ; le poste l'emporte, l'équipe impose ou recommande comme pour les parties ; servi dans la même cloison, il reçoit `window.escouade` (titre, agent, ticket, projet, thème, langue, données, demande, `choose`). Écarté : le poste seulement.

## Conséquences

Pas d'accord comme une commande de lancement : la page ne fait rien tourner hors de la cloison. AP-7.
