---
number: null
id: "navigateurs-et-pages-des-agents-construi"
title: "Navigateurs et pages des agents : construits et documentés sur le site (DOC)"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-686"
pages: ["kanban/navigateurs-pilotes","harness/pages-des-agents","kanban/page-du-lancement"]
tickets: ["ESC-686"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":16,"removed":7},{"page":"harness/pages-des-agents","added":5,"removed":3},{"page":"kanban/page-du-lancement","added":4,"removed":3}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le lot des navigateurs et des pages des agents (ESC-667 à ESC-685, liste de docs/AGENT-BROWSERS-TICKETS.md) est sur main, AP-9 (ESC-683) et AB-12 (ESC-685) compris. La spec disait encore « Brouillon, en partie construit » pour kanban/navigateurs-pilotes, et le site n'avait pas de page pour l'utilisateur : seulement les outils dans « Le serveur MCP », sans drive_browser. La page du lancement disait les outils des agents « à venir ».

## Décision

DOC (ESC-686, AP-8 et AB-13 réunis) : deux pages du site en fr et en, « Les pages des agents » (agent-pages) et « Les agents et les navigateurs » (agent-browsers), dans le groupe du Kanban après « La page du lancement » : à quoi ça sert, les deux modes, l'écran jamais pris, ce que voit l'utilisateur, la main, les paliers, les accords, les refus, se connecter dans un onglet, « Laisser piloter par… » et les deux interrupteurs. « Le serveur MCP » ajoute drive_browser, et custom et ask de show_page. Une seule capture, celle de l'e2e agent-browsers-band sur ses pages locales de test, jamais un compte réel. Dans la spec, kanban/navigateurs-pilotes n'est plus un brouillon (tout le lot 2.2 construit), harness/pages-des-agents compte AP-9 et DOC, et « Isolation » de kanban/page-du-lancement dit ce qui change avec les agents.

## Conséquences

Rien ne change dans l'app. Ce qui reste est dit dans les pages (2.3 : AB-R, Fonctionnalités, Overseer ; P9 et la capture d'une page d'agent ; l'ouverture comme geste numéroté). Les tests du site tiennent les deux pages (sections, liens, insécables, captures).
