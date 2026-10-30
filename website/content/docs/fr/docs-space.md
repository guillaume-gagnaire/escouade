# L’espace Docs

L’onglet Docs tient la spécification du produit, rangée par sujet, avec ce qui la sépare du code ; ses décisions et ses documents. Tout vit dans le dépôt, et tu le partages avec l’équipe d’un clic.

## L’onglet

La barre latérale a trois onglets : « Agents », « Kanban » et « Docs ». Sous « Docs », « Nouvelle discussion », [les discussions](#les-discussions), puis trois espaces :

- **Spécification** : les pages de la spec, avec le nombre d’écarts avec le code.
- **Décisions** : le registre des décisions sur le produit, numérotées et datées.
- **Documents** : les fichiers déposés pour le projet (cahiers des charges, comptes rendus, maquettes).

Tout est écrit sous `.escouade/` de la [branche des docs](#la-branche-des-docs) : `.escouade/spec/<sujet>/<page>.md`, `.escouade/decisions/`, `.escouade/documents/`. Ce sont des fichiers Markdown que tu peux aussi lire et modifier à la main.

## La spécification

L’espace s’ouvre sur **la carte des sujets** : une carte par sujet, avec son état face au code.

- « À jour » : la dernière comparaison au code n’a rien trouvé.
- « 1 écart » : une page du sujet dit autre chose que le code.
- « Brouillon » : toutes ses pages sont encore en brouillon.

Chaque carte dit aussi combien le sujet a de pages, de tickets et de décisions. La dernière carte, « Ce qui a bougé », donne les derniers changements partagés de la spec, et les pages modifiées ici, pas encore partagées. En tête de l’espace : les pages à jour, les écarts, les brouillons, et quand la spec a été comparée au code pour la dernière fois.

À gauche, **l’arbre des pages**, rangé sujet par sujet comme dans un wiki : un clic sur un sujet le replie ou le déplie, un clic sur une page l’ouvre. Une page marquée « écart » dit autre chose que le code ; « à partager », elle a été modifiée ici et pas encore partagée.

« Nouvelle page » crée une page : un sujet (« Paiement ») et un titre (« En plusieurs fois »). Elle s’ouvre prête à être écrite.

## Une page

Une page montre son texte Markdown mis en forme et, à côté, ses sources (les documents, fichiers ou liens d’où elle vient), ses tickets et ses décisions, et la date de sa dernière comparaison au code.

Quand la comparaison a trouvé **des écarts avec le code**, ils sont en tête de la page, chacun avec ce qui diffère et les fichiers concernés :

- « Mettre la spec à jour » ouvre une [discussion](#les-discussions) avec l’agent rédacteur, à qui l’écart est dit : il lit le code et propose la page qui dit ce que fait le code ; versée dans la spec, l’écart disparaît.
- « Ouvrir un ticket pour le code » ouvre la fenêtre d’un nouveau ticket, son titre et sa description déjà écrits depuis l’écart : c’est le code qui doit suivre la spec.
- « Voir le code » ouvre dans l’[éditeur](editor) le fichier que l’écart nomme.

« Modifier » ouvre la page dans un éditeur simple : son titre et son texte. « Enregistrer » écrit le fichier, son en-tête (sources, tickets, décisions, écarts) gardé tel quel. Avec la branche courante, « Ouvrir dans l’éditeur » ouvre le fichier de la page dans l’éditeur d’Escouade.

## Les discussions

« Nouvelle discussion », en tête de l’onglet (Ctrl+B), fait parler de la spec avec un agent. On choisit l’un des deux :

- **Discussion** : l’agent rédacteur de spec, pour interroger la spec ou la mettre à jour simplement (« Que dit la spec sur le paiement en plusieurs fois ? », « Ajoute que le reçu fiscal part par e-mail »).
- **Brainstorm** : un agent produit, à la manière de BMAD, qui fait mûrir une idée : usages et parcours, découverte, conformité et RGPD, ce que font d’autres secteurs. Il propose des fonctionnalités, des pages de spec, des décisions, et des tickets, qu’il dit sans les créer.

On donne un sujet. Inutile de choisir des documents : l’agent les a tous, par l’index. On peut **joindre de nouveaux fichiers** : ils sont rangés et indexés seuls dans les Documents. Le pied de la fenêtre montre le harness, le compte et le modèle de l’agent, ceux du projet par défaut, et laisse les changer.

Ce sont des agents d’Escouade comme les autres : la même conversation, et leurs questions se répondent aussi depuis la liste des agents. Ils travaillent dans le dossier de la branche des docs, lisent le code sans jamais le modifier et n’écrivent que les docs, par les outils de docs d’Escouade. Leur harness les en empêche lui-même, dans tous les modes : Claude Code n’a plus ses outils qui écrivent, Codex lit seulement dans son bac à sable, OpenCode, Copilot CLI et Antigravity CLI voient leurs écritures et leurs commandes refusées. Cursor CLI et Mistral Vibe modifient les fichiers sans passer par Escouade : une discussion ne part pas sur eux, et la fenêtre le dit.

Les discussions sont en cartes sous « Nouvelle discussion », chacune marquée « Brainstorm » ou « Discussion », avec son état : « Question », « Répond », « Brouillon » (des changements proposés pas encore versés), « Versé ».

Une discussion ouverte montre la conversation à gauche, et à droite **« Ce qui ira dans la spec »** : les changements proposés page par page (une section nouvelle, les phrases retirées barrées et les nouvelles en vert) et les décisions proposées. Une croix retire une proposition. **« Verser dans la spec »** écrit les pages sur la branche des docs, et les décisions dans le registre, proposées : elles s’acceptent d’un clic dans les Décisions. Rien n’est partagé : la barre d’état dit combien de changements attendent « Partager ».

« Demander à la spec », en tête de la spécification, ouvre une discussion avec l’agent rédacteur. Un projet sans spec propose « Écrire une première spec depuis le code » : l’agent rédacteur lit le code et propose une première version, rangée par sujet (cela coûte des tokens, d’où le geste).

## Planifier : de la spec aux tickets

Quand la spec a changé, **« Planifier »** demande à un agent planificateur les tickets qui la mettront en œuvre. Le bouton est sur une discussion dont les changements ont été versés, et en tête de la spécification tant que des changements n’ont pas de tickets (« 3 changements sans tickets · Planifier ») : les pages versées ou écrites à la main, et les décisions proposées, depuis la dernière planification.

Le planificateur lit ces changements et ces décisions, le code (les fichiers que l’index du code en rapproche, et le dossier du projet, sans rien modifier) et les tickets du Kanban. Il crée les tickets dans « À faire », chacun avec sa description, ses critères d’acceptation, les tickets qu’il suit (« Après ») et les pages et décisions qu’il implémente, que sa description cite pour son agent. Un ticket que le Kanban a déjà, même terminé, n’est pas recréé : il est dit « Déjà au Kanban ». Les tickets ne sont pas lancés par le planificateur : le pilote auto et les règles du Kanban s’appliquent comme pour tout ticket.

Ce qu’il a créé s’affiche sous la discussion, ou sur la carte des sujets, avec **« Annuler »** : les tickets pas encore lancés sont retirés, et les changements sont de nouveau à planifier. La carte d’un sujet montre les clés de ses tickets, et la carte d’un ticket au Kanban les pages qu’il implémente, qui s’ouvrent d’un clic dans l’onglet Docs.

## La spec toujours à jour

La spec suit le code de deux façons.

- **Les agents des tickets la tiennent.** La consigne de chaque ticket demande à son agent, quand le ticket change le comportement du produit, de mettre à jour les pages qu’il touche et de laisser une décision qui dit ce qui a changé et pourquoi, avant de le rendre. Rien ne bloque la validation : c’est à la revue de le voir. Avec la branche courante, ces changements partent dans la branche du ticket et se fusionnent avec son code. Avec une [branche fixe](#la-branche-des-docs), ils attendent dans une copie tenue par Escouade, et ne sont versés sur la branche des docs **qu’à la validation du ticket** : la spec ne décrit jamais un comportement pas encore fusionné. Ce que l’équipe a écrit entre-temps sur la branche reste.
- **La comparaison au code.** Après chaque ticket validé et fusionné, un agent court lit le code, sans rien modifier, et compare les pages que le ticket touche : celles qu’il a modifiées, celles qui le citent, et celles qui parlent de ses fichiers ou de leurs symboles (par l’[index du code](code-index) s’il est allumé). Ce qu’il relève s’affiche sur la carte des sujets (« 1 écart », « comparé au code il y a 2 h ») et en haut de la page. Escouade le garde sur ce poste, pas dans la page : une comparaison ne laisse rien à commiter.

« Comparer au code » le fait à la demande : en tête de la spécification pour toutes les pages (40 au plus, les moins récemment comparées d’abord), ou en tête d’une page pour elle seule. Chaque page comparée coûte des tokens, sur le compte et le harness de « Modèle des textes rédigés ».

## Les décisions

L’espace Décisions est un registre, pas un document : une ligne par décision sur le produit, la plus récente en tête, avec son numéro, son titre, d’où elle vient (un brainstorm, une discussion, un ticket, un document), son état, ce qu’elle a changé dans la spec et sa date. En tête : combien de décisions sont acceptées, proposées et remplacées. Le menu « Toutes » filtre le registre par état.

Un clic sur une ligne ouvre son détail à droite : le contexte, la décision, ses conséquences, les pages de la spec qu’elle a changées (avec les lignes ajoutées et retirées ; un clic ouvre la page), ses liens (tickets, origine, documents), et ce qu’elle remplace ou ce qui la remplace.

- **Proposée** : un agent, ou quelqu’un de l’équipe, la propose ; elle ne vaut décision qu’une fois acceptée. « Accepter », sur sa ligne ou dans son détail, l’accepte d’un clic, à ton nom.
- **Acceptée** : elle ne se réécrit plus. Pour revenir dessus, « La remplacer » propose une nouvelle décision qui la remplace.
- **Remplacée** : la nouvelle décision a été acceptée ; l’ancienne reste dans le registre, telle quelle, et dit par laquelle.

« Proposer une décision » en écrit une à la main : son titre, son contexte, la décision, ses conséquences, et la décision acceptée qu’elle remplace s’il y en a une.

Les agents proposent leurs décisions par le [serveur MCP](mcp). Celle que laisse l’agent d’un ticket est acceptée quand tu valides le ticket : elle part dans le même commit que son code. Chaque décision est un fichier de `.escouade/decisions/`, numéroté (`0014-tarif-famille.md`), qui se partage avec les autres docs.

## Importer une spec existante

Un projet sans spec part d’un espace vide. Si Escouade trouve une spec dans le projet (un `SPEC.md` à la racine ou dans `docs/`, un dossier `spec` ou `specs`, un dossier `docs/` de fichiers Markdown), il la propose :

1. « Voir le découpage » montre comment Escouade la découperait, par ses titres : un sujet par grand titre, une page par section ; dans un dossier, un sujet par fichier. Un titre seul en tête du fichier, le nom du document, devient le titre de l’introduction.
2. Décoche les pages que tu ne veux pas.
3. « Importer » écrit les pages. Chacune cite le fichier d’où elle vient et garde sa place dans l’ordre de la spec d’origine.

Rien n’est écrit avant « Importer », et une page qui existe déjà reste telle quelle. Le fichier d’origine n’est ni modifié ni supprimé : c’est à toi de le retirer du dépôt si tu le veux.

Escouade retient les chemins importés avec les docs, sans doublon, et les partage avec l’équipe. Dès que le projet a une spec dans Escouade, tout agent qu’Escouade lance dessus (ticket, discussion, planificateur…), quel que soit son harness, est prévenu qu’elle et les décisions du projet sont dans Escouade et sont les seules à suivre ; l’ancien fichier est nommé : il ne se lit plus comme la spec et ne se met plus à jour.

## Les documents

L’espace Documents est le drive du projet : les cahiers des charges, comptes rendus, maquettes et pièces juridiques, rangés en dossiers sous `.escouade/documents/`.

- À gauche, **les dossiers**, avec le nombre de fichiers de chacun ; « Tous les documents » les montre tous. Un dossier qui en contient d’autres se déplie.
- À droite, **les fichiers** du dossier ouvert, chacun avec qui l’a ajouté, sa taille, sa date, et ce que les agents en ont fait : « ✓ lu par les agents » dès qu’un agent l’a lu, « cité 3 fois » quand des pages de la spec ou des décisions le citent.
- Un clic sur un fichier montre le texte qu’Escouade en a lu.

**Déposer des fichiers.** Glisse des fichiers sur l’espace, ou « Déposer des fichiers ». Chaque fichier est **lu et indexé sur ta machine** : rien ne part sur Internet. Escouade lit le texte des PDF, des fichiers Word (`.docx`) et OpenDocument (`.odt`), du Markdown et du texte ; d’une image, il ne garde que son format et sa taille. Le texte lu reste dans le dossier d’Escouade, jamais dans le dépôt ; le fichier, lui, va dans le dépôt, sur la branche des docs.

**Le rangement.** Un fichier déposé dans un dossier ouvert y reste. Déposé sur « Tous les documents », Escouade le range seul, dans cet ordre :

1. le dossier dont son nom ou son titre porte le nom (« Compte rendu atelier » va dans « Comptes rendus ») ;
2. le dossier de sa nature, si le drive en a un : un compte rendu, un cahier des charges, une maquette, une pièce juridique, un devis ou une facture, une présentation, un planning ;
3. le dossier dont les fichiers lui ressemblent le plus par le sens, quand le [modèle des vecteurs](shared-memory) est téléchargé, puis par les mots ;
4. sinon, un **nouveau dossier**, créé pour lui : celui de sa nature (« Planning »), « Images » pour une image, ou « Divers ».

Escouade dit où il l’a rangé et pourquoi ; « Déplacer vers » le change de dossier. Tu peux aussi glisser une carte sur un dossier de gauche. « Nouveau dossier » crée un dossier dans celui qui est ouvert.

**Rechercher dans les documents** cherche dans les noms et le texte de tous les documents, par le sens et les mots quand le modèle des vecteurs est là, par les mots sinon, et montre le passage qui répond.

**Les fichiers lourds.** Si le dépôt a des règles Git LFS (`.gitattributes`) qui prennent le fichier, elles s’appliquent : rien à faire. Sinon, au-delà de 50 Mo, Escouade prévient avant de l’ajouter, parce que git garde chaque version en entier et que chacun de l’équipe la télécharge : « Ajouter quand même » ou « Ne pas les ajouter ».

**Les agents** cherchent et lisent les documents par les outils d’Escouade (`search_documents`, `read_document`), quel que soit leur harness, et citent ceux d’où vient ce qu’ils écrivent dans les sources d’une page de la spec. Les documents d’un projet privé ne sont vus que de ses propres agents.

## Partager les docs

Tant que l’onglet Docs est ouvert, ou que les docs ont des changements, la barre d’état dit « Docs sur la branche main · 2 changements à partager ». Les changements sont les fichiers de `.escouade/spec`, `decisions` et `documents` modifiés et pas encore envoyés.

« Partager » récupère d’abord ce que les autres ont partagé, puis enregistre (commit) les fichiers des docs, et eux seuls, et les envoie (push). Le code que tu modifies en même temps n’est pas touché.

- Si la branche a aussi des commits de code pas encore envoyés, Escouade te le dit avant : « Tout partager » les envoie avec les docs, « Annuler » ne fait rien.
- Si deux personnes ont modifié les mêmes pages, la barre d’état les nomme : « L’agent résout » confie le conflit à un agent rédacteur, qui garde ce que chacun voulait dire ; « Annuler » remet les docs comme avant. Tu partages à nouveau une fois qu’il a fini.

> [!NOTE]
> Escouade ne partage jamais seul : rien n’est commité ni poussé sans un clic sur « Partager ».

## La branche des docs

Réglages › Projets › Git, « Branche des docs » :

- **La branche courante** (par défaut) : les docs vivent dans la branche de chaque dossier. Celles d’un ticket suivent sa branche et se fusionnent avec son code.
- **Une branche fixe** (`docs`, par exemple) : les docs vivent toujours sur cette branche, qu’Escouade tient dans un worktree à part, dans son dossier système. Escouade la crée au besoin, depuis la branche distante si elle existe, et ne la fusionne jamais ailleurs : c’est à l’équipe de le faire.
