---
number: 339
id: "l-api-distante-refuse-une-requete-a-deux"
title: "L'API distante refuse une requête à deux noms (Host, Origin doublés, URI absolue d'une autre autorité)"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-688"
pages: ["a-distance/api-distante"]
tickets: ["ESC-688"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":2,"removed":0}]
accepted_at: "2026-11-26T15:11:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'étude ESC-655 (§ 5) a relevé que le garde de l'API distante lisait le premier Host et la première Origin d'une requête et ignorait l'autorité d'une URI absolue : derrière un proxy (--listen, --allow-host, --trust-proxy), le proxy et le serveur pouvaient ne pas lire le même nom.

## Décision

Avant l'ouverture de la WebSocket, par toutes les voies, le cœur refuse en HTTP 400, refus nommé au journal, une requête avec plus d'un Host, plus d'une Origin, ou une URI absolue dont l'autorité diffère du Host (casse ignorée). Tout le reste du garde est inchangé.

## Conséquences

Un client conforme n'envoie jamais ces requêtes : aucun changement pour la fenêtre, le téléphone, escouade-server ni un proxy usuel. Une URI absolue de la même autorité passe toujours. Un proxy qui réécrirait le Host sans réécrire une URI absolue verrait ses requêtes refusées (aucun connu ne le fait).
