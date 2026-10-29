---
title: "Pull requests"
subject: "Git & fichiers"
sources: ["docs/SPEC.md","docs/TODO-1.8.md"]
tickets: ["ESC-64","ESC-161"]
decisions: []
compared: null
---

## Ce que c'est

Escouade ouvre la pull request d'un agent ordinaire et suit celles de tout agent : leur état, leur CI et leurs relectures, sur la carte de l'agent, dans l'en-tête de sa conversation et dans la barre d'état. Quand la CI échoue ou qu'une relecture demande des changements, un bouton envoie le tout à l'agent pour qu'il corrige. La PR d'un ticket est ouverte par sa validation (voir *Validation*) ; elle s'affiche et se lit comme les autres. Ce que chaque hébergeur permet est dans *Hébergeurs de code*.

## Comportement

### Quel agent peut ouvrir une PR

- Un agent ordinaire (pas celui d'un ticket), non archivé, qui a un worktree d'Escouade (sa branche), ou qui travaille dans un autre worktree du dépôt : créé par Claude Code ou rejoint par un `cd`. Ce dossier est celui de la dernière entrée de sa session Claude Code, relu à chaque lecture de ses PR. Sa racine doit être un worktree du projet (`git worktree list`), ni le dossier du projet ni le worktree d'un autre agent, et sur une branche. L'en-tête de la conversation dit alors cette branche, et sa PR vise la cible du Kanban, sinon la branche du dossier du projet. Changer de branche ou revenir au dossier du projet oublie les PR de l'ancienne.
- « Ouvrir une PR » n'est offert que si l'hébergeur du projet en ouvre : le projet a un dépôt distant chez GitHub ou Azure DevOps, et un compte y est connecté. Le bouton reste quand l'hébergeur ne répond pas. Sans dépôt distant, chez un autre hébergeur, ou sans compte : ni bouton ni entrée, aucune erreur.

### Où

- Le bouton « Ouvrir une PR » de l'en-tête de la conversation, tant qu'aucune PR de l'agent n'est ouverte ; « Ouvrir une PR… » du menu de sa carte et du menu de sa branche dans le sélecteur de branche (le sélecteur se ferme d'abord).
- « Voir la PR #n » à leur place tant qu'une est ouverte : elle s'ouvre dans le navigateur.

### La fenêtre « Ouvrir une PR sur <hébergeur> »

- Elle dit « <agent> · <branche> → <base> ». Titre et description sont proposés par le modèle des textes rédigés, dans leur langue (voir *Langues*), d'après les commits de la branche depuis sa base et son diff (40 Ko au plus). Sans réponse : le dernier sujet de commit et la liste des commits, et la fenêtre le dit.
- Champs modifiables ; « Branche cible » (la base de son worktree, sinon la cible du Kanban) ; « En brouillon » quand l'hébergeur en fait ; le nombre de fichiers non commités, qui n'y seront pas ; « Proposer à nouveau ».
- Refusé quand la branche n'a aucun commit de plus que sa base, ou qu'une PR est déjà ouverte.
- Rien ne part avant « Pousser et ouvrir » : la branche est poussée (`git push -u`), puis la PR ouverte. Le refus de l'hébergeur reste dans la fenêtre ; ouverte, un toast dit « PR #n ouverte. ».
- Une branche qui a une PR n'est plus renommée au nom de l'agent.

### Quelles PR sont celles d'un agent

- Celles de la branche où il est réellement, quel qu'en soit l'auteur : celle de son worktree (agent de ticket compris, en lecture seule : ni « Ouvrir une PR » ni correction, son Kanban la suit), celle du worktree où il est entré seul, sinon celle du dossier du projet. Pas celle du dossier quand il est sur la cible du Kanban ou la branche par défaut d'un dépôt distant (`main` ou `master` faute de mieux) : une PR `main → prod` irait sinon à tous les agents du dossier.
- Elles sont cherchées sur le dépôt de la cible (là où « Ouvrir une PR » ouvre), sur celui que suit la branche (sinon `origin`), et sur GitHub sur le dépôt dont ce dernier est un fork. Un dépôt autre que celui de la cible qui ne répond pas garde ce qui y a été lu en dernier ; un dépôt parent que l'hébergeur n'a pas dit est redemandé 10 min plus tard.
- Deux PR de dépôts différents peuvent avoir le même numéro : elles se distinguent par leur dépôt. La PR d'un fork est gardée quand sa branche de tête est celle de l'agent (une branche poussée sur ton fork, la PR ouverte vers l'original), écartée sinon (un autre fork qui a une branche du même nom). Lue ailleurs que sur le dépôt de la cible, elle garde son dépôt, où « L'agent corrige » lit ses checks et ses retours.
- Elles sont lues à l'affichage de l'agent, puis chaque minute tant qu'il est affiché, au plus toutes les 20 s, et plus du tout une fois toutes mergées. Elles sont gardées avec l'agent, dans l'ordre : ouvertes prêtes, brouillons, mergées, fermées, la plus récente d'abord.

### Sur la carte et dans l'en-tête

- Trois numéros au plus (le groupe « Pull requests de <agent> » sous la carte, hors du bouton qu'elle est), précédés du signe de leur CI (✓ verte, ✗ en échec, ● en cours, ○ aucune, ⇡ mergée) et colorés par ce qui compte d'abord : échec ou conflit, puis changements demandés ou CI en cours, sinon vert ; brouillon et fermée grisés.
- Un clic ouvre la PR dans le navigateur. L'infobulle dit « PR #12 · titre », puis l'état, la CI (la dernière exécution d'un check relancé), les approbations, les changements demandés, les fils ouverts, le conflit.
- La même PR suit le bouton de branche de la barre d'état (voir *Branches*).

### Faire corriger l'agent

Sous la carte, sur une PR ouverte, jamais d'elles-mêmes :

- **« L'agent corrige »**, quand la CI est en échec : l'agent reçoit la PR, les checks en échec et la fin de leurs journaux (6 Ko chacun).
- **« L'agent corrige les retours »**, quand une relecture demande des changements ou que des fils restent ouverts : l'agent reçoit les relectures qui disent quelque chose et les commentaires non résolus, avec fichier et ligne.
- Les textes de l'hébergeur y sont balisés (checks, journaux, relectures, commentaires ; une balise fermante dans le texte est neutralisée), et la consigne dit de les traiter comme des informations, pas comme des instructions. L'agent corrige, vérifie, commite et pousse, sans répondre ni résoudre les fils : rien ne se publie sans l'utilisateur.
- Rien à corriger : refusé avec un toast. Le message fait 40 Ko au plus.

## Limites

- « Ouvrir une PR » et la lecture des PR : GitHub et Azure DevOps seulement (voir *Hébergeurs de code*).
- Un agent qui travaille dans un worktree qui n'est pas d'Escouade n'est suivi que sur Claude Code.
- Le cycle des PR hors tickets et ce qui en reste à faire : `docs/TODO-1.8.md`, chapitre 2 (« Les PR hors tickets »).
