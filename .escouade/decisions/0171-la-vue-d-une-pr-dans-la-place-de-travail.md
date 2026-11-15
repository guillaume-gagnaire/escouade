---
number: 171
id: "la-vue-d-une-pr-dans-la-place-de-travail"
title: "La vue d'une PR dans la place de travail, en lecture seule"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-457"
pages: ["git-fichiers/pull-requests","git-fichiers/vue-pull-requests"]
tickets: ["ESC-457"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":28,"removed":7},{"page":"git-fichiers/vue-pull-requests","added":6,"removed":5}]
accepted_at: "2026-11-15T07:37:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PR-2 des PR dans Escouade (design/pr-2-3, direction A choisie par l'utilisateur le 2026-11-14). PR-1 (ESC-452) a donné au cœur la lecture d'une PR entière (pr_contents) ; rien ne la montrait. Chaque geste « ouvrir la PR » ouvrait le navigateur, et ce qu'on savait d'une PR était réparti entre la vue « Pull requests », l'onglet « PR et CI », la carte « En revue » et la relecture.

## Décision

Une PR a sa vue dans la place de travail, à la place de la liste de la vue « Pull requests » (comme la relecture d'ESC-66) : en-tête (numéro, titre, état, branches, ticket, agent ou auteur, « Ouvrir sur <hébergeur> » qui reste un bouton), onglets Conversation (description marquée « contenu d'un tiers », rendue comme les messages des agents sans rien charger seule ; chronologie des commits, commentaires et fils avec leur état ; relectures), Commits (un clic montre le message entier), Contrôles (le composant de l'onglet « PR et CI » : journal et « Relancer les échecs ») et Fichiers grisé jusqu'à PR-3 ; une colonne de droite (relecteurs, étiquettes, brouillon, conflit, ticket) qui gardera la boîte de merge de PR-8. En lecture seule : rien ne s'écrit chez l'hébergeur sauf la relance au clic. Elle s'ouvre par un clic sur une PR de la liste, le numéro de la PR d'un ticket « En revue », « Ouvrir la PR » de l'onglet « PR et CI » (et ses autres PR, et les PR du projet sans agent) et le numéro sous la carte d'un agent ; ces numéros n'ouvrent plus le navigateur (l'en-tête de la conversation et la barre d'état, si). Elle se relit quand le suivi des PR la voit changer (prChanged). Le cœur gagne pr_detail, pr_log et pr_rerun pour une PR du projet, lue par l'agent ou le ticket qui la tient, sinon sur la cible.

## Conséquences

Une PR se lit sans quitter Escouade, d'où qu'on vienne. Le navigateur reste au menu, à l'icône des listes et au bouton de la vue. La relance est encore celle de tous les échecs (relancer un seul job : PR-8). Les relectures n'ayant pas de date chez l'hébergeur, elles suivent la chronologie. Les e2e qui ouvraient le navigateur par la carte de l'agent ou du ticket passent par la vue (pr-open, prs, azure-devops).
