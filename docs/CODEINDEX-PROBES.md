# Index du code : sondes face à grep

Mesures du 2026-10-29 (ESC-294) : la justesse, la vitesse et le coût en tokens des cinq outils de l'index du code (`index_search`, `index_context`, `index_callers`, `index_callees`, `index_impact`), face à la recherche par `rg` qu'écrirait un agent, sur ce dépôt. Rien n'est corrigé ici : ces mesures sont le point de départ des tickets qui suivent, et chacun sera mesuré par le même banc, avant et après.

## Rejouer

```sh
cd src-tauri
cargo run --release --example codeindex_probes                                                   # ≈ 3 min une fois construit
cargo run --release --example codeindex_probes -- --compare ../docs/codeindex-probes-reference.json   # face à la référence
```

- **Commit mesuré** : `303fb5f2e447de6eb5bac91ecface391b5c97e75`. Il est écrit dans `docs/codeindex-probes.json` ; `--commit <rev>` en prend un autre, mais les réponses attendues valent pour celui-là.
- Le banc clone le dépôt à ce commit (`git clone --shared`) dans un dossier temporaire, et l'indexe là par un `Service` à lui. L'index de l'app, ses réglages et le profil de l'utilisateur ne sont jamais touchés. Il passe par la bibliothèque, pas par une app lancée : aucun port, aucun serveur MCP.
- Il lui faut un vrai ripgrep : `rg` sur le PATH, ou `--rg <programme>` / `ESCOUADE_PROBES_RG`. Dans le Bash de Claude Code, `rg` est une fonction du shell, pas un programme.
- Options : `--only id1,id2` (quelques sondes), `--out <dossier>`, `--keep` (garde le clone), `--compare <référence>`.
- Sorties, dans `target/codeindex-probes/` :
  - `results.md` : les tableaux de la fin de ce document ;
  - `results.json` : tout, dont chaque résultat ✓/✗ ;
  - `reference.json` : les chiffres qu'une prochaine passe compare.
- **La référence d'aujourd'hui** est `docs/codeindex-probes-reference.json`. Un ticket qui touche l'index rejoue le banc avec `--compare` et donne l'écart : par outil et pour les sondes « tâche », le rappel@10, la précision@10 et les tokens réels, avant et après, puis les sondes dont le rappel a bougé. Les sondes et leurs réponses sont déterministes : deux passes donnent les mêmes chiffres, temps à part.
- Les temps : build release, Windows 11, le poste de l'utilisateur, partagé avec d'autres agents (builds, tests). Les temps bougent de ±50 % d'une passe à l'autre ; les ordres de grandeur, non.

## Ce qui est mesuré, et comment

**Les sondes** (`docs/codeindex-probes.json`, 54) : chacune a une question, les arguments de l'outil, la réponse attendue écrite à la main, et la recherche par `rg` qu'un agent écrirait, en une ou deux tentatives. La réponse attendue se donne en `chemin:ligne`, `chemin:de-à` ou fichier entier ; `a|b` pour deux réponses équivalentes. Elles couvrent :

- les cinq outils ; Rust, TypeScript, Svelte, le Vue du store et le Markdown ;
- des homonymes (`forget`, `new`, `load`, `place`, `scope`, `lock`) ;
- une même question en français et en anglais : la limite de mémoire d'un plugin, la fréquence de la vérification des fichiers, l'index sur tous les projets ;
- des notions dont la question n'a pas les mots du symbole (une attaque par le temps, la fréquence) ;
- un worktree où `tokens_of` devient `estimated_tokens` : sa définition, ses appelants, et l'ancien nom, qui ne doit plus rien donner ;
- la fraîcheur : une fonction ajoutée juste avant l'appel, le dernier contrôle des fichiers ayant moins de 3 s ; puis 3 s plus tard ;
- les deux cas du 2026-10-29 (`cas-memoire-fr`, `cas-memoire-en`, `cas-forget`) ;
- **17 sondes « tâche »** : une question en mots, sans le nom du symbole, comme un agent la pose en début de ticket. Elles portent sur le reste du code : harness et modèles, comptes et quotas, worktrees, Kanban, MCP, terminaux, réglages et i18n, Plugins, store. Huit (marquées `réel`) reprennent le motif cherché par de vrais agents de tickets passés, lu dans leurs sessions Claude Code (la question et le motif seuls).

**Deux greps** pour les sondes « tâche » :

- *informé* : les motifs écrits en sachant la réponse, ou ceux des vrais agents ;
- *à l'aveugle* : d'abord une recherche faite des seuls mots de la question (en anglais, sans dossier, sans casse), puis la recherche informée si celle-là n'a rien trouvé.

Chaque recherche est vue comme la montre le Grep de Claude Code : 250 lignes au plus, une ligne de plus de 500 caractères coupée.

**La justesse** : un résultat est juste s'il tombe dans une réponse attendue (même fichier, ligne dans la plage).

- Précision = résultats justes / résultats ; rappel = réponses attendues trouvées / réponses attendues ; rang = place du premier résultat juste.
- L'index est mesuré à la limite par défaut (10) et à 50 ; grep, sur tout ce qu'il sort.
- Les résultats d'un outil sont, dans l'ordre :
  - `index_search` : ses symboles ;
  - `index_context` : ses points d'entrée, puis ses « liés » ;
  - `index_callers` : ses définitions, puis ses appelants ;
  - `index_callees` : ses définitions, puis ce qu'elles appellent ;
  - `index_impact` : ses cibles, ses appelants, ses sous-types, puis les fichiers qui l'importent.
- Une réponse attendue vide (l'ancien nom dans le worktree) veut dire que rien ne doit sortir.

**Les tokens** : une seule façon de compter, des deux côtés, celle de l'app (`codeindex::tokens_of`) : un token pour quatre octets, arrondi au-dessus.

- *Index* : la réponse telle que le serveur MCP la rend (`result`, `saving`, la phrase « Économie… »). La part des résultats faux est comptée à part : leur JSON, et leur code pour les points d'entrée de `index_context`.
- *grep* : chaque commande écrite, et ce qu'elle sort.
- *Lectures pour conclure*, la même règle des deux côtés :
  - ±20 lignes autour de chaque résultat, sur les 10 premiers, les fenêtres d'un même fichier fusionnées ;
  - une question à une seule réponse : l'agent s'arrête au premier résultat juste (il lit tout s'il n'y en a pas) ;
  - une liste (appelants, appelés, impact) : il lit les 10, chacun étant à vérifier ;
  - un point d'entrée dont `index_context` donne déjà le code n'est pas relu ; un fichier entier se lit par son début ;
  - une ligne est coupée à 2 000 caractères, comme le fait le Read de Claude Code.
- *Coût réel de l'index* : sa réponse et ses lectures ; quand il ne trouve rien de juste, plus le grep informé que l'agent refait.
- *Économie réelle* = (coût grep − coût réel de l'index) / coût grep. L'*économie annoncée* est celle que l'outil affiche : les fichiers lus en entier, face à sa réponse.
- *Appels* (sondes « tâche ») : côté index, l'outil, une lecture par fenêtre, et ceux du grep refait ; côté grep, chaque recherche puis chaque lecture.

**La vitesse** :

- l'index *à froid* : un `Service` neuf (la vérification des fichiers : `git ls-files`, un `stat` de chacun) plus l'appel ;
- l'index *à chaud* : le même aussitôt après (moins de 3 s, donc sans vérification) ;
- `rg`, lancé deux fois : le deuxième temps est le chaud ;
- l'indexation du dépôt entier, le delta d'un worktree, la mise à jour après un commit, après un changement de branche (30 commits en arrière) et au retour.

**Le modèle de la recherche par le sens** : l'index du code n'en utilise aucun. `codeindex` cherche les mots des noms (coupés à la casse et à `_`), des docs et des chemins, par préfixe, avec une liste de mots vides français et anglais. multilingual-e5 ne sert qu'à la mémoire partagée et aux docs (`vectors::`). Les réponses de `index_context` sont donc les mêmes avec ou sans le modèle téléchargé : il n'y a rien à mesurer là. Ce que la recherche par le sens aurait apporté a été mesuré à part, puis abandonné (« Par le sens », plus bas).

## Ce qu'il faut retenir

- **L'« Économie » annoncée ne dit rien de la justesse.** Elle va de 92 à 99 % sur les 54 sondes, y compris quand rien de juste ne sort (26 sondes à rappel nul). Comptée face à ce qu'un agent lit vraiment, avec le grep refait quand l'index ne trouve rien, l'index coûte **2,5 fois** le grep informé : 350 005 tokens contre 139 237 (économie réelle −151 %).
- **`index_context`, l'outil d'un début de ticket, ne trouve presque rien.**
  - Rappel@10 moyen de 16 %, précision de 2 %, et 6 % sur les 17 sondes « tâche » : une sur dix-sept, celle du zoom.
  - Une question en français ramène de la doc française : 16 sondes « tâche » sur 17 ont un fichier Markdown en premier résultat.
  - Ses « liés » donnent 2 résultats justes sur 84, et font passer la réponse de 9,5 ms à 3,8 s (médianes).
- **Les appelants mêlent les homonymes.** `callers` ne se sert de `path` que pour les définitions, et les liens sont pris par le nom seul. Rappel@10 de 17 à 33 % pour `new`, `load` et `place` ; 62 % (précision 45 %) pour `Runtime::forget`.
- **Là où l'index est juste, il est rapide** : 1 à 6 ms à chaud pour les appelants et la recherche d'un nom, contre 13 à 615 ms pour `rg` (45 ms en médiane). À froid, la vérification des fichiers ajoute ≈ 160 ms en moyenne.
- **Face à un grep à l'aveugle** (celui d'un agent qui ne connaît pas le code), l'index reprend l'avantage sur les tâches.
  - Repli sur le grep informé compté, l'index coûte 209 811 tokens et 210 appels ; le grep à l'aveugle, 388 819 tokens et 198 appels (économie réelle de 46 %).
  - Mais si son agent se replie sur le grep à l'aveugle, l'index coûte 504 465 tokens (−29 %) : il ne vaut que par ce qu'il trouve.

## Où l'index gagne, où grep gagne

**L'index gagne** (5 sondes sur 54, plus 1 égalité) :

- **Un titre de doc** (`md-search-socle`) : le titre juste vient en premier, et l'économie réelle est de 39 % ; le grep de « socle » ramène tout le dossier.
- **Les appelants d'un nom unique** (`rs-callers-tokens_of`, `svelte-callers-planbanner`, `wt-callers-renamed`) : 100 % de rappel et de précision, en 1 à 2 ms. Le grep est aussi juste, et le coût se joue sur les lectures, des deux côtés.
- **Une question floue où grep se noie** (`ts-context-tooltip-en`) : le grep de « tooltip » sort 79 lignes avant la bonne. L'index la met 4e, son code dans la réponse (57 % d'économie).
- **Le seul cas de tâche** (`tache-zoom-borne`) : les mots de la question sont ceux du code (`zoom`). L'index trouve `fit_zoom` au 5e rang en un appel, quand le grep informé en fait cinq.
- **Le worktree** : le symbole renommé est vu, et l'ancien nom ne donne plus rien (`wt-*`, 100 %).

**grep gagne** (48 sondes), pour cinq raisons :

1. **Une question en mots** (`index_context`, 26 sondes sur 28) : quand ses mots ne sont ni dans un nom, ni dans une doc, ni dans un chemin, rien ne sort.
   - En français, ce sont les pages françaises du site et des notes de design qui sortent (`cas-memoire-fr`, `rs-notion-gap-fr`, toutes les sondes « tâche »).
   - En anglais, le bon test peut sortir, mais pas la vérification elle-même, qui est au milieu d'une fonction (`cas-memoire-en`).
2. **Les homonymes** (`cas-forget`, `rs-homonyme-*`, `ts-callers-tip-place`, `vue-callers-hueOf`).
   - Les appelants sont pris par le nom seul.
   - Un appelé est résolu vers la première définition du nom dans l'ordre des chemins : le `place` de `definitions.ts` devient celui de `docsdrive.rs`.
   - Sans tenir compte de la casse : `refresh` devient la structure `Refresh`.
   - Et les méthodes des bibliothèques (`get`, `insert`) deviennent des fonctions du projet du même nom (`rs-callees-ensure_fresh`, `ts-callees-definitions`).
3. **Le bruit autour d'un nom exact** (`rs-search-*`, `wt-search-renamed`) : le bon symbole sort premier, puis neuf autres qui partagent un mot (`of`, `tokens`). La réponse coûte 9 à 28 fois la ligne de grep.
4. **Ce que l'index ne lit pas** : les `.vue` du store (`vue-search-plugintile`, et les appelants de `hueOf` dans les composants Vue) ; le corps des fonctions (une condition, une ligne).
5. **La fraîcheur** (`fresh-now`) : une fonction ajoutée moins de 3 s après la dernière vérification n'est pas vue, et la recherche répond dix résultats sans rapport au lieu de « rien ». Trois secondes plus tard, elle est trouvée (`fresh-after-gap`).

**La vitesse de l'index lui-même** :

- le dépôt entier se lit en 20 à 30 s (1 575 fichiers, 34 797 symboles, 293 741 liens, 46,8 Mo) ;
- un commit se rattrape en 0,1 s, un changement de branche de 517 fichiers en 4 à 7 s ;
- mais **la première lecture d'un worktree prend 11 à 16 s pour deux fichiers changés** : elle relit et hache tous les fichiers pour les comparer à la base (`scan.rs`).

## Les pistes, classées

- *Gain* : **mesuré** quand le banc le chiffre, *estimé* sinon.
- *Coût* : faible (quelques heures, sans changer le schéma), moyen (un jour, ou le schéma), élevé (plusieurs jours).
- Chaque piste se vérifie au banc, avant et après, avec `--compare ../docs/codeindex-probes-reference.json`.

### En tête : faire comme tokensave

tokensave a été lu au commit `580936b`, comme une donnée : rien n'en a été lancé.

- Il n'a **aucun** vecteur : il les a retirés (CHANGELOG « Removed: Vector/embedding module ») au profit d'un argument `keywords`.
- Ses gains viennent du classement autour de FTS5.
- Il ne mesure nulle part un rappel@k.

**C'est le plan des tickets qui suivent**, dans cet ordre :

| # | Piste (référence dans tokensave) | Gain | Coût | Sondes qui la montrent |
|---|---|---|---|---|
| T1 | **`keywords` donnés par l'agent**, avec le diagnostic « termes sans résultat » qui le pousse à reposer la question (`mcp/tools/definitions.rs:218-239, 731-735`, `context/formatter.rs:201-233`) | **mesuré** par le mode « mots du code » (la question réécrite avec les mots du code, en anglais) : rappel@10 de 6 à 24 % sur les tâches, de 32 à 59 % sur les autres questions | faible | `tache-compte-limite`, `tache-permission-plugin`, `tache-store-controles`, `tache-xterm-couleurs`, `rs-notion-timing-en`, `svelte-context-banner-fr` |
| T2 | **Reclassement multiplicatif** : fonction ×2, type ×1,5, champ ×0,5 ; pub ×1,5 ; tests ×0,4, fixtures ×0,1 ; `.md` ×0,3 ; `src/` ×1,25 ; plus le nombre d'appelants, en log (`src/context/ranking.rs:4-94, 173-266`) | *estimé* fort sur `index_context` : 16 tâches sur 17 ont un `.md` en premier, et le bon test passe avant le code (`cas-memoire-en`) | faible | toutes les `tache-*`, `cas-memoire-fr`, `cas-memoire-en`, `rs-notion-gap-fr` |
| T3 | **Canal du nom exact** : les mots voisins de la question deviennent snake/camel/Pascal et sont cherchés comme noms (`context/builder.rs:329-373, 935-979`). Chez nous, `task_context` passe `exact = ""` (`api.rs:203`) | *estimé* : « switch on limit » donne `switch_on_limit`, « plan resume » `plan_resume`, « first message » | faible | `tache-compte-limite`, `tache-reprise-limite`, `tache-branche-premier-message`, `tache-cree-agent` |
| T4 | **Porter stemming**, et la signature et le nom qualifié dans FTS ; poids name 10, qualified 5, doc 1, sig 2, parts 3 (`db/migrations.rs:224-228`, `db/queries/search.rs:354`) | *estimé* : « checked » / « checks » face à `check`, « limits » face à `limit` ; le nom qualifié aide les homonymes en recherche | moyen (schéma relevé, index refait) | `cas-memoire-en`, `rs-notion-gap-en`, `rs-search-tokens_of`, `ts-search-tip-place` |
| T5 | **Plein texte dans le corps des fonctions**, ramené à la plus petite fonction englobante (`search.rs:102-194`, `builder.rs:384-399`) | *estimé* fort pour ce qui est une ligne dans une fonction | moyen à élevé (taille de l'index) | `cas-memoire-*` (`limits.memory` dans `spawn`), `tache-ticket-termine` (`Column::Done`), `tache-taille-terminal` (`ResizeObserver`) |
| T6 | **Appels résolus vers un id de symbole** à l'indexation, avec `score_candidate` (même fichier, dossier, langue, import, pub) ; les égalités rangées à part plutôt que devinées (`resolution/resolver.rs:2052-2216`, `extraction/rust_extractor.rs:1461-1486` pour `self.m()`) | *estimé* : la précision des appelants de `forget` passe de 45 % à ≈ 100 %, et les appelés sont justes | élevé (schéma, résolveur, une langue après l'autre) | `cas-forget`, `rs-homonyme-new`, `-load`, `-place`, `ts-callers-tip-place`, `vue-callers-hueOf`, `rs-callees-ensure_fresh`, `ts-callees-definitions` |
| T7 | **Économie plafonnée** à 4 fois la réponse (`mcp/tools/accounting.rs:52-106`) | **mesuré** : l'annoncée est à 96 % en moyenne, la réelle à −151 % | faible | toutes |

### Les autres pistes

| Piste | Gain | Coût | Sondes |
|---|---|---|---|
| **Les hooks d'ESC-253 n'envoient vers l'index que ce qu'il tient** : la recherche d'une déclaration (`fn x`, `class X`) et d'un nom unique. Pas un appel `nom(` / `.nom(`, ni une question en mots, tant que T1 à T6 ne sont pas faits | **mesuré** : `index_search` d'un nom exact trouve à 100 % ; les appelants d'un homonyme, à 17-62 % ; une question de tâche, à 6 % | faible (`hint.rs`) | `rs-search-*` et `svelte-search-planbanner`, face à `cas-forget`, `rs-homonyme-*` et `tache-*` |
| **Couper les « liés » de `index_context`**, ou ne les calculer que pour quelques noms | **mesuré** : 2 justes sur 84 ; 3,8 s contre 9,5 ms (médianes) | faible | `cas-memoire-en`, `rs-notion-*-en`, `vue-context-tile-en` |
| **`index_search` s'arrête au nom exact** quand il y en a un, et ne repasse pas en « un des mots » pour un identifiant introuvable (il répond « rien ») | **mesuré** : 719 tokens au lieu de ≈ 150 pour `tokens_of` ; 539 tokens de bruit pour une fonction pas encore vue | faible | `rs-search-tokens_of`, `wt-search-renamed`, `fresh-now` |
| **`callers` tient `path`**, sans attendre T6 : les appels du fichier de la définition, de ceux qui importent son module, et ceux dont le receveur le dit (`rt.`, `accounts::`) | *estimé* : l'essentiel du gain de T6 sur les appelants | moyen | `cas-forget`, `rs-homonyme-*`, `ts-callers-tip-place` |
| **Noms résolus en respectant la casse**, la définition du même fichier d'abord | **mesuré** : `refresh` résolu vers `Refresh`, `place` vers une fonction d'un autre langage | faible | `rs-callees-ensure_fresh`, `ts-callees-definitions` |
| **L'impact d'un fichier borné** : ses symboles à la limite, puis ce qui l'appelle et l'importe | **mesuré** : 90 cibles avant le premier fichier qui l'importe ; 4 420 tokens, dont 4 250 hors sujet | faible | `ts-impact-definitions-file`, `rs-impact-hint-file` |
| **La fraîcheur sans attendre 3 s** : un fichier écrit par l'agent (hook après Edit/Write) marqué à relire, ou le `stat` des seuls fichiers de la réponse | **mesuré** : rappel de 0 % avant 3 s, de 100 % après | faible à moyen | `fresh-now`, `fresh-after-gap` |
| **La première lecture d'un worktree** par la taille et la date face à la base, ou par `git diff --name-only`, au lieu de tout hacher | **mesuré** : 11 à 16 s pour 2 fichiers changés, contre 0,1 s pour un commit | moyen | tableau « Indexation » |
| **Lire les `.vue`** : leur script, comme pour Svelte | **mesuré** : rappel de 0 % sur les composants du store ; appelants Vue absents | moyen | `vue-search-plugintile`, `vue-callers-hueOf`, `vue-context-tile-en` |
| **Les limites par défaut** : garder 10, une fois T6 fait. À 50, le rappel des appelants passe de 59 à 77 %, mais leur précision s'effondre | **mesuré** | faible | `cas-forget`, `rs-homonyme-place`, `ts-callers-tip-place` |

## Par le sens : mesuré, puis abandonné

L'index du code n'utilise aucun vecteur ; une grille a été jouée dans le banc pour voir ce que la recherche par le sens apporterait.

- Les symboles (genre, nom, chemin, signature, doc) étaient donnés à multilingual-e5, téléchargé dans le dossier du banc.
- Les 28 questions en mots (17 « tâche », 11 fr/en) étaient posées par le sens seul, et mêlées aux mots (fusion des rangs).
- La grille a été arrêtée en cours, sur décision de l'utilisateur : **les vecteurs sont abandonnés, au profit de la méthode de tokensave**, qui les a lui-même retirés. Leur code a quitté le banc.

Ce qui a été mesuré : le rappel@10, et le temps CPU et l'heure murale du calcul des vecteurs du dépôt entier, sur le poste partagé avec d'autres agents.

| Modèle (à télécharger) | Symboles | CPU | Murale | Vecteurs | Question | Tâche : sens / mêlé | fr/en : sens / mêlé |
|---|---|---|---|---|---|---|---|
| Les mots seuls (l'app) | — | — | — | — | — | 6 % | 32 % |
| Les mots du code, sans modèle (T1) | — | — | — | — | — | 24 % | 59 % |
| e5-small int8 (118 Mo) | code (13 057) | 586 s | 51 s | 4,8 Mo | 12 ms | 35 / 29 % | 41 / 14 % |
| e5-small int8 | tous (34 797) | 1 279 s | 119 s | 12,9 Mo | 19 ms | 12 / 6 % | 36 / 32 % |
| e5-small fp32 (470 Mo) | code | 1 053 s | 78 s | 19,1 Mo | 14 ms | **53** / 35 % | 41 / 14 % |
| e5-small fp32 | tous | 2 348 s | 177 s | 51 Mo | 16 ms | 18 / 6 % | 41 / 32 % |
| e5-base int8 (278 Mo) | code | — | 110-117 s | 9,6 Mo | 19 ms | 24-29 / 29-35 % | 18-36 / 23 % |
| e5-base int8 | tous | — | 214-254 s | 25,6 Mo | 37 ms | 24-29 / 12 % | 27-41 / 32 % |
| e5-base fp32 (1,1 Go) | — | non mesuré (grille arrêtée) | | | | | |

- Couper chaque symbole à 128, 256 ou 512 tokens ne change ni le temps ni la justesse : les textes des symboles sont courts.
- Garder le code seul (sans tests, champs, titres ni doublons) triple la justesse des tâches, mais fait perdre les questions dont la réponse est un titre de doc.
- Mêlés aux mots, les vecteurs perdent souvent face au sens seul : les mots ramènent de la doc.
- e5-base n'a pas fait mieux qu'e5-small fp32 sur ces questions.
- Les réglages « Rapide » et « Précis » envisagés n'ont plus d'objet. Pour mémoire, le seul écart notable était e5-small fp32 face à int8 sur le code (53 % contre 35 %), pour 4 fois le téléchargement et 1,8 fois le CPU.
- **Pourquoi l'abandon** :
  - la meilleure combinaison (53 %) demande 470 Mo à télécharger, et 1 à 3 minutes de calcul par dépôt (10 à 40 min de CPU) ;
  - les mots du code, sans modèle (T1), font déjà 24 % sur les tâches et 59 % sur les autres questions ;
  - et tokensave a eu ses gains sans vecteurs.

## Relu à la main

Le banc a été joué en entier, et ses résultats relus à la main sur huit sondes. Pour chacune, chaque résultat ✓/✗ de `results.json` a été contrôlé dans le code :

- `cas-forget` : les 7 appels de `Runtime::forget` sont `commands.rs:328, 348, 388, 433`, `runtime.rs:410` et `plugins_runtime_tests.rs:594, 601`. À 10, la définition et les quatre appels de `commands.rs` sont justes ; `sidechain.rs:1014` et `attachfiles.rs:536` sont d'autres `forget`.
- `rs-callees-ensure_fresh` : `scan.rs:18` est `pub struct Refresh`, donné pour l'appel `scan::refresh` ; `branches_tests.rs:24` est un `open` de test.
- `vue-callers-hueOf` : `src/lib/plugins.ts:283` et `src/lib/sharing.ts:32` sont deux autres `hueOf` (ceux de l'app), et les cinq appels de `StoreAccount.vue` manquent (le Vue n'est pas lu).
- `rs-impact-hint-file` : ses 17 cibles sont les symboles de `hint.rs` (justes) ; `agent.rs:3755` et `agent/claude.rs:1010` appellent d'autres `reply` et `rule`.
- `ts-search-tip-place` : les dix premiers résultats sont les paramètres du `place` de `tip.ts` (lignes 293 à 296) ; la fonction n'arrive qu'au 11e rang.
- `fresh-now` : dix symboles sans rapport (`wizard.ts:383`, `voice.ts:51`…), pour une fonction pas encore vue.
- `tache-modeles-harness` : les titres de `website/content/docs/fr/harness.md` et des notes de design ; aucun code.
- `svelte-context-banner-fr` : les pages du plan du site et des TODO, pas `PlanBanner.svelte`.

## Ticket après ticket

Une colonne par ticket, le même banc sur les mêmes 54 sondes (`--compare` à la colonne précédente). Le détail de chaque colonne : ce qui a été fait, les écarts et pourquoi, sonde par sonde.

- **ESC-294** : la référence (ci-dessous, `docs/codeindex-probes-reference.json`).
- **IDX-1 (ESC-296)** : la recherche et le contexte de tokensave portés à l'identique (`docs/CODEINDEX-IDX-1.md`, référence `docs/codeindex-probes-reference-idx1.json`).
- **IDX-2 (ESC-297)** : la résolution des appels de tokensave, d'un symbole à un symbole (§ « IDX-2 » plus bas, référence `docs/codeindex-probes-reference-idx2.json`).

| Mesure | ESC-294 | IDX-1 | IDX-2 |
|---|---|---|---|
| R@10 moyen, tout (54) | 39 | 38 | 47 |
| P@10 moyen, tout | 23 | 22 | 33 |
| R@50 moyen, tout | 49 | 40 | 46 |
| R@10 `index_search` (9) | 67 | 78 | 78 |
| R@10 `index_context` (28) | 16 | 11 | 11 |
| R@10 `callers` / `callees` / `impact` | 59 / 49 / 85 | 59 / 49 / 85 | **87 / 95 / 95** |
| R@10 sondes « tâche » (17), la question | 6 | 6 | 6 |
| R@50 sondes « tâche » | 18 | 12 | 12 |
| Sondes « tâche » trouvées | 1 / 17 | 1 / 17 | 1 / 17 |
| R@10 sondes « tâche », mots du code | 24 | 18 | 24 |
| R@10 questions en français (5) | 40 | 20 | 20 |
| R@10 questions en anglais (7) | 36 | 29 | 29 |
| R@10 autres questions fr/en (11), mots du code | 59 | 41 | 14 |
| Tokens réels de l'index, tout | 350 005 | 252 722 | 244 487 |
| Tokens réels, sondes « tâche » | 209 811 | 139 003 | 134 450 |
| Économie réelle (totaux) | −151 % | −81 % | −75 % |
| Sondes gagnées : index / grep / = | 5 / 48 / 1 | 4 / 48 / 2 | 8 / 43 / 3 |

## Les mesures

Ce qui suit est le `results.md` du banc, recopié tel quel (ses trois premiers intertitres ajoutés). Le détail de chaque résultat est dans `results.json`, la référence dans `docs/codeindex-probes-reference.json`.

Commit `303fb5f2e447de6eb5bac91ecface391b5c97e75`, 2026-10-30 00:29, ripgrep 15.0.0 (rev 3a612f88b8) ; lecture pour conclure : ±20 lignes autour des 10 premiers résultats.

### L'indexation

| Indexation | Mesure |
|---|---|
| Dépôt entier | 24714 ms, 1575 fichiers, 34797 symboles, 293741 liens, 46.8 Mo |
| Delta d'un worktree (2 fichiers changés) | 14798 ms, 2 fichiers relus |
| Après un commit (1 fichier) | 123 ms, 1 fichier(s) relu(s) |
| Changement de branche (30 commits en arrière, 517 fichiers différents) | 4451 ms, 167 fichiers relus |
| Retour sur la branche | 7175 ms, 390 fichiers relus |

### Par sonde

| Sonde | Outil | Lang. | P@10 | R@10 | Rang@10 | R@50 | Rang@50 | P grep | R grep | Rang grep | Index froid / chaud (ms) | rg (ms) | Tok. réponse (faux) | Tok. lectures index | Tok. grep + lectures | Éco. annoncée | Éco. réelle | Gagnant |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| cas-memoire-en | context | rust | 0 | 0 | — | 0 | — | 24 | 100 | 14 | 2195.1 / 2001 | 31.7 | 2301 (2212) | 4003 | 400 + 2091 | 99 % | -253 % | grep |
| cas-memoire-fr | context | rust | 0 | 0 | — | 0 | — | 24 | 100 | 14 | 307.6 / 5.7 | 11.7 | 2212 (2124) | 2647 | 400 + 2091 | 97 % | -195 % | grep |
| cas-forget | callers | rust | 45 | 62 | 1 | 100 | 1 | 30 | 100 | 6 | 190.9 / 3.1 | 54 | 471 (185) | 3764 | 549 + 3713 | 99 % | 0 % | grep |
| rs-search-tokens_of | search | rust | 10 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 298.6 / 5.5 | 47.5 | 719 (585) | 239 | 26 + 239 | 99 % | -261 % | grep |
| rs-callers-tokens_of | callers | rust | 100 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 146.7 / 2.8 | 79.5 | 196 (0) | 602 | 67 + 602 | 98 % | -19 % | grep |
| rs-search-saving | search | rust | 50 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 162.5 / 2.8 | 60.7 | 193 (59) | 289 | 22 + 289 | 99 % | -54 % | grep |
| rs-callees-ensure_fresh | callees | rust | 45 | 71 | 1 | 71 | 1 | 100 | 100 | 1 | 235.8 / 64.4 | 72.4 | 655 (239) | 3071 | 170 + 1667 | 98 % | -102 % | grep |
| rs-callees-task_context | callees | rust | 18 | 25 | 1 | 62 | 1 | 100 | 100 | 1 | 217.6 / 55.4 | 167.1 | 694 (476) | 3637 | 221 + 2426 | 99 % | -63 % | grep |
| rs-impact-scope_of | impact | rust | 71 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 185.6 / 37.7 | 224.5 | 376 (71) | 2477 | 110 + 1746 | 99 % | -53 % | grep |
| rs-impact-tokens_of | impact | rust | 89 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 168.9 / 29.9 | 219.6 | 375 (8) | 2839 | 181 + 2520 | 96 % | -18 % | grep |
| rs-impact-hint-file | impact | rust | 46 | 40 | 1 | 40 | 1 | 100 | 80 | 1 | 159 / 27.2 | 103.5 | 1781 (415) | 1870 | 106 + 1822 | 99 % | -89 % | grep |
| rs-homonyme-new | callers | rust | 9 | 25 | 1 | 25 | 1 | 43 | 75 | 1 | 127.5 / 15.1 | 44.6 | 394 (270) | 3071 | 155 + 2712 | 99 % | -20 % | grep |
| rs-homonyme-load | callers | rust | 9 | 20 | 1 | 20 | 1 | 16 | 100 | 27 | 294.7 / 3.9 | 167.6 | 527 (386) | 3130 | 698 + 2209 | 99 % | -25 % | grep |
| rs-homonyme-place | callers | rust | 9 | 33 | 1 | 100 | 1 | 100 | 100 | 1 | 133.6 / 2.8 | 81 | 611 (394) | 1711 | 100 + 1214 | 97 % | -76 % | grep |
| rs-notion-timing-en | context | rust | 0 | 0 | — | 0 | — | 17 | 100 | 1 | 2190.5 / 2488.9 | 251.1 | 1823 (1733) | 3703 | 376 + 359 | 98 % | -751 % | grep |
| rs-notion-gap-fr | context | rust | 0 | 0 | — | 0 | — | 60 | 100 | 4 | 173.9 / 6 | 22.1 | 2481 (2392) | 7418 | 240 + 1362 | 99 % | -617 % | grep |
| rs-notion-gap-en | context | rust | 0 | 0 | — | 0 | — | 60 | 100 | 4 | 4323.5 / 4788 | 176.6 | 2973 (2884) | 3713 | 240 + 1362 | 99 % | -417 % | grep |
| rs-context-hint-en | context | rust | 9 | 50 | 1 | 50 | 1 | 60 | 100 | 2 | 4028.8 / 5112.5 | 398.2 | 2697 (2330) | 6589 | 117 + 1805 | 98 % | -383 % | grep |
| ts-search-tip-place | search | ts | 0 | 0 | — | 100 | 11 | 50 | 100 | 2 | 907.1 / 2.7 | 577.7 | 378 (301) | 549 | 44 + 974 | 93 % | -91 % | grep |
| ts-callers-tip-place | callers | ts | 9 | 17 | 1 | 100 | 1 | 24 | 100 | 20 | 869.9 / 4.2 | 108.5 | 544 (394) | 1834 | 631 + 3587 | 97 % | 43 % | grep |
| ts-callees-definitions | callees | ts | 50 | 50 | 1 | 50 | 1 | 89 | 100 | 1 | 298.7 / 71.2 | 263.1 | 644 (275) | 3212 | 299 + 2812 | 98 % | -23 % | grep |
| ts-impact-definitions-file | impact | ts | 75 | 100 | 1 | 100 | 1 | 100 | 67 | 1 | 397.6 / 102.9 | 129.5 | 4420 (752) | 1259 | 67 + 688 | 95 % | -652 % | index |
| ts-context-tooltip-en | context | ts | 9 | 100 | 4 | 100 | 4 | 1 | 100 | 79 | 3723.2 / 3652.4 | 49.6 | 2832 (2522) | 0 | 2914 + 3705 | 98 % | 57 % | index |
| svelte-search-planbanner | search | svelte | 60 | 100 | 1 | 100 | 1 | 50 | 100 | 1 | 186.7 / 4.1 | 18.8 | 487 (190) | 259 | 25 + 517 | 98 % | -37 % | = |
| svelte-callers-planbanner | callers | svelte | 100 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 155.8 / 3 | 203.7 | 150 (0) | 653 | 39 + 911 | 99 % | 15 % | index |
| svelte-context-banner-fr | context | svelte | 0 | 0 | — | 0 | — | 50 | 100 | 1 | 136 / 12.9 | 15.3 | 4821 (4730) | 3931 | 27 + 517 | 97 % | -1608 % | grep |
| vue-search-plugintile | search | vue | 0 | 0 | — | 0 | — | 100 | 100 | 1 | 873.3 / 3 | 26.6 | 490 (412) | 1233 | 19 + 509 | 92 % | -326 % | grep |
| vue-callers-hueOf | callers | vue | 18 | 29 | 3 | 29 | 3 | 88 | 100 | 1 | 212.9 / 3.2 | 276.1 | 449 (281) | 1950 | 242 + 3003 | 97 % | 26 % | grep |
| vue-context-tile-en | context | vue | 0 | 0 | — | 0 | — | 24 | 100 | 5 | 3280.4 / 4182.8 | 218.4 | 1625 (1538) | 4889 | 698 + 2146 | 97 % | -229 % | grep |
| md-search-socle | search | md | 33 | 100 | 1 | 100 | 1 | 4 | 100 | 2 | 129.5 / 2.7 | 88.3 | 282 (148) | 1266 | 1177 + 1385 | 96 % | 39 % | index |
| md-context-allumer-fr | context | md | 14 | 100 | 1 | 100 | 1 | 20 | 100 | 2 | 835.5 / 669.1 | 23 | 2291 (1213) | 0 | 835 + 835 | 98 % | -37 % | grep |
| md-context-turnon-en | context | md | 11 | 100 | 1 | 100 | 1 | 20 | 100 | 2 | 1704.4 / 1893.2 | 76.7 | 2620 (1598) | 0 | 765 + 759 | 97 % | -71 % | grep |
| tache-modeles-harness | context | rust | 0 | 0 | — | 0 | — | 4 | 100 | 33 | 126.1 / 6.7 | 21.2 | 2876 (2785) | 6640 | 987 + 3136 | 98 % | -230 % | grep |
| tache-compte-limite | context | rust | 0 | 0 | — | 0 | — | 4 | 100 | 16 | 403.2 / 12.1 | 40.9 | 3031 (2941) | 5458 | 569 + 2860 | 95 % | -247 % | grep |
| tache-mcp-codex | context | rust | 0 | 0 | — | 0 | — | 0 | 0 | — | 196.6 / 9.2 | 18.4 | 5115 (5025) | 11452 | 1637 + 1500 | 98 % | -528 % | grep |
| tache-branche-premier-message | context | rust | 0 | 0 | — | 0 | — | 1 | 100 | 30 | 682.5 / 630.8 | 44.1 | 3309 (3219) | 4073 | 3683 + 4029 | 98 % | -95 % | grep |
| tache-reprise-limite | context | rust | 0 | 0 | — | 0 | — | 1 | 100 | 82 | 114 / 3.5 | 43.2 | 3164 (3074) | 3680 | 3860 + 2933 | 97 % | -100 % | grep |
| tache-zoom-borne | context | rust | 13 | 100 | 5 | 100 | 5 | 4 | 100 | 55 | 2146.4 / 2123.2 | 83.1 | 1430 (1128) | 0 | 5331 + 1835 | 98 % | 80 % | index |
| tache-xterm-couleurs | context | rust | 0 | 0 | — | 0 | — | 8 | 100 | 9 | 109.2 / 2.5 | 50.1 | 4027 (3937) | 1887 | 272 + 4095 | 97 % | -135 % | grep |
| tache-lance-processus | context | rust | 0 | 0 | — | 0 | — | 40 | 100 | 4 | 107.3 / 8.7 | 55.1 | 3004 (2913) | 5892 | 137 + 1090 | 98 % | -725 % | grep |
| tache-taille-terminal | context | rust | 0 | 0 | — | 0 | — | 8 | 100 | 1 | 130.9 / 4.3 | 54.5 | 2163 (2073) | 6439 | 1342 + 272 | 97 % | -532 % | grep |
| tache-ticket-termine | context | rust | 0 | 0 | — | 0 | — | 1 | 100 | 85 | 239.4 / 6.2 | 126.5 | 4723 (4632) | 3631 | 2834 + 3584 | 97 % | -130 % | grep |
| tache-langue-defaut | context | rust | 0 | 0 | — | 0 | — | 1 | 100 | 25 | 267.8 / 4 | 119.7 | 2536 (2445) | 4843 | 1984 + 3920 | 96 % | -124 % | grep |
| tache-permission-plugin | context | rust | 0 | 0 | — | 100 | 16 | 2 | 100 | 22 | 253.2 / 6 | 59.6 | 1656 (1567) | 4184 | 1531 + 4497 | 96 % | -96 % | grep |
| tache-store-controles | context | rust | 0 | 0 | — | 100 | 39 | 8 | 100 | 23 | 340.6 / 8.6 | 76.7 | 3624 (3534) | 2812 | 289 + 4616 | 95 % | -131 % | grep |
| tache-sauvegarde-etat | context | rust | 0 | 0 | — | 0 | — | 5 | 100 | 11 | 253.2 / 6.9 | 44.4 | 3162 (3070) | 6926 | 580 + 2862 | 97 % | -293 % | grep |
| tache-worktree-cree | context | rust | 0 | 0 | — | 0 | — | 25 | 100 | 2 | 313.8 / 12.6 | 46.9 | 2664 (2572) | 7528 | 431 + 820 | 98 % | -814 % | grep |
| tache-quota-compte | context | rust | 0 | 0 | — | 0 | — | 6 | 100 | 10 | 182.7 / 5.4 | 46.3 | 5133 (5042) | 5305 | 853 + 3719 | 97 % | -228 % | grep |
| tache-cree-agent | context | rust | 0 | 0 | — | 0 | — | 43 | 100 | 3 | 528.8 / 16.9 | 45.8 | 2880 (2789) | 8442 | 134 + 1066 | 98 % | -943 % | grep |
| wt-search-renamed | search | rust | 12 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 626.3 / 1.8 | 67.9 | 613 (475) | 241 | 30 + 241 | 99 % | -215 % | grep |
| wt-callers-renamed | callers | rust | 100 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 178.3 / 1.7 | 44.8 | 200 (0) | 607 | 74 + 607 | 98 % | -18 % | grep |
| wt-callers-old-name | callers | rust | 100 | 100 | — | 100 | — | 100 | 100 | — | 344.5 / 1.6 | 40.2 | 61 (0) | 0 | 8 + 0 | 0 % | -662 % | grep |
| fresh-now | search | rust | 0 | 0 | — | 0 | — | 100 | 100 | 1 | 2.3 / 2.3 | 43 | 539 (458) | 4199 | 29 + 197 | 99 % | -2096 % | grep |
| fresh-after-gap | search | rust | 100 | 100 | 1 | 100 | 1 | 100 | 100 | 1 | 412 / 412 | 44.5 | 136 (0) | 197 | 29 + 197 | 95 % | -47 % | grep |

### Totaux par outil

| Outil | Sondes | P@10 moy. | R@10 moy. | R@50 moy. | P grep moy. | R grep moy. | Index chaud moy. (ms) | rg moy. (ms) | Tok. index (réel) | dont faux | Tok. grep | Éco. annoncée moy. | Éco. réelle (totaux) | Index / grep / = |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| search | 9 | 30 | 67 | 78 | 78 | 100 | 48.5 | 99.3 | 14081 | 2628 | 5949 | 97 % | -136 % | 1 / 7 / 1 |
| context | 28 | 2 | 16 | 23 | 19 | 96 | 988.6 | 91.1 | 287689 | 78022 | 97332 | 97 % | -195 % | 2 / 26 / 0 |
| callers | 10 | 50 | 59 | 77 | 70 | 98 | 4.1 | 68.2 | 20925 | 1910 | 21121 | 88 % | 0 % | 1 / 9 / 0 |
| callees | 3 | 38 | 49 | 61 | 96 | 100 | 63.7 | 81.6 | 11913 | 990 | 7595 | 98 % | -56 % | 0 / 3 / 0 |
| impact | 4 | 70 | 85 | 85 | 100 | 87 | 49.4 | 99.0 | 15397 | 1246 | 7240 | 97 % | -112 % | 1 / 3 / 0 |
| **Tout** | 54 | 23 | 39 | 49 | 48 | 97 | 528.7 | 88.3 | 350005 | 84796 | 139237 | 96 % | -151 % | 5 / 48 / 1 |

### Sondes « tâche »

Grep informé : les motifs écrits en sachant la réponse (huit repris de vrais agents). Grep à l'aveugle : d'abord une recherche faite des seuls mots de la question (en anglais, sans dossier, sans casse), puis la recherche informée si elle n'a rien trouvé.

| Sonde | R@10 index | Rang | Appels index (réel) | Tok. index (réel) | R grep informé | Appels | Tokens | R 1er essai aveugle | Appels aveugle | Tok. aveugle | Éco. / informé | Éco. / aveugle | Gagnant |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tache-modeles-harness | 0 | — | 5 (14) | 9516 (13639) | 100 | 9 | 4123 | 100 | 11 | 30022 | -230 % | 54 % | grep |
| tache-compte-limite | 0 | — | 6 (12) | 8489 (11918) | 100 | 6 | 3429 | 0 | 10 | 24298 | -247 % | 50 % | grep |
| tache-mcp-codex | 0 | — | 6 (10) | 16567 (19704) | 0 | 4 | 3137 | 0 | 11 | 27612 | -528 % | 28 % | grep |
| tache-branche-premier-message | 0 | — | 9 (17) | 7382 (15094) | 100 | 8 | 7712 | 100 | 9 | 16104 | -95 % | 6 % | grep |
| tache-reprise-limite | 0 | — | 6 (12) | 6844 (13637) | 100 | 6 | 6793 | 100 | 10 | 48380 | -100 % | 71 % | grep |
| tache-zoom-borne | 100 | 5 | 1 (1) | 1430 (1430) | 100 | 5 | 7166 | 100 | 9 | 28043 | 80 % | 94 % | index |
| tache-xterm-couleurs | 0 | — | 5 (15) | 5914 (10281) | 100 | 10 | 4367 | 0 | 20 | 12757 | -135 % | 19 % | grep |
| tache-lance-processus | 0 | — | 6 (11) | 8896 (10123) | 100 | 5 | 1227 | 0 | 9 | 27015 | -725 % | 62 % | grep |
| tache-taille-terminal | 0 | — | 5 (7) | 8602 (10216) | 100 | 2 | 1614 | 0 | 12 | 18540 | -532 % | 44 % | grep |
| tache-ticket-termine | 0 | — | 6 (16) | 8354 (14772) | 100 | 10 | 6418 | 0 | 21 | 20011 | -130 % | 26 % | grep |
| tache-langue-defaut | 0 | — | 6 (15) | 7379 (13283) | 100 | 9 | 5904 | 0 | 12 | 7329 | -124 % | -81 % | grep |
| tache-permission-plugin | 0 | — | 5 (17) | 5840 (11868) | 100 | 12 | 6028 | 100 | 5 | 12992 | -96 % | 8 % | grep |
| tache-store-controles | 0 | — | 6 (18) | 6436 (11341) | 100 | 12 | 4905 | 0 | 22 | 14668 | -131 % | 22 % | grep |
| tache-sauvegarde-etat | 0 | — | 6 (11) | 10088 (13530) | 100 | 5 | 3442 | 100 | 11 | 10237 | -293 % | -32 % | grep |
| tache-worktree-cree | 0 | — | 6 (9) | 10192 (11443) | 100 | 3 | 1251 | 0 | 10 | 44522 | -814 % | 74 % | grep |
| tache-quota-compte | 0 | — | 5 (15) | 10438 (15010) | 100 | 10 | 4572 | 100 | 8 | 19009 | -228 % | 21 % | grep |
| tache-cree-agent | 0 | — | 6 (10) | 11322 (12522) | 100 | 4 | 1200 | 0 | 8 | 27280 | -943 % | 54 % | grep |
| **Total (17)** | 6 | | 210 | 209811 | 94 | 120 | 73288 | 41 | 198 | 388819 | -186 % | 46 % | 1 / 16 / 0 |

### Les mots de la question, ou ceux du code

| Sonde | Question R@10 (rang) | Mots du code R@10 (rang) |
|---|---|---|
| cas-memoire-en | 0 (—) | 0 (—) |
| cas-memoire-fr | 0 (—) | 0 (—) |
| rs-notion-timing-en | 0 (—) | 100 (1) |
| rs-notion-gap-fr | 0 (—) | 0 (—) |
| rs-notion-gap-en | 0 (—) | 0 (—) |
| rs-context-hint-en | 50 (1) | 100 (2) |
| ts-context-tooltip-en | 100 (4) | 100 (2) |
| svelte-context-banner-fr | 0 (—) | 100 (2) |
| vue-context-tile-en | 0 (—) | 50 (2) |
| md-context-allumer-fr | 100 (1) | 100 (1) |
| md-context-turnon-en | 100 (1) | 100 (1) |
| tache-modeles-harness | 0 (—) | 0 (—) |
| tache-compte-limite | 0 (—) | 100 (1) |
| tache-mcp-codex | 0 (—) | 0 (—) |
| tache-branche-premier-message | 0 (—) | 0 (—) |
| tache-reprise-limite | 0 (—) | 0 (—) |
| tache-zoom-borne | 100 (5) | 0 (—) |
| tache-xterm-couleurs | 0 (—) | 100 (5) |
| tache-lance-processus | 0 (—) | 0 (—) |
| tache-taille-terminal | 0 (—) | 0 (—) |
| tache-ticket-termine | 0 (—) | 0 (—) |
| tache-langue-defaut | 0 (—) | 0 (—) |
| tache-permission-plugin | 0 (—) | 100 (1) |
| tache-store-controles | 0 (—) | 100 (3) |
| tache-sauvegarde-etat | 0 (—) | 0 (—) |
| tache-worktree-cree | 0 (—) | 0 (—) |
| tache-quota-compte | 0 (—) | 0 (—) |
| tache-cree-agent | 0 (—) | 0 (—) |
| **tâche (17)** | 6 | 24 |
| **autres (fr/en) (11)** | 32 | 59 |


## IDX-2 (ESC-297) : les appels résolus comme tokensave

La résolution des appels de tokensave (github.com/aovestdipaperino/tokensave, commit `580936b`, MIT, © 2025 Enzo Lombardi) est portée dans `src-tauri/src/codeindex/` : `resolver.rs` (les stratégies dans l'ordre de `resolve_one`, `score_candidate`, `find_best_matches`, `explain_ambiguity`, `CROSS_FILE_BLOCKLIST`, le seuil de 0,6, `bare_name_needs_evidence` et `is_plausibly_reachable`, `js_specifier.rs`, `go_import.rs`), `links.rs` (`touched.rs` : la relecture partielle), `receivers.rs` (les receveurs typés des extracteurs Rust, TypeScript et Python), `lang.rs` (les noms des appels et des imports, extracteur par extracteur), `calls.rs` (`symbol_target.rs` : un id, ou un nom répondu définition par définition). Les liens vont d'un id de symbole à un id (`links`), les égalités vont à part (`ambiguous`). Les tests de tokensave pour la résolution sont portés dans `codeindex_links_tests.rs`, avec les nôtres.

### Les écarts, et pourquoi

| Écart | Raison |
|---|---|
| **Le delta des worktrees** : un worktree résout ses références et celles du projet que sa différence peut changer (le même ensemble touché, des symboles qu'il cache et des siens), contre ce qu'il voit ; ses liens masquent ceux du projet pour ces références (`overrides`), refaits quand le worktree ou le projet change | tokensave n'a qu'un dossier ; chez nous la base sert à tous les worktrees, qui n'en gardent que la différence |
| **La clôture des projets privés** : rien de ce qu'elle refuse n'est lu, donc rien n'est candidat | inchangée, elle est meilleure chez nous |
| **Les noms des outils** `index_*`, l'argument `name` (et non `symbol`), `max_matches` ; la forme des réponses (`matches`, `more`, `ambiguous`) ; un appelant par site d'appel (avec sa ligne), et non une fois avec `lines` | nos outils gardent leurs noms ; un site par ligne se lit et se compte comme grep |
| **En Rust, `a.champ.m()` et `self.champ.m()` passent par le type du champ** (`via` 2) : celui des champs de ce nom du fichier de l'appel s'ils sont d'accord, sinon de tous ceux du projet s'ils le sont ; `Arc<T>`, `Box<T>`, `Mutex<T>`… comptent pour `T` | tokensave ne type en Rust que `self`, les paramètres et les `let`. Sans ce pas, les 4 appels de `Runtime::forget` par `core.plugin_runtime` restent ambigus (le critère du ticket : ses 7 appelants) |
| **Les frères d'un appel typé** (`recv.m`, `m`) et leur égalité tombent quand `Type::m` s'est résolu par son type, dans tous les langages | tokensave le fait pour GDScript et C# (`suppress_gdscript_typed_siblings`, `finalize_ambiguous`) ; en Rust il garde le lien du nom nu, qui peut viser une autre méthode du même nom |
| **Un composant Svelte est une cible d'appel**, ses balises l'appellent | notre extracteur Svelte les lit déjà ; celui de tokensave ne lit que le script |
| **Non portés** : les stratégies de Ruby, GDScript, Terraform, VHDL, des en-têtes C/C++, la propagation des variantes de build, et les receveurs typés de C# (`try_csharp_typed_match`) | langages qu'Escouade ne lit pas ; C# passe par le chemin commun (nom qualifié, appel pointé, nom nu), sans ses tables de types |
| **Les imports ne deviennent pas des liens** : ils restent du texte, servent de preuve (`import_index`) et à `index_impact` (les fichiers qui importent) | aucune réponse ne lit un lien d'import ; c'est autant de résolution en moins |
| **Une ambiguïté par site d'appel**, même quand plusieurs références du site (Rust : `recv.m` et `m`) font la même égalité | la réponse dit l'appel une fois |
| **Les appels hors fonction** (le haut d'un fichier, l'initialiseur d'une constante) sont gardés, rattachés à ce qui les contient | notre extracteur les lisait déjà ; tokensave ne lit les appels que dans les corps |
| **La sélection de la relecture partielle se fait en SQL** (`links::select` : le fichier, le nom simple, le champ de `via` 2) ; les références d'un même site sont toujours jugées ensemble | sur ce dépôt, 450 000 références à charger et filtrer coûtaient 2 s par relecture |
| **La limite des fichiers passe de 400 Kio à 1 Mio** (`max_file_size` de tokensave) | `core.rs` (430 Ko) n'était pas indexé du tout |

### Le banc, avant et après

Rejoué le 2026-10-30 sur les mêmes 54 sondes, sans en changer aucune, sur main avec IDX-1 : `cargo run --release --example codeindex_probes -- --compare ../docs/codeindex-probes-reference.json`, puis `--compare ../docs/codeindex-probes-reference-idx1.json`. Sa référence est `docs/codeindex-probes-reference-idx2.json`. Seul l'adaptateur des réponses du banc a suivi la forme nouvelle (`matches`, `more` ; les égalités, dites à part, ne sont pas comptées comme résultats).

| Groupe | Sondes | R@10 ESC-294 | R@10 IDX-1 | R@10 IDX-2 | P@10 ESC-294 | P@10 IDX-1 | P@10 IDX-2 | Tok. réels IDX-1 | Tok. réels IDX-2 |
|---|---|---|---|---|---|---|---|---|---|
| search | 9 | 67 | 78 | 78 | 30 | 26 | 26 | 9 058 | 9 217 |
| context | 28 | 16 | 11 | 11 | 2 | 1 | 1 | 195 429 | 194 590 |
| **callers** | 10 | 59 | 59 | **87** | 50 | 50 | **92** | 20 925 | **14 042** |
| **callees** | 3 | 49 | 49 | **95** | 38 | 38 | **96** | 11 913 | **8 743** |
| **impact** | 4 | 85 | 85 | **95** | 70 | 70 | **80** | 15 397 | 17 895 |
| « tâche » | 17 | 6 | 6 | 6 | 1 | 1 | 1 | 139 003 | 134 450 |
| **Tout** | 54 | 39 | 38 | **47** | 23 | 22 | **33** | 252 722 | **244 487** |

- **Sondes gagnées par l'index** : 4 (IDX-1) → **8** ; grep 48 → 43, égalités 2 → 3. Les appelants : 5 gagnées sur 10, et une économie réelle de +33 % face au grep (0 % avant).
- **Rappel changé face à IDX-1**, en hausse seulement : `cas-forget` 62 → **100** (précision 45 → 100 : les 7 appelants de `Runtime::forget`, rien d'autre), `rs-homonyme-load` 20 → 100, `ts-callers-tip-place` 17 → 100, `rs-callees-task_context` 25 → 100, `ts-callees-definitions` 50 → 100, `rs-homonyme-new` 25 → 75, `rs-homonyme-place` 33 → 67, `rs-callees-ensure_fresh` 71 → 86, `rs-impact-hint-file` 40 → 80. Aucune sonde n'a perdu de rappel.
- **`index_search`, `index_context` et les « tâche »** gardent leur rappel d'IDX-1, sur les questions telles qu'écrites. Le reclassement d'IDX-1 compte désormais les appelants par les liens résolus (et non plus tous les appels du même nom), et les voisins d'un point d'entrée sont ceux des liens. Sur la variante « mots du code » (hors des sondes comptées), trois questions perdent leur bon résultat (`rs-notion-gap-fr`, `rs-notion-gap-en`, `ts-context-tooltip-en`) et une le gagne (`tache-zoom-borne`) : 41 → 14 sur les 11 fr/en, 18 → 24 sur les tâches. La cause n'est pas isolée entre ces deux changements ; c'est à regarder avec IDX-3.
- **Temps à chaud** (moyennes ; le poste était chargé, `rg` y a pris 327 ms en moyenne contre 88 ms à la référence) : appelants 20 ms (4 ms avant), appelés 19 ms, impact 31 ms (49 ms avant).

**L'indexation** (même passe) :

| Indexation | ESC-294 | IDX-1 | IDX-2 |
|---|---|---|---|
| Dépôt entier | 24,7 s, 46,8 Mo | 20,3 s, 68,6 Mo | 31,8 s, 1 578 fichiers (dont `core.rs`, 430 Ko), 35 799 symboles, 93 054 liens résolus, 123,5 Mo |
| Après un commit (1 fichier) | 0,12 s | 0,10 s | 0,25 s |
| Delta d'un worktree (2 fichiers) | 11 à 16 s | 12,6 s | 13,3 s |
| Changement de branche (517 fichiers) | 4 à 7 s | 4,5 s | 7,4 s |

Ce que coûte la résolution : la base grossit de 55 Mo (les ≈ 450 000 références brutes gardées pour la relecture partielle et leurs index, les liens, les égalités), et chaque relecture refait les caches du résolveur sur tous les symboles puis résout de nouveau les références touchées, souvent des dizaines de milliers quand un fichier définit des noms courants (`new`, `get`). Sur le dépôt entier en build de debug, un fichier relu prend 1,6 s dont 0,9 s de résolution.

### Les tests

- **Portés de tokensave** (`codeindex_links_tests.rs`) : `resolution_test.rs` (nom exact, nom qualifié, `resolve_all`, introuvable, plusieurs candidats, `Self::helper`, appel pointé, autre langage sous le seuil), `ambiguous_calls_test.rs`, `ts_phantom_call_edges_test.rs`, `python_phantom_call_edges_test.rs`, `resolution_order_independence_test.rs`, `incremental_resolution_test.rs` (la série de sept changements, relecture partielle égale à une résolution complète ; les clés de l'ensemble touché égales à celles du résolveur), et les tests de `js_specifier.rs` et `go_import.rs`. Plus les poids de `score_candidate`, la liste des noms courants et le seuil.
- **Les nôtres** : les 15 `forget` du dépôt reproduits (`Runtime::forget` a ses 7 appelants, par le nom qualifié, `Type.méthode`, `path` et l'id ; chaque autre `forget` les siens ; aucun appelant compté deux fois), un appel ambigu dit à part et jamais comme appelant, un fichier changé relu seul (1 fichier relu, liens égaux à une résolution complète, une définition retirée emporte ses liens), un worktree (un appel retiré, un ajouté dans un fichier nouveau, une définition déplacée suivie depuis un fichier inchangé, le projet intact, puis le projet qui change), les receveurs typés de Rust, TypeScript et Python. Et, à la main sur le dépôt entier : `cargo nextest run --lib on_this_repository_runtime_forget --run-ignored ignored-only --no-capture` (7 appelants, lus par grep dans les fichiers).
