---
number: 119
id: "les-reglages-s-appliquent-et-s-enregistr"
title: "Les réglages s'appliquent et s'enregistrent au geste, sans « Annuler » ni « Enregistrer »"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-380"
pages: ["fenetre/reglages","fenetre/fenetre","fenetre/barre-laterale","kanban/integrations","introduction/livraison"]
tickets: ["ESC-380"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":19,"removed":11},{"page":"fenetre/fenetre","added":3,"removed":3},{"page":"fenetre/barre-laterale","added":4,"removed":4},{"page":"kanban/integrations","added":1,"removed":1},{"page":"introduction/livraison","added":1,"removed":1}]
accepted_at: "2026-11-08T03:41:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Dans la fenêtre des réglages, certaines rangées s'appliquaient au geste (thème, zoom, comptes, serveur MCP, modèles de la voix…) et les autres attendaient « Enregistrer », sans que rien ne le dise : « Annuler » laissait croire qu'on pouvait revenir sur un compte supprimé (PROPOSITION.md des réglages 2.2, cause 6). Le §7 de l'étude laissait à l'utilisateur la décision 2 : garder le brouillon et marquer les rangées « appliqué tout de suite », ou tout appliquer au geste comme macOS et VS Code. Sans réponse, le chef a pris la recommandation de l'étude, dans un lot à part (R2b, ESC-380) qu'on peut retirer seul.

## Décision

Chaque réglage s'applique et s'enregistre au geste : un choix (interrupteur, segments, menu, raccourci, commande ajoutée, proposition prise, logo) aussitôt, un texte quand il perd le focus ou après 0,7 s sans frappe ; fermer la fenêtre enregistre ce qui attendait. Les enregistrements partent un à la fois, chacun avec ce qui a changé par-dessus ce que l'app a. Une saisie invalide (projet sans nom, commande sans nom ou sans ligne, dossier des worktrees ou préfixe refusé, nombre vidé ou négatif) est dite sur sa rangée et n'est pas enregistrée : la valeur d'avant reste, le reste s'enregistre. Le pied « Annuler / Enregistrer », le point de changement par page et le toast « Réglages enregistrés » disparaissent ; la fenêtre se ferme sans question. Aucune clé enregistrée ne change.

## Conséquences

Plus simple à l'usage, et ce qui s'enregistre déjà au geste n'est plus une exception. On ne peut plus jeter d'un coup une série de changements : pour défaire un réglage, on le change de nouveau. Les e2e ferment la fenêtre par closeSettings() (le champ quitté, data-saving à false) au lieu de cliquer « Enregistrer » ; un e2e change des réglages, ferme aussitôt, redémarre et les retrouve. L'aperçu design/settings-2-2/index.html#a-* et ses captures n'ont plus de pied. Restent à reprendre au lot de la documentation (après R8) : la doc du site, qui décrit encore les onglets et « Enregistrer », et deux tournures de la page Comptes (« hors du brouillon »).
