---
number: null
id: "le-cœur-lit-un-navigateur-dans-un-monde"
title: "Le cœur lit un navigateur dans un monde isolé, et chaque navigateur tient sa console dès sa création"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-668"
pages: ["kanban/page-du-lancement","kanban/navigateurs-pilotes"]
tickets: ["ESC-668"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/page-du-lancement","added":10,"removed":0},{"page":"kanban/navigateurs-pilotes","added":3,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Les outils des agents qui liront les navigateurs (lot 1 de docs/AGENT-BROWSERS-TICKETS.md, AB-2 à AB-4) ont besoin d'un pont entre le cœur et la webview d'un navigateur. La sonde d'ESC-663 a montré que le monde de la page ment (JSON.stringify et querySelector réécrits), qu'un passe-plat vers le protocole DevTools lit cookies et stockage, et que WebKit ne donne pas la console à l'app.

## Décision

Le pont de lecture natif (browserread.rs, ESC-668) : le cœur ne lit une page que par des scripts fixes d'Escouade exécutés dans le monde isolé escouade-agent, par la poignée native de la webview (WebView2 : Page.createIsolatedWorld et Runtime.evaluate ; WebKitGTK : run_javascript_in_world ; WKWebView : WKContentWorld), jamais par eval ni un script d'agent. Sous Windows, une liste fermée de méthodes du protocole DevTools, sans port ; Runtime.evaluate limité au monde et aux scripts d'Escouade. Chaque navigateur garde dès la création de sa webview 500 entrées de console et 500 requêtes en échec, adresses sans chaîne de requête : par les événements du protocole sous Windows, par un relais posé au début de chaque document dans le monde de la page sous WebKit (marqué « rapporté par la page »). Une lecture du HTML masque mots de passe, champs cachés, metas de jeton, scripts en ligne et jetons visibles. Captures par système (vue, élément, document), 1 600 px au plus.

## Conséquences

Rien ne change à l'écran : les outils MCP viennent avec AB-2 à AB-4. Sous Linux et macOS, un script de plus tourne dans chaque page de navigateur (il enveloppe console, fetch et XHR) : une page peut le voir. Sous Windows, Runtime, Log et Network sont activés pour chaque navigateur (sans garder les corps). La branche macOS compile mais n'a tourné sur aucun Mac : la CI Mac d'AB-5 en fera le premier essai.
