---
number: null
id: "la-porte-du-serveur-mcp-refuse-les-outil"
title: "La porte du serveur MCP refuse les outils inconnus et plafonne les requêtes sans pièce jointe"
date: "2026-11-12"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-422"
pages: ["harness/serveur-mcp"]
tickets: ["ESC-422"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/serveur-mcp","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'audit de sécurité de novembre 2026 (SEC-04, RUSTSEC-2026-0461) a montré qu'un jeton suffit à épuiser la mémoire de l'app : rmcp garde le nom de chaque outil demandé, même inconnu, et une requête pouvait peser 32 Mio.

## Décision

En plus de la mise à jour de rmcp (3.9), Gate::admit lit le corps après le jeton : un tools/call d'un outil inconnu est refusé (400, nom jamais répété) et une requête qui n'appelle ni create_ticket ni update_ticket est plafonnée à 2 Mio (413) ; 32 Mio restent permis à ces deux outils.

## Conséquences

Un client qui appelait un outil inexistant reçoit une erreur HTTP 400 au lieu d'une erreur de protocole. Tout nouvel outil qui prend des images ou des fichiers en base64 doit s'ajouter à BULKY_TOOLS (http.rs).
