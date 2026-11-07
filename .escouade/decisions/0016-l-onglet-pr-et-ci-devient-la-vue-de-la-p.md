---
number: 16
title: "L'onglet « PR et CI » devient la vue de la PR de l'agent (CI, journal, relance, revues, déploiements)"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-305"
pages: ["git-fichiers/pull-requests","git-fichiers/git-fichiers"]
tickets: ["ESC-305"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":27,"removed":8},{"page":"git-fichiers/git-fichiers","added":2,"removed":0}]
accepted_at: "2026-10-30T18:16:57+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis la 2.0, le panneau latéral gardait un onglet « PR et CI · dans la 2.1 » avec un texte d'attente, alors que le backend savait déjà lire une PR, ses checks, leurs journaux, ses revues et ses déploiements chez GitHub et Azure DevOps. L'utilisateur a demandé la vraie vue le 2026-10-30 (ESC-305).

## Décision

L'onglet montre, pour l'agent ouvert, sa première PR (état, branche → cible, conflit, âge, auteur), sa CI vérification par vérification avec état et durée, le journal (fin, 20 Ko) d'une vérification échouée à son clic, « Relancer les échecs » au clic seulement, ses revues et commentaires en lecture (rien publié au nom de l'utilisateur), ses déploiements avec leur adresse ; sans PR, « Ouvrir une PR… » ou la raison ; sans agent, les 30 PR ouvertes du projet. La CI est relue toutes les 20 s seulement tant qu'une vérification tourne et que l'onglet est affiché. Les erreurs de l'hébergeur sont classées (jeton refusé, limite d'API avec son heure, injoignable) ; une 5xx est relue une fois. Le libellé suit le mot de l'hébergeur (« MR et CI »), avec une pastille quand la CI de la PR a échoué. La carte d'un agent dit « #41 → main » et l'état de la CI en mots, qui ouvre l'onglet : à 340 px l'onglet quitte toujours la bande (choix 189 gardé), et ces mots sont alors la porte vers la vue.

## Conséquences

PullRequest gagne author et created, Check gagne started, finished et cancelled ; le trait Forge gagne open_pull_requests (GitHub en une requête GraphQL, Azure DevOps). Un appel de lecture de plus par onglet affiché, et un toutes les 20 s pendant qu'une CI tourne. GitLab, Bitbucket et Gitea montrent la raison en attendant leurs clients (ESC-56 à 58) ; la vue projet complète reste à ESC-65.
