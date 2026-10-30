# L’index du code

Escouade range le code d’un projet en symboles et en liens, pour que ses agents trouvent ce qu’ils cherchent en un appel, plus vite et pour moins de tokens qu’en lisant des fichiers entiers.

## Ce qu’il fait gagner

Sans index, un agent cherche à coups de grep, puis lit les fichiers qu’il a trouvés, souvent en entier. Avec l’index, il demande en un appel :

- où est un symbole (fonction, méthode, classe, type, champ, titre Markdown), avec sa signature et sa doc ;
- le contexte d’une tâche : les points d’entrée et les symboles liés, avec le code des points d’entrée (cinq blocs au plus, de 1 500 octets au plus chacun ; un bloc coupé nomme l’appel qui donne la suite) ;
- le corps entier d’un symbole, par son nom ou son id ;
- un texte dans le code (un message d’erreur, une chaîne), fichier par fichier et ligne par ligne ;

Chaque réponse finit par ce qu’elle a fait économiser, compté comme le compte tokensave : les fichiers qu’elle a touchés, lus en entier, mais **plafonnés à quatre fois la réponse**, contre ce que l’agent a payé pour l’appel (la réponse, sa requête et, une fois par session, les descriptions des outils). Un appel qui coûte plus qu’il n’économise reporte son déficit sur les suivants. Les [statistiques](stats) font le total de ce qui reste, par projet.

Les langages lus : Rust, TypeScript, JavaScript, Svelte, Python, Go, Java, C#, PHP et Markdown.

## Comment il cherche

L’index cherche par les **mots du code**, comme [tokensave](https://github.com/aovestdipaperino/tokensave) à son commit `580936b`, dont Escouade reprend la méthode : un index plein texte des noms, noms qualifiés, docs, signatures et corps des fonctions, pondéré par colonne, puis un classement par genre, visibilité et appelants. Il n’y a **pas de vecteurs pour le code** : ils ont été mesurés sur 54 sondes du code d’Escouade, puis abandonnés.

Ce qui en découle :

- une question dite avec les identifiants du code (en anglais ici) trouve mieux qu’une paraphrase ; les descriptions des outils demandent aux agents de l’écrire ainsi ;
- quand un terme ne trouve rien, la réponse le dit (`### Retrieval`), et l’agent redemande avec des synonymes dans `keywords` ;
- un texte qui n’est pas un symbole (un message, une chaîne) se trouve par la recherche littérale, pas par la recherche classée ;
- les chemins de `.escouade/queryignore` (un motif par ligne, `*` pour un glob, `#` pour un commentaire) sont laissés hors des recherches, jamais de l’index ; le fichier est lu dans le dossier de l’agent, jamais par un lien qui en sort ;
- grep gagne encore là où il faut une regex, ou quand les mots d’une tâche ne sont pas ceux du code.

## L’allumer

L’index est **coupé par défaut**. Escouade le propose :

- pour tous les projets d’un coup, dans **Réglages › Projets › Tous les projets** (**Index du code sur tous les projets**) ;
- à l’ajout d’un projet, à la dernière étape de l’assistant « Nouveau projet » (**Indexer le code**), qui part de ce réglage ;
- dans **Réglages › Projets › Index du code** (**Indexer ce projet**), projet par projet, à tout moment.

Allumé, le projet est lu une fois, puis tenu à jour sans guetteur de fichiers : à chaque question des agents (au plus toutes les 3 secondes), aux commits, aux changements de branche et aux fins de tour. Le réglage dit ce qu’il contient : ses fichiers, ses symboles, sa taille et quand il a été lu. Le couper efface l’index.

### Sur tous les projets

Chaque projet suit **Index du code sur tous les projets** tant qu’il n’a pas fait son propre choix. Un projet allumé ou coupé dans ses réglages garde son choix, quoi que dise la case de l’app ; sa section dit d’où vient son état (« Suit le réglage de l’app » ou « Choisi pour ce projet ») et **Suivre le réglage de l’app** le lui rend.

- **Allumer la case** lit les projets qui la suivent un par un, jamais tous à la fois, sans bloquer la fenêtre. Un message dit combien seront lus ; la section de chacun dit s’il attend son tour, s’il est en lecture, puis ce qu’il contient.
- **La couper** efface l’index des projets qui la suivent. Escouade demande d’abord, en disant combien de projets perdent leur index ; ceux qui ont fait leur propre choix gardent le leur.
- Le modèle de la [recherche par le sens](#la-recherche-par-le-sens) ne se télécharge jamais pour cette case : il reste à télécharger à la main.

Un projet allumé avant cette case garde son index, comme un choix à lui ; un projet qui était coupé suit désormais la case.

Les worktrees des agents ne sont pas indexés en entier : l’index de la branche de base sert à tous, et chaque worktree n’ajoute que ce qui en diffère.

## Rien ne quitte la machine

L’index est calculé sur ta machine et gardé dans le [dossier d’Escouade](data), jamais dans le dépôt ni dans un service en ligne. Seuls les agents en reçoivent des extraits, comme ils reçoivent déjà du code. Les fichiers ignorés par git ne sont pas lus.

## Les projets privés

L’index d’un [projet privé](mcp#les-projets-prives) ne sert qu’à ses propres agents : ceux des autres projets n’y ont jamais accès, ni par le nom ni par l’id. Ce que sa clôture refuse n’est jamais lu ni indexé ; quand un autre projet est rangé dans son dossier, le réglage le nomme : « Laissé hors de l’index : … ». De même, un projet ouvert qui contient un projet privé dans son dossier le laisse hors de son index.

## La recherche par le sens

En plus des mots, la [mémoire partagée](shared-memory) et les docs d’un projet se cherchent par le sens, avec un petit modèle multilingue (multilingual-e5-base, licence MIT) qui tourne sur ta machine. Il se télécharge une fois pour tous les projets, depuis **Réglages › Projets › Index du code › Recherche par le sens** (282 Mo) :

- une jauge montre où en est le téléchargement, et **Annuler** l’arrête ;
- ce qui est venu est gardé : **Reprendre** repart de là ;
- le fichier est vérifié par son empreinte SHA-256, et passe par le proxy des [réglages réseau](settings-network) ;
- **Effacer** le retire ; la recherche se fait alors par les mots seuls.

Une fois le modèle téléchargé, rien ne sort plus de la machine.

## Ce que voient les agents

Les agents passent par six outils du [serveur MCP](mcp), les mêmes pour [tous les harness](harness) : `index_search`, `index_context`, `index_body`, `index_callers`, `index_callees` et `index_impact`. Leurs réponses sont celles de tokensave : du Markdown pour le contexte, une ligne par symbole pour la recherche, le corps en texte. Dans les vues de Claude Code et de Codex, une recherche qui vise un symbole reçoit une ligne qui dit que l’index y répond en un appel ; ailleurs, c’est une règle de leur consigne. Une recherche n’est jamais bloquée : à la différence de tokensave, Escouade n’arrête ni un grep ni un agent Explore.

## Ce qu’il a fait économiser

Les [statistiques](stats) ont une partie **Économies de l’index du code**, sur la période choisie : par projet, le nombre de réponses de l’index, les tokens économisés (comptés comme plus haut, les déficits payés d’abord) et leur valeur en dollars. Les dollars sont une estimation : les tokens économisés au prix d’entrée du modèle sur lequel tournait chaque agent.