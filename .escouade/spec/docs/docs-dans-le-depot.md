---
title: "Docs du projet"
subject: "Docs"
sources: ["docs/SPEC.md"]
tickets: ["ESC-256","ESC-257","ESC-258","ESC-534","ESC-643"]
decisions: [245]
compared: "2026-10-30T05:35:38+01:00"
gaps: [{"text":"La page dit que « Partager » est dans la barre d'état de l'onglet Docs, et que les autres onglets ne l'affichent que tant que les docs ont des changements ou un conflit. Dans le code, le bouton « Partager » n'apparaît que si le compte est supérieur à 0, y compris sur l'onglet Docs. Il y est donc absent quand il n'y a rien à partager.","code":["src/components/docs/DocsShareItem.svelte"]}]
---

## Ce que c'est

Les docs d'un projet sont sa spécification, ses décisions et ses documents. Ce sont des fichiers sous `.escouade/` de la branche des docs, que l'équipe partage par git. Escouade les écrit, mais ne les commite ni ne les pousse jamais d'elle-même : tout part d'un geste de l'utilisateur, « Synchroniser ». Celui qui ne voit jamais git ne voit que le badge de l'onglet « Docs » (« N changements des docs à partager ») et « Synchroniser ».

## Comportement

### Les fichiers

- `.escouade/spec/<sujet>/<page>.md` : les pages de la spec (voir *Spécification*).
- `.escouade/decisions/<NNNN>-<titre>.md` : une décision par fichier (voir *Spécification*).
- `.escouade/documents/<dossiers>/<fichier>` : les fichiers déposés (voir *Documents*).

Ce qu'Escouade tire de ces fichiers (texte extrait, lectures, index) vit dans son dossier de données, jamais dans le dépôt.

### La branche des docs

Elle se règle par projet.

- **La branche courante** (par défaut) : les docs sont là où travaille celui qui les lit ou les écrit. Un agent écrit dans son propre dossier : la spec d'un ticket suit sa branche et se fusionne avec son code. La fenêtre et Claude hors d'Escouade utilisent le dossier du projet.
- **Une branche fixe** : les docs vivent dans un worktree à elles, tenu par Escouade dans son dossier de données, ou là où git a déjà cette branche. La branche est créée au besoin, depuis `origin/<nom>` si elle y est, sinon depuis HEAD. Escouade ne la fusionne jamais ailleurs.

### Ce qui reste à partager

Les fichiers des docs (`.escouade/spec`, `decisions` et `documents` ; ni les tickets ni le code) changés et pas commités, plus ceux des commits pas encore poussés (comparés à la branche distante suivie, ou à `origin/<branche>` si elle existe). Le compte se lit quand le projet s'affiche, puis se met à jour à chaque écriture des docs, par la fenêtre ou par un agent, et à chaque synchronisation.

### Où se lit l'état

**Plus rien des docs dans la barre d'état.**

- **Le badge de l'onglet « Docs »** de la barre latérale (Agents · Kanban · PR · Docs), sur n'importe quel onglet : le nombre de changements à partager, dans le même badge que ceux des autres onglets (même place, même taille, même couleur d'attente) ; un conflit qui attend, l'agent rédacteur en train de le résoudre compris, en rouge avec « ! ». Pas de badge quand tout est partagé. Son infobulle dit « 3 changements des docs à partager » ou « Conflit dans les docs » ; le nom accessible de l'onglet le reprend (« Docs, 3 changements des docs à partager », « Docs, conflit dans les docs »).
- **L'en-tête de chaque espace des docs** (Spécification, Décisions, Documents, une discussion), à gauche de ses propres boutons : la branche des docs (X, la branche fixe, sinon celle du dossier du projet ; son infobulle « Docs sur la branche X »), ce qui reste à partager (« 2 à partager », ou « à jour ») et le bouton « Synchroniser », toujours là. Un conflit prend sa place : « Conflit dans les docs » (son infobulle nomme les pages), avec « L'agent résout » et « Annuler », puis « L'agent rédacteur résout le conflit ».

### « Synchroniser »

Une synchronisation à la fois (avec « L'agent résout » et « Annuler ») : pendant qu'elle tourne, le bouton dit « Synchronisation… » et ne se relance pas. Elle relit d'abord ce qui reste à partager.

**Avec des changements des docs à partager**, c'est le partage, en quatre temps :

1. il récupère d'abord la branche distante (`git fetch`, avec les identifiants que git a déjà, comme le pull et le push du panneau Git) ;
2. il commite seulement les fichiers des docs, le reste de l'index et du dossier laissé tel quel, avec le message donné ou un message qui dit ce que c'est (« Docs : 2 pages de la spec, 1 décision, 1 document ») ;
3. il fusionne ce que les autres ont partagé ;
4. il pousse, en publiant la branche si elle ne l'est pas encore (la branche fixe, la première fois).

Il dit « N changements des docs partagés. », et « M changements récupérés. » quand les autres en avaient poussé.

**Sans rien des docs à partager**, il récupère seulement : `git fetch`, puis la fusion de la branche distante dans la branche des docs (en avance rapide quand la branche n'a rien à elle, sinon un commit de fusion), sans rien commiter ni pousser, et sans demander de confirmation pour le code pas encore poussé, qui ne part pas. Il dit « Docs à jour. » quand rien n'est venu, sinon « 2 changements récupérés. » (les fichiers des docs que la fusion a changés). Une branche pas encore publiée n'a rien à récupérer : « Docs à jour. ». Un conflit s'arrête et se défait comme pour le partage.

**Le code ne part pas avec les docs sans confirmation.** Si la branche a aussi des commits de code pas encore poussés et des docs à partager, la fenêtre demande d'abord : « Cette branche a aussi N commits de code pas encore partagés : la synchronisation les publierait avec les docs. Tout partager ? ». « Tout partager » publie tout. Appelé sans confirmation (l'outil MCP), le partage commite les docs en local (rien ne sort), puis refuse de fusionner et de pousser avec la même phrase.

### Un conflit

Deux personnes ont touché la même page : le merge est défait aussitôt, le dépôt distant et les fichiers de l'utilisateur intacts. La fenêtre nomme les pages, jamais en lignes de git, et propose :

- « L'agent résout » : le merge est refait dans le dossier des docs, conflits laissés dans les fichiers. Un agent rédacteur (`redacteur-docs`, sur le harness et le compte par défaut du projet) les résout en gardant les deux sens, retire les marques et commite le merge sans pousser. L'utilisateur synchronise ensuite de nouveau.
- « Annuler » : le merge est défait et le conflit oublié ; les docs restent à partager.

Tant que le conflit attend son geste, une nouvelle synchronisation le dit en une phrase ; tant que le merge de l'agent n'est pas fini, elle dit qu'il faut attendre l'agent ou annuler.

### Les erreurs

Elles se disent en phrase simple, jamais la sortie de git en tête : identifiants refusés (« Escouade n'a pas pu s'identifier auprès du dépôt distant… »), dépôt qui ne répond pas, pas de dépôt distant, HEAD détachée, modifications du code qui gênent la récupération.

### Pour les agents (MCP)

- `get_docs_to_share` : ce qui reste à partager. En lecture, ouvert à tous pour les projets qu'ils voient.
- `share_docs` : partage. **Partager est un geste humain** : un agent d'Escouade est refusé, sauf si l'utilisateur le lui a demandé et qu'il le dit (`userAskedForIt: true`). Claude hors d'Escouade, lancé par l'utilisateur, partage comme la fenêtre. `withCode: true` publie aussi le code, comme « Tout partager ». Sa réponse dit aussi combien de fichiers des docs ont été récupérés (`received`).

### Sécurité

- Aucun chemin des docs ne sort du projet : ni `.escouade` ni son dossier ne peuvent être des liens, aucun lien symbolique sur le chemin d'une page ou d'un document, ni `..`, ni lettre de lecteur, ni chemin absolu.
- Cette garde tient seule, quel que soit l'appelant, au plus près de l'accès aux fichiers. Avant de toucher au disque, elle refuse tout segment vide, `.` ou `..`, absolu, avec un lecteur (`C:`), un préfixe UNC ou de périphérique, un `\`, un `:` (lecteur ou flux NTFS), un caractère de contrôle, une fin en point ou en espace (que Windows retire), ou un nom réservé de Windows (`CON`, `NUL`, `COM1`…, quelle que soit l'extension). Le refus dit « chemin hors du dossier » et sa raison.
- Le chemin est ensuite vérifié sous le dossier, lexicalement en entier, et sur le disque pour la partie qui existe déjà : un dossier pas encore créé ne saute aucun contrôle. Après la création des dossiers, l'écriture vérifie de nouveau qu'aucun n'est un lien et que le fichier reste sous `.escouade`.
- Un projet privé ne dit rien de ses docs aux agents des autres projets (voir *Serveur MCP*).
- Le texte d'une page ou d'un document n'entre jamais dans le journal d'activité.

## Réglages

- Réglages › Projets › Git › « Branche des docs » : « La branche courante » (par défaut) ou « Une branche fixe » et son nom (`docs` proposé), vérifié à l'enregistrement.

## Limites

- « L'agent résout », « Annuler » et la récupération seule n'ont pas d'outil MCP : seule la fenêtre les propose.
- La récupération fusionne la branche distante entière, comme le partage : sur la branche courante, le code que les autres y ont poussé vient avec les docs.
