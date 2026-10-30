---
number: 4
title: "Les plugins d'exemple dans plugin-examples/ du dépôt d'Escouade"
date: "2026-10-30"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-182"
pages: ["plugins/plugins-d-exemple"]
tickets: ["ESC-182"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/plugins-d-exemple","added":18,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

PLG-10 demandait les exemples dans un dépôt à part, `escouade-plugins`. Ce nom est celui de l'exécutable des plugins de première main (INT-4), et créer un dépôt GitHub public est un geste externe. Trois remplaçants vivaient déjà dans tests/fixtures/plugins/examples/.

## Décision

Les exemples vivent dans `plugin-examples/` du dépôt d'Escouade, un dossier lisible seul par plugin (Contraste élevé, Webhook Discord, Slack, Règles de l'équipe), et ils remplacent les fixtures. Gemini CLI sort de la liste (il est dans le cœur) ; la source tierce attend son point d'extension. Un dépôt public, s'il en sort un, s'appellera escouade-plugin-examples.

## Conséquences

Une CI suffit : plugin-examples/ci.mjs (tests, dist/ comparé, validate, banc) dans le job Linux, et le site vérifie le code qu'il montre sans rien télécharger. Les exemples ne versionnent pas de package-lock.json (la revue du store y lirait des hôtes) et figent esbuild.
