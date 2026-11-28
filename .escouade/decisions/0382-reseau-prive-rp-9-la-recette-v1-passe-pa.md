---
number: 382
id: "reseau-prive-rp-9-la-recette-v1-passe-pa"
title: "Réseau privé, RP-9 : la recette V1 passe par Privoxy devant le SOCKS du client VPN, derrière la porte"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-604"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-604"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":13,"removed":0}]
accepted_at: "2026-11-28T19:50:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

R1 met V1 (le serveur client du VPN) en recette du guide : « un SOCKS en espace utilisateur, derrière la porte ». Mais la route « vers un SOCKS local » de la porte n'a pas été construite (RP-1) : la porte ne parle au proxy de l'espace qu'en http://, et un proxy socks5:// réglé dans l'espace fait refuser toute sortie. Il faut aussi que le reste d'Internet (les modèles) ne passe pas par le VPN, ce que les exclusions (NO_PROXY, qui sortent directement) ne savent pas dire à l'envers.

## Décision

La recette du site (« Le serveur client du VPN ») met Privoxy devant le SOCKS du client VPN en espace utilisateur (wireproxy pour WireGuard, openconnect et ocproxy pour AnyConnect ou GlobalProtect) : `forward-socks5t .acme.internal 127.0.0.1:1080 .` envoie les noms de l'entreprise au SOCKS, le reste directement, et `+limit-connect{,}` laisse passer le CONNECT de ssh. Privoxy est le proxy de l'espace du serveur (http://127.0.0.1:8118), derrière la porte allumée par --gate, pour que ssh passe par escouade-server via. En Docker, le client VPN et Privoxy dans un conteneur qui partage le réseau du serveur (network_mode: service:escouade), l'image du serveur inchangée. Rien n'est construit dans Escouade pour V1.

## Conséquences

V1 marche avec ce qui est construit, sans route nouvelle à la porte ni droits au conteneur. Ses réserves restent dites : un compte VPN pour une machine, l'authentification forte qui casse le H24, et tout ce que le compte VPN joint, un agent le joint (les listes des projets ne valent que pour les connecteurs). La recette n'est pas essayée contre un vrai VPN (interdit au ticket) : à essayer par l'utilisateur s'il en a l'usage.
