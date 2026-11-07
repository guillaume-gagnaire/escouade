---
number: 48
id: "linear-sort-du-cœur-plugin-de-premiere-m"
title: "Linear sort du cœur : plugin de première main en process, sur le modèle de Trello"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-214"
pages: ["kanban/integrations"]
tickets: ["ESC-214"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/integrations","added":6,"removed":0}]
accepted_at: "2026-11-02T01:31:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Tout l'externe passe en plugins (ESC-203). Après Trello (INT-7.1, décision 0042), Linear est la deuxième source du Kanban à sortir (INT-7.2) ; Jira et ClickUp suivront.

## Décision

Linear est le plugin builtin `dev.escouade.linear`, une crate de `plugins/builtin/dev.escouade.linear/` appelée par `escouade-plugins linear`, posée sur le kit `escouade-plugin-kit` comme Trello. Le cœur ne garde que le nom de la source ; `Core::client_for` l'envoie à `ProcessTicketSource` et prête au plugin la clé que le cœur tient. Le texte de formulaire de la source est dans son manifeste. Le kit n'est pas modifié.

## Conséquences

Aucun changement visible, sauf Linear dans la vue Plugins, coupable pour l'app ou un projet. Les tests de l'ancien client sont rejoués contre le plugin lancé par le runtime (`plugins_linear_tests.rs`).
