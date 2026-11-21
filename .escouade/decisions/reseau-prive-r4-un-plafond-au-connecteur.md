---
number: null
id: "reseau-prive-r4-un-plafond-au-connecteur"
title: "Réseau privé, R4 : un plafond au connecteur tenu par l'entreprise, une liste par projet au serveur"
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

Un agent détourné par un contenu piégé ne doit joindre qu'une liste d'hôtes du réseau de l'entreprise. Où tenir cette liste (docs/PRIVATE-NETWORK.md § 4.4, R4) ?

## Décision

Deux listes : le plafond, dans connector.toml sur la machine du connecteur, écrit par l'entreprise, que le serveur ne peut pas élargir (des hôte:port, des motifs, au besoin des plages d'adresses permises) ; et une liste par projet au serveur, choisie dans ce plafond, vide par défaut.

## Conséquences

La borne qui tient face à un agent détourné est hors de la machine des agents. La liste par projet n'est qu'une défense de plus (même compte pour tous les agents, comme la clôture d'un projet privé, SEC-02), ce que l'interface dit ; elle évite qu'un autre projet du serveur joigne le réseau par erreur.
