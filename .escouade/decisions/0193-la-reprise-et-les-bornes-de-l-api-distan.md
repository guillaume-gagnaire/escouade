---
number: 193
id: "la-reprise-et-les-bornes-de-l-api-distan"
title: "La reprise et les bornes de l'API distante : la numérotation dans le seul protocole, l'état relu en entier, jamais un appel rejoué"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-495"
pages: ["a-distance/api-distante"]
tickets: ["ESC-495"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":34,"removed":12}]
accepted_at: "2026-11-17T11:11:54+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La fondation 2b d'« Escouade à distance » (ESC-495) met la fenêtre sur l'API distante aussi bien que sur l'IPC, et doit laisser un client qui perd sa connexion la retrouver sans rien perdre ; la revue d'ESC-478 avait relevé une file d'envoi sans borne. Il fallait choisir où numéroter la sortie des terminaux, comment reprendre un terminal et l'état, que faire d'un appel sans réponse, et comment borner ce qu'un client lent ou muet coûte au cœur.

## Décision

1. La sortie des terminaux est numérotée par sa place dans toute la sortie (l'octet n), dans un seul anneau par terminal côté cœur (1 Mo ; 4 Mo pour l'interface d'un agent, l'anneau qu'elle avait déjà). La numérotation ne passe que dans le protocole de l'API (trame Frame::Term) : l'IPC de la fenêtre reçoit les octets seuls, comme avant.
2. La reprise passe par une session de l'appareil, rendue au welcome et redonnée au hello : le client y reprend ses terminaux par leur canal et la fin des derniers octets reçus (resume), jamais ceux d'un autre appareil. Trop en retard, il reçoit tout ce qui reste et le compte de ce qui est perdu.
3. L'état (les événements de l'interface) n'est pas numéroté : après une coupure, ou quand la file déborde, il est relu en entier (subscribe), les choix de la fenêtre gardés. Les événements en trop se fondent en cette relecture : le dernier état gagne.
4. Un appel resté sans réponse à la coupure est rendu en erreur, jamais rejoué de lui-même (une commande n'est pas idempotente) ; un appel fait pendant la coupure échoue aussitôt. Un jeton refusé, une autre version ou la stratégie arrêtent les essais.
5. La file d'envoi est bornée en messages et en octets ; les réponses ne sont jamais jetées, mais une connexion qui laisse trop en attente ou ne lit plus depuis 30 s est fermée (4002, slow-client). Les connexions sont bornées par appareil et en tout, comme les poignées de main en cours et ratées, et les appels en attente.
6. Les commandes du poste sont faites par la fenêtre elle-même : le welcome les nomme.

## Conséquences

Rien ne change pour l'utilisateur tant que l'API est éteinte (pas de ligne de CHANGELOG). Un client distant retrouve ses terminaux sans trou ni doublon et l'état exact après une coupure ; il peut perdre un appel en vol, qu'il voit en erreur et refait lui-même. Un client lent coûte au plus la file bornée et les anneaux des terminaux (1 à 4 Mo chacun, gardés 15 min dans sa session). Un futur client (téléphone) suit le même protocole : session, resume, mark, lost, gap, resync. La relecture entière coûte un InitialState à chaque coupure ou débordement, ce qui est rare.
