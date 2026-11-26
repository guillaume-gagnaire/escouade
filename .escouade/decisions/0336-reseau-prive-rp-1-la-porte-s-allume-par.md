---
number: 336
id: "reseau-prive-rp-1-la-porte-s-allume-par"
title: "Réseau privé RP-1 : la porte s'allume par --gate, un jeton par agent et par projet"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-597"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-597"]
documents: ["docs/PRIVATE-NETWORK.md"]
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":41,"removed":4}]
accepted_at: "2026-11-26T12:53:49+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

RP-1 (ESC-597) construit la porte de escouade-server avant l'entrée des connecteurs (RP-4). L'étude (docs/PRIVATE-NETWORK.md § 4.3) dit « quand le serveur a la porte active » sans dire comment elle s'allume, ni qui d'autre que les agents reçoit un jeton (terminaux, lancements, git du cœur), ni comment le jeton est gardé.

## Décision

La porte est éteinte par défaut ; `escouade-server --gate` ou `ESCOUADE_SERVER_GATE=1` l'allume avant le premier agent, jusqu'à ce que RP-4 l'allume avec l'entrée des connecteurs. Elle écoute sur un port de 127.0.0.1 choisi par le système, HTTP et SOCKS5 sur le même port. Chaque agent a son jeton (32 octets aléatoires), gardé tel quel aux redémarrages de son process et repris quand l'agent est supprimé ; les terminaux, les lancements (et les étapes des worktrees) et les git du cœur d'un projet ont chacun un jeton du projet. La porte ne garde que l'empreinte SHA-256 des jetons, en mémoire ; aucun n'est écrit dans un fichier ni au journal. Le poste de bureau n'a jamais de porte.

## Conséquences

Un serveur sans --gate se comporte exactement comme avant. Un serveur redémarré donne de nouveaux jetons (ses agents redémarrent aussi). Un process partagé de harness ne se partage plus entre agents tant que la porte tourne (leurs variables diffèrent). RP-4 remplace l'interrupteur par l'entrée des connecteurs et sa stratégie servers.privateNetwork.
