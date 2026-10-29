---
title: "Barre des projets"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: ["ESC-158","ESC-72","ESC-167","ESC-168"]
decisions: []
compared: null
---

## Ce que c'est

Une colonne à gauche de la fenêtre, de la barre de titre à la barre de statut. Elle donne accès à la vue d'ensemble, à chaque projet ouvert par sa tuile, à l'ajout d'un projet, aux stats et aux réglages. Chaque tuile dit où en sont les agents de son projet. Au-dessus, la barre de titre porte le projet affiché et la recherche.

## Comportement

### La colonne

- De haut en bas : « Vue d'ensemble » (voir *Vue d'ensemble*), un filet, une tuile de 36 px par projet ouvert, `+`, puis « Stats » et « Réglages » en bas.
- Au premier démarrage, sans projet : `+` et « Réglages » seuls.
- Trop de projets : la liste défile, avec des bords fondus. Le projet actif revient dans la vue à chaque choix. Une pastille ambre sous la liste (ou sur elle) signale un projet hors de vue qui attend, et le ramène dans la vue.

### Une tuile

- Fond : la couleur du projet. Dessus, au choix :
  - deux lettres : les initiales des deux premiers mots du nom, sinon ses deux premières lettres ;
  - les « Lettres de la tuile » choisies : une à trois lettres ou chiffres, ou un emoji seul ;
  - le « Logo » du projet.
- Logo : PNG, JPEG, WebP, SVG ou ICO de 1 Mo au plus, reconnu par son contenu. Il est copié sous `projects/<id>/` du dossier de données. Un SVG n'exécute rien. Le logo tient dans la forme de la tuile, sur sa couleur là où il est transparent. « Retirer » le retire.
- Lettres, logo et libellé sont propres au poste, comme la couleur. Ils sont gardés quand le projet est fermé ; la copie du logo est effacée avec l'historique du projet.
- Signes sur la tuile :
  - pastille ambre : le nombre d'agents en attente ;
  - point vert : un agent tourne ;
  - cadenas : projet privé (voir *Serveur MCP*) ;
  - anneau qui respire, de la couleur de l'alerte : un agent à voir.
- Le projet actif a un trait au bord et un anneau.
- Infobulle, à droite : nom, `Ctrl n`, agents en attente, au travail, à voir, `Δ n` (fichiers non commités, worktrees des agents compris), « Projet privé ».

### Libellé

- « Libellé sous la tuile » (éteint par défaut) et son texte : 12 graphèmes au plus ; vide, c'est le nom du projet.
- Il tient sur une ligne sous la tuile, coupé en « … ». Celui du projet actif est à l'encre du projet.

### Largeur

- Le bord droit de la barre se tire, de 52 à 300 px. Au clavier : ←/→ par 16 px, Début, Fin. Un double clic rend la largeur par défaut : 52 px, ou 72 dès qu'un projet a son libellé.
- La largeur est gardée, la même pour toute l'app. La barre ne prend jamais plus que la largeur de la fenêtre moins 948 px.
- Dès 160 px, chaque projet tient une rangée : la tuile, son libellé (ou son nom) à droite, ses agents en attente au bout. Les tuiles gardent 36 px et leurs signes. L'infobulle s'ouvre alors au bord de la barre.

### Gestes

- Clic ou `Ctrl+1…9` : afficher le projet.
- Ordre : glisser-déposer, ou `Alt+↑` / `Alt+↓` sur la tuile qui a le focus. L'ordre est gardé.
- Clic droit (ou clic du milieu) sur une tuile, comme sur le nom du projet dans la barre de titre : renommer, couleur, worktree par agent, « Réglages du projet… » (onglet « Projets » des réglages), ouvrir le dossier, Monter, Descendre, fermer le projet (voir *Fermer un projet*).
- `+` ouvre « Nouveau projet » (voir *Nouveau projet*). S'il y a des projets fermés, il ouvre d'abord un menu : « Ajouter un projet… », les cinq derniers projets fermés (« Rouvrir »), puis « Tous les projets fermés… » (Réglages › Projets).

### Barre de titre

- 44 px de haut, au-dessus de la zone de travail.
- À gauche, au-dessus de la barre des projets : le logo d'Escouade (Windows, Linux) ou les feux de macOS.
- Puis le projet affiché : pastille de couleur, nom, cadenas, `Δ n`, menu.
- Au centre : « Rechercher » (`Ctrl+K`, voir *Rechercher dans les conversations*).
- À droite : « Éditeur » et les boutons de la fenêtre.

## Réglages

- Lettres, logo, libellé et son texte : Réglages › Projets › « Identité », et l'assistant « Nouveau projet ».
- La largeur de la barre se règle à la souris ou au clavier, sur son bord.
