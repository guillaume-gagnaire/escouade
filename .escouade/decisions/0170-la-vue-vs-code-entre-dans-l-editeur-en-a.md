---
number: 170
id: "la-vue-vs-code-entre-dans-l-editeur-en-a"
title: "La vue VS Code devient l'éditeur de projet, sur les fichiers du cœur"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-447"
pages: ["editeur/editeur"]
tickets: ["ESC-447","ESC-444"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/editeur","added":21,"removed":100}]
accepted_at: "2026-11-15T02:53:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur a choisi le 2026-11-14 la vue VS Code (monaco-vscode-api) pour l'éditeur de la 2.3 (étude ESC-444, docs/EDITOR-STUDY.md). ESC-447 l'a d'abord mise derrière un réglage d'aperçu ; le même jour, l'utilisateur a décidé que rien de la 2.3 ne serait publié avant que tout soit prêt, donc sans réglage d'aperçu : la vue devient tout de suite, et seule, l'éditeur d'un projet.

## Décision

1) La vue VS Code est l'éditeur d'un projet, dans la zone de travail comme dans un volet ; pas de réglage, pas d'aiguillage entre deux éditeurs. L'éditeur de projet en CodeMirror (son arborescence, ses onglets, sa recherche, ses marques, sa comparaison) est retiré du code ; git le garde, A7 s'en inspirera. Les paquets CodeMirror, que plus rien n'importait, sont retirés aussi. 2) monaco-vscode-api et ses paquets épinglés à 37.6.1 (VS Code 1.138, la version mesurée sur les trois systèmes). 3) Le fournisseur de fichiers ne passe que par les commandes du cœur : lecture et écriture en octets (`fs_read_bytes`, `fs_write_bytes`), état d'un fichier (`fs_stat`, temps et taille réels, que VS Code compare pour montrer son conflit), arborescence, création, renommage, suppression ; jamais hors du dossier de la source (projet ou worktree d'un agent, chacun un dossier de l'espace de travail). 4) Le cœur pousse les chemins que voit son observateur de fichiers (`fsChanged`, 512 au plus entre deux lectures, sinon « tout ») : la vue relit un fichier ouvert non modifié. 5) Pas d'hôte d'extensions dans la page tant que A5/A6 ne sont pas faits : la CSP ne gagne pas `unsafe-eval`, seulement `blob:` et `data:` pour les workers et les fichiers des extensions par défaut. 6) Les e2e de l'éditeur de projet qui vérifient ce que la vue ne fait pas encore sont des `test.fixme` nommant le ticket qui les ramène (A3, A4, A7).

## Conséquences

La 2.3 n'a qu'un éditeur de projet, celui de VS Code, et ne se publie qu'une fois A1 à A8 faits. D'ici là, la vue n'a ni les raccourcis partagés ni la question des modifications non enregistrées en quittant (A3), ni le thème suivi en direct (A4), ni les marques des changements et la comparaison avec le disque (A7) : les pages Navigation de code, Recherche dans les fichiers et Voir les changements décrivent l'ancien éditeur jusqu'à ces tickets. L'installeur grossit d'environ 3,3 Mo (brotli), chargés à la première ouverture. Une montée de monaco-vscode-api est un ticket à part, toutes ses versions ensemble.
