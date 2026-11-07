---
number: 49
id: "jira-sort-du-cœur-plugin-de-premiere-mai"
title: "Jira sort du cœur : plugin de première main en process, sur le modèle de Trello et Linear"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-215"
pages: ["kanban/integrations"]
tickets: ["ESC-215"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/integrations","added":6,"removed":0}]
accepted_at: "2026-11-02T03:44:51+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Tout l'externe passe en plugins (ESC-203). Après Trello (INT-7.1) et Linear (INT-7.2), Jira est la troisième source du Kanban à sortir (INT-7.3) ; ClickUp suivra.

## Décision

Jira est le plugin builtin `dev.escouade.jira`, une crate de `plugins/builtin/dev.escouade.jira/` appelée par `escouade-plugins jira`, posée sur le kit `escouade-plugin-kit`. Le cœur ne garde que le nom de la source, l'adresse d'un site et la règle https ; `Core::client_for` prête au plugin le jeton (seul secret ; le site et l'e-mail lui sont dits). Les textes de formulaire sont dans le manifeste. Le kit gagne `SourceKind::account_name` (par défaut l'hôte de l'instance, ici toujours `jira`), sans rien changer pour les autres.

## Conséquences

Aucun changement visible, sauf Jira dans la vue Plugins, coupable pour l'app ou un projet. Les tests de l'ancien client sont rejoués contre le plugin lancé par le runtime (`plugins_jira_tests.rs`).
