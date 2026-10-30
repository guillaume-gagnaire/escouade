# L’index du code

Escouade range le code d’un projet en symboles et en liens, pour que ses agents trouvent ce qu’ils cherchent en un appel, plus vite et pour moins de tokens qu’en lisant des fichiers entiers.

## Ce qu’il fait gagner

Sans index, un agent cherche à coups de grep, puis lit les fichiers qu’il a trouvés, souvent en entier. Avec l’index, il demande en un appel :

- où est un symbole (fonction, méthode, classe, type, champ, titre Markdown), avec sa signature et sa doc ;
- le contexte d’une tâche : les points d’entrée et les symboles liés, avec leurs extraits ;
- les appelants et les appelés d’une fonction, résolus d’un symbole à l’autre à la lecture du code, comme le fait [tokensave](https://github.com/aovestdipaperino/tokensave) : un nom porté par plusieurs fonctions est répondu définition par définition, et un appel qui peut en viser plusieurs est dit à part, jamais deviné ;
- ce qu’un changement toucherait.

Chaque réponse dit ce qu’elle a fait économiser : les tokens des fichiers dont elle tire, lus en entier, contre les siens. Les [statistiques](stats) en font le total par projet.

Les langages lus : Rust, TypeScript, JavaScript, Svelte, Python, Go, Java, C#, PHP et Markdown.

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

Les agents passent par cinq outils du [serveur MCP](mcp), les mêmes pour [tous les harness](harness) : `index_search`, `index_context`, `index_callers`, `index_callees` et `index_impact`. Dans les vues de Claude Code et de Codex, une recherche qui vise un symbole reçoit une ligne qui dit que l’index y répond en un appel ; ailleurs, c’est une règle de leur consigne. Une recherche de texte n’est jamais bloquée.

## Ce qu’il a fait économiser

Les [statistiques](stats) ont une partie **Économies de l’index du code**, sur la période choisie : par projet, le nombre de réponses de l’index, les tokens économisés et leur valeur en dollars. Les dollars sont une estimation : les tokens économisés au prix d’entrée du modèle sur lequel tournait chaque agent.
