---
number: null
id: "un-agent-ne-repond-aux-demandes-d-un-aut"
title: "Un agent ne répond aux demandes d'un autre agent que si l'utilisateur l'a choisi"
date: "2026-11-12"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-423"
pages: ["harness/serveur-mcp"]
tickets: ["ESC-423","ESC-418"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/serveur-mcp","added":13,"removed":5}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Audit ESC-418, fiche SEC-03 (élevée, démontrée) : avec « Les agents peuvent piloter Escouade », un agent accordait par answer_permission (ou répondait par answer_question) ce qu'un autre agent demandait ; le seul refus portait sur l'appelant lui-même. Un agent détourné par une injection de consignes pouvait créer un agent et autoriser chacune de ses demandes : l'humain sortait de la boucle. La décision du 2026-10-15 (« tout client répond, sans réglage de plus ») l'assumait. Le principal d'un dossier ne pouvait pas servir de chef : create_agent rend principal tout agent qui en crée un autre.

## Décision

« Piloter Escouade » ne donne plus answer_permission ni answer_question aux agents. Un réglage global, Réglages › Agents › Piloter Escouade › « Les agents peuvent répondre aux questions et aux permissions des autres agents », a trois valeurs : Personne (défaut, nouvelle installation comme existante ; le chef compris), Seulement le chef d'escouade, Tous les agents (avec un avertissement : ça retire l'humain de la boucle). Le chef est un réglage du projet, Réglages › Projets › Équipe › « Chef d'escouade », posé par la fenêtre seule (aucun outil MCP), sans reprise à la mise à jour ; le principal d'un dossier ne l'est pas. Claude hors Escouade répond toujours. Le refus dit la raison et où changer le réglage. list_requests ne change pas ; une réponse d'agent reste tracée chez l'agent qui demandait. Choix de l'utilisateur (réponse du 2026-11-12 dans ESC-423).

## Conséquences

Après la mise à jour, aucun agent ne répond pour un autre, le chef d'escouade de ce projet (overseer) compris, tant que l'utilisateur n'a pas choisi « Seulement le chef d'escouade » et désigné son chef dans Réglages › Projets › Équipe. Remplace la règle du 2026-10-15. Réglages : un champ agentsAnswer (Settings) et chef (Project), local, jamais dans .escouade.yml.
