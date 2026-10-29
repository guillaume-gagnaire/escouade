---
title: "Vue d'ensemble"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Tous les agents de tous les projets sur un seul écran, ceux qui attendent une réponse en tête. On y voit ce que fait chacun, ce qu'il coûte et depuis quand. On peut répondre sur place à une autorisation simple, ou ouvrir l'agent.

## Comportement

### Ouvrir et fermer

- « Vue d'ensemble », en haut de la barre des projets (infobulle « Tous les agents de tous les projets (Ctrl+Maj+A) »), ou `Ctrl+Maj+A`. Le raccourci marche aussi depuis un terminal, sans rien envoyer au shell.
- La vue remplace toute la zone à droite de la barre des projets, barre latérale comprise, comme « Stats ».
- Le bouton ou le raccourci de nouveau, ou Échap, reviennent à la vue d'avant. Échap ne fait rien derrière une boîte de dialogue ou un menu ouvert.
- La vue ouverte est mémorisée d'un lancement à l'autre.

### Contenu

- En-tête : « Vue d'ensemble », « 3 projets · 8 agents · 2 en cours » et « ↑ ↓ choisir · Entrée ouvrir · Échap revenir ».
- Groupes :
  - d'abord « Attend ta réponse » : les agents de tous les projets qui ont une question ou une autorisation en attente, celui qui attend depuis le plus longtemps d'abord, chaque ligne avec son projet ;
  - puis un groupe par projet, dans l'ordre de la barre des projets, ses agents dans l'ordre de la barre latérale.
- Un agent n'apparaît qu'une fois. Ni agents archivés, ni projets sans agent.
- Sans agent : « Aucun agent pour l'instant. »

### Une ligne

- Statut, nom, ticket (« ▸ DEM-1 · boucle 2/5 »).
- Activité, comme sur les cartes du Kanban : « Lit src/db.ts », « Prépare le worktree · … », « Reprise à … », sinon le statut.
- Modèle ; coût de la session (« ≈ » pendant un tour) ; contexte utilisé (en couleur d'attente au-delà de 80 %, « — » tant qu'il n'est pas connu) ; « Depuis » : le dernier changement de statut ou le dernier message.

### Répondre sur place

- Une ligne en attente montre sa plus ancienne demande, résumée sans charger la conversation : textes de 2 000 caractères au plus, listes de 10.
- Autorisation : le badge de l'outil et son argument (4 lignes au plus ; caractères invisibles ou de direction écrits en clair, comme `⟨U+202E⟩`, et texte affiché de gauche à droite), la description et la raison.
  - « Autoriser » et « Refuser » sur place, seulement pour un outil dont le résumé dit tout (Bash, PowerShell, commande slash, Read, Edit, Write, MultiEdit, NotebookEdit, Grep, Glob, WebFetch, WebSearch), avec un argument non vide affiché en entier, et que Claude Code n'invite pas à refuser par défaut.
  - Sinon « Répondre », qui ouvre l'agent, avec « À lire dans la conversation avant de répondre. » (autre outil, MCP compris, argument vide, ou demande que Claude Code refuserait par défaut) ou « Trop long pour être lu ici : lis-la et réponds dans la conversation. » (argument coupé, de plus de 4 lignes, ou qui déborde une fois dessiné).
- Question : son texte et « Répondre ». Plan à approuver : « Claude propose un plan » et « Répondre ».
- Une ligne dont la demande, le contenu ou la place change pendant que la vue est ouverte (celle du dessus vient d'être réglée, la demande suivante arrive) garde ses boutons fermés 500 ms.

### Clavier

- Une seule ligne dans l'ordre de tabulation ; ↑ ↓ Début Fin passent d'une ligne à l'autre.
- Entrée, Espace ou un clic ouvrent l'agent dans son projet, l'éditeur fermé.
- `Ctrl+Entrée` n'y répond à rien.
- Après une réponse au clavier, le focus revient sur la ligne de l'agent.

## Limites

- On ne répond sur place qu'aux autorisations simples ; les questions, les plans et les autres autorisations se répondent dans la conversation.
