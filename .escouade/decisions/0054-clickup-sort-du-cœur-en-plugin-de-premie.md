---
number: 54
id: "clickup-sort-du-cœur-en-plugin-de-premie"
title: "ClickUp sort du cœur en plugin de première main, en process"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-216"
pages: ["escouade/spec/kanban/integrations"]
tickets: ["ESC-216"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-02T06:28:08+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

« Tout l'externe en plugins » (ESC-203), INT-7.4 : après Trello, Linear et Jira, ClickUp doit sortir du cœur.

## Décision

ClickUp est le plugin dev.escouade.clickup (crate escouade-plugin-clickup, process `escouade-plugins clickup`), livré et signé avec l'app, sur le kit escouade-plugin-kit. Le cœur ne garde que le nom de la source ; le jeton est prêté par le cœur, l'adresse de l'API et l'utilisateur sont dits au plugin ; ClickUp garde sa vue propre des états (states_view : défauts par noms puis par types).

## Conséquences

Une seule modification du kit : la clé du cache des clients de source contient l'utilisateur du compte, sinon le client créé à « Connecter » (utilisateur encore inconnu) servait « Mes tâches » sans filtre. L'interface lit la source depuis le registre ; un arrêt du plugin se dit à la synchro ; les tests de l'ancien client sont rejoués contre le plugin.
