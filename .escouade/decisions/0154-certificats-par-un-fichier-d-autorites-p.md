---
number: 154
id: "certificats-par-un-fichier-d-autorites-p"
title: "Certificats par un fichier d'autorités, projet privé dit tel qu'il est, mode choisi au premier démarrage"
date: "2026-11-12"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-425"
pages: ["fenetre/reseau","projets/projets-prives","harness/comptes"]
tickets: ["ESC-425","ESC-418"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reseau","added":15,"removed":0},{"page":"projets/projets-prives","added":3,"removed":0},{"page":"harness/comptes","added":7,"removed":0}]
accepted_at: "2026-11-12T20:50:01+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de sécurité ESC-418 (fiches SEC-07, SEC-08, SEC-09, Moyennes) relève : une case qui coupe la vérification TLS de toutes les connexions de l'app et des agents ; « Projet privé » présenté comme un cloisonnement alors que sous Windows ce n'est qu'une consigne au harness, et un harness tiers allumé d'office dans un projet privé par la migration d'ESC-404 ; un mode par défaut `auto` et `--allow-dangerously-skip-permissions` passé à chaque Claude Code. L'utilisateur se sert de la case sur son poste derrière Zscaler : elle ne doit pas être retirée en silence.

## Décision

1) Un fichier « Autorités de certification supplémentaires » (.pem) sert à toutes les connexions de l'app et des plugins, et aux agents par NODE_EXTRA_CA_CERTS et CODEX_CA_CERTIFICATE. La case « Ignorer la vérification » reste, ne vaut que pour les connexions qui passent par le proxy configuré, et un bandeau permanent le dit tant qu'elle est cochée. 2) Sous Windows, la page du projet privé dit que la clôture est une consigne ; un harness tiers n'y est lancé qu'une fois allumé par l'utilisateur pour ce projet, la migration n'allumant que ceux d'Escouade. 3) Le mode des nouveaux agents est choisi au premier démarrage, « Édits auto » (acceptEdits) recommandé ; « Autoriser le mode Bypass », éteint par défaut, ouvre le mode Bypass et l'argument --allow-dangerously-skip-permissions ; une installation existante garde ses modes (agentMode auto, Bypass allumé si elle l'utilisait).

## Conséquences

Un poste derrière un proxy qui inspecte le TLS peut tout vérifier avec le certificat racine du proxy. La case cochée ne protège plus rien sur un autre réseau sans proxy (elle n'a plus d'effet) mais reste dangereuse derrière le proxy. Une nouvelle installation demande plus de permissions qu'avant (chaque commande). Les tests qui lançaient un agent en Bypass doivent allumer le réglage. Les plugins de première main n'ont pas à changer : le cœur retire l'argument lui-même.
