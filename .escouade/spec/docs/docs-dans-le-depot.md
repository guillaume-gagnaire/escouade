---
title: "Docs du projet"
subject: "Docs"
sources: ["docs/SPEC.md"]
tickets: ["ESC-256","ESC-257","ESC-258"]
decisions: []
compared: null
---

## Ce que c'est

Les docs d'un projet sont sa spécification, ses décisions et ses documents. Ce sont des fichiers sous `.escouade/` de la branche des docs, que l'équipe partage par git. Escouade les écrit, mais ne les commite ni ne les pousse jamais d'elle-même : tout part d'un geste de l'utilisateur, « Partager ». Celui qui ne voit jamais git ne voit que « N changements à partager » et « Partager ».

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

Les fichiers des docs (`.escouade/spec`, `decisions` et `documents` ; ni les tickets ni le code) changés et pas commités, plus ceux des commits pas encore poussés (comparés à la branche distante suivie, ou à `origin/<branche>` si elle existe). Le compte se met à jour à chaque écriture des docs, par la fenêtre ou par un agent, et à chaque partage.

### « Partager »

Un partage à la fois, en quatre temps :

1. il récupère d'abord la branche distante (`git fetch`, avec les identifiants que git a déjà, comme le pull et le push du panneau Git) ;
2. il commite seulement les fichiers des docs, le reste de l'index et du dossier laissé tel quel, avec le message donné ou un message qui dit ce que c'est (« Docs : 2 pages de la spec, 1 décision, 1 document ») ;
3. il fusionne ce que les autres ont partagé ;
4. il pousse, en publiant la branche si elle ne l'est pas encore (la branche fixe, la première fois).

**Le code ne part pas avec les docs sans confirmation.** Si la branche a aussi des commits de code pas encore poussés, « Partager » commite les docs en local (rien ne sort), puis refuse de fusionner et de pousser : « Cette branche a aussi N commits de code pas encore partagés : « Partager » les publierait avec les docs. Confirme pour tout partager, ou partage le code depuis le panneau Git. ». « Tout partager » publie tout.

### Un conflit

Deux personnes ont touché la même page : le merge est défait aussitôt, le dépôt distant et les fichiers de l'utilisateur intacts. La fenêtre nomme les pages, jamais en lignes de git, et propose :

- « L'agent résout » : le merge est refait dans le dossier des docs, conflits laissés dans les fichiers. Un agent rédacteur (`redacteur-docs`, sur le harness et le compte par défaut du projet) les résout en gardant les deux sens, retire les marques et commite le merge sans pousser. L'utilisateur partage ensuite de nouveau.
- « Annuler » : le merge est défait et le conflit oublié ; les docs restent à partager.

Tant que le merge n'est pas fini, « Partager » dit en une phrase qu'il faut attendre l'agent ou annuler.

### Les erreurs

Elles se disent en phrase simple, jamais la sortie de git en tête : identifiants refusés (« Escouade n'a pas pu s'identifier auprès du dépôt distant… »), dépôt qui ne répond pas, pas de dépôt distant, HEAD détachée, modifications du code qui gênent la récupération.

### Dans la barre d'état

« Docs sur la branche X · N changements à partager » et « Partager », sur l'onglet Docs, et ailleurs tant que les docs ont des changements ou un conflit. X est la branche fixe, sinon celle du dossier du projet. « Partager » relit d'abord ce qui reste à partager : des commits de code pas encore poussés font demander « Tout partager ». Un conflit y nomme les pages, avec « L'agent résout » et « Annuler », puis « L'agent rédacteur résout le conflit ».

### Pour les agents (MCP)

- `get_docs_to_share` : ce qui reste à partager. En lecture, ouvert à tous pour les projets qu'ils voient.
- `share_docs` : partage. **Partager est un geste humain** : un agent d'Escouade est refusé, sauf si l'utilisateur le lui a demandé et qu'il le dit (`userAskedForIt: true`). Claude hors d'Escouade, lancé par l'utilisateur, partage comme la fenêtre. `withCode: true` publie aussi le code, comme « Tout partager ».

### Sécurité

- Aucun chemin des docs ne sort du projet : ni `.escouade` ni son dossier ne peuvent être des liens, aucun lien symbolique sur le chemin d'une page ou d'un document, ni `..`, ni lettre de lecteur, ni chemin absolu.
- Un projet privé ne dit rien de ses docs aux agents des autres projets (voir *Serveur MCP*).
- Le texte d'une page ou d'un document n'entre jamais dans le journal d'activité.

## Réglages

- Réglages › Projets › Git › « Branche des docs » : « La branche courante » (par défaut) ou « Une branche fixe » et son nom (`docs` proposé), vérifié à l'enregistrement.

## Limites

- « L'agent résout » et « Annuler » n'ont pas d'outil MCP : seule la fenêtre les propose.
