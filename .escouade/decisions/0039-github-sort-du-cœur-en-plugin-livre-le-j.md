---
number: 39
id: "github-sort-du-cœur-en-plugin-livre-le-j"
title: "GitHub sort du cœur en plugin livré, le jeton prêté par le cœur"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-217"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-217"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":8,"removed":2}]
accepted_at: "2026-11-01T19:46:39+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-8.1 (ESC-217) : premier hébergeur sorti du cœur en plugin de première main en process (docs/PLUGINS.md §4.4), modèle de GitLab, Bitbucket, Gitea et Azure DevOps. Il fallait dire où vit le code commun, comment le plugin obtient le jeton (y compris celui de gh) et comment un arrêt se voit.

## Décision

dev.escouade.github est livré dans les releases (registre builtin) et sert l'hébergeur github et GitHub Issues sur le même compte. Le commun des hébergeurs est la crate escouade-plugin-kit (boucle asynchrone, service forge/* et tickets/*, trait Forge partagé avec le cœur, erreurs typées, limite d'appels) ; les types de données sont dans escouade-plugin-protocol. Le jeton d'un compte de première main est prêté par le cœur (Runtime::lend), celui de gh compris : le plugin ne lance pas gh. Un plugin livré reçoit langue et réseau à son lancement. Coupé ou arrêté, la carte de l'agent et la vue des hébergeurs le disent, les PR lues en dernier restent.

## Conséquences

Aucun changement à l'usage ni de données ; les tests GitHub passent par le process (un peu plus lents). Les hébergeurs suivants s'ajoutent par une crate sur le kit, une entrée d'escouade-plugins, une ligne du registre et Provider::plugin(). Les crates des plugins sont membres d'un workspace dont src-tauri est la racine.
