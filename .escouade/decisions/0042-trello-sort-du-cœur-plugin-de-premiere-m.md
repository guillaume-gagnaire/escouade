---
number: 42
id: "trello-sort-du-cœur-plugin-de-premiere-m"
title: "Trello sort du cœur : plugin de première main en process, sur le kit des plugins"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-213"
pages: ["kanban/integrations"]
tickets: ["ESC-213"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/integrations","added":6,"removed":0}]
accepted_at: "2026-11-01T22:20:03+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Tout l'externe passe en plugins (ESC-203). Trello est la première source du Kanban (INT-7.1) ; Linear, Jira et ClickUp suivront son modèle. Une seule façon de servir une source : le kit.

## Décision

Trello est le plugin builtin `dev.escouade.trello`, une crate de `plugins/builtin/dev.escouade.trello/` appelée par `escouade-plugins trello`, posée sur le kit `escouade-plugin-kit` (`SourceKind`/`TicketSource`) comme GitHub Issues. Le cœur ne garde que le nom de la source ; `Core::client_for` l'envoie à `ProcessTicketSource` et prête au plugin la clé et le jeton qu'il tient (`Runtime::lend`). Les textes de formulaire de la source sont dans son manifeste.

## Conséquences

Aucun changement visible, sauf Trello dans la vue Plugins, coupable pour l'app ou un projet. Le plugin reçoit le secret du cœur même quand le trousseau le refuse. Les trois autres sources répètent le geste.
