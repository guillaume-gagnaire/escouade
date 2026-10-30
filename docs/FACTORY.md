# La fabrique de produit : index local, mémoire partagée, onglet Docs

Conception du 2026-10-27, faite avec l'utilisateur, pour une 2.x. Les maquettes validées en chemin sont dans `design/factory/` (version 3 : brainstorm en cours, spécification, « Nouvelle discussion », décisions, documents). Ce document dit ce qui a été décidé par l'utilisateur, ce que le chef d'escouade a tranché (marqué **« choix du chef »**) et ce qui reste au constructeur.

## L'intention

Escouade devient une vraie **product factory** : de l'idée au produit livré, avec la documentation du produit comme source de vérité. Trois constats de l'utilisateur :

- **La spec ment sans que personne le sache.** Le dernier raté : une information était juste dans le code et fausse dans `SPEC.md`, et l'agent a cru la spec. Exigence de l'utilisateur : « la spec doit **toujours** être à jour ».
- **La MOA doit pouvoir s'en servir.** Une MOA (maîtrise d'ouvrage, non développeuse) ouvre Escouade pour brainstormer et enrichir la spec au fil du projet, et y déposer ses documents (cahiers des charges, comptes rendus, maquettes).
- **Les agents cherchent le code trop cher.** Ils lisent à coups de grep et de fichiers entiers ; ils doivent trouver le code plus vite et pour moins de tokens.
- **La réflexion se perd quand on change de harness.** Claude Code tient sa propre mémoire ; un agent qui passe sur Codex, pour une revue ou parce que le quota de Claude est épuisé, doit garder la même base de réflexion.

## Objectifs

1. Une spec toujours à jour avec le code, rangée par sujet, que tout le monde lit et enrichit dans Escouade.
2. Un espace pour brainstormer et discuter de la spec avec des agents spécialisés, d'où sort ce qui va dans la spec.
3. Un registre précis des décisions sur le produit.
4. Un drive des documents du projet, lus et cités par les agents.
5. Une mémoire de travail des agents, commune à tous les harness et à l'équipe.
6. Un index du code qui fait trouver le code aux agents plus vite et pour moins de tokens.

## Ce qui n'en fait pas partie

- **Pas de vue à part pour la MOA** : la même app, les mêmes écrans pour tout le monde (décidé).
- **Pas tokensave embarqué** : Escouade construit son propre index, en reprenant ses idées (décidé, voir plus bas).
- **Pas de service en ligne par défaut** pour analyser ou indexer : tout se fait sur la machine (décidé).
- **Pas de synchronisation avec Confluence, Google Drive ou Notion** dans cette version (choix du chef : le dépôt git est la seule source ; une source externe pourra venir par les plugins).

## Ce qui serait un échec, même si tout marche

- Une spec qui dit autre chose que le code sans que l'écran le montre.
- Escouade qui commite ou pousse seul, sans geste de l'utilisateur.
- Un document, un extrait de code ou une mémoire qui quitte la machine sans que ce soit dit (seuls les agents en reçoivent des extraits, comme ils reçoivent déjà du code).
- Un index qui emballe la machine (tokensave a retiré son guetteur de fichiers pour cette raison : processeur et mémoire en emballement sur de gros dépôts).
- Un projet privé (ESC-140, `fence.rs`) dont un fichier interdit serait indexé, ou montré à un agent d'un autre projet.

## Contraintes

- **Tout vit dans le dépôt git, sous `.escouade/`** : la spec, les décisions, les documents déposés (même lourds) et la mémoire (décidé). Ce qui se recalcule (l'index, les vecteurs, le texte extrait des documents) vit dans le dossier système d'Escouade, à côté des historiques (choix du chef : rien de dérivé dans le dépôt).
- **La branche des docs** se règle par projet : la branche courante, ou une branche fixe (décidé). La MOA fait du git **à travers Escouade** : elle ne voit jamais git, seulement « N changements à partager » et « Partager » (commit et push, au geste de l'utilisateur).
  - **Choix du chef** : une branche fixe a son propre worktree, tenu par Escouade ; Escouade ne la fusionne jamais ailleurs (c'est à l'équipe de le faire, ou c'est `main` elle-même).
  - **Choix du chef** : « Partager » récupère d'abord la branche distante ; si deux personnes ont touché la même page, le conflit va à l'agent rédacteur, comme un conflit de ticket (« L'agent résout » / « Annuler »), jamais à la MOA en lignes de git.
  - **Choix du chef** : une MOA qui n'a pas le dépôt l'obtient par « Nouveau projet », en le clonant depuis son hébergeur ; si l'assistant ne sait pas encore cloner, c'est un ticket du chantier.
  - **Choix du chef** : les documents lourds suivent les règles LFS du dépôt s'il en a ; sinon, au-delà de 50 Mo, la fenêtre prévient avant d'ajouter le fichier.
- **Tous les harness** (Claude Code, Codex, Copilot CLI, Cursor CLI, OpenCode) passent par le serveur MCP d'Escouade pour l'index, la mémoire et les docs.
- **Les règles existantes tiennent** : `.escouade/` n'est jamais lu ni écrit à travers un lien symbolique ni hors du projet (`workflowfiles::unsafe_dir`, `sharing.rs`) ; Escouade ne commite jamais seul les fichiers partagés ; les projets privés gardent leurs fichiers interdits hors de tout index.
- **Ce qui existe déjà** : ONNX Runtime est embarqué (`ort` 2.0.0-rc.13, chargé à l'exécution, pour les voix) ; SQLite aussi (`rusqlite`, bundled) ; les hooks d'Escouade sont posés dans les quatre harness à hooks ; le protocole des tickets passe par `board::protocol_prompt_for`.

## L'ordre de construction (décidé)

1. **Le socle d'index local** : la structure du code, puis les vecteurs.
2. **La mémoire partagée**, petite et utile tout de suite.
3. **L'onglet Docs**, le plus gros, qui s'appuie sur les deux.

## 1. Le socle d'index local

### Le code : la structure d'abord (décidé)

Escouade construit son propre index, à la manière de tokensave (Rust, MIT, regardé le 2026-10-27) :

- **tree-sitter** découpe le code en symboles et en liens : fonctions, classes, méthodes, champs, imports, appels, héritage, docstrings. Les titres Markdown deviennent des nœuds.
- Le tout dans une base SQLite par projet, avec recherche plein texte (FTS5).
- Le gain vient de la structure, pas des vecteurs : en **un appel**, l'agent obtient le contexte d'une tâche (points d'entrée, symboles liés, extraits), les appelants et appelés d'une fonction, ou ce qu'un changement toucherait, au lieu de lire 20 à 50 fichiers. tokensave annonce 88 % de tokens en moins en moyenne sur 10 requêtes de son dépôt (ses chiffres, face à des lectures de fichiers entiers).
- **Pas de vecteurs pour le code** (décidé le 2026-10-29, après les sondes d'ESC-294) : mesurés, ils trouvaient moins que la méthode de tokensave, qui les a retirés elle aussi au profit de `keywords`. Escouade porte cette méthode **à l'identique** (tokensave au commit `580936b`, MIT, `THIRD-PARTY-NOTICES.md`) : l'index plein texte et le classement (ESC-296), les appels résolus d'un symbole à l'autre (ESC-297), les réponses, le compte de l'économie et les consignes (ESC-298). Les sondes jugent chaque étape (`docs/CODEINDEX-PROBES.md`).

Ce qu'en voient les agents : six outils MCP, sur le modèle des outils de base de tokensave : `index_search` (un symbole, ou un texte exact avec `literal`), `index_context` (le contexte d'une tâche, en Markdown, cinq blocs de code de 1 500 octets au plus), `index_body` (le corps d'un symbole, que nomme un bloc coupé), `index_callers`, `index_callees` et `index_impact`. Chacun dit dans sa réponse ce qu'il a fait économiser (`index_metrics`, comme `tokensave_metrics`), compté comme tokensave : les fichiers touchés plafonnés à quatre fois la réponse, contre la réponse, la requête et une fois par session les schémas des outils, un déficit reporté sur les appels suivants ; les statistiques d'Escouade suivent ce compte. Les descriptions des outils ne changent pas d'un appel à l'autre (le cache de prompt) et demandent les mots du code, puis `keywords` quand un terme ne trouve rien. `.escouade/queryignore` tient les chemins laissés hors des recherches.

**Les orienter vers l'index** : les hooks d'Escouade renvoient vers l'index les recherches qui visent un symbole (grep, rg) là où le harness le permet ; ailleurs, une règle dans la consigne de l'agent. **Jamais de refus** : à la différence de tokensave, qui refuse certains grep et les agents Explore, Escouade conseille et laisse faire ; ce blocage ne serait repris que sur une décision de l'utilisateur.

**Rester à jour** : pas de guetteur de fichiers du système. **Choix du chef**, repris de tokensave : une vérification de fraîcheur à chaque appel (avec un délai minimal entre deux), et une mise à jour sur les événements git qu'Escouade voit déjà (commit, changement de branche, fin de tour d'un agent).

**Les worktrees** : un projet en a souvent beaucoup (un par agent). **Choix du chef** : un index de la branche de base, et pour chaque worktree seulement ce qui diffère ; le constructeur peut faire autrement s'il le mesure.

**Proposé d'office, en option** (décidé) : Escouade propose d'indexer un projet (à l'ajout, et dans les réglages du projet) ; on peut refuser ou couper. Un projet privé indexe seulement ce que sa clôture autorise.

### Les vecteurs : calculés par Escouade, en local (décidé)

Aucun harness ne fournit d'embeddings (Anthropic n'a pas de modèle d'embeddings ; l'API d'OpenAI en a, mais pas la connexion ChatGPT de Codex ; Copilot et Cursor indexent pour eux seuls ; OpenCode passerait par Ollama). Escouade les calcule donc lui-même :

- par ONNX Runtime, déjà embarqué ;
- avec un petit modèle **multilingue** (les docs sont souvent en français), téléchargé à la première activation, comme une voix (de 100 à 500 Mo selon le modèle : choix au constructeur, sur un essai en français et en anglais) ;
- rien ne sort de la machine ; un service en ligne pourra être une option plus tard, jamais par défaut.

Les vecteurs servent d'abord aux docs : la spec, les décisions, les documents déposés, la mémoire. La recherche par le sens dans les docs (RAG) est le cas de départ ; une recherche en arbre de résumés (RAPTOR) se compare plus tard sur une vraie doc de projet, si le RAG simple ne suffit pas.

## 2. La mémoire partagée

**Ce que c'est** (décidé) : le carnet de travail des agents. Ils y notent eux-mêmes leurs façons de travailler et les écueils à éviter (« lancer les e2e après un build de debug », « ce test est instable sous charge »). **Pas le produit** : la spec et les décisions parlent du produit, la mémoire parle du travail.

- Une mémoire par projet, dans le dépôt, sous `.escouade/` (décidé) : l'équipe la partage par git. Escouade ne la commite jamais seul.
- **Choix du chef** : un fichier par souvenir, en Markdown avec un en-tête (nom, description, sorte), et un index court, comme la mémoire de Claude Code, pour que les agents qui la connaissent s'y retrouvent.
- **Choix du chef** : elle vit dans le dossier du projet (comme les tickets partagés), pas dans le worktree de chaque agent : tous les agents du projet lisent et écrivent la même, quelle que soit leur branche.
- Tous les harness la lisent et l'écrivent par le MCP : chercher, lire, écrire, oublier. La consigne de chaque agent lui dit de la consulter au départ et d'y noter ce qu'il a appris ; un agent qui change de harness la retrouve telle quelle.
- **Choix du chef** : Escouade propose de reprendre ce que Claude Code a déjà retenu pour ce projet (sa mémoire de projet), une fois, avec l'accord de l'utilisateur, souvenir par souvenir.
- Elle est indexée (vecteurs) pour que la recherche trouve un souvenir par le sens.

**Fait (ESC-255, 2026-10-28)** : `memory.rs` et `mcp/memory.rs`. Ce que le constructeur a tranché :

- quatre outils, `search_memory`, `read_memory`, `write_memory`, `forget_memory` ; le projet se déduit pour un agent d'Escouade ;
- les sortes de Claude Code (`feedback`, `project`, `reference`, `user`) et son en-tête (`metadata.type`), lus aussi à l'ancienne (`type` en haut) ;
- la recherche va par les mots, et par le sens (hybride) dès que le modèle des vecteurs est téléchargé ; l'index se met à jour à chaque recherche (`projects/<id>/vectors.db`) ;
- la consigne est ajoutée aux instructions de **chaque nouvel agent**, après le protocole d'un ticket, dans la place que son harness lui donne ;
- `git::stage_all` laisse toujours `.escouade/memory/` de côté, et elle ne bloque pas une fusion ;
- la reprise de Claude Code : une fenêtre à l'affichage d'un projet pas encore interrogé, aucun souvenir coché au départ ; « Reprendre » et « Ne rien reprendre » règlent la question pour de bon, fermer sans choisir la repose au démarrage suivant. Elle lit `projects/<chemin>/memory/` sous le dossier de chaque compte Claude Code, rien d'autre (pas un compte Claude Code sous WSL pour l'instant) ;
- la liste des réglages oublie un souvenir au second clic (« Confirmer l'oubli »).

## 3. L'onglet Docs

Un troisième onglet de la barre latérale : **Agents · Kanban · Docs** (décidé). Les mêmes écrans pour tout le monde.

### La barre latérale de l'onglet

- « Nouvelle discussion » en tête, puis **les discussions** en cartes, comme les agents, chacune marquée « Brainstorm » ou « Discussion », avec son état (question, répond, versé, brouillon).
- Dessous, **trois espaces** : Spécification (avec ses écarts), Décisions, Documents.

### « Nouvelle discussion » (décidé)

Une fenêtre fait choisir entre deux agents spécialisés :

- **Discussion** : un agent rédacteur de spec, pour interroger la spec ou la mettre à jour simplement (« Que dit la spec sur le paiement en plusieurs fois ? », « Ajoute que le reçu fiscal part par e-mail »).
- **Brainstorm** : un agent produit, à la manière de BMAD, qui parle produit, conformité, RGPD, découverte, usages et parallèles avec d'autres secteurs, pour faire gagner le produit en fonctionnalités et en maturité. Il propose des pages de spec, des décisions et des tickets.

On donne un sujet. **On ne choisit pas de documents** : l'agent a accès à tous ceux du projet par l'index. On peut **joindre de nouveaux fichiers**, dans la fenêtre comme dans la conversation : ils sont rangés seuls dans les Documents et indexés.

**Choix du chef** : ces agents partent sur le harness, le compte et le modèle par défaut du projet, que le pied de la fenêtre montre et laisse changer ; ce sont des agents d'Escouade comme les autres (même conversation, mêmes réponses depuis la liste), avec une consigne propre à leur rôle. Ils travaillent dans le worktree de la branche des docs, lisent le code par l'index sans jamais le modifier, et n'écrivent que les docs, par les outils MCP de docs.

### Une discussion ouverte (piste B, retenue)

- À gauche, la conversation avec l'agent (questions à choix comme partout).
- À droite, **« Ce qui ira dans la spec »** : les changements proposés, page par page (sections ajoutées, phrases changées), et les décisions proposées.
- **« Verser dans la spec »** les écrit sur la branche des docs. Ce n'est pas encore partagé : la barre d'état dit « N changements à partager ».

**Fait (ESC-261, 2026-10-28)** : `discussion.rs`, `components/docs/NewDiscussionModal.svelte` et `DiscussionView.svelte`. Ce que le constructeur a tranché :

- une discussion est un agent marqué `discussion` (rôle, sujet, propositions), sans worktree à lui, dans le dossier de la branche des docs, sa consigne ajoutée à celle de tout agent (mémoire, règle de l'index) ;
- « sans jamais modifier le code » est tenu par le harness lui-même, dans tous les modes (`Spawn::read_only`, repris à la revue du chef) : Claude Code sans ses outils qui écrivent (`--disallowedTools`), Codex en bac à sable lecture seule, OpenCode avec `edit` et `bash` en `deny`, Copilot CLI sans écriture acceptée par les règles d'Escouade. Cursor CLI et Mistral Vibe écrivent dans leur dossier sans passer par Escouade : pas de discussion sur eux (`discussion::holds`), la fenêtre part sur un autre et le dit. Ce qui reste demandé est répondu sur-le-champ : écriture, commande ou « Permissions » refusées ; outils d'Escouade qui lisent et ceux qui écrivent les docs acceptés ; le reste (créer ou lancer un ticket, écrire à un agent, partager) va à l'utilisateur ;
- `write_spec_page`, `propose_decision` et `replace_decision` d'une discussion ne touchent pas les docs : ils restent sur la discussion (une page relit sa propre proposition), que « Verser » écrit ; les décisions arrivent au registre **proposées**, l'humain les accepte là ; une croix retire une proposition ;
- les fichiers joints passent par le drive (`dropDocuments`) avant le départ, et le premier message les nomme ;
- « Mettre la spec à jour » ouvre une discussion avec l'agent rédacteur, à qui l'écart est dit, à la place de l'agent court d'ESC-263 ; l'écart part de la page quand elle est versée. « Demander à la spec » en tête de la spécification, « Écrire une première spec depuis le code » sur un projet sans spec ;
- les cartes disent « Question », « Répond », « Brouillon » (des propositions pas versées), « Versé » ou « En pause ».

### « Planifier » : de la spec aux tickets (décidé le 2026-10-28)

Quand on a fini d'éditer la spec, un bouton **« Planifier »** appelle un **agent planificateur**, qui ajoute au Kanban les tickets nécessaires pour implémenter les modifications. (Il remplace l'idée d'une « Performance Loop » façon BMAD, abandonnée.)

- **Où** (choix du chef) : sur une discussion dont les changements ont été versés, et en tête de l'espace Spécification tant que des changements de spec n'ont pas de tickets (« 3 changements sans tickets · Planifier »).
- **Ce qu'il lit** (choix du chef) : les changements de spec et les décisions depuis la dernière planification, le code par l'index, et les tickets du Kanban, pour ne rien créer en double ni recréer ce qui est déjà fait.
- **Ce qu'il fait** (choix du chef) : il crée les tickets dans « À faire », chacun avec sa description, ses critères d'acceptation, ses dépendances (« Après ») et le lien vers les pages et décisions qu'il implémente. Il ne les lance pas lui-même : le pilote auto et les règles du Kanban s'appliquent comme pour tout ticket. Il dit dans la discussion ce qu'il a créé, avec « Annuler ».
- Les agents de ces tickets tiennent ensuite la spec à jour comme les autres (plus bas), et la carte d'un sujet montre ses tickets.

**Fait (ESC-262, 2026-10-28)** : `docsplan.rs`, `components/docs/PlanResult.svelte`. Ce que le constructeur a tranché :

- « depuis la dernière planification » : Escouade note, par projet, dans `planning.json` du dossier système (pas dans le dépôt), les pages versées par une discussion ou écrites dans la fenêtre, et les décisions proposées dans la fenêtre ou versées ; une planification les retire, « Annuler » les remet. Les pages écrites par l'agent d'un ticket ont déjà leur ticket ; un import n'est pas noté (toute la spec serait « sans tickets ») ;
- le planificateur est un agent court en lecture seule (`Weight::Reading`, comme la comparaison au code, 10 min au plus), une planification à la fois par projet. Il reçoit les pages changées entières, les décisions, une ligne par ticket du Kanban (clé, colonne, titre, début de la description) et, l'index allumé, les fichiers que l'index rapproche des changements ; il lit le reste du code dans le dossier du projet. Il répond un JSON de tickets ;
- « rien en double » : Escouade écarte un ticket dont le titre (sans casse, accents ni ponctuation) est celui d'un ticket du Kanban, quelle que soit sa colonne, ou d'un autre de la liste ; un ticket qui suit un écarté suit le ticket du Kanban qu'il répète. 12 tickets au plus ;
- les liens : le ticket garde ses pages et ses décisions (`Ticket::pages`, `Ticket::decisions`, sa carte au Kanban nomme ses pages, qui s'ouvrent dans Docs), sa description finit par la phrase qui les cite pour son agent, et chaque page prend sa clé dans ses `tickets` (la carte d'un sujet montre les clés). Une décision acceptée ne se réécrit pas : le lien d'une décision reste du côté du ticket ;
- le résultat se dit sous la discussion (« Tickets ») ou sur la carte des sujets (le dernier, pendant un jour), avec « Annuler », qui retire les tickets pas encore lancés et garde les autres en le disant ;
- en anglais, le bouton dit « Plan tickets » (« Plan » est déjà le mode de permission).

### La spécification (piste C, retenue pour cet espace)

- On arrive sur **la carte des sujets** : chaque sujet en carte, avec son état face au code (à jour, écart, brouillon), ses pages, ses tickets, ses décisions, et ce qui a bougé.
- À gauche, **l'arbre des pages, comme un Confluence**, rangé automatiquement sujet par sujet : une page nouvelle trouve seule sa place (c'est l'agent rédacteur qui range).
- Une page s'ouvre avec, en haut, **ses écarts avec le code** et de quoi les régler : « Mettre la spec à jour », « Ouvrir un ticket pour le code », « Voir le code ». Elle cite ses sources (documents, tickets, décisions).

### Les décisions : un registre, pas un document (décidé)

- Une liste numérotée et datée : la décision, son état (proposée, acceptée, remplacée), d'où elle vient (brainstorm, discussion, ticket, document), et **ce qu'elle a changé dans la spec** (pages, lignes ajoutées et retirées).
- Le détail : contexte, décision, conséquences, liens.
- Une décision acceptée ne se réécrit pas : une nouvelle la remplace, et l'ancienne dit par laquelle.
- **Choix du chef** : un agent propose, un humain accepte (un clic dans le registre) ; la décision laissée par l'agent d'un ticket est acceptée quand le ticket est validé. Un fichier par décision dans le dépôt, avec son en-tête.

### Les documents : un drive (décidé)

- Des dossiers, le glisser-déposer, une recherche ; chaque fichier dit s'il a été lu par les agents et combien de fois il est cité.
- Un fichier déposé (ici, dans une discussion ou dans « Nouvelle discussion ») est **analysé et indexé en local, puis rangé seul** dans le bon dossier (décidé). Le texte extrait reste dans le dossier système ; le fichier va dans le dépôt, sur la branche des docs.
- Formats de départ : PDF, Word, Markdown, texte, images (choix au constructeur pour les autres).

### La spec toujours à jour

- **Les agents des tickets la tiennent à jour eux-mêmes** (décidé) : avant de rendre un ticket qui change le comportement du produit, l'agent met à jour les pages touchées et laisse une décision. Rien ne bloque la validation : c'est la consigne du ticket (`protocol_prompt_for`) qui le demande, et la revue qui le voit.
- **Choix du chef** : tous les écrits de spec et de décisions passent par les outils MCP de docs d'Escouade, qui écrivent là où vivent les docs. Avec la branche courante, c'est la branche du ticket (la spec suit le code dans la même fusion) ; avec une branche fixe, les changements de spec d'un ticket sont versés sur la branche des docs **quand le ticket est validé**, pour que la spec ne décrive jamais un comportement pas encore fusionné.
- **L'écran montre les écarts** : « comparée au code il y a 2 h », « 1 écart ». La comparaison tourne **après chaque ticket validé**, sur les pages qu'il touche, et à la demande (voir les réponses de l'utilisateur, plus bas).

**Fait (ESC-263, 2026-10-28)** : `docsticket.rs`, `docscompare.rs`. Ce que le constructeur a tranché :

- la consigne de chaque ticket (`board::spec_rule`, dans `protocol`) le demande toujours, en nommant les outils de docs ; une page écrite par l'agent d'un ticket prend sa clé dans ses tickets, comme une décision prend son origine ;
- avec une branche fixe, l'agent d'un ticket (en cours ou à tester) lit et écrit une copie de la spec et des décisions de la branche (`ticket-docs/<clé>/` du dossier système, avec la copie d'origine à côté), les documents restant ceux de la branche ; à la validation, seul ce que le ticket a changé est versé (ce que l'équipe a écrit entre-temps reste ; à page égale, celle du ticket l'emporte), une décision nouvelle dont le numéro a été pris entre-temps prend le suivant ; la copie part avec un ticket supprimé ;
- la comparaison est un agent court en lecture seule (`Weight::Reading`, le harness de « Modèle des textes rédigés »), une page à la fois, 5 min au plus chacune ; il reçoit la page et, l'index allumé, les fichiers que l'index lui rapproche ; il répond un JSON d'écarts, gardés seulement sur des fichiers du projet présents et hors clôture ;
- les pages touchées par un ticket : celles qu'il a écrites, celles qui le citent, puis celles qui nomment un fichier qu'il a changé (chemin, ou nom pas trop commun) ou un symbole de ces fichiers (par l'index) ; 12 au plus. Elle tourne seulement quand le ticket est fusionné (« Valider et merger ») : poussé ou en PR, le code n'est pas encore là où la spec est comparée ;
- « Comparer au code » sur toute la spec : 40 pages au plus, les moins récemment comparées d'abord ;
- « Mettre la spec à jour » sur un écart : le même agent court réécrit la page sur ce point, l'écart retiré (en attendant l'agent rédacteur des discussions, ESC-261).

## Ce qui reste au constructeur

- Les noms et le nombre exact des outils MCP (index, mémoire, docs), et leurs réponses.
- Le modèle d'embeddings (multilingue, petit), la taille des morceaux, le stockage des vecteurs (dans SQLite ou à côté).
- Les grammaires tree-sitter embarquées au départ (au moins Rust, TypeScript, JavaScript, Svelte, Python, Go, Java, C#, PHP, Markdown).
- Le dosage du renvoi des grep vers l'index, harness par harness.
- La forme exacte des fichiers de spec, de décisions et de mémoire dans le dépôt (Markdown avec en-tête, un fichier par page, par décision, par souvenir).
- Le découpage en tickets, dans l'ordre décidé (socle, mémoire, Docs).

## Réponses de l'utilisateur à la relecture (2026-10-27)

1. **La spec vit sous `.escouade/`** (décidé), comme la mémoire, les décisions et les documents. **Une spec existante détectée** (`docs/SPEC.md`, `SPEC.md`, un dossier `docs/`…) : Escouade **propose de l'importer**, découpée par sujet ; rien sans accord (décidé). **Choix du chef** : un projet sans spec part d'un espace vide, avec la proposition de faire écrire une première version par l'agent rédacteur depuis le code (un geste de l'utilisateur, parce que ça coûte des tokens).
2. **La spec est comparée au code après chaque ticket validé** (décidé), sur les pages que le ticket touche : celles liées au code qu'il a changé (par l'index) et celles qu'il a modifiées. **Choix du chef** : « Comparer au code » reste aussi possible à la demande, page par page ou sur toute la spec.
