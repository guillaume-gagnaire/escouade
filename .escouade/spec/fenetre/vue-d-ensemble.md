---
title: "Vue d'ensemble"
subject: "Fenêtre"
sources: ["docs/SPEC.md","docs/TODO-1.8.md (chapitre 2, « Notifications, vue d'ensemble, serveur MCP »)","design/esc-68/README.md","src/lib/prlist.ts"]
tickets: ["ESC-68"]
decisions: []
compared: null
---

## Ce que c'est

Tous les agents de tous les projets sur un seul écran, ceux qui attendent une réponse en tête, puis les pull requests qui attendent l'utilisateur. On y voit ce que fait chacun, ce qu'il coûte et depuis quand. On peut répondre sur place à une autorisation simple, ou ouvrir l'agent.

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
  - puis « Pull requests » (voir plus bas), quand une PR attend l'utilisateur ;
  - puis un groupe par projet, dans l'ordre de la barre des projets, ses agents dans l'ordre de la barre latérale.
- Un agent n'apparaît qu'une fois. Ni agents archivés, ni projets sans agent.
- Sans agent ni PR : « Aucun agent pour l'instant. »

### Une ligne

- Statut, nom, ticket (« ▸ DEM-1 · boucle 2/5 »).
- Activité, comme sur les cartes du Kanban : « Lit src/db.ts », « Prépare le worktree · … », « Reprise à … », sinon le statut.
- Modèle ; coût de la session (« ≈ » pendant un tour) ; contexte utilisé (en couleur d'attente au-delà de 80 %, « — » tant qu'il n'est pas connu) ; « Depuis » : le dernier changement de statut ou le dernier message.

### Le groupe « Pull requests »

- Trois parties, chacune avec son compte, la plus pressante d'abord, le plus grand numéro en tête ; une partie vide ne se montre pas :
  - « À corriger » : ouverte et prête, CI en échec, changements demandés ou conflit ;
  - « À merger » : ouverte et prête, au moins une approbation, ni CI en cours ni rien à corriger ;
  - « Relectures demandées » : une relecture est demandée à l'utilisateur sur la PR de quelqu'un d'autre.
- Sont regardées : les PR des tickets « En revue », celles des agents non archivés, et dans les listes des projets celles que l'utilisateur a ouvertes et celles dont la relecture lui est demandée. Une même PR n'a qu'une ligne, deux PR de même numéro dans deux dépôts en ont deux. La PR de quelqu'un d'autre qui ne demande rien à l'utilisateur reste dans la vue « Pull requests ». Un brouillon n'est ni à corriger ni à merger.
- Une ligne suit les colonnes des agents : le signe de sa CI, « PR #12 » (« MR #12 » sur GitLab), son projet, son ticket ou son agent ; son titre sous « Activité » ; ses deux premières pastilles de la vue « Pull requests », sinon sa CI en mots, à la place du modèle, du coût et du contexte ; son âge sous « Depuis ». Sous 720 px, une pastille seulement.
- Le titre est « Pull requests » ; « Merge requests » quand toutes viennent de GitLab.
- Un clic ou Entrée ouvre son agent, sinon le Kanban de son ticket, sinon la vue « Pull requests » de son projet. Les flèches passent de ces lignes à celles des agents.
- Les listes se lisent à l'ouverture de la vue, puis chaque minute tant qu'elle est affichée et la fenêtre visible, au plus une fois par minute par projet, celles que la vue « Pull requests » vient de lire comprises. Il reste moins d'un dixième de la limite d'appels d'un hébergeur : la dernière liste sert jusqu'à sa remise à zéro. Une liste qui échoue est laissée de côté sans message : la vue « Pull requests » dit pourquoi, et les lignes des tickets et des agents restent.

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
- La PR d'un agent ordinaire est celle qu'il a lue en dernier, quand il était affiché.
- Sur GitLab, la liste ne dit ni la CI ni les approbations : une MR de la liste qu'aucun ticket ni agent ne tient n'y est à corriger que pour un conflit, et jamais à merger.
- 30 PR au plus par projet, les plus récemment mises à jour.
