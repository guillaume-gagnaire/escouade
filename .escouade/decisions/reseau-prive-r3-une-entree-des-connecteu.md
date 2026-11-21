---
number: null
id: "reseau-prive-r3-une-entree-des-connecteu"
title: "Réseau privé, R3 : une entrée des connecteurs à part, éteinte par défaut, jamais l'API distante"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-551"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-551", "ESC-587"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: ""
accepted_by: ""
---

## Contexte

Le connecteur ouvre sa connexion vers le serveur Escouade, qui doit donc l'attendre quelque part de joignable depuis Internet, alors que l'API distante n'écoute que sur 127.0.0.1 (docs/PRIVATE-NETWORK.md § 4.1, R3).

## Décision

escouade-server a une entrée à part pour les connecteurs (--connector-listen, ESCOUADE_SERVER_CONNECTOR_LISTEN), éteinte par défaut, qui ne parle que le protocole des connecteurs ; elle peut être servie derrière le proxy TLS d'un domaine. Sans clé connue, elle ferme après le premier message Noise. Une stratégie servers.privateNetwork à false l'empêche.

## Conséquences

L'API distante reste sur 127.0.0.1, jamais sur Internet. Ce qui est exposé se réduit à TLS, une WebSocket et une poignée de main Noise ; chaque essai raté compte comme une poignée de main ratée. Un port de plus à ouvrir sur le VPS (pas chez l'entreprise).
