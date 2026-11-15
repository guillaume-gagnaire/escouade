---
title: "Éditeur"
subject: "Éditeur"
sources: ["docs/SPEC.md","docs/EDITOR-STUDY.md","docs/TODO-2.3.md"]
tickets: ["ESC-447"]
decisions: [170]
compared: "2026-10-30T05:39:53+01:00"
gaps: [{"text":"La spec dit que le double-clic sur la poignée ramène la colonne des fichiers à 240 px. Le code revient à 200 px (TREE_NARROW) quand la zone de l'éditeur fait 904 px ou moins, et à 240 px seulement au-delà.","code":["src/lib/editor/layout.ts","src/components/editor/EditorView.svelte"]}]
order: 25
---

## Ce que c'est

Le mode éditeur prend la zone principale d'un projet, ou un volet de l'écran partagé, pour lire et modifier le code d'une source : la branche du projet ou le worktree d'un agent. Depuis la 2.3 (ESC-447), c'est la vue de VS Code : son plan de travail (`@codingame/monaco-vscode-api`, épinglé à une version, VS Code 1.138), avec son explorateur, ses onglets, son éditeur, sa recherche et sa palette, aux couleurs d'Escouade, sur des fichiers lus et écrits par le cœur. Il remplace l'éditeur en CodeMirror des versions précédentes, sans réglage pour choisir.

Ce que l'ancien éditeur faisait et que la vue ne fait pas encore revient par les tickets de l'éditeur de la 2.3 (voir `docs/EDITOR-STUDY.md`, §7) : A3 (raccourcis partagés avec Escouade, focus, liens des conversations vers une ligne, fermeture avec des modifications non enregistrées), A4 (thème et polices suivis en direct, langue), A5 (extensions), A7 (marques des changements par rapport à la base, comparaison avec le disque, choix de la source dans la vue), A1 (serveurs de langage). Les pages *Navigation de code*, *Recherche dans les fichiers* et *Voir les changements* décrivent l'ancien éditeur jusqu'à ces tickets.

## Comportement

### Ouvrir l'éditeur

- « Ouvrir dans l'éditeur », au clic droit sur un agent ou dans « … » de l'en-tête de sa conversation : son worktree, ou le projet s'il n'a pas de worktree.
- « Parcourir » sur la ligne de la branche, dans le pied de la barre latérale, et « Éditeur » dans la barre d'onglets : la branche du projet.
- Le bouton `</>` d'un fichier non commité, « Voir le code » d'un écart de la spec, un chemin de fichier dans la conversation, « Ouvrir un fichier » (`Ctrl+P`, voir *Ouvrir un fichier*) : le fichier s'ouvre dans un onglet de la vue.
- Un fichier hors du dossier de la source ne s'ouvre pas (message).
- Une seule vue VS Code par fenêtre : seule dans la zone de travail ou dans un volet (voir *Écran partagé*). Elle se déplace, se redimensionne, se ferme et se rouvre en gardant ses onglets et leurs modifications. Ses groupes d'onglets peuvent se mettre côte à côte dans la colonne. Un clic dans la vue active son volet ; son survol montre le ×.
- La barre au-dessus : « Conversation », le sélecteur de source, « VS Code ». Le menu de la source s'ouvre par-dessus la vue.
- À la première ouverture dans la fenêtre, « Chargement de la vue VS Code… » ; si elle ne démarre pas : « La vue VS Code n'a pas démarré : <raison> ».

### Sources

- Sélecteur de source : la branche du projet ou le worktree d'un agent, avec son nombre de modifications. Chaque source choisie devient un dossier de l'espace de travail de VS Code, nommé d'après le projet, ou d'après l'agent et son worktree (« agent-1 · .claude/worktrees/agent-1 »).
- L'éditeur et les terminaux refusent un agent d'un autre projet. Un worktree supprimé : « Le worktree de l'agent n'existe plus ».

### Les fichiers, par le cœur

- La page ne lit ni n'écrit jamais le disque elle-même : tout passe par les commandes de l'éditeur (arborescence, état d'un fichier, lecture et écriture en octets, création, renommage, suppression vers la corbeille), jamais hors du dossier de la source. Une adresse hors des sources servies n'est pas demandée au cœur.
- L'explorateur montre les fichiers connus de git et les « Fichiers copiés dans les worktrees » du projet (voir *Git & fichiers*) ; aucun autre fichier ignoré. Un worktree d'agent dans le dossier du projet est sa propre source.
- Un fichier se lit en octets, texte ou non (64 Mo au plus). VS Code garde son encodage, son BOM et ses fins de ligne ; un fichier binaire s'affiche comme VS Code les affiche.
- Créer, renommer, supprimer : par l'explorateur de VS Code. La suppression envoie à la corbeille du système.
- Écriture atomique, jamais par-dessus un fichier en lecture seule ni un dossier.

### Dossiers protégés

Ils ne se renomment ni ne se suppriment, et rien n'y est créé depuis la source du projet : ni dossier, ni nouveau fichier. Un fichier de worktree déjà là, ouvert par un lien, s'enregistre.

- Les worktrees des agents : `.claude/worktrees` à toute profondeur, le « Dossier des worktrees » du projet quand il est dans la source (voir *Git & fichiers*), ceux que `git worktree list` donne pour le dépôt, et ceux des agents de l'app.
- Le dossier de chaque projet de l'app qui se trouve dans la source.
- Un dossier qui contient l'un d'eux.

La règle est vérifiée sur le chemin écrit et sur le chemin réel (liens suivis, noms courts de Windows comme `WORKTR~1`).

### Changements sur le disque et conflit

- Le cœur pousse à la vue les chemins que son observateur de fichiers a vus changer (plus de 512 entre deux lectures : « tout a pu changer »). Un fichier ouvert et non modifié qu'un agent a changé se relit tout de suite ; les fichiers ajoutés ou supprimés apparaissent ou disparaissent ensuite dans l'explorateur.
- Un fichier modifié dans la vue et changé sur le disque n'est pas relu. `Ctrl+S` montre alors le conflit de VS Code (« The content of the file is newer… », avec « Compare » et « Overwrite ») et n'écrit rien. « Overwrite » écrit par-dessus.
- Le cœur refuse aussi une écriture quand le fichier a changé entre la vérification de VS Code et l'écriture (`changed`), ou a disparu (`deleted`).

## Réglages

- Taille du texte : Réglages › Général › Avancé › « Taille du texte de l'éditeur », lue à l'ouverture de la vue (voir *Réglages*).
- Les « Fichiers copiés dans les worktrees » et le « Dossier des worktrees » du projet décident des fichiers montrés et des dossiers protégés (voir *Git & fichiers*).

## Limites

- Les messages propres à VS Code sont en anglais (le pack de langue vient avec A4). Le thème d'Escouade est lu à l'ouverture de la vue.
- Pas encore de serveurs de langage (A1) ni d'extensions (A5) : la vue n'a que les extensions déclaratives qu'elle embarque (langages, grammaires, thèmes).
- Quitter Escouade ou fermer un projet ne prévient pas encore des modifications non enregistrées dans la vue (A3).
- L'installeur grossit d'environ 3,3 Mo ; ces fichiers ne se chargent qu'à la première ouverture de la vue.
- Sur un partage réseau sans corbeille, Windows peut supprimer pour de bon.
