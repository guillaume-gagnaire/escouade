---
number: null
id: "reseau-prive-r1-le-serveur-dans-l-entrep"
title: "Réseau privé, R1 : le serveur dans l'entreprise guidé, escouade-connector construit, V1 en recette, pas de relais par le poste"
date: "2026-11-21"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-551"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-551", "ESC-587"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":23,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Des agents sur un serveur Escouade doivent joindre un git, un registre et des API derrière le VPN d'une entreprise. Quatre voies (docs/PRIVATE-NETWORK.md § 3) : V0 le serveur dans le réseau de l'entreprise, V1 le serveur client du VPN, V2 escouade-connector, V3 le poste de l'utilisateur en relais.

La réponse de l'utilisateur, le 2026-11-21 (la demande a-22 de la boîte « À toi », reprise par ESC-587) : « A · Les recommandations : V0 guidée, V2 construite ». Il n'a rien dit d'autre : les choix R2 à R12 suivent les recommandations du § 5 de l'étude.

## Décision

La voie choisie par l'utilisateur le 2026-11-21 : V0 est dite et guidée (rien à construire) ; V2, escouade-connector, est construite pour un serveur hors de l'entreprise (RP-1 à RP-9) ; V1 n'a qu'une recette dans le guide (un SOCKS en espace utilisateur derrière la porte) ; V3 n'est pas construite (pas de RP-10).

## Conséquences

V0 laisse la borne au pare-feu de l'entreprise et ne coûte qu'un guide. V2 garde le H24, n'ouvre aucun port entrant chez l'entreprise et lui laisse la liste : ≈ 27 h d'agent, ≈ 10 h de bout en bout. V1 dépend d'un compte VPN pour une machine, souvent refusé, et l'authentification forte casse le H24. V3 perd le H24 et contourne la règle du VPN ; elle resterait possible plus tard (RP-10, ≈ 3 h), si l'utilisateur la demande.
