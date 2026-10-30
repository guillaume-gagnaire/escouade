---
title: "Index du code"
subject: "Docs"
sources: ["docs/SPEC.md","docs/CODEINDEX-PROBES.md","github.com/aovestdipaperino/tokensave@580936b (MIT)"]
tickets: ["ESC-251","ESC-252","ESC-253","ESC-254","ESC-293","ESC-297"]
decisions: []
compared: null
---

## Ce que c'est

Un index local du code de chaque projet. Il fait trouver le code aux agents pour bien moins de tokens que grep ou la lecture de fichiers entiers. Il découpe le code en symboles et en liens, et répond à cinq requêtes : chercher un symbole, le contexte d'une tâche, les appelants, les appelés, l'impact d'un changement. Une recherche par le sens, facultative, en est le complément. Rien ne quitte la machine, sauf le téléchargement du modèle de la recherche par le sens. La conception est dans `docs/FACTORY.md` (« Le socle d'index local »).

## Comportement

### Ce qu'il lit

- Les fichiers d'un langage connu, découpés en **symboles** : fonctions, méthodes, classes, structs, enums, interfaces, traits, `impl`, types, constantes, champs, modules, composants Svelte, titres Markdown. Chacun avec son conteneur, ses lignes, sa première ligne, et sa docstring ou le commentaire au-dessus. Et en **références** : `import`, `call`, `extends`, `implements`, nommées comme les nomme tokensave (`obj.method`, `a::b`, et `Type::method` quand le type du receveur se lit dans le code).
- Langages : Rust, TypeScript (et TSX), JavaScript, Svelte (son `<script>` lu en TypeScript, ses balises de composants comme des appels), Python, Go, Java, C#, PHP, Markdown (une section par titre, imbriquée selon son niveau).
- Écartés : les fichiers de plus de 1 Mio (la limite de tokensave), les binaires, `*.min.*`, `*.d.ts`, `node_modules`, `target`, `dist` et `vendor`.
- La base, une par projet, vit dans le dossier de données d'Escouade (`projects/<id>/code-index.db`), jamais dans le dépôt. Fermer ou retirer un projet l'efface : elle se recalcule.

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

- `index_search` : les symboles, par les mots de leur nom (`parseConfig` donne `parse` et `config`), de leur doc ou de leur chemin, par préfixe. Le nom exact vient en tête. Tous les mots d'abord ; si rien, l'un d'eux. Un filtre par genre de symbole. Chaque symbole porte son `id`.
- `index_context` : le contexte d'une tâche dite en mots. Ses points d'entrée (trois au plus par fichier), les symboles liés (appelés et appelants des quatre premiers, par les liens résolus), et le code des points d'entrée, lu dans le dossier de l'agent.
- `index_callers`, `index_callees` et `index_impact` prennent un **id** de symbole, ou un **nom** (`forget`, `Runtime::forget`, `Runtime.forget`, ou le nom qualifié) répondu **définition par définition**, chacune dans son groupe : les meilleures sortes seulement (une fonction et un type avant un champ), les surcharges ensemble, dans l'ordre de `index_search`, jusqu'à `max_matches` (3 par défaut, 20 au plus) ; les autres définitions sont listées sans résultat, à redemander par leur id. `path` ne garde que les définitions de ce fichier, et donc leurs seuls appelants.
- `index_callers` : par définition, ses appelants (l'id du symbole qui appelle, la ligne de l'appel) et leur nombre ; à part, les appels ambigus qui peuvent la viser.
- `index_callees` : les définitions qu'elle appelle ; à part, ses appels ambigus ; et les noms externes.
- `index_impact` : d'un symbole (un type compte ses méthodes) ou d'un chemin, les appelants jusqu'à la profondeur donnée, les types qui l'étendent ou l'implémentent, les fichiers qui l'importent, les fichiers touchés, et à part les appels ambigus.

### L'économie

Chaque réponse porte `saving`. On y trouve les tokens des fichiers dont elle est tirée, comme s'ils étaient lus en entier (octets ÷ 4), ceux de la réponse, les tokens gagnés, le pourcentage et le nombre de fichiers. C'est une estimation, face à la lecture de fichiers entiers. Les Stats en font le total par projet : réponses, tokens et dollars économisés sur la période, tous comptes et harness confondus. Les dollars sont comptés au prix d'entrée du dernier modèle sur lequel l'agent a tourné, sinon de son réglage, sinon 0.

### Toujours à jour, sans guetteur

Aucun guetteur de fichiers. L'index se revérifie :

- **à chaque requête**, au plus une fois toutes les 3 s par dossier. Il prend la liste des fichiers de git (suivis, et non suivis non ignorés ; sinon un parcours du dossier) et ne relit que les fichiers dont la taille ou la date a changé ;
- **aux événements git qu'Escouade voit déjà** : un commit, un changement de branche, la fin d'un tour d'agent. La relecture se fait aussitôt, en arrière-plan.

### Worktrees

Le dossier du projet est indexé en entier. Un worktree ne garde que ce qui en diffère : les fichiers changés, ajoutés ou retirés, et les liens que cette différence change. Une requête dans un worktree voit le projet, moins ces fichiers, plus les siens. Quand le projet prend le changement (une fusion), la différence se vide au passage suivant ; un worktree disparu est oublié. L'index du projet laisse de côté le dossier de ses worktrees et ceux de ses agents. Un dossier qui n'est ni le projet ni l'un de ses worktrees est refusé.

### Projets privés

Certains fichiers ne sont jamais listés, lus ni indexés : ceux que la clôture d'un agent lui refuserait. C'est le dossier d'un projet privé rangé sous le projet, le dossier de données d'Escouade et les sessions des harness. Ils sont revérifiés avant de montrer un extrait. Sans projet privé, rien n'est écarté. Un projet privé indexe son propre dossier, pour ses seuls agents. Les réglages du projet montrent ce que sa clôture écarte : « Laissé hors de l'index : … ».

### La recherche par le sens

- **Le modèle** : multilingual-e5-base de Microsoft (8 bits, 768 dimensions, licence MIT, 295 Mo : 278 Mo de modèle et 17 Mo de tokenizer). Un seul pour tous les projets, lu sur la machine par le même moteur que les voix. Les textes portent le préfixe `query: ` ou `passage: ` du modèle.
- **Le téléchargement**, comme celui d'une voix : par le proxy des réglages, avec une jauge, et annulable. Chaque fichier n'est gardé qu'entier, de la bonne taille et de la bonne empreinte SHA-256 ; un fichier qui n'est pas le bon est supprimé. Après une coupure ou une annulation, le téléchargement reprend là où il en était, sauf avec un serveur qui ne sait pas reprendre.
- **Le découpage** : le Markdown par titres (un morceau ne traverse pas un titre et se lit avec le chemin des titres au-dessus, `A › B`), le texte par paragraphes, le code par blocs séparés d'une ligne vide. Les petits voisins sont joints jusqu'à 900 caractères. Aucun morceau ne dépasse 1 400 caractères (environ 300 jetons ; le modèle en lit 512). Un bloc de code entre ``` n'est coupé ni par un titre ni par une ligne vide.
- **L'index** : une base par projet dans le dossier de données d'Escouade (`projects/<id>/vectors.db`), jamais dans le dépôt. Un texte inchangé ne coûte rien. D'un texte modifié, seuls les morceaux nouveaux passent par le modèle. Un index fait par un autre modèle est vidé.
- **La recherche** : les plus proches par le sens, sur tous les morceaux, sans approximation. Elle est fusionnée avec la recherche par les mots, pour qu'un mot rare (un nom, un identifiant) retrouve son morceau. Elle sait chercher dans la spec, les décisions, les documents, la mémoire et le code. Les documents s'en servent (voir *Documents*).
- Elle ne reçoit que des textes que la clôture du projet autorise : un fichier interdit n'entre dans aucun index ni dans aucune réponse.

### Mesures

- Index du code, sur le dépôt d'Escouade (1 211 fichiers de code et de Markdown, 28 473 symboles, 249 897 liens ; Windows, build optimisé). Indexation complète : 4,8 s. Vérification sans changement : 61 ms. Un fichier touché : 74 ms. Base : 39,2 Mo. Contexte d'une tâche : de 154 000 à 270 000 tokens de fichiers entiers contre 2 300 à 3 200 tokens de réponse, soit 98 % d'économie estimée.
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
