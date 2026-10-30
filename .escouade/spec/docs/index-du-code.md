---
title: "Index du code"
subject: "Docs"
sources: ["docs/SPEC.md","github.com/aovestdipaperino/tokensave @ 580936b (MIT, © 2025 Enzo Lombardi)","docs/CODEINDEX-PROBES.md","THIRD-PARTY-NOTICES.md"]
tickets: ["ESC-251","ESC-252","ESC-253","ESC-254","ESC-293","ESC-296"]
decisions: []
compared: null
---

## Ce que c'est

Un index local du code de chaque projet. Il fait trouver le code aux agents pour bien moins de tokens que grep ou la lecture de fichiers entiers. Il découpe le code en symboles et en liens, et répond à cinq requêtes : chercher un symbole, le contexte d'une tâche, les appelants, les appelés, l'impact d'un changement. La recherche et le contexte suivent la méthode de tokensave (github.com/aovestdipaperino/tokensave, commit `580936b`, MIT, © 2025 Enzo Lombardi), portée à l'identique, mêmes étapes et mêmes constantes ; sa licence est dans `THIRD-PARTY-NOTICES.md` et les crédits. Aucune recherche par le sens pour le code : elle reste à la mémoire et aux docs. Rien ne quitte la machine, sauf le téléchargement du modèle de la recherche par le sens. La conception est dans `docs/FACTORY.md` (« Le socle d'index local »).

## Comportement

### Ce qu'il lit

- Les fichiers d'un langage connu, découpés en **symboles** : fonctions, méthodes, classes, structs, enums, interfaces, traits, `impl`, types, constantes, champs, modules, composants Svelte, titres Markdown. Chacun avec son conteneur, ses lignes, sa première ligne, sa docstring ou le commentaire au-dessus, et sa **visibilité** (publique, de la crate, du parent, privée), lue comme les extracteurs de tokensave la lisent (`pub` de Rust, `export` et `private`/`protected` de TypeScript, `_` de Python, majuscule de Go, modificateurs de Java, C# et PHP ; un titre Markdown est public). Et en **liens**, par nom : `import`, `call`, `extends`, `implements`.
- Langages : Rust, TypeScript (et TSX), JavaScript, Svelte (son `<script>` lu en TypeScript, ses balises de composants comme des appels), Python, Go, Java, C#, PHP, Markdown (une section par titre, imbriquée selon son niveau).
- Écartés : les fichiers de plus de 400 Kio, les binaires, `*.min.*`, `*.d.ts`, `node_modules`, `target`, `dist` et `vendor`.
- La base, une par projet, vit dans le dossier de données d'Escouade (`projects/<id>/code-index.db`), jamais dans le dépôt. Fermer ou retirer un projet l'efface : elle se recalcule. Quand sa forme change avec une version d'Escouade, l'index d'un projet est relu une fois, sans rien demander.

### L'index plein texte

- Chaque symbole y entre par cinq colonnes, pesées comme chez tokensave : le nom (10), le nom qualifié `chemin::Conteneur::nom` (5), la doc (1), la signature, sa première ligne (2), et les mots d'un nom en camelCase (3). Le tokenizer Porter ramène les mots à leur racine : « sending requests » trouve `send_request`.
- Le corps de chaque fonction et méthode est indexé à part. Une tâche qui décrit un comportement trouve la plus petite fonction dont le corps contient au moins deux de ses mots (par leurs cinq premières lettres).

### Les requêtes (outils MCP)

- `index_search` : le classement de tokensave. Chaque mot par préfixe, joints par « ou » ; une phrase de plusieurs mots cherchée aussi telle quelle, et ces symboles en tête ; si rien, une recherche « contient ». Le nom exact (et la dernière partie d'un `a::b`) s'ajoute. Les définitions (fonctions, types) passent avant les `impl`, puis les constantes et champs, puis les modules et titres ; dans chaque groupe, le score, plus un bonus de genre et 10 pour le nom exact, multiplié selon le chemin (`src/`, `app/`, `lib/` en hausse ; tests, exemples, Markdown, dépendances en baisse). Un filtre par genre de symbole.
- `index_context` : le contexte d'une tâche dite en mots, par le pipeline de tokensave :
  - la tâche lue : ses identifiants (camelCase, snake_case, `a::b`), les composés Pascal et camel de deux mots voisins (« promo banner » → `PromoBanner`), sans les mots vides ; ses mots de quatre lettres et plus ; les `keywords` donnés ;
  - un `Type::Variant` ou une phrase entre guillemets (8 caractères et plus, deux mots) cherchés tels quels dans le code : le plus petit symbole qui les contient est un point d'entrée sûr ;
  - les dix meilleurs symboles de chaque terme, mêlés à tour de rôle ; le nom exact écrit par l'utilisateur à 100 ; les corps des fonctions ; les méthodes d'un type trouvé, à la moitié de son score ;
  - le reclassement multiplicatif : genre, visibilité, fichiers de test et fixtures, tests dans le code, Markdown, `src/`, appelants (par les liens d'aujourd'hui, par nom) ; puis les symboles qui portent plusieurs mots de la tâche ;
  - trois points d'entrée au plus (plus les sûrs), un tiers au plus d'un même fichier ; puis les symboles à un lien d'eux (appelés, appelants, types étendus ou implémentés), 20 en tout par défaut (`limit`, 100 au plus) ; le code de cinq points d'entrée, 1 500 octets chacun au plus, la suite nommée par ses lignes.
  - **`keywords`** : des synonymes de la tâche (pour « authentification » : `login`, `session`, `token`), cherchés chacun ; un mot-clé de plusieurs mots est aussi cherché tel quel dans le code.
  - **Le diagnostic « Retrieval »** (`retrieval`, et la même chose en mots) : la correspondance — `exact` (un nom exact ou une source exacte), `strong` (score de 10 et plus), `fts-only` —, le meilleur score, les résultats de chaque terme et les termes sans résultat, avec le conseil d'ajouter des `keywords` ou d'essayer `index_search`.
- `index_callers` et `index_callees` : les définitions trouvées ; les appelants, avec leur ligne ; les appelés définis dans le projet et, à part, les noms externes.
- `index_impact` : d'un nom ou d'un chemin, les appelants jusqu'à la profondeur donnée, les types qui l'étendent ou l'implémentent, les fichiers qui l'importent, et les fichiers touchés.

Les liens se résolvent par le nom, sans typage : un nom défini à plusieurs endroits est ambigu, et un chemin le précise.

### Ce qui diffère de tokensave

- Le delta des worktrees et la clôture des projets privés : chaque requête, le corps des fonctions et la recherche dans la source compris, ne voit que ce que voit le dossier qui la pose, et ne lit aucun fichier que la clôture refuse.
- Les noms `index_*` des outils, la réponse en JSON avec le diagnostic en mots, en français ou en anglais.
- Les liens par nom jusqu'à leur résolution (IDX-2) : les appelants comptés et les voisins d'un point d'entrée en dépendent.
- Les branches et boucles d'un corps (un petit bonus du corps) se comptent par ses mots : l'index ne garde pas la complexité.
- Un composant Svelte compte comme une classe, un titre Markdown comme un module (comme chez tokensave) ; il n'y a ni nœud de fichier ni doc compagnon à écarter.
- `index_search` garde son filtre par genre ; `index_context` donne toujours le code de ses points d'entrée.

### L'économie

Chaque réponse porte `saving`. On y trouve les tokens des fichiers dont elle est tirée, comme s'ils étaient lus en entier (octets ÷ 4), ceux de la réponse, les tokens gagnés, le pourcentage et le nombre de fichiers. C'est une estimation, face à la lecture de fichiers entiers. Les Stats en font le total par projet : réponses, tokens et dollars économisés sur la période, tous comptes et harness confondus. Les dollars sont comptés au prix d'entrée du dernier modèle sur lequel l'agent a tourné, sinon de son réglage, sinon 0.

### Toujours à jour, sans guetteur

Aucun guetteur de fichiers. L'index se revérifie :

- **à chaque requête**, au plus une fois toutes les 3 s par dossier. Il prend la liste des fichiers de git (suivis, et non suivis non ignorés ; sinon un parcours du dossier) et ne relit que les fichiers dont la taille ou la date a changé ;
- **aux événements git qu'Escouade voit déjà** : un commit, un changement de branche, la fin d'un tour d'agent. La relecture se fait aussitôt, en arrière-plan.

### Worktrees

Le dossier du projet est indexé en entier. Un worktree ne garde que ce qui en diffère : les fichiers changés, ajoutés ou retirés. Une requête dans un worktree voit le projet, moins ces fichiers, plus les siens. Quand le projet prend le changement (une fusion), la différence se vide au passage suivant ; un worktree disparu est oublié. L'index du projet laisse de côté le dossier de ses worktrees et ceux de ses agents. Un dossier qui n'est ni le projet ni l'un de ses worktrees est refusé.

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

- Le banc des sondes (`docs/CODEINDEX-PROBES.md`, `cargo run --release --example codeindex_probes`) mesure la justesse, la vitesse et le coût des cinq outils face à grep, sur 54 sondes ; chaque ticket de l'index y ajoute sa colonne, avant et après.
- Recherche par le sens (build de debug, Windows). Chargement du modèle : 1,2 s. 195 passages courts par seconde, et 27 morceaux par seconde sur de longs documents. Ajouter une section à un document indexé relit 1 morceau sur 30, en 11 ms. Une recherche prend 8 ms sur 506 morceaux, 17 ms sur 1 000, 173 ms sur 10 000 et 860 ms sur 50 000.

## Réglages

- **Réglages › Projets › Tous les projets › « Index du code sur tous les projets »** : coupé par défaut, enregistré aussitôt. Chaque projet le suit tant qu'il n'a pas fait son propre choix. Allumé, les projets qui le suivent sont lus un par un, les autres en attente ; un projet ajouté qui le suit aussi. Coupé, leur index est effacé, après une confirmation qui dit combien ; ceux qui ont fait leur choix le gardent. Ce réglage ne télécharge jamais le modèle de la recherche par le sens.
- **Réglages › Projets › « Index du code »** d'un projet : « Indexer ce projet », enregistré aussitôt. C'est le choix du projet ; il peut revenir au réglage de l'app. Allumé, le projet est lu aussitôt ; coupé, son index est effacé. Le texte dit ce que l'index fait gagner et que rien ne quitte la machine ; pour un projet privé, que son index ne sert qu'à ses agents.
- **« Nouveau projet »**, dernière étape : « Indexer le code », parti du réglage de l'app. Le changer en fait le choix du projet, lu en arrière-plan une fois le projet créé.
- **Réglages › Projets › Index du code › « Recherche par le sens »** : le modèle, son téléchargement avec sa jauge, « Annuler », « Reprendre (N %) » et « Effacer ».
- Ni le `.escouade.yml` ni la copie des réglages d'un projet ne portent ce réglage.
- Après la mise à jour depuis une version où il n'était que par projet : un projet qui l'avait allumé garde un choix allumé ; un projet coupé suit le réglage de l'app.

## Limites

- Les liens se résolvent par le nom, sans typage (jusqu'à IDX-2).
- Les langages hors de la liste ne sont pas indexés.
- La carte d'un agent ne dit pas qu'il s'est servi de l'index ; les Stats le disent.
