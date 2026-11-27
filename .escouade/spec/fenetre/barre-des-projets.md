---
title: "Barre des projets"
subject: "Fenêtre"
sources: ["docs/SPEC.md","docs/INBOX.md (§ 6, § 10.1, § 15)","design/inbox-2-3/README.md (direction B)","docs/SERVER-UX.md (§ 4, § 5, § 6 direction B, § 11)","design/server-2-3 (captures b-*)"]
tickets: ["ESC-158","ESC-72","ESC-167","ESC-168","ESC-504","ESC-510","ESC-537","ESC-539"]
decisions: [240,218]
compared: null
---

## Ce que c'est

Une colonne à gauche de la fenêtre, de la barre de titre à la barre de statut. Elle donne accès à la vue d'ensemble, à la boîte « À toi » (serveur MCP allumé), à chaque projet ouvert par sa tuile, à l'ajout d'un projet, à l'aide, aux stats, aux plugins et aux réglages. Chaque tuile dit où en sont les agents de son projet. Au-dessus, la barre de titre porte le projet affiché et la recherche. Pendant le premier lancement (voir *Premier lancement*), elle laisse sa place au parcours. Dès que ce poste connaît un serveur, elle range aussi les espaces (voir *Les espaces*).

## Comportement

### La colonne

- De haut en bas : « Vue d'ensemble » (voir *Vue d'ensemble*), « À toi » quand le serveur MCP est allumé (voir plus bas), un filet, une tuile de 36 px par projet ouvert, `+`, puis « Aide », « Stats », « Plugins » et « Réglages » en bas.
- Au premier démarrage, sans projet : `+`, « Aide » et « Réglages » seuls.
- Trop de projets : la liste défile, avec des bords fondus. Le projet actif revient dans la vue à chaque choix. Une pastille ambre sous la liste (ou sur elle) signale un projet hors de vue qui attend, et le ramène dans la vue.

### La tuile « À toi » (ESC-510)

- Sous « Vue d'ensemble », l'icône de la boîte ; elle ouvre la vue de la boîte (direction B « La file », tous projets) à la place du projet, des stats ou de la vue d'ensemble, et la referme (on revient à ce qu'elle couvrait). `Ctrl+Maj+J` fait de même (voir *Raccourcis*). Pressée, elle a le trait au bord et l'anneau d'une vue ouverte.
- **Seulement quand le serveur MCP est allumé** (Réglages › Agents › « Les agents peuvent piloter Escouade ») : éteint, ni tuile, ni compteur, ni raccourci ; éteint pendant que la boîte est ouverte, elle se ferme et on revient au projet. Allumé ou éteint, la tuile apparaît ou disparaît aussitôt, sans redémarrer.
- **Son compteur**, sur le coin de la tuile : tout ce qui attend l'utilisateur dans la boîte, tous projets, c'est-à-dire les demandes ouvertes (une demande reportée ne compte qu'à son retour) et les agents arrêtés sur une question, une autorisation, un plan ou une suppression, ceux-là mêmes que compte « N en attente » de la barre de statut. Teal quand seules des demandes attendent, ambre dès qu'un agent est arrêté ; rien quand rien n'attend.
- Infobulle : « À toi », `Ctrl Maj J`, « Ce que tes agents attendent de toi, tous projets. » et ce que dit le compteur (« 7 demandes de tes agents · 2 agents arrêtés (en attente, Ctrl J) »). Lecteur d'écran : « À toi, 9 choses t'attendent ».

### Les espaces

- Un seul espace (ce poste ne connaît aucun serveur) : la colonne est celle d'avant, rien de plus.
- Dès deux espaces : **l'espace ouvert** au-dessus de ses projets (sa plaque, son nom court, « Poste » pour ce poste ; un clic ouvre son menu), puis, sous le `+` et un filet, **une pile par autre espace** : deux de ses projets en cartes décalées derrière sa plaque, ce qui y attend dans une pastille ambre, son nom dessous (en ambre s'il est coupé). Un clic y mène, le clic droit ouvre son menu.
- **Au-delà de trois espaces, ou à 1024 px de large, les piles se tassent en simples plaques** (la plaque et sa pastille) : la barre ne défile jamais pour elles.

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
- Dès 160 px, chaque projet tient une rangée : la tuile, son libellé (ou son nom) à droite, ses agents en attente au bout. Les tuiles gardent 36 px et leurs signes ; la tuile « À toi » a son nom et son compte à droite, elle aussi. L'infobulle s'ouvre alors au bord de la barre.

### Gestes

- Clic ou `Ctrl+1…9` : afficher le projet. `Alt+1…9` (`⌃1…9` sous macOS) et `Ctrl+²` : un espace (voir *Les espaces*).
- Ordre : glisser-déposer, ou `Alt+↑` / `Alt+↓` sur la tuile qui a le focus. L'ordre est gardé.
- Clic droit (ou clic du milieu) sur une tuile, comme sur le nom du projet dans la barre de titre : renommer, couleur, worktree par agent, « Réglages du projet… » (onglet « Projets » des réglages), ouvrir le dossier, Monter, Descendre, fermer le projet (voir *Fermer un projet*).
- `+` ouvre « Nouveau projet » (voir *Nouveau projet*). S'il y a des projets fermés, ou qu'un serveur est connu, il ouvre d'abord un menu : « Ajouter un projet… », les cinq derniers projets fermés (« Rouvrir »), « Tous les projets fermés… » (Réglages › Projets), puis « Se connecter à un serveur… ».
- « Aide » (icône d'information, infobulle « Le premier lancement, la documentation ») ouvre un menu, au clavier aussi : « Premier lancement » (le parcours refait, voir *Premier lancement*), « Documentation » (escouade.dev/docs, ou /en/docs, dans le navigateur) et « Signaler un problème » (les tickets du dépôt sur GitHub).
- « Plugins » ouvre la vue des plugins (voir *Les plugins : vue d'ensemble*), et la referme.

### Barre de titre

- 44 px de haut, au-dessus de la zone de travail.
- À gauche, au-dessus de la barre des projets : le logo d'Escouade (Windows, Linux) ou les feux de macOS.
- **La machine**, dès que ce poste connaît un serveur : la plaque et le nom de l'espace avant le projet, avec l'hôte SSH d'un serveur (masqué sous 1100 px de large), « Ce poste » sur ce poste ; en ambre avec « reconnexion » quand la connexion est coupée. Un `status` pour les lecteurs d'écran. Sur « Partout », « Tous les espaces / Vue d'ensemble » : aucune machine en particulier, chaque ligne dit la sienne. Une fenêtre d'espace a sa propre barre, donc sa machine.
- Puis le projet affiché : pastille de couleur, nom, cadenas, `Δ n`, menu.
- Au centre : « Rechercher » (`Ctrl+K`, voir *Rechercher dans les conversations*).
- À droite : « Éditeur » et les boutons de la fenêtre. « Éditeur » ouvre un menu, « Éditeur » ou « Workspace » (voir *Workspace*).

## Réglages

- Lettres, logo, libellé et son texte : Réglages › Projets › « Identité », et l'assistant « Nouveau projet ».
- La largeur de la barre se règle à la souris ou au clavier, sur son bord.
- La tuile « À toi » suit « Les agents peuvent piloter Escouade » (Réglages › Agents › Piloter Escouade).
