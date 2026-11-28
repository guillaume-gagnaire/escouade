---
number: 366
id: "une-reponse-tardive-de-git-remplace-l-et"
title: "Une réponse tardive de git remplace l'état inconnu de la suppression d'un agent"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-522"
pages: ["fenetre/barre-laterale"]
tickets: ["ESC-522","ESC-511"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/barre-laterale","added":4,"removed":0}]
accepted_at: "2026-11-21T18:00:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

« Supprimer l'agent » lisait git sur le worktree en 3 s au plus (LOSS_TIMEOUT, deleteask.rs), puis jetait la lecture et disait « État du worktree inconnu » pour de bon. Sur un poste chargé, git met plus de 3 s : la modale disait « inconnu » alors que git allait répondre (l'e2e app.spec.ts:414 est tombé ainsi sous charge, ESC-522).

## Décision

Au bout de 3 s, la modale dit « État du worktree inconnu pour l'instant : git n'a pas encore répondu. » et libère la case, décochée, mais la lecture continue. La réponse de git remplace l'état inconnu quand elle arrive, et la case la suit tant que l'utilisateur n'y a pas touché. Le cœur ne renonce qu'au bout de 60 s, contre un git bloqué.

## Conséquences

La modale ne fait pas attendre plus longtemps qu'avant, et dit vrai dès que git répond. Un nouveau texte (fr/en) pour l'état inconnu provisoire. Supprimer pendant l'état inconnu garde le worktree, comme avant.
