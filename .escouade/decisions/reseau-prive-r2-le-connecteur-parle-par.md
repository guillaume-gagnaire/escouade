---
number: null
id: "reseau-prive-r2-le-connecteur-parle-par"
title: "Réseau privé, R2 : le connecteur parle par un tunnel TLS sortant multiplexé, Noise épinglé à l'intérieur"
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

escouade-connector doit relier une machine de l'entreprise au serveur Escouade sans port entrant chez l'entreprise. Trois protocoles comparés (docs/PRIVATE-NETWORK.md § 4.2) : WireGuard embarqué (celui prévu pour le téléphone), un tunnel TLS sortant multiplexé, SSH inversé.

## Décision

Une WebSocket sur TLS, sortante vers le serveur (par le proxy de l'entreprise s'il y en a un), avec à l'intérieur une poignée de main Noise IK dont les deux clés sont épinglées à l'appairage, puis des flux « ouvre hôte:port » à contrôle de débit. Une crate escouade-connector-proto partagée. Le serveur accepte la version du connecteur et les deux mineures d'avant.

## Conséquences

Il passe le pare-feu d'une entreprise par une ligne dans son proxy ; la liste par nom et le DNS restent chez l'entreprise ; une inspection TLS ne peut ni lire ni imiter le serveur. Il reprend la WebSocket, les bornes et l'appairage de l'API distante. Écartés : WireGuard (l'UDP sortant est fermé chez la plupart des entreprises, liste par adresse, pas commencé) et SSH inversé (aucune liste côté entreprise). Le serveur expose un serveur TLS au lieu d'un port UDP muet.
