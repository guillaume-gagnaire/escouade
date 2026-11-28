---
number: null
id: "un-agent-ouvre-un-onglet-a-cote-de-sa-co"
title: "Un agent ouvre un onglet à côté de sa conversation ; ce poste hors des ports déclarés est refusé"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-678"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-678"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":23,"removed":5}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-6 (ESC-678) construit le second mode voulu par l'utilisateur : un vrai onglet de navigateur qu'un agent ouvre à côté de sa conversation (open_browser) et ferme (close_browser), pour montrer le lancement du projet, une adresse ou l'artefact Claude qu'il vient de publier. L'étude (docs/AGENT-BROWSERS.md § 5.1) demandait à l'utilisateur toute adresse hors des origines permises, ce poste à un port non déclaré compris.

## Décision

open_browser ouvre url, launch (la page du lancement de l'agent) ou artifact ("last" ou une adresse que l'outil Artifact de l'agent a rendue dans sa conversation, une publication réussie seulement), à la place beside (défaut), below, full ou quiet ; waiting quand l'utilisateur ne regarde pas la conversation de l'agent, la ligne s'ouvrant à son retour. Sans demander : les origines des commandes de lancement et ce poste aux ports déclarés (jamais au port du serveur MCP). Demandée une fois par origine dans une bande de la conversation (C3) : toute autre adresse d'internet ; « Permettre une fois » l'ouvre et la permet dans ce navigateur, « Refuser » tient. Toujours refusés, même avec l'accord : un autre schéma que http(s), une adresse avec identifiants, une adresse d'Escouade, et, plus strict que l'étude, ce poste (localhost et ses sous-domaines, 127.0.0.0/8, [::1], 0.0.0.0, en toute graphie) à un port qu'aucun lancement ne déclare : refusé, pas demandé. Bornes : 3 navigateurs pilotés par agent, 10 ouvertures par heure, 5 demandes en attente, un volet par agent. L'onglet garde le stockage du projet, jamais celui de la fenêtre. close_browser ferme un onglet que l'agent a ouvert, arrête le pilotage d'une ligne de lancement (la ligne reste), jamais un navigateur de l'utilisateur ni d'un autre agent.

## Conséquences

Un agent ne peut pas, même avec un clic distrait de l'utilisateur, ouvrir un service du poste ou d'Escouade (une base, un débogueur, le serveur MCP) : pour en montrer un, il faut le déclarer comme commande de lancement du projet. Le reste du pilotage (drive_browser, la bande, la main) vient avec AB-7 et AB-8 ; AB-9 affinera l'écran (rien de rouvert après une fermeture, « Laisser piloter par… »).
