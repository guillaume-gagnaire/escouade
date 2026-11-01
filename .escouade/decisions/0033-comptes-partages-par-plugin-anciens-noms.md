---
number: 33
id: "comptes-partages-par-plugin-anciens-noms"
title: "Comptes partagés par plugin : anciens noms du trousseau gardés"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-212"
pages: ["plugins/comptes-partages","kanban/integrations","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-212"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/comptes-partages","added":23,"removed":0},{"page":"kanban/integrations","added":3,"removed":0},{"page":"git-fichiers/hebergeurs-de-code","added":3,"removed":0}]
accepted_at: "2026-11-01T15:14:44+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-6 (ESC-212) donne à chaque compte un genre déclaré par le plugin qui le sert, et met ses secrets au trousseau sous `plugin:<id>:account:<compte>`. La migration des noms devait être réversible : un retour à une version d'avant ne doit pas perdre les comptes. Deux voies : déplacer les entrées et revenir par une commande, ou garder les anciennes entrées à côté.

## Décision

Les anciennes entrées (`jira`, `trello`, `github`, `linear`, `clickup`, `forge:<hôte>`) restent tenues à côté des nouvelles pour les comptes servis par Escouade : écrites et effacées avec elles, et lues d'abord tant qu'elles sont tenues (un jeton changé par une version d'avant est repris et recopié). `integrations.json` garde sa forme et dit `vault: 2` à sa prochaine écriture. Le compte d'un genre à un seul compte s'appelle par l'id du genre, celui d'une instance par son hôte. Un compte peut être servi par un autre plugin, choisi à son ajout (« Servi par »), jamais en silence ; il passe alors par ce plugin et n'a pas d'ancienne entrée. La limite d'appels est comptée par compte. (Choix de l'utilisateur le 2026-11-01 : « Anciens noms gardés ».)

## Conséquences

Une version d'avant relit ses comptes sans rien faire. Le trousseau tient deux entrées par compte de première main, jusqu'à la version qui retirera les anciennes (elle gardera `revert` comme retour). Les plugins de première main à venir (INT-7, INT-8) et les clients GitLab, Bitbucket et Gitea (ESC-56 à 58) lisent leur jeton par `secrets/get` de `account:<compte>`.
