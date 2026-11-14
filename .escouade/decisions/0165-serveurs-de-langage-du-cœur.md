---
number: 165
id: "serveurs-de-langage-du-cœur"
title: "Serveurs de langage du cœur"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-446"
pages: ["editeur/serveurs-de-langage","projets/escouade-yml"]
tickets: ["ESC-446"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/serveurs-de-langage","added":54,"removed":11},{"page":"projets/escouade-yml","added":10,"removed":4}]
accepted_at: "2026-11-14T21:26:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La 2.3 met la vue VS Code dans l'éditeur (étude ESC-444). Toute voie d'éditeur demande des serveurs de langage lancés par le cœur, jamais par la page. Il faut décider lesquels, où les trouver, comment les lancer et ce qu'un dépôt cloné peut en obtenir. À la relecture, deux trous sont apparus. D'abord, sous Windows, un shim npm du PATH qui appelle `node` le chercherait dans le dossier courant, la racine du dépôt. Ensuite, des serveurs installés sur le poste exécutent eux aussi le code du dépôt.

## Décision

Un catalogue de sept serveurs : TypeScript/JavaScript, Rust, Python (basedpyright, puis pyright), Go, JSON, CSS et HTML. Ordre de recherche : le chemin réglé à la main, le path du .escouade.yml, puis node_modules/.bin du worktree et du projet, le PATH, l'endroit standard de l'outil. Rien n'est téléchargé ; un serveur absent est dit avec sa commande d'installation. Lancement dans un Job, une instance par dossier et serveur partagée par les vues, arrêt 60 s après la dernière vue, au plus 5 relances de suite avec un délai qui double. Messages bornés à 32 Mo. Sous Windows, chaque serveur est lancé avec NoDefaultCurrentDirectoryInExePath=1. Une confiance par projet, gardée par le cœur, est demandée dans deux cas : quand le dépôt choisit le serveur (node_modules, path et args du .escouade.yml), et quand le serveur exécute du code du projet (rust-analyzer : build.rs et macros ; typescript-language-server : le tsserver du projet ; gopls : toolchain du go.mod et cgo, par précaution), même installé sur le poste ou réglé à la main. pyright, basedpyright, JSON, CSS et HTML restent libres. Réglages dans Général › Avancé et dans language_servers.<id> du .escouade.yml.

## Conséquences

Les tickets suivants de l'éditeur (A2 et après, ou la voie C) s'appuient sur lsp_start, lsp_send et les événements lsp-message et lsp-state. Un dépôt ne peut plus faire lancer un programme ni exécuter son code par un serveur sans un geste de l'utilisateur. En contrepartie, Rust, TypeScript et Go demandent une confirmation par projet avant de marcher. L'éditeur (A3) doit poser la question au premier fichier ouvert. Le gestionnaire écrit dans un Sink, donc l'API distante pourra le servir aussi. Un réglage changé vaut au prochain lancement du serveur.
