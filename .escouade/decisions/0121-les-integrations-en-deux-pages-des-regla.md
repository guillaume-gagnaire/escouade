---
number: 121
id: "les-integrations-en-deux-pages-des-regla"
title: "Les intégrations en deux pages des réglages : Services connectés (app) et Tickets importés (projet)"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-383"
pages: ["kanban/integrations","fenetre/reglages","git-fichiers/hebergeurs-de-code","git-fichiers/pull-requests","git-fichiers/vue-pull-requests"]
tickets: ["ESC-383"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/integrations","added":21,"removed":14},{"page":"fenetre/reglages","added":7,"removed":5},{"page":"git-fichiers/hebergeurs-de-code","added":3,"removed":2},{"page":"git-fichiers/pull-requests","added":2,"removed":2},{"page":"git-fichiers/vue-pull-requests","added":1,"removed":1}]
accepted_at: "2026-11-08T04:56:15+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'onglet Intégrations mêlait des réglages de l'app (hébergeurs de code, comptes des sources de tickets, synchronisation, import automatique) et ceux d'un projet (sources liées, correspondance des statuts), sous la pastille « Projet » : on changeait un réglage de toute l'app en croyant ne toucher qu'un projet (cause 2 du diagnostic de design/settings-2-2/PROPOSITION.md). La direction A des réglages a été choisie le 2026-11-07 ; ce ticket est son lot R5.

## Décision

L'onglet est coupé en deux pages. « Services connectés », une page d'Escouade qui ne demande aucun projet : « Hébergeurs de code », puis « Sources de tickets » avec leurs comptes, « Synchronisation » et « Import automatique ». La synchronisation et l'import automatique restent des réglages de l'app, comme le recommandait l'étude (§7, point 4). « Tickets importés », une page de chaque projet : les sources liées et la correspondance des statuts. Sans compte connecté, elle renvoie à Services connectés par un bouton. Les renvois de l'import (« Lier une source », « ⚙ Gérer les sources ») mènent à la page Tickets importés de leur projet. Ceux des PR (« Réglages › Services connectés » de l'onglet PR et CI et de la vue Pull requests) mènent à Services connectés. Les textes du cœur qui nommaient « Réglages › Intégrations » disent « Réglages › Services connectés ». Aucune clé enregistrée ne change.

## Conséquences

Chaque portée a sa page et sa teinte. Pour lier une source après avoir connecté un compte, il faut passer d'une page à l'autre, et le bouton de Tickets importés fait ce trajet. Les messages des plugins de première main (Jira, ClickUp, Azure DevOps, le kit des hébergeurs) disent encore « Réglages › Intégrations » jusqu'à leur prochaine version publiée au store. IntegrationsTab.svelte est remplacé par ServicesPage.svelte et TicketsPage.svelte.
