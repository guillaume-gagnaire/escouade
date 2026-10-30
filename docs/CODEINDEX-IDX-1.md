# Index du code — IDX-1 (ESC-296) : la recherche et le contexte de tokensave

Le 2026-10-30. Le détail de la colonne IDX-1 du banc des sondes d’ESC-294 (`docs/CODEINDEX-PROBES.md`, « Ticket après ticket »). Mêmes 54 sondes, aucune changée ; « avant » est la référence d’ESC-294 (`docs/codeindex-probes-reference.json`, commit mesuré `303fb5f2`), « après » ce ticket. La référence d'après est `docs/codeindex-probes-reference-idx1.json`, pour `--compare` d'IDX-2 et IDX-3.

```sh
cd src-tauri
cargo run --release --example codeindex_probes -- --compare ../docs/codeindex-probes-reference.json
```

## Ce qui a été porté

De tokensave (github.com/aovestdipaperino/tokensave, commit `580936b`, 7.17.0, MIT, © 2025 Enzo Lombardi ; licence et liste des fichiers dans `THIRD-PARTY-NOTICES.md`), à l'identique, mêmes constantes :

| Pièce | tokensave | Escouade |
|---|---|---|
| Tokenizer Porter, colonnes nom 10, nom qualifié 5, doc 1, signature 2, parties du nom 3 | `src/db/migrations.rs`, `src/db/queries/search.rs`, `src/text.rs` | `codeindex/store.rs`, `search.rs`, `text.rs` |
| Corps des fonctions (préfixes de 5 lettres, deux concepts, plus petite fonction) | `search_executable_bodies`, `build_executable_body_documents` | `search.rs`, `scan.rs` |
| La question : identifiants, composés snake/camel/Pascal, mots vides, `Type::Variant`, phrases entre guillemets | `src/context/builder.rs` | `context.rs` |
| Canal de phrase, repli LIKE, nom exact | `search_nodes`, `search_nodes_phrase`, `search_nodes_by_exact_name` | `search.rs` |
| Pipeline du contexte : top-K 10 par terme en tourniquet, nom exact 100, corps, méthodes à 0,5×, reclassement, appelants, cooccurrence, plafond par fichier, 3 racines, profondeur 1, 20 nœuds | `find_entry_points`, `expand_subgraph`, `BuildContextOptions`, handler `tokensave_context` | `context.rs` |
| Reclassement multiplicatif : genre, visibilité, tests, fixtures, Markdown, `src/` | `src/context/ranking.rs` | `ranking.rs` |
| Classement d'`index_search` : rangs de genre, bonus, +10 nom exact, chemin, phrase en tête | `TokenSave::search`, `kind_tier`, `kind_rank_bonus` | `search.rs::ranked_search`, `ranking.rs` |
| `keywords`, diagnostic « Retrieval » | `definitions.rs`, `formatter.rs` | `mcp/index.rs` |
| Visibilité de chaque symbole | les extracteurs (`extract_visibility`…) | `lang.rs::Walker::visibility` |

L'index monte en version (schéma 2) : un projet indexé est relu une fois, sans rien demander. Tests portés de tokensave (`text`, `ranking`, `builder`) et nôtres (`codeindex_tests.rs`, `mcp/index_tests.rs`).

## Les écarts, et pourquoi

- **Worktrees** : chaque requête, le corps des fonctions et la recherche de source exacte compris, ne voit que ce que voit le dossier qui la pose (le delta du worktree). tokensave n'a qu'un dossier.
- **Projets privés** : la source exacte et les extraits ne lisent aucun fichier que la clôture refuse.
- **Noms** : `index_search`, `index_context` ; la réponse est en JSON, le diagnostic en mots en français ou en anglais (`retrieval`), au lieu du Markdown de tokensave.
- **Liens par nom jusqu'à IDX-2** : les appelants comptés par le reclassement et les voisins d'un point d'entrée sont ceux des liens d'aujourd'hui, résolus par le nom (la première définition). Un nom courant (`new`, `send`, `index`) compte donc trop d'appelants ; le bonus est plafonné à 2× comme chez tokensave.
- **Branches et boucles d'un corps** : tokensave les tient de l'arbre (`branches + loops`) ; notre index ne garde pas la complexité, elles sont comptées par les mots du corps (`if`, `match`, `for`, `while`, `loop`…).
- **Genres** : notre genre est un mot, lu comme le `NodeKind` de tokensave ; un titre Markdown est un module (comme chez tokensave), un composant Svelte une classe (tokensave n'en a pas). Il n'y a ni nœud de fichier ni doc compagnon à écarter du lot.
- **`index_search`** garde son filtre par genre, appliqué dans les requêtes.
- **`index_context`** donne toujours le code de ses points d'entrée (`include_code` est faux par défaut chez tokensave ; c'était déjà notre contrat), la suite d'un extrait tronqué nommée par ses lignes au lieu de `tokensave_body`. `limit` y est `max_nodes` : 20 par défaut, 100 au plus, comme chez tokensave.
- Un index corrompu n'est pas reconstruit à la requête : la lecture le refait depuis les fichiers quand son schéma change.

## Les chiffres

Commit mesuré `303fb5f2`, build release, Windows 11, poste partagé avec d'autres agents (les temps sont des temps d'horloge, à lire avec prudence ; la justesse et les tokens n'en dépendent pas). `index_context` est mesuré à `limit` 10 et 50 comme dans la référence (le défaut de l'outil est désormais 20).

### Par outil (R@10 et P@10 moyens, tokens réels de l'index)

| Groupe | Sondes | R@10 avant → IDX-1 | P@10 avant → IDX-1 | R@50 avant → IDX-1 | Tok. index réels avant → IDX-1 |
|---|---|---|---|---|---|
| search | 9 | 67 → 78 | 30 → 26 | 78 → 67 | 14 081 → 9 058 |
| context | 28 | 16 → 11 | 2 → 1 | 23 → 9 | 287 689 → 195 429 |
| callers | 10 | 59 → 59 | 50 → 50 | — | 20 925 → 20 925 |
| callees | 3 | 49 → 49 | 38 → 38 | — | 11 913 → 11 913 |
| impact | 4 | 85 → 85 | 70 → 70 | — | 15 397 → 15 397 |
| **Tout** | 54 | 39 → 38 | 23 → 22 | 49 → 40 | 350 005 → 252 722 |

Gagnant par sonde (index / grep / égalité) : 5 / 48 / 1 → 4 / 48 / 2. Économie réelle (totaux) : −151 % → −81 % (l'index coûte 1,8 fois grep au lieu de 2,5).

### Sondes « tâche » (17)

| | Avant | IDX-1 |
|---|---|---|
| R@10 moyen (question) | 6 | 6 |
| R@50 moyen | 18 | 12 |
| R@10 avec les mots du code | 24 | 18 |
| Trouvées (sur 17) | 1 | 1 (`tache-zoom-borne`, rang 5 → 2) |
| Tokens réels de l'index | 209 811 | 139 003 |
| Économie / grep informé | −186 % | −89 % |
| Économie / grep à l'aveugle | 46 % | 64 % |

### Les autres questions, français et anglais à part

| | Sondes | R@10 avant → IDX-1 | R@50 avant → IDX-1 |
|---|---|---|---|
| en français | 5 | 40 → 20 | 40 → 0 |
| en anglais | 7 | 36 → 29 | 36 → 7 |
| fr/en, avec les mots du code | 11 | 59 → 41 | — |

Rappel changé sonde par sonde : `rs-notion-gap-en` 0 → 100 ; `ts-search-tip-place` 0 → 100 ; `rs-context-hint-en` 50 → 0 ; `md-context-allumer-fr` 100 → 0 ; `md-context-turnon-en` 100 → 0.

### Ce que disent ces chiffres

- `index_search` gagne (67 → 78 % de rappel, un tiers de tokens en moins) : Porter, les cinq colonnes et le nom exact font leur part.
- `index_context` répond un tiers moins cher, mais trouve moins. Trois raisons, toutes de la méthode portée :
  - le Markdown est multiplié par 0,3 (comme chez tokensave) : les deux sondes de documentation tombent ;
  - trois racines seulement, et leurs voisins remplissent le reste : à 50, une racine très appelée occupe tout, d'où un R@50 plus bas que le R@10 ;
  - les appelants sont comptés par le nom, en attendant IDX-2 : une fonction au nom courant monte.
- Les questions en français ne trouvent presque rien : tokensave ne lit que l'anglais (Porter anglais, mots vides anglais), et le code est écrit en anglais. C'est le rôle de `keywords` (des synonymes en anglais) ; les sondes n'en passent pas, n'ayant pas été changées.
- Les outils d'appels (`callers`, `callees`, `impact`) ne changent pas : ils sont d'IDX-2.
