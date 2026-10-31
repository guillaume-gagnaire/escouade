---
number: 26
title: "La page d'un lancement s'ouvre dans Escouade, à part, plutôt que dans le navigateur du système"
date: "2026-10-31"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-311"
pages: ["kanban/page-du-lancement","kanban/lancement-de-test"]
tickets: ["ESC-311"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/page-du-lancement","added":29,"removed":1},{"page":"kanban/lancement-de-test","added":4,"removed":4}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur veut tester l'app que l'agent développe sans quitter Escouade (2026-10-31) : jusqu'ici « ▶ Tester » ouvrait sa page dans le navigateur du système. Parmi trois directions montrées sur aperçu (A : une vue « Page » dans l'agent, B : un panneau à côté de la conversation, C : une fenêtre à part), il a choisi A, avec l'option d'ouvrir la page dans une fenêtre Escouade à part (C), et un mode responsive pour passer vite de la pleine largeur à la tablette et au mobile.

## Décision

1. La page d'un lancement s'ouvre dans Escouade quand le lancement répond, là où le projet le dit (réglage gardé sur la machine, jamais dans `.escouade.yml`) : dans la vue « Page » de l'agent (par défaut), dans la fenêtre « Pages » du projet (un onglet par worktree), ou dans le navigateur du système comme avant. Une page par agent : relancer recharge la même.
2. La page est le contenu de l'app développée, auquel Escouade ne fait pas confiance : une webview à part (`devpage-*`) à qui aucune commande d'Escouade ni d'un plugin ne répond (refus par son label, capacité limitée aux webviews `main` et `pagesbar-*`), avec le stockage de son projet et non celui de la fenêtre, qui reste sur l'origine de son lancement (une autre origine s'ouvre dans le navigateur du système ; les adresses d'Escouade, les fichiers et les autres schémas nulle part).
3. Ses devtools sont les siens (bouton, F12, Ctrl+Maj+I, ⌥⌘I) ; ceux de la fenêtre d'Escouade restent coupés dans une version publiée.
4. Trois largeurs (pleine, tablette 768 px, mobile 390 px), Ctrl+Maj+M pour passer à la suivante, gardées par page ; seule la largeur change, ni le user agent ni le tactile ne sont émulés.

## Conséquences

Tauri passe avec les fonctionnalités devtools et unstable ; la capacité nomme des webviews (main, pagesbar-*) et non plus la fenêtre. La webview native passe au-dessus du DOM : elle se masque sous les menus et dialogues. Sous Linux, Tauri 2.12 range les webviews enfants dans une GtkBox : la page passe dans un gtk::Layout pour être placée. Sous macOS, l'inspecteur attaché imposait la largeur de la page : il est détaché dans sa propre fenêtre. Le user agent, le devicePixelRatio et le tactile ne sont pas émulés (possible sous Windows par le protocole des devtools, UA seul ailleurs). L'accès des agents à la page (MCP) reste à faire.
