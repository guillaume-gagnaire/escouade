---
number: 376
id: "bail-d-equipe-oe-2-une-ecriture-sous-un"
title: "Bail d'équipe (OE-2) : une écriture sous un bail fini est refusée, et le bail est aux machines seules"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-611"
pages: ["serveur-equipe/overseer-equipe"]
tickets: ["ESC-611"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/overseer-equipe","added":11,"removed":3}]
accepted_at: "2026-11-28T16:34:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

OE-2 (ESC-611) construit le bail d'équipe de l'Overseer d'équipe (O6-A, docs/TEAM-OVERSEER.md §7.1) sur le modèle de la prise des tickets (TS-6). Trois points n'étaient pas tranchés par l'étude : une écriture de l'Overseer qui porte le jeton courant d'un bail fini (personne d'autre ne l'a pris) ; qui peut lire le bail ; la forme du nom.

## Décision

1. Une écriture portée par le jeton d'un bail est refusée (409 stale_token) non seulement sous un jeton dépassé ou celui d'un autre appareil, mais aussi quand le bail est fini, même si aucune autre machine ne l'a pris : la machine renouvelle (permis tant que personne n'a repris) avant de reparler au nom de l'Overseer. Les prises de tickets, elles, ne regardent que le jeton. 2. Les quatre routes /v1/team/leases/{nom}, la lecture comprise, sont réservées aux appareils de service ; l'appareil d'une personne, administrateur compris, reçoit 403. Ce que les membres voient de l'Overseer d'équipe passera par le fil (OE-4), chiffré. 3. Le nom d'un bail : 1 à 64 caractères parmi a-z, 0-9, '.', '_', '-' (overseer aujourd'hui, un nom par projet plus tard, O6-B), 256 noms au plus par équipe. 4. Le nom de la machine qui tient le bail (« build-2 ») est lu par la machine qui attend dans les projets de l'équipe, où chaque appareil écrit son nom chiffré ; le serveur ne le connaît pas.

## Conséquences

Une machine revenue d'une coupure après la fin de son bail ne peut rien écrire au nom de l'Overseer, même si l'autre machine n'a pas encore pris le bail : plus sûr pour le fil, au prix d'un renouvellement avant de reparler (le gardien du client le fait seul). Les administrateurs n'ont pas de route pour voir le bail au serveur. Le bail n'est pas encore dans l'audit du serveur (AU-2) : à ajouter avec la suite de l'audit.
