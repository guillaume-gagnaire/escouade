---
number: 60
id: "vue-pull-requests-du-projet-en-liste-den"
title: "Vue « Pull requests » du projet en liste dense"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-65"
pages: ["git-fichiers/vue-pull-requests","git-fichiers/pull-requests"]
tickets: ["ESC-65"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/vue-pull-requests","added":33,"removed":0},{"page":"git-fichiers/pull-requests","added":3,"removed":3}]
accepted_at: "2026-11-02T12:14:36+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-65 : une vue des PR ouvertes du dépôt à côté du Kanban, avec trois filtres, la CI, les relectures, le conflit et l'âge, et « Lancer un agent sur cette PR », fork compris. Trois directions ont été montrées sur aperçu (design/esc-65/) : A liste dense, B colonnes par état façon Kanban, C liste et détail.

## Décision

L'utilisateur a choisi A (2026-11-02) : une ligne de 34 px par PR, colonnes alignées, actions sur la ligne survolée ou choisie, Entrée lance l'agent ; « PR » (« MR » sur GitLab) devient un quatrième bouton de la barre latérale, entre Kanban et Docs. « Les miennes » suit le login du compte de l'hébergeur ; « Relecture demandée » les relectures demandées et pas encore rendues, que chaque plugin d'hébergeur dit désormais (champ `reviewers` du protocole). L'agent d'une PR de fork travaille sur une branche locale `pr-<n>` / `mr-<n>`, récupérée par la référence que garde l'hébergeur (`Provider::pull_fetch` : `refs/pull/<n>/head`, `refs/merge-requests/<n>/head`, second parent de `refs/pull/<n>/merge` sur Azure DevOps, la branche du fork sur Bitbucket Cloud). Rien n'est écrit chez l'hébergeur.

## Conséquences

B écartée parce qu'elle doublait le Kanban et sa future colonne « En revue » (ESC-59) ; C parce que le détail d'une PR est déjà l'onglet « PR et CI ». La carte d'un agent sur une PR de fork ne montre pas cette PR (sa branche n'est pas celle du fork). Les hébergeurs autres que GitHub restent à essayer en vrai.
