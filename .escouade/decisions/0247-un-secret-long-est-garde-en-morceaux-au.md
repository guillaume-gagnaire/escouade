---
number: 247
id: "un-secret-long-est-garde-en-morceaux-au"
title: "Un secret long est gardé en morceaux au trousseau, avec un en-tête et son empreinte"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-527"
pages: ["plugins/vue-d-ensemble"]
tickets: ["ESC-527","ESC-501"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/vue-d-ensemble","added":11,"removed":0}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le trousseau de Windows refuse un secret de plus de 1 280 caractères. Une configuration OpenVPN avec ses certificats fait 4 à 8 Ko, et les secrets multi-ligne arrivent (ESC-526). L'étude ESC-501 (§4.4, choix n° 5, tranché le 2026-11-17) a choisi des morceaux au trousseau plutôt qu'un fichier chiffré.

## Décision

Le magasin des secrets (Chunked) enveloppe le trousseau sur les trois systèmes. Un secret de plus de 1 200 unités UTF-16 est coupé en morceaux sous <nom>#a1… ou #b1…. L'entrée <nom> garde l'en-tête escouade-pieces:v1:<emplacement>:<nombre>:<SHA-256>. Un secret court reste une seule entrée, comme avant.

Le ticket nommait les morceaux #1, #2… Deux emplacements alternés (a, b) les remplacent : sans eux, on ne peut pas écrire les nouveaux morceaux avant de basculer l'en-tête sans écraser les anciens. Or l'ancien secret doit rester lisible après une écriture coupée.

La lecture vérifie l'empreinte, et rend une erreur (-32002 pour un plugin) plutôt qu'un secret tronqué. L'ordre d'écriture est : nouveaux morceaux, bascule de l'en-tête, puis effacement des anciens, du dernier au premier. « Oublier » efface tous les morceaux, puis l'en-tête. Un secret fait 64 Kio au plus.

# est réservé aux morceaux : une clé qui en contient n'est jamais celle d'un plugin. Le journal masque aussi chaque ligne de plus de 8 caractères d'un secret multi-ligne.

## Conséquences

Tous les secrets de l'app passent par ce format : plugins, comptes, voix, jeton MCP. Une version d'avant lirait l'en-tête d'un secret long comme sa valeur, mais une version d'avant ne pouvait pas garder ce secret sous Windows. Le fichier lisible de toi seul (ESC-499) garde un secret d'un seul tenant. Une écriture courte coûte deux lectures de plus au trousseau (la recherche des restes).
