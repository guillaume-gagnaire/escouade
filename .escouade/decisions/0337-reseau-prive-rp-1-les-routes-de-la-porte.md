---
number: 337
id: "reseau-prive-rp-1-les-routes-de-la-porte"
title: "Réseau privé RP-1 : les routes de la porte, un nom privé refusé sur tout port"
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
accepted_at: "2026-11-26T12:53:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'étude (§ 4.1, § 4.3) dit qu'un hôte privé hors de la liste du projet est refusé et que le reste sort directement ou par le proxy de l'espace. Restaient : ce qu'est un « hôte privé » (un plafond nomme hôte:port), une adresse donnée en SOCKS5 par un agent qui a résolu lui-même, un proxy d'espace que la porte ne sait pas parler, une requête en HTTP clair.

## Décision

Un nom est d'un réseau privé quand un plafond de connecteur ou la liste d'un projet le nomment, sur n'importe quel port : hors de la liste du projet il est refusé, jamais résolu par le serveur ni envoyé dehors. Une adresse IP reçue en SOCKS5 (l'agent a résolu lui-même) est prise pour une adresse, jamais pour un nom d'un réseau privé. Dans la liste mais hors du plafond connu, ou le connecteur coupé : refusé aussitôt, sans le demander au connecteur. Le proxy de l'espace passe derrière la porte en HTTP seulement (avec son mot de passe et ses exclusions) ; un proxy https:// ou socks5:// réglé laisse sortir ses seules exclusions, plutôt que de le contourner. Une requête en HTTP clair part sous sa forme d'origine, sans le jeton ni les en-têtes du proxy, avec Connection: close. Les refus : 403 (porte ou connecteur), 407 (jeton absent ou inconnu), 502 (injoignable), la phrase en corps et en raison ASCII ; SOCKS5 0x02 pour un refus, 0x04 pour un échec.

## Conséquences

Un nom interne n'interroge jamais le DNS du serveur. Une requête HTTP clair par connexion (un peu plus de connexions pour npm en http://, rare). Un utilisateur dont le proxy d'espace est en https:// ou socks5:// doit le passer en http:// pour que ses agents sortent par la porte.
