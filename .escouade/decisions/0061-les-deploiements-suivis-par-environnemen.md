---
number: 61
id: "les-deploiements-suivis-par-environnemen"
title: "Les déploiements suivis par environnement, vue « Environnements » en couloirs"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-67"
pages: ["kanban/environnements","kanban/en-revue","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-67"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/environnements","added":35,"removed":0},{"page":"kanban/en-revue","added":4,"removed":1},{"page":"git-fichiers/hebergeurs-de-code","added":7,"removed":0}]
accepted_at: "2026-11-02T12:42:24+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Après le merge, rien ne disait si le code d'un ticket tournait en staging ou en production, ni où était l'aperçu d'une PR. Les hébergeurs sont des plugins : toute opération nouvelle passe par le trait Forge. Escouade ne publie rien au nom de l'utilisateur (décision du 2026-10-16).

## Décision

Une opération `environments` des hébergeurs (GitHub, GitLab, Bitbucket ; Gitea, Forgejo et Azure DevOps répondent qu'ils ne savent pas) donne les déploiements récents du dépôt ; une PR mergée dit le commit de son merge. Un ticket terminé est dans un environnement quand le dernier déploiement réussi y contient ce commit (`git merge-base --is-ancestor`, après un fetch). La lecture se fait dans la boucle des PR des tickets, pour les projets qui ont un ticket mergé depuis moins de 30 jours ou leur vue ouverte. La carte « Terminé » dit « staging ✓ · production ⏳ » avec les adresses ; la vue « Environnements » du Kanban montre un couloir par environnement (direction A, choisie sur aperçu le 2026-11-02) ; un échec de déploiement est notifié une fois et marqué avec le lien du job ; « Ouvrir l'aperçu » sur la carte « En revue ». Escouade ne fait que lire.

## Conséquences

Le protocole des plugins gagne `forge/environments` et des champs facultatifs (commit, date, auteur, job d'un déploiement ; commit de merge d'une PR), compatibles avec les plugins d'avant. Un appel de plus par PR en revue (son aperçu) et quelques appels par projet suivi à chaque passage de la boucle. Essais réels à faire plus tard (ESC-53, ESC-70).
