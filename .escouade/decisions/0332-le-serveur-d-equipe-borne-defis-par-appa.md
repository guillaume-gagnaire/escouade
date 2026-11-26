---
number: 332
id: "le-serveur-d-equipe-borne-defis-par-appa"
title: "Le serveur d'équipe borné : défis par appareil, délais, connexions plafonnées, code court de 10 caractères"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-639"
pages: ["serveur-equipe/serveur","serveur-equipe/entree"]
tickets: ["ESC-639"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/serveur","added":7,"removed":3},{"page":"serveur-equipe/entree","added":2,"removed":1}]
accepted_at: "2026-11-26T08:33:20+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-625) a relevé trois constats sur le serveur d'équipe : SR2-06, les défis en attente sont communs à tout le serveur (une équipe empêche toutes les autres de se connecter) ; SR2-07, aucun délai sur la lecture d'un corps ni plafond du nombre de connexions (un inconnu tient les connexions ouvertes) ; SR2-05, le relais d'appairage se parcourt sans session (32 768 créneaux, un inconnu brûle l'unique essai d'un code court). Le ticket demandait un identifiant de créneau d'au moins 128 bits, mais le nouvel appareil ne connaît que le code tapé à la main : 128 bits tapés feraient environ 31 caractères.

## Décision

1) Les défis en attente sont comptés par appareil (deux, un nouveau prend la place du plus ancien) et par équipe (1 024), en plus de la borne du serveur (100 000) ; les sessions par appareil (huit). 2) La limite de débit compte une adresse IPv6 par son /64. 3) Le corps d'une requête arrive en 30 s plus sa longueur à 64 Kio/s, sinon 408 ; une connexion muette 60 s se ferme ; 8 192 connexions ouvertes en tout et 512 par adresse (le plafond global seul derrière --trust-proxy). 4) Le code court passe de 8 à 10 caractères (créneau de 5, un parmi 33 millions ; secret de 5 inchangé), les étapes sans session du relais ont un débit à elles par adresse (10 d'un coup, une toutes les 5 s), et l'essai rend un jeton de 128 bits sans lequel la réponse ne se prend pas (choix de l'overseer, option recommandée, au lieu d'un créneau de 128 bits à taper). 5) La doc d'installation dit de garder le serveur derrière son proxy (Caddy, Traefik) s'il est joignable depuis Internet.

## Conséquences

Un code court d'avant (8 caractères) n'est plus accepté : l'appareil dans l'équipe en affiche un nouveau, de 10. Le relais change de routes (POST /v1/pair/{slot}/take avec le jeton de l'essai remplace GET /v1/pair/{slot}/answer) : serveur et client de la 2.3 vont ensemble, aucune version publiée n'est concernée. Les bornes ne se règlent pas encore par option de la ligne de commande (des valeurs par défaut dans config::Limits et config::Rates). Une métrique de plus : escouade_team_connections_refused_total.
