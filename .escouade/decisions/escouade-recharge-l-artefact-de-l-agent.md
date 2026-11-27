---
number: null
id: "escouade-recharge-l-artefact-de-l-agent"
title: "Escouade recharge l'artefact de l'agent à chaque version qu'il publie, sauf quand l'utilisateur a la main"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-681"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-681"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":9,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-10 : un agent de Claude Code ouvre son artefact à côté de sa conversation (open_browser, artifact: "last"). L'étude (docs/AGENT-BROWSERS.md § 2 bis.4) faisait recharger l'onglet par l'agent (drive_browser reload) après chaque republication. Il fallait aussi décider d'où vient l'adresse et ce qui se passe quand l'utilisateur a la main.

## Décision

Une nouvelle publication de l'outil Artifact de l'agent, lue dans son propre journal (une publication réussie, jamais une lecture, un envoi de ressources, une adresse donnée par une page, un fichier ou le texte de l'agent), fait recharger par Escouade chaque onglet que cet agent a ouvert sur cet artefact. La bande dit « Recharge : version N » (N lu dans le résultat de l'outil). Ce n'est pas un geste de l'agent (hors de son rythme). Si l'utilisateur a la main ou a coupé le pilotage, l'onglet reste comme il l'a ; une version publiée après « La rendre » recharge de nouveau. Les paliers de C4 restent : montrer sans demander, lire et agir avec accord, jamais de cookies ni de stockage. Pour l'e2e, seul un build de débogage accepte une origine d'artefact locale (ESCOUADE_E2E_ARTIFACT_ORIGIN, http://127.0.0.1:port).

## Conséquences

L'agent n'a plus besoin d'appeler reload après chaque version (il peut encore le faire, et wait_user). Un utilisateur qui fait défiler l'artefact garde sa vue jusqu'à rendre la main. L'essai réel de la connexion à claude.ai dans un onglet d'Escouade reste à faire par l'utilisateur.
