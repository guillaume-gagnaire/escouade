---
number: 50
id: "pr-des-tickets-par-l-api-de-l-hebergeur"
title: "PR des tickets par l'API de l'hébergeur, suivies « En revue » jusqu'au merge"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-59"
pages: ["kanban/validation","kanban/en-revue","kanban/kanban"]
tickets: ["ESC-59"]
documents: ["docs/TODO-1.8.md"]
supersedes: []
superseded_by: null
changes: [{"page":"kanban/validation","added":13,"removed":8},{"page":"kanban/en-revue","added":37,"removed":0},{"page":"kanban/kanban","added":4,"removed":4}]
accepted_at: "2026-11-02T04:53:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

« Valider + PR » ouvrait la PR d'un ticket avec `gh` sur GitHub seulement (Azure DevOps par son API), puis le ticket passait « Terminé » : Escouade ne savait plus ce que devenait la PR, et les tickets qui l'attendaient partaient avant son merge. Les hébergeurs sont désormais tous des plugins livrés, servis par le trait `Forge`.

## Décision

1. La PR d'un ticket s'ouvre par l'API de l'hébergeur (`Forge::create_pr`, servi par son plugin) avec le compte de l'instance : brouillon, relecteurs et étiquettes par défaut du Kanban (nouveaux réglages), « Closes #n » pour une issue GitHub ou GitLab du même dépôt. `gh` n'est plus appelé ; son jeton reste prêté par le cœur. Sans compte, ou refusée, la page de création de l'hébergeur s'ouvre, remplie, et le ticket passe « Terminé » comme avant.
2. Une colonne « En revue » (`pr` ; `in-review` dans les tickets partagés), entre « À tester » et « Terminé », visible quand le Kanban ouvre des PR ou qu'un ticket y est. Le ticket y reste jusqu'au merge, et y retient les tickets qui l'attendent.
3. Sa carte montre numéro, état, CI et jobs, approbations, fils ouverts et conflit. Mergée : « Terminé », branche locale supprimée selon le réglage de nettoyage, fetch et avance rapide de la cible. Fermée sans merge : bloquée avec « Remettre à faire » et « Terminer quand même ».
4. Lecture sans webhook : toutes les minutes au premier plan, 5 min sinon, tout de suite après un push d'Escouade et au retour de la fenêtre, et sur « Rafraîchir ». Une erreur d'hébergeur est dite sur la carte et ne déplace jamais un ticket.
5. Décidé par l'utilisateur le 2026-10-16 : le mot de l'hébergeur, « MR » sur GitLab et « PR » ailleurs ; le worktree d'un ticket est supprimé à la validation comme avant, et sera recréé depuis sa branche quand l'agent devra corriger.

## Conséquences

Un ticket validé par PR n'est plus « Terminé » tout de suite : les tickets « Après » attendent le merge. Une version d'Escouade antérieure ne lit pas un ticket partagé `in-review`. Restent à faire (TODO-1.8) : « L'agent corrige » (CI, retours, conflit), « Merger » depuis la carte, et les essais réels hors GitHub. Les PR d'un projet à plusieurs dépôts ne passent pas « En revue ».
