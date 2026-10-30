---
title: "Index du code"
subject: "Docs"
sources: ["docs/SPEC.md","github.com/aovestdipaperino/tokensave @ 580936b (MIT, © 2025 Enzo Lombardi)","docs/CODEINDEX-PROBES.md","THIRD-PARTY-NOTICES.md","docs/codeindex-probes-sufficiency.json"]
tickets: ["ESC-251","ESC-252","ESC-253","ESC-254","ESC-293","ESC-296","ESC-297","ESC-298"]
decisions: []
compared: "2026-10-30T03:07:48+01:00"
---

## Ce que c'est

Un index local du code de chaque projet. Il fait trouver le code aux agents pour bien moins de tokens que grep ou la lecture de fichiers entiers. Il découpe le code en symboles et en liens, et répond à six requêtes : chercher un symbole ou un texte, le contexte d'une tâche, le corps d'un symbole, les appelants, les appelés, l'impact d'un changement. La recherche, le contexte, les liens, les réponses et le compte de l'économie suivent la méthode de tokensave (github.com/aovestdipaperino/tokensave, commit `580936b`, MIT, © 2025 Enzo Lombardi), portée à l'identique, mêmes étapes et mêmes constantes ; sa licence est dans `THIRD-PARTY-NOTICES.md` et les crédits. Aucune recherche par le sens pour le code (des vecteurs ont été mesurés puis abandonnés, ESC-294) : elle reste à la mémoire et aux docs. L'index cherche par les mots du code : une question dite avec ses identifiants trouve mieux qu'une paraphrase. Rien ne quitte la machine, sauf le téléchargement du modèle de la recherche par le sens. La conception est dans `docs/FACTORY.md` (« Le socle d'index local »).

## Comportement

### Ce qu'il lit

- Les fichiers d'un langage connu, découpés en **symboles** : fonctions, méthodes, classes, structs, enums, interfaces, traits, `impl`, types, constantes, champs, modules, composants Svelte, titres Markdown. Chacun avec son conteneur, ses lignes, sa première ligne, sa docstring ou le commentaire au-dessus, et sa **visibilité** (publique, de la crate, du parent, privée), lue comme les extracteurs de tokensave la lisent (`pub` de Rust, `export` et `private`/`protected` de TypeScript, `_` de Python, majuscule de Go, modificateurs de Java, C# et PHP ; un titre Markdown est public). Et en **références** : `import`, `call`, `extends`, `implements`, nommées comme les nomme tokensave (`obj.method`, `a::b`, et `Type::method` quand le type du receveur se lit dans le code), résolues en liens (voir plus bas).
- Langages : Rust, TypeScript (et TSX), JavaScript, Svelte (son `<script>` lu en TypeScript, ses balises de composants comme des appels), Python, Go, Java, C#, PHP, Markdown (une section par titre, imbriquée selon son niveau).
- Écartés : les fichiers de plus de 1 Mio (la limite de tokensave), les binaires, `*.min.*`, `*.d.ts`, `node_modules`, `target`, `dist` et `vendor`.
- La base, une par projet, vit dans le dossier de données d'Escouade (`projects/<id>/code-index.db`), jamais dans le dépôt. Fermer ou retirer un projet l'efface : elle se recalcule. Quand sa forme change avec une version d'Escouade, l'index d'un projet est relu une fois, sans rien demander.

### L'index plein texte

- Chaque symbole y entre par cinq colonnes, pesées comme chez tokensave : le nom (10), le nom qualifié `chemin::Conteneur::nom` (5), la doc (1), la signature, sa première ligne (2), et les mots d'un nom en camelCase (3). Le tokenizer Porter ramène les mots à leur racine : « sending requests » trouve `send_request`.
- Le corps de chaque fonction et méthode est indexé à part. Une tâche qui décrit un comportement trouve la plus petite fonction dont le corps contient au moins deux de ses mots (par leurs cinq premières lettres).

### Les liens, résolus comme tokensave (ESC-297)

La méthode est celle de tokensave (github.com/aovestdipaperino/tokensave, commit `580936b`, MIT), portée à l'identique ; ses écarts et leurs raisons sont écrits dans `docs/CODEINDEX-PROBES.md` (§ IDX-2).

- **D'un symbole à un symbole.** Chaque appel, `extends` et `implements` est résolu à l'indexation vers l'id du symbole qu'il vise, ou vers rien. Les réponses lisent ces liens, jamais un nom.
- **Les receveurs typés.** Dans l'extracteur, `self.m()` / `this.m()`, un appel sur un paramètre typé, sur une variable construite (`let x = T::new()`, `new T()`, `T()`) ou, en TypeScript, sur `this.champ` deviennent aussi `Type::m` (Rust, TypeScript/JavaScript/Svelte, Python). En Rust, `a.champ.m()` passe par le type du champ (le nôtre).
- **Les stratégies, dans l'ordre** : le nom qualifié (`A::b`, par le nom complet ou sa fin), puis la fin du chemin ; l'appel pointé (`obj.m`) par le sélecteur d'import de Go, sinon par le nom simple, avec preuve ; le nom nu.
- **Plusieurs candidats** se départagent par le score de tokensave : même fichier +100 et proximité de ligne (jusqu'à +20), dossiers communs +5 chacun (+40 au plus), même langage +50 ou autre langage −80, `pub` +10, appelable +25, nom importé +30.
- **Une égalité n'est jamais devinée** : l'appel n'a pas de lien et va dans la table des appels ambigus, avec ses candidats. Un lien en dessous de 0,6 de confiance n'est pas gardé (un seul candidat d'un autre langage vaut 0,5). Les noms trop courants (`new`, `get`, `clone`, `run`…) ne se résolvent que dans leur fichier. En Python, JavaScript et TypeScript, un candidat seul doit être joignable : même fichier, même dossier, nom, conteneur ou module importé, fichier importé en relatif.
- **La relecture partielle** : un fichier relu fait résoudre de nouveau ses références, et celles des autres fichiers dont le nom touche un symbole apparu ou disparu ; tout le reste se résout comme avant.
- **Les worktrees** : un worktree résout ses propres références, et celles du projet que sa différence peut changer, contre ce qu'il voit ; ses liens remplacent alors ceux du projet pour ces références. Ils sont refaits quand le worktree ou le projet change.

### Les requêtes (outils MCP)

- **Les réponses sont celles de tokensave** (ESC-298) : du texte que lit le modèle, pas du JSON, sauf pour `index_callers`, `index_callees` et `index_impact` (du JSON, comme chez tokensave). Une réponse de plus de 15 000 caractères est coupée (« [... truncated at N chars] ») ; celle du contexte garde sa fin à partir de `### Retrieval`.
- `index_search` : le classement de tokensave. Chaque mot par préfixe, joints par « ou » ; une phrase de plusieurs mots cherchée aussi telle quelle, et ces symboles en tête ; si rien, une recherche « contient ». Le nom exact (et la dernière partie d'un `a::b`) s'ajoute. Les définitions (fonctions, types) passent avant les `impl`, puis les constantes et champs, puis les modules et titres ; dans chaque groupe, le score, plus un bonus de genre et 10 pour le nom exact, multiplié selon le chemin (`src/`, `app/`, `lib/` en hausse ; tests, exemples, Markdown, dépendances en baisse). Un filtre par genre de symbole. La réponse : `count: N` (avec « truncated » quand d'autres résultats existent au-delà de `limit`, 10 par défaut, 500 au plus), puis une ligne par symbole, `chemin:ligne-fin: nom (genre) — signature` (la signature sur une ligne, 200 octets au plus) ; l'id du symbole avec `ids` (` [id: N]`), pour `index_body`, `index_callers`, `index_callees` et `index_impact`.
  - **`literal`** : un texte exact (sensible à la casse) dans la source des fichiers que voit le dossier, triés par chemin, un fichier de plus de 2 Mio sauté : `chemin:ligne: texte`, et avec `ids` le plus petit symbole qui le contient et son id. `total` compte toutes les occurrences, sauf avec `exact_total: false` (la recherche s'arrête à `limit`, le total est un minimum) ; `timeout_ms` (30 000 par défaut, 0 sans limite) arrête une recherche trop longue avec ce qu'elle a trouvé, et le dit.
  - **`path_include` / `path_exclude`** : ne garder que les résultats dont le chemin contient l'une des chaînes, ou écarter ceux qui en contiennent une (l'exclusion l'emporte) ; la recherche classée en prend alors 5 fois `limit` (50 au moins) avant de filtrer.
- `index_context` : le contexte d'une tâche dite en mots, par le pipeline de tokensave :
  - la tâche lue : ses identifiants (camelCase, snake_case, `a::b`), les composés Pascal et camel de deux mots voisins (« promo banner » → `PromoBanner`), sans les mots vides ; ses mots de quatre lettres et plus ; les `keywords` donnés ;
  - un `Type::Variant` ou une phrase entre guillemets (8 caractères et plus, deux mots) cherchés tels quels dans le code : le plus petit symbole qui les contient est un point d'entrée sûr ;
  - les dix meilleurs symboles de chaque terme, mêlés à tour de rôle ; le nom exact écrit par l'utilisateur à 100 ; les corps des fonctions ; les méthodes d'un type trouvé, à la moitié de son score ;
  - le reclassement multiplicatif : genre, visibilité, fichiers de test et fixtures, tests dans le code, Markdown, `src/`, appelants (par les liens résolus) ; puis les symboles qui portent plusieurs mots de la tâche ;
  - trois points d'entrée au plus (plus les sûrs), un tiers au plus d'un même fichier (`max_per_file`) ; puis les symboles à un lien d'eux (appelés, appelants, types étendus ou implémentés), 20 en tout par défaut (`max_nodes`, ou `limit`, 100 au plus) ; le code de cinq points d'entrée (`max_code_blocks`, 20 au plus), 1 500 octets chacun au plus, coupé à une fin de ligne (`max_code_lines` coupe aussi au nombre de lignes, de 1 à 1 000) ; un bloc coupé finit par `... [truncated — full body: index_body node_id=N]`. `include_code: false` ne donne pas de code.
  - **La réponse, en Markdown** : `## Code Context`, la tâche, `### Entry Points` (nom, genre, chemin et ligne, signature, première ligne de la doc), `### Related Symbols` (par fichier, `nom:ligne` ; les symboles des tests comptés sur une ligne), `### Code` (un bloc par point d'entrée, `#### nom (chemin:ligne)` et son langage), `### Retrieval`, puis `seen_node_ids: [ids]`.
  - **`exclude_node_ids`** : les ids déjà vus (le `seen_node_ids` de l'appel d'avant) ne reviennent pas, par aucun canal.
  - **`path`, `path_include`, `path_exclude`** : les points d'entrée gardés, dans ce dossier ou ce fichier, par sous-chaîne du chemin ; les voisins peuvent en sortir.
  - **`keywords`** : des synonymes de la tâche (pour « authentification » : `login`, `session`, `token`), cherchés chacun ; un mot-clé de plusieurs mots est aussi cherché tel quel dans le code.
  - **Le diagnostic « Retrieval »** (la section `### Retrieval`) : la correspondance — `exact` (un nom exact ou une source exacte), `strong` (score de 10 et plus), `fts-only` —, le meilleur score, les résultats de chaque terme et les termes sans résultat, avec le conseil d'ajouter des `keywords` (avec les mots du code) ou d'essayer `index_search`. Il est là quand des termes ont été cherchés, ou quand rien n'a été trouvé.
- `index_body` : le corps entier d'un symbole, par son id (`node_id`, celui d'un bloc coupé, de `seen_node_ids` ou de `index_search` avec `ids`) ou par son nom (le nom qualifié, ses derniers segments `Type::membre` ou `Type.membre`, sinon les 20 meilleurs de la recherche) ; la meilleure sorte gardée (une fonction avant un type, avant un `impl`, une constante, un champ). Une seule définition : `match_count: 1`, `file: chemin:ligne-fin: nom (genre)` et le corps. Plusieurs : `match_count: 0`, leur nombre, le conseil de redemander par un nom qualifié ou un id, et la liste (nom qualifié, genre, chemin, lignes, id), 20 par défaut (`limit`, 50 au plus).
- **Les chemins ignorés** : `.escouade/queryignore` dans le dossier du projet (un motif par ligne ; `*` en fait un glob sur tout le chemin, sinon une sous-chaîne ; `#` un commentaire) écarte ces chemins des résultats de `index_search` (classé et littéral) et des points d'entrée de `index_context`, jamais de l'index. Il est lu dans le dossier de l'agent (son worktree), jamais par un lien qui sort de ce dossier, ni quand la clôture refuse le fichier.
- `index_callers`, `index_callees` et `index_impact` prennent un **id** de symbole, ou un **nom** (`forget`, `Runtime::forget`, `Runtime.forget`, ou le nom qualifié) répondu **définition par définition**, chacune dans son groupe : les meilleures sortes seulement (une fonction et un type avant un champ), les surcharges ensemble, dans l'ordre de `index_search`, jusqu'à `max_matches` (3 par défaut, 20 au plus) ; les autres définitions sont listées sans résultat, à redemander par leur id. `path` ne garde que les définitions de ce fichier, et donc leurs seuls appelants.
- `index_callers` : par définition, ses appelants (l'id du symbole qui appelle, la ligne de l'appel) et leur nombre ; à part, les appels ambigus qui peuvent la viser.
- `index_callees` : les définitions qu'elle appelle ; à part, ses appels ambigus ; et les noms externes.
- `index_impact` : d'un symbole (un type compte ses méthodes) ou d'un chemin, les appelants jusqu'à la profondeur donnée, les types qui l'étendent ou l'implémentent, les fichiers qui l'importent, les fichiers touchés, et à part les appels ambigus.

### Ce qui diffère de tokensave

- Le delta des worktrees et la clôture des projets privés : chaque requête, le corps des fonctions et la recherche dans la source compris, ne voit que ce que voit le dossier qui la pose, et ne lit aucun fichier que la clôture refuse. La recherche littérale lit ces fichiers l'un après l'autre et n'a pas le compte des fichiers que l'index ne connaît pas (`unscanned` de tokensave).
- Les noms `index_*` des outils, les ids en nombres ; les phrases en français ou en anglais, les titres et les clés de tokensave.
- `index_context` donne le code de ses points d'entrée par défaut (`include_code` vaut vrai) : chez tokensave, l'outil n'en donne que sur demande. Le but de l'index est que l'agent ait le code dont il a besoin sans le relire.
- La ligne de mesure est toujours envoyée (tokensave la coupe par défaut) ; la session est celle de l'appelant (un agent, ou les clients hors Escouade), pas une connexion MCP ; les schémas comptés sont ceux des six outils de l'index.
- **Le hook ne refuse rien** : tokensave refuse certains grep et les agents Explore ; Escouade conseille l'index et laisse toute recherche se faire (décision d'ESC-253), sauf décision contraire de l'utilisateur.
- Pas repris : `merge_adjacent` et le mode `plan` de `tokensave_context`, le format `json` de la recherche et du corps.
- Les appels : ceux de la résolution, avec ses propres écarts (§ « Les liens, résolus comme tokensave » et `docs/CODEINDEX-PROBES.md`, § IDX-2) ; les appelants comptés et les voisins d’un point d’entrée lisent les liens résolus.
- Les branches et boucles d'un corps (un petit bonus du corps) se comptent par ses mots : l'index ne garde pas la complexité.
- Un composant Svelte compte comme une classe, un titre Markdown comme un module (comme chez tokensave) ; il n'y a ni nœud de fichier ni doc compagnon à écarter.
- `index_search` garde son filtre par genre.

### L'économie

Chaque réponse finit par la ligne `index_metrics: before=… after=… result=… saved=…`, puis la même chose en mots, comptée comme tokensave la compte (`src/mcp/tools/accounting.rs`, `src/mcp/server.rs`) :

- **before** : les fichiers que la réponse a touchés, lus en entier (leur taille ÷ 4), **plafonnés à 4 fois la réponse** ; à la réponse elle-même pour une recherche littérale (grep en rendrait autant) ;
- **after** : ce que l'agent a payé pour l'appel : la réponse (ses octets ÷ 4), sa requête (le nom de l'outil, ses arguments et 8 pour l'enveloppe JSON-RPC), **une fois par session** les schémas des outils de l'index, et la ligne de mesure elle-même ;
- **saved** : before − after, jamais moins que rien ;
- **le déficit est reporté** : un appel qui coûte plus qu'il n'économise (le premier d'une session, qui paie les schémas) ne crédite rien, et ce qu'il doit est payé par les appels suivants de la session avant qu'ils ne créditent.

Une réponse qui n'a touché aucun fichier n'a pas de ligne. Les Stats font le total de ce qui a été crédité, par projet : réponses, tokens et dollars économisés sur la période, tous comptes et harness confondus ; une réponse comptée avant ESC-298 garde ce qu'elle avait dit (fichiers moins réponse). Les dollars sont comptés au prix d'entrée du dernier modèle sur lequel l'agent a tourné, sinon de son réglage, sinon 0.

**Les descriptions des outils** ne changent pas d'un appel à l'autre (rien n'y dépend de l'index : le cache de prompt du client reste bon). Elles demandent d'écrire avec les mots du code (en anglais ici), de redemander avec `keywords` quand un terme ne trouve rien, de passer `seen_node_ids` à `exclude_node_ids`, et disent ce que compte la ligne de mesure.

### Toujours à jour, sans guetteur

Aucun guetteur de fichiers. L'index se revérifie :

- **à chaque requête**, au plus une fois toutes les 3 s par dossier. Il prend la liste des fichiers de git (suivis, et non suivis non ignorés ; sinon un parcours du dossier) et ne relit que les fichiers dont la taille ou la date a changé ;
- **aux événements git qu'Escouade voit déjà** : un commit, un changement de branche, la fin d'un tour d'agent. La relecture se fait aussitôt, en arrière-plan.

### Worktrees

Le dossier du projet est indexé en entier. Un worktree ne garde que ce qui en diffère : les fichiers changés, ajoutés ou retirés, et les liens que cette différence change. Une requête dans un worktree voit le projet, moins ces fichiers, plus les siens. Quand le projet prend le changement (une fusion), la différence se vide au passage suivant ; un worktree disparu est oublié. L'index du projet laisse de côté le dossier de ses worktrees et ceux de ses agents. Un dossier qui n'est ni le projet ni l'un de ses worktrees est refusé.

### Projets privés

Certains fichiers ne sont jamais listés, lus ni indexés : ceux que la clôture d'un agent lui refuserait. C'est le dossier d'un projet privé rangé sous le projet, le dossier de données d'Escouade et les sessions des harness. Ils sont revérifiés avant de montrer un extrait ou d'y chercher une source exacte. Sans projet privé, rien n'est écarté. Un projet privé indexe son propre dossier, pour ses seuls agents. Les réglages du projet montrent ce que sa clôture écarte : « Laissé hors de l'index : … ».

### La recherche par le sens

- **Le modèle** : multilingual-e5-base de Microsoft (8 bits, 768 dimensions, licence MIT, 295 Mo : 278 Mo de modèle et 17 Mo de tokenizer). Un seul pour tous les projets, lu sur la machine par le même moteur que les voix. Les textes portent le préfixe `query: ` ou `passage: ` du modèle.
- **Le téléchargement**, comme celui d'une voix : par le proxy des réglages, avec une jauge, et annulable. Chaque fichier n'est gardé qu'entier, de la bonne taille et de la bonne empreinte SHA-256 ; un fichier qui n'est pas le bon est supprimé. Après une coupure ou une annulation, le téléchargement reprend là où il en était, sauf avec un serveur qui ne sait pas reprendre.
- **Le découpage** : le Markdown par titres (un morceau ne traverse pas un titre et se lit avec le chemin des titres au-dessus, `A › B`), le texte par paragraphes, le code par blocs séparés d'une ligne vide. Les petits voisins sont joints jusqu'à 900 caractères. Aucun morceau ne dépasse 1 400 caractères (environ 300 jetons ; le modèle en lit 512). Un bloc de code entre ``` n'est coupé ni par un titre ni par une ligne vide.
- **L'index** : une base par projet dans le dossier de données d'Escouade (`projects/<id>/vectors.db`), jamais dans le dépôt. Un texte inchangé ne coûte rien. D'un texte modifié, seuls les morceaux nouveaux passent par le modèle. Un index fait par un autre modèle est vidé.
- **La recherche** : les plus proches par le sens, sur tous les morceaux, sans approximation. Elle est fusionnée avec la recherche par les mots, pour qu'un mot rare (un nom, un identifiant) retrouve son morceau. Elle sait chercher dans la spec, les décisions, les documents et la mémoire ; l'index du code n'en utilise pas. Les documents s'en servent (voir *Documents*).
- Elle ne reçoit que des textes que la clôture du projet autorise : un fichier interdit n'entre dans aucun index ni dans aucune réponse.

### Mesures

- Le banc des sondes (`docs/CODEINDEX-PROBES.md`, `cargo run --release --example codeindex_probes`) mesure la justesse, la vitesse et le coût des outils face à grep, sur 54 sondes, la réponse comptée telle que le serveur MCP l'envoie ; chaque ticket de l'index y ajoute sa colonne, avant et après. Pour les 17 sondes « tâche », il mesure aussi la **suffisance** : la part du code nécessaire (des plages écrites à la main, `docs/codeindex-probes-sufficiency.json`) que la réponse contient déjà, et les appels de plus pour l'avoir en entier (ESC-298).
- Les liens résolus (ESC-297) et leur justesse face à grep : le banc des sondes, `docs/CODEINDEX-PROBES.md`, colonne IDX-2.
- Recherche par le sens (build de debug, Windows). Chargement du modèle : 1,2 s. 195 passages courts par seconde, et 27 morceaux par seconde sur de longs documents. Ajouter une section à un document indexé relit 1 morceau sur 30, en 11 ms. Une recherche prend 8 ms sur 506 morceaux, 17 ms sur 1 000, 173 ms sur 10 000 et 860 ms sur 50 000.

## Réglages

- **Réglages › Projets › Tous les projets › « Index du code sur tous les projets »** : coupé par défaut, enregistré aussitôt. Chaque projet le suit tant qu'il n'a pas fait son propre choix. Allumé, les projets qui le suivent sont lus un par un, les autres en attente ; un projet ajouté qui le suit aussi. Coupé, leur index est effacé, après une confirmation qui dit combien ; ceux qui ont fait leur choix le gardent. Ce réglage ne télécharge jamais le modèle de la recherche par le sens.
- **Réglages › Projets › « Index du code »** d'un projet : « Indexer ce projet », enregistré aussitôt. C'est le choix du projet ; il peut revenir au réglage de l'app. Allumé, le projet est lu aussitôt ; coupé, son index est effacé. Le texte dit ce que l'index fait gagner et que rien ne quitte la machine ; pour un projet privé, que son index ne sert qu'à ses agents.
- **« Nouveau projet »**, dernière étape : « Indexer le code », parti du réglage de l'app. Le changer en fait le choix du projet, lu en arrière-plan une fois le projet créé.
- **Réglages › Projets › Index du code › « Recherche par le sens »** : le modèle, son téléchargement avec sa jauge, « Annuler », « Reprendre (N %) » et « Effacer ».
- Ni le `.escouade.yml` ni la copie des réglages d'un projet ne portent ce réglage.
- Après la mise à jour depuis une version où il n'était que par projet : un projet qui l'avait allumé garde un choix allumé ; un projet coupé suit le réglage de l'app.

## Limites

- Un appel dont le receveur n'a pas de type lisible se résout par le nom et le score ; à égalité, il reste ambigu, sans lien. Les imports ne sont pas résolus en liens (ils servent de preuve et à `index_impact`).
- Les langages hors de la liste ne sont pas indexés.
- La carte d'un agent ne dit pas qu'il s'est servi de l'index ; les Stats le disent.
- Une tâche dite avec des mots qui ne sont pas ceux du code trouve mal ; grep reste meilleur pour une regex et pour ces tâches (le détail, sonde par sonde : `docs/CODEINDEX-PROBES.md`, § IDX-3).
