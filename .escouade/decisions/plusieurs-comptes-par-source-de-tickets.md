---
number: null
id: "plusieurs-comptes-par-source-de-tickets"
title: "Plusieurs comptes par source de tickets, choisis par projet"
date: "2026-11-09"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-397"
pages: ["kanban/integrations"]
tickets: ["ESC-397"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/integrations","added":19,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Un seul compte par source de tickets : connecter un second Jira remplaçait le premier, alors que l'utilisateur veut « un Jira différent par projet (des comptes différents) ». Le changement de forme des réglages devait rester lisible par une 2.1.

## Décision

Jira, Trello, Linear et ClickUp prennent plusieurs comptes, chacun avec un id stable et son secret au trousseau ; un compte identique (Jira : site et e-mail ; autres : utilisateur) est mis à jour. Le premier compte reste dans son champ d'integrations.json et ses entrées d'avant ; les suivants vont dans ticket-accounts.json, que la 2.1 ne réécrit pas (choix de l'utilisateur via l'overseer). Chaque source liée d'un projet nomme son compte ; un ticket importé garde le sien. Un compte déconnecté met en pause les sources qui l'utilisent, configuration gardée, avec le point ambre de R8. GitHub Issues, GitLab Issues et Azure Boards gardent les comptes de leur hébergeur.

## Conséquences

Aucune migration à faire : le compte d'avant est le premier. Un retour en 2.1 ne voit que le premier compte de chaque service. Les plugins Jira, Trello, Linear et ClickUp passent en 2.2.4 (le kit lit le secret du compte nommé) et doivent être publiés au store ; une version plus ancienne installée depuis le store refuse un second compte en le disant.
