---
number: 1
title: "STORE-8 : file de revue réservée aux administrateurs, jeton d'administration, signature sur le poste seulement"
date: "2026-10-29"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-235"
pages: ["store/revue-et-signature"]
tickets: ["ESC-235"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":17,"removed":0}]
accepted_at: "2026-10-29T23:56:30+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

STORE-8 construit la file /review, le jeton d'administration, le dépôt des signatures et la liste de révocation signée (docs/STORE.md §4.1, §4.5, §4.6). Il fallait trancher les choix pris en construisant.

## Décision

La file est visible des seuls comptes de STORE_ADMINS, sans bouton « Vérifier ». Le jeton d'administration (esc_admin_…) est créé dans le compte, montré une fois, gardé haché ; il donne accès aux routes /api/v1/review/* (qui ne lisent pas de cookie) et ne permet ni de publier ni de supprimer. Le script le prend dans ESCOUADE_STORE_TOKEN ou dans le trousseau du système. Signatures et listes : minisign sur le poste avec la clé de l'utilisateur, vérifiées localement puis par le store avant d'être acceptées, avec le commentaire de confiance `escouade-verified-v1 <id>@<version> <date>`. Les signatures sont dans une table à part, que la purge d'un plugin ne supprime pas. Un binaire non attesté empêche la vérification. Tant que l'utilisateur n'a pas donné les clés publiques, l'app embarque une liste vide qui refuse tout badge.

## Conséquences

L'utilisateur doit créer les deux paires de clés (procédure §4.5), puis donner les clés publiques ; le chef les met dans store/keys/ et dans STORE_PUBLIC_KEYS. Il pose aussi STORE_ADMINS dans Coolify. STORE-9 utilise store_keys.rs pour afficher le badge dans l'app.
