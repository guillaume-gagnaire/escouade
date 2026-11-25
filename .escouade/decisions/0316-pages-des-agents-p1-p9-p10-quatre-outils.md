---
number: 316
id: "pages-des-agents-p1-p9-p10-quatre-outils"
title: "Pages des agents P1, P9, P10 : les outils MCP, qui montre, durée de vie"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-661"
pages: ["harness/pages-des-agents"]
tickets: ["ESC-661","ESC-663"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":22,"removed":0}]
accepted_at: "2026-11-26T00:19:16+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-661 : un agent de tout harness doit pouvoir montrer une page HTML dans Escouade (revue avec choix, schéma, page à soi) et la piloter. Il faut le contrat MCP, qui peut s'en servir et combien de temps une page vit.

## Décision

Choisi par l'utilisateur le 2026-11-25 (demande a-36, recommandations acceptées). P1 : `show_page` (modèle, données ou HTML / fichier du worktree, versions, `ask`), `get_page`, `close_page`, et le pilotage par `drive_browser`, l'outil unique des lignes de « Navigateurs » d'ESC-663 (cible `page: "p-12"` ou `browser: "page:p-12"`, en AP-0) : scroll_to, highlight, clear, width, tab, version, par des scripts fixes d'Escouade exécutés dans un monde isolé de la page (jamais `webview.eval`, que la page peut fausser) ; écarté : un seul outil `page` à actions. P9 : agents, Overseer, discussions ; Claude hors Escouade seulement avec une demande. P10 : la page vit avec son agent puis le délai des fichiers de la boîte (30 j par défaut), 20 pages par agent, 10 versions ; écarté : le temps du lancement d'Escouade seulement.

## Conséquences

Un registre `p-<n>` dans l'état durable, des copies dans `<données>/pages/`, AP-2 et AP-6 (docs/AGENT-PAGES.md § 14). Les outils partent avec la fonctionnalité et la partie « Lancement et navigateurs ».
