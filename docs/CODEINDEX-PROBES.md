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
- Options : `--only id1,id2` (quelques sondes), `--out <dossier>`, `--keep` (garde le clone), `--compare <référence>`, `--tokensave <exe> --tokensave-project <copie sans .git>` (les mêmes sondes posées à tokensave, voir « tokensave face à face »).
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

- **Avec de vrais agents (IDX-4, 2026-10-30)** : face à grep seul, l'index d'Escouade et son hook font dépenser 8 % de moins à Haiku, à justesse égale (−13 % sur les questions, +14 % sur les changements), et tokensave 18 % de plus. Détail en fin de document.
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
- **IDX-3 (ESC-298)** : les réponses, le compte de l'économie et les consignes de tokensave, et la suffisance des réponses « tâche » (§ « IDX-3 » plus bas, référence `docs/codeindex-probes-reference-idx3.json`).
- **IDX-4 (ESC-299)** : pas de changement de l'index, donc pas de colonne ; de vrais agents jouent 20 tâches avec et sans l'index, et avec tokensave (§ « Avec de vrais agents » en fin de document).

| Mesure | ESC-294 | IDX-1 | IDX-2 | IDX-3 |
|---|---|---|---|---|
| R@10 moyen, tout (54) | 39 | 38 | 47 | 47 |
| P@10 moyen, tout | 23 | 22 | 33 | 33 |
| R@50 moyen, tout | 49 | 40 | 46 | 46 |
| R@10 `index_search` (9) | 67 | 78 | 78 | 78 |
| R@10 `index_context` (28) | 16 | 11 | 11 | 11 |
| R@10 `callers` / `callees` / `impact` | 59 / 49 / 85 | 59 / 49 / 85 | **87 / 95 / 95** | 87 / 95 / 95 |
| R@10 sondes « tâche » (17), la question | 6 | 6 | 6 | 6 |
| R@50 sondes « tâche » | 18 | 12 | 12 | 12 |
| Sondes « tâche » trouvées | 1 / 17 | 1 / 17 | 1 / 17 | 1 / 17 |
| R@10 sondes « tâche », mots du code | 24 | 18 | 24 | 24 |
| R@10 questions en français (5) | 40 | 20 | 20 | 20 |
| R@10 questions en anglais (7) | 36 | 29 | 29 | 29 |
| R@10 autres questions fr/en (11), mots du code | 59 | 41 | 14 | 14 |
| Tokens des réponses, tout | 99 558 | 57 313 | 62 325 | **44 360** |
| Tokens réels de l'index, tout | 350 005 | 252 722 | 244 487 | **227 524** |
| Tokens réels, sondes « tâche » | 209 811 | 139 003 | 134 450 | 125 884 |
| Économie annoncée (moyenne) | 96 % | 93 % | 92 % | **67 %** |
| Économie réelle (totaux) | −151 % | −81 % | −75 % | −63 % |
| Sondes gagnées : index / grep / = | 5 / 48 / 1 | 4 / 48 / 2 | 8 / 43 / 3 | 9 / 42 / 3 |
| Suffisance « tâche » : lignes nécessaires dans la réponse | 1 % | 1 % | 1 % | 1 % |
| Suffisance « tâche » : un appel à l'index suffit | 0 / 17 | 0 / 17 | 0 / 17 | 0 / 17 |
| Suffisance « tâche » : appels (tokens) de plus | 67 (35 718) | 68 (36 153) | 66 (32 189) | 66 (32 189) |

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

## IDX-3 (ESC-298) : les réponses, l'économie et les consignes, comme tokensave

Portés de tokensave (commit `580936b`, MIT, © 2025 Enzo Lombardi ; fichiers listés dans `THIRD-PARTY-NOTICES.md`) :

- **Les réponses** (`codeindex/format.rs`, `codeindex/answers.rs`) : `index_context` en Markdown (`format_context_as_markdown` : points d'entrée, symboles liés par fichier, 5 blocs de code de 1 500 octets au plus, `### Retrieval`, `seen_node_ids`), un bloc coupé finissant par `... [truncated — full body: index_body node_id=N]` (`truncate_code_block`) ; `index_search` une ligne par symbole (`count: N`, `chemin:ligne-fin: nom (genre) — signature`, l'id avec `ids`), et en mode `literal` un texte exact `chemin:ligne: texte` (`handle_literal_search`, `exact_total`, `timeout_ms`) ; le nouvel outil `index_body` (`handle_body` : par id ou par nom, la meilleure sorte, la liste des candidats quand le nom est ambigu) ; `exclude_node_ids`, `path`, `path_include`, `path_exclude`, `max_code_blocks`, `max_code_lines`, `max_per_file`, `include_code` ; `.escouade/queryignore` (`QueryIgnore`) ; la coupe des réponses à 15 000 caractères (`truncate_response`, `truncate_response_keep_tail`).
- **L'économie** (`codeindex/accounting.rs`, compte de `src/mcp/server.rs`) : before = fichiers touchés ÷ 4, plafonnés à 4 fois la réponse (à la réponse pour `literal`) ; after = réponse + requête (nom, arguments, 8 d'enveloppe) + schémas des outils une fois par session + la ligne de mesure ; un déficit reporté sur les appels suivants (`settle_session_debt`). Les Stats suivent ce qui est crédité (colonne `saved_tokens`).
- **Les descriptions** des outils (`definitions.rs`, `CONTEXT_DESCRIPTION`) : fixes (aucun chiffre de l'index n'y entre), elles demandent les mots du code (en anglais ici), `keywords` quand un terme ne trouve rien, `seen_node_ids` → `exclude_node_ids`.
- **Les tests portés** : ceux de `accounting.rs` (le plafond, le littéral, la requête, le déficit sur une session), de `formatter.rs` (en-tête, points d'entrée, blocs et langage, Retrieval, tests repliés, docstring, `compact_signature`), de `builder.rs` (la coupe et sa poignée, multioctet, `max_code_lines`), de `config.rs` (`QueryIgnore`). Les nôtres : les six outils par un vrai client MCP (`mcp/index_tests.rs` : texte, ids, littéral, Markdown, `exclude_node_ids`, filtres, `include_code: false`, `index_body` par nom et par id, ligne de mesure, `queryignore`, descriptions identiques d'une liste à l'autre), un `queryignore` lié hors du dossier refusé, la session et ses schémas.

### Les écarts, et pourquoi

| Écart | Raison |
|---|---|
| **`include_code` vaut vrai par défaut** (tokensave : faux dans l'outil, vrai dans `BuildContextOptions`) | le ticket veut que l'agent ait le code sans relire : la réponse sans code laisserait la suffisance à 0 |
| **La ligne de mesure toujours envoyée** (tokensave la coupe par défaut, `report_savings`, #561), avec une phrase en français ou en anglais | chaque réponse d'Escouade dit ce qu'elle a fait économiser depuis ESC-253 ; son coût est compté dans `after` |
| **La session est celle de l'appelant** (un agent, ou les clients hors Escouade), pas une connexion MCP ; les schémas comptés sont ceux des six outils `index_*`, chargés d'emblée (pas d'`anthropic/alwaysLoad`) | le serveur d'Escouade fait un handler par requête et sert tous les outils d'Escouade, dont l'index n'est qu'une part |
| **Le hook ne refuse rien** : tokensave refuse certains grep et les agents Explore ; Escouade conseille l'index et laisse faire | règle d'ESC-253 ; le blocage ne serait repris que sur une décision de l'utilisateur |
| **La recherche littérale** lit les fichiers que voit le dossier, l'un après l'autre, à travers la clôture ; pas de rapport `unscanned` | le delta des worktrees et la clôture des projets privés ; notre index liste les fichiers de git, il n'en ignore pas qu'il saurait lire |
| **`.escouade/queryignore`** (tokensave : `.tokensave/queryignore`), lu dans le dossier de l'agent, jamais par un lien qui en sort ni quand la clôture le refuse | le dossier d'Escouade dans le dépôt ; un lien ne doit pas faire lire un fichier hors du projet |
| **Les noms `index_*`, des ids en nombres, les phrases fr/en** ; non portés : `merge_adjacent`, le mode `plan`, le format `json` de la recherche et du corps | nos outils gardent leurs noms ; ces options ne sont pas demandées par le ticket |
| `index_callers`, `index_callees`, `index_impact` gardent leur JSON (celui d'IDX-2) | tokensave répond aussi ces trois-là en JSON |

### Le banc, avant et après

Rejoué le 2026-10-30 sur les mêmes 54 sondes, sans en changer aucune (`--compare ../docs/codeindex-probes-reference-idx2.json`) ; référence `docs/codeindex-probes-reference-idx3.json`. Le banc compte désormais **la réponse telle que le serveur MCP l'envoie** (le texte de `format.rs` et sa ligne de mesure, en français, un appel dans une session déjà ouverte), et non plus un JSON de bibliothèque ; l'économie annoncée est celle de la ligne `index_metrics`.

| Groupe | Sondes | R@10 IDX-2 → IDX-3 | P@10 | Tok. des réponses IDX-2 → IDX-3 | Tok. réels IDX-2 → IDX-3 | Éco. annoncée IDX-3 | Éco. réelle IDX-3 | Index / grep / = |
|---|---|---|---|---|---|---|---|---|
| search | 9 | 78 → 78 | 26 | — | 9 217 → 8 284 | 54 % | −39 % | 1 / 6 / 2 |
| context | 28 | 11 → 11 | 1 | — | 194 590 → 179 588 | 72 % | −84 % | 2 / 26 / 0 |
| callers | 10 | 87 → 87 | 92 | — | 14 042 → 13 648 | 60 % | +35 % | 5 / 5 / 0 |
| callees | 3 | 95 → 95 | 96 | — | 8 743 → 8 634 | 72 % | −13 % | 0 / 2 / 1 |
| impact | 4 | 95 → 95 | 80 | — | 17 895 → 17 370 | 72 % | −139 % | 1 / 3 / 0 |
| « tâche » | 17 | 6 → 6 | 1 | — | 134 450 → 125 884 | 72 % | −71 % | 1 / 16 / 0 |
| **Tout** | 54 | 47 → 47 | 33 | 62 325 → **44 360** | 244 487 → **227 524** | **67 %** (92 % avant) | **−63 %** (−75 % avant) | **9 / 42 / 3** |

- **La justesse ne bouge pas, et ne devait pas bouger** : IDX-3 change ce que rendent les outils et comment l'économie est comptée, pas le classement. Aucune sonde n'a changé de rappel.
- **Les réponses coûtent 29 % de tokens de moins** (62 325 → 44 360) : le Markdown et une ligne par symbole au lieu du JSON. Le coût réel baisse de 7 % (les lectures et le repli sur grep restent).
- **L'économie annoncée passe de 92 % à 67 %** grâce au plafond à 4 fois la réponse et aux coûts de l'appel. Elle reste très loin de l'économie réelle (−63 %) : elle compare la réponse à la lecture des fichiers touchés, alors que l'agent, lui, aurait lu des morceaux trouvés par grep, et elle ne sait pas si la réponse était juste. C'est la définition de tokensave, portée telle quelle.
- **Les sondes gagnées** : 9 (8 avant), par la seule taille des réponses : `svelte-search-planbanner` passe de l'égalité à l'index, `ts-search-tip-place` de grep à l'égalité.
- **L'indexation** (même passe, poste chargé par trois builds) : dépôt entier 33,5 s, 123,5 Mo, 93 054 liens ; delta d'un worktree 15,1 s ; commit 0,21 s ; changement de branche 7,5 s. **À noter, sans le corriger ici** : la base est passée de 47 à 123 Mo avec IDX-2, et le delta d'un worktree prend toujours 13 à 15 s.

### La suffisance des réponses « tâche »

La question de l'utilisateur (2026-10-30) : la réponse donne-t-elle à l'agent le code dont il a besoin, pour qu'il n'ait pas à lire, voir qu'il en manque, et re-grep ? Pour chacune des 17 sondes « tâche », l'ensemble du code nécessaire est écrit à la main dans `docs/codeindex-probes-sufficiency.json` (2 à 5 plages : le corps de la fonction, le type ou l'aide qu'elle prend, un appelant, le test qui la couvre). Le banc mesure la part de ces lignes que la réponse (ses blocs de code) contient déjà, puis les appels de plus pour les avoir toutes : `index_body` pour un bloc coupé d'un symbole qui les tient (IDX-3 seulement), un Read des lignes manquantes d'un fichier que la réponse nomme, sinon le grep informé puis un Read. Face à lui, le chemin de grep : ses recherches, puis un Read par plage. Les versions d'avant sont rejouées à leur commit (`8e7d2be2`, `4ea19200`, `5a2e825a`), leur banc patché pour écrire chaque réponse (`--old <label>=<dossier>`).

| Version | Lignes nécessaires dans la réponse | Un appel à l'index suffit | Appels de plus (tokens) | Grep informé (appels, tokens) | Grep à l'aveugle |
|---|---|---|---|---|---|
| ESC-294 | 1 % | 0 / 17 | 67 (35 718) | 71 (41 908) | 80 (116 038) |
| IDX-1 | 1 % | 0 / 17 | 68 (36 153) | | |
| IDX-2 | 1 % | 0 / 17 | 66 (32 189) | | |
| **IDX-3** | **1 %** | **0 / 17** | **66 (32 189)** | 71 (41 908) | 80 (116 038) |

- **Aucune réponse ne suffit, dans aucune version.** Deux sondes seulement tiennent une part du code : `tache-zoom-borne` (23 %, la seule « tâche » trouvée) et `tache-reprise-limite` (25 %, depuis IDX-2). Les 15 autres ne contiennent aucune des lignes nécessaires : l'agent doit tout chercher ailleurs. Avec l'index, cela fait en plus de son appel 66 appels et 32 189 tokens ; le grep informé y arrive en 71 appels et 41 908 tokens au total, sans l'appel à l'index.
- **La cause est la justesse, pas le format** : les blocs de code sont ceux des points d'entrée, et 16 « tâche » sur 17 ont de mauvais points d'entrée. IDX-3 rend le bloc et sa suite (`index_body`) ; il ne peut pas rendre un code qu'il n'a pas trouvé.

### Où grep gagne encore, et pourquoi

grep gagne 42 sondes sur 54 (43 avant).

- **Les tâches dites en mots (`index_context`, 26 sur 28 pour grep)** : la cause principale des 16 « tâche » ratées sur 17 et du rappel de 11 %. Les questions sont des paraphrases françaises (« où la langue de l'interface est choisie par défaut d'après le système ») ; l'index plein texte cherche des mots du code, en anglais, réduits par le stemmer Porter (anglais). Une paraphrase ne touche presque aucun nom, et les trois points d'entrée (`search_limit` 3 de tokensave) se remplissent de symboles qui partagent un mot courant. Dite avec les mots du code, la même tâche passe de 6 à 24 % (4 sur 17) : c'est ce que les descriptions demandent désormais aux agents, et que seul un vrai agent dira (IDX-4). Le grep informé, lui, part d'un identifiant qu'il connaît déjà.
- **La recherche d'un symbole (`index_search`, 6 sur 9)** : à justesse égale, grep coûte moins quand le nom est rare et le fichier court ; l'index gagne sur les homonymes et les noms communs.
- **Les appelés et l'impact** (2 et 3 sur 3 et 4) : justes à 95 %, mais l'impact rend tout ce qu'il touche (fichiers, types, importeurs) et coûte plus que le grep d'un nom.
- **Les appelants** sont partagés (5 / 5) : l'index gagne sur les homonymes (`forget`, `load`, `new`, `place`), grep sur un nom unique.

**Pourquoi « mots du code » est tombé de 41 à 14 avec IDX-2** (sur les 11 questions fr/en ; rejoué sur IDX-1 et IDX-2 avec leur banc, les résultats écrits) : IDX-1 comptait les appelants d'un symbole **par son nom** (`batch_incoming_call_counts` sur `edges`, `lower(target)`). `scan.rs:29` (`refresh`) recevait ainsi tous les appels nommés `refresh` du dépôt, montait en tête, et son voisin par le nom, `Service::ensure_fresh` (`mod.rs:135`, la bonne réponse de `rs-notion-gap-fr` et `-en`), entrait dans les symboles liés. IDX-2 compte les appelants résolus, comme tokensave : `refresh` n'a plus que ses vrais appelants, ne remonte plus, et `ensure_fresh` disparaît des dix premiers. Même chose pour `ts-context-tooltip-en` (`tip.ts:180`, troisième point d'entrée sous IDX-1 par le même bonus). Les bons résultats d'IDX-1 venaient d'un bonus faux ; IDX-3 rend exactement les résultats d'IDX-2 sur ces questions.

### Rejouer

```sh
cd src-tauri
cargo run --release --example codeindex_probes -- --compare ../docs/codeindex-probes-reference-idx2.json \
  --old "ESC-294=<dossier>" --old "IDX-1=<dossier>" --old "IDX-2=<dossier>"
```

`--old` prend un dossier de réponses `<id>.txt` d'une version d'avant (sous Windows, un chemin `C:/…` : l'exécutable ne lit pas `/c/…`). Pour les refaire : un `git worktree` au commit de la version, son `probes/mod.rs` patché pour écrire `response(&a10)` dans `$ESCOUADE_PROBES_DUMP/<id>.txt` juste avant `index_side`, puis `cargo run --release --example codeindex_probes -- --probes <ces sondes> --only <les « tâche »>` (un `CARGO_TARGET_DIR` commun aux trois : ≈ 12 min la première fois, 8 ensuite). Chaque passe écrit aussi ses réponses dans `target/codeindex-probes/responses/` (`<id>.txt` tel qu'envoyé, `<id>.json`, `<id>.code-words.txt`).

## tokensave face à face (IDX-5, ESC-304)

La question de l'utilisateur (2026-10-30) : « les index, on est aussi performants que tokensave ? ». IDX-1 à IDX-3 ont porté sa méthode sans jamais le faire tourner. Cette fois, tokensave 7.17.0 (commit `580936b`, MIT, © 2025 Enzo Lombardi) répond aux mêmes sondes, notées par le même code.

**En une phrase : Escouade est meilleur sur le graphe des appels et la recherche d'un symbole, et un peu moins bon sur `context` : tokensave y trouve un peu plus (15 % contre 11 % au rappel@10, à égalité sur les « tâche ») et se trompe pour moins cher ; sur les 45 sondes que les deux savent jouer, Escouade a 45 % de rappel@10 et tokensave 38 %.**

### Comment il a été joué

- **Compilé hors des dépôts** : `cargo build --release` dans `C:\tmp\ext\tokensave`, avec `CARGO_TARGET_DIR=C:\tmp\tokensave-target` (9 min 26 s). La source n'a pas été mise à jour, et rien n'en est copié ici.
- **Première commande : `tokensave disable-upload-counter`**. Ensuite, `~/.tokensave/config.toml` dit `upload_enabled = false`. Chaque appel du banc passe aussi `TOKENSAVE_UPDATE_CHECK=off` (pas de requête à GitHub).
- **Aucun `install`, aucun hook, aucun `serve`.** Les seules commandes lancées sont `init`, `tool`, `--version` et `--help`. `init` a tourné une seule fois par passe, avec `--no-git-hook`, et seulement dans `C:\tmp\tokensave-snapshot`. Ce dossier est fait par `git archive 303fb5f2 | tar -x`, au commit des sondes et de la référence d'IDX-3. Le banc refuse un dossier qui a un `.git`, en lui ou au-dessus. Les réglages des agents (`~/.claude/settings.json`, `~/.codex/config.toml`, `~/.gemini/settings.json`, `~/.cursor/mcp.json`, OpenCode, Copilot) ont la même empreinte avant et après.
- **Le banc** (`--tokensave <exe> --tokensave-project <copie>`, module `codeindex/probes/tokensave.rs`) pose chaque sonde à `tokensave tool <outil> --project <copie> --args <json>`. C'est le gestionnaire qu'appelle son serveur MCP. La correspondance des outils :

  | Escouade | tokensave |
  |---|---|
  | `index_search` | `tokensave_search` (`query`, `limit`) |
  | `index_context` | `tokensave_context` (`task`, `max_nodes`) |
  | `index_callers` | `tokensave_callers` (`symbol`, `limit`) |
  | `index_callees` | `tokensave_callees` (`symbol`, `limit`) |
  | `index_impact` | `tokensave_impact` (`symbol`, `max_depth`, `limit`) |

  Les limites sont celles d'Escouade : 10, puis 50. Le banc lit la réponse et la note avec `score`, `windows` et `suffice`, comme celle d'Escouade.
- **Les mêmes paramètres des deux côtés** :
  - `include_code: true` pour `context`, chez les deux. C'est le défaut d'Escouade ; celui de tokensave est faux, un écart voulu et écrit dans IDX-3.
  - Le reste suit les défauts de tokensave :
    - pas de `path` pour les appelants (il n'en a pas) ;
    - pas de `kind` pour la recherche (il n'en a pas) ;
    - `max_depth` 3 pour les appelés.
- **Les tokens** :
  - La réponse est coupée à 15 000 caractères, comme la coupe son serveur MCP (la ligne de commande ne coupe pas).
  - La ligne `tokensave_metrics:` que son serveur ajouterait est recomptée avec notre port de sa comptabilité, la base étant les fichiers que nomme la réponse.
  - Les lectures, le repli sur grep quand rien n'est juste et la suffisance sont comptés par le même code que pour Escouade.
- **La notation, à grain égal** :
  - Un appelant compte pour chacune de ses lignes d'appel (`lines`), comme chez Escouade.
  - Un symbole donné entier (impact, appelés, en-tête d'un groupe) compte pour toute son étendue (`line` à `end_line`). Escouade donne la ligne de l'appel, tokensave le symbole qui la contient : sans cela, `rs-impact-scope_of` aurait eu 20 % au lieu de 100 %.
  - Une réponse JSON coupée est lue jusqu'à la coupe.
- **Le temps** compte un processus par appel, qui ouvre une base de 230 Mo : 40 ms au moins, jusqu'à 400 ms sur ce poste partagé. Escouade, lui, est mesuré dans le processus. Les temps ne se comparent donc pas : `tokensave serve` garderait la base ouverte. On ne l'a pas lancé, par règle.

### L'indexation de la même copie

| | Escouade | tokensave |
|---|---|---|
| Dépôt entier | 21,0 s | 13,3 s (14,0 s la première fois, avec son banc du matériel) |
| Base | 123,5 Mo | 229,7 Mo |
| Contenu | 1 578 fichiers, 35 799 symboles, 93 054 liens | 1 588 fichiers, 52 361 nœuds, 147 408 arêtes (101 658 dans `status`) |
| Non lus | — | 83 fichiers, dont les 71 `.vue` |

tokensave indexe plus vite, avec une base presque deux fois plus grosse. Ses nœuds en plus ne sont pas tous des symboles :

- 8 059 `use` ;
- 5 023 annotations ;
- un nœud par fichier ;
- les blocs `describe`/`it` des tests, comptés comme des fonctions.

### Côte à côte

Rejouer : `cargo run --release --example codeindex_probes -- --tokensave 'C:\tmp\tokensave-target\release\tokensave.exe' --tokensave-project 'C:\tmp\tokensave-snapshot'`. La référence est `docs/codeindex-probes-reference-tokensave.json`. Rappel@10, précision@10 et rappel@50 sont des moyennes en % ; les tokens sont des totaux.

| Groupe | Sondes jouées (sans objet) | R@10 Escouade / tokensave | P@10 | R@50 | Tok. réponse | Tok. réels | Meilleur : Escouade / tokensave / = |
|---|---|---|---|---|---|---|---|
| search | 5 (4) | 100 / 100 | 20 / 12 | 80 / 80 | 1 555 / 1 665 | 6 433 / 13 372 | 5 / 0 / 0 |
| context | 27 (1) | 11 / **15** | 1 / 3 | 13 / 9 | 29 744 / **22 257** | 174 231 / **144 704** | 9 / **16** / 2 |
| callers | 6 (4) | **90** / 35 | **96** / 19 | 90 / 27 | **2 420** / 13 056 | **9 609** / 25 826 | 6 / 0 / 0 |
| callees | 3 (0) | 95 / 95 | **96** / 67 | 95 / 95 | **2 107** / 2 790 | **8 634** / 11 900 | 3 / 0 / 0 |
| impact | 4 (0) | **95** / 80 | 80 / **90** | 100 / 80 | 6 483 / **1 771** | 17 370 / **8 268** | 3 / 1 / 0 |
| « tâche » | 17 (0) | 6 / 6 | 1 / 1 | 12 / 6 | 18 150 / **13 755** | 125 884 / **110 020** | 6 / **10** / 1 |
| **Tout** | **45 (9)** | **45** / 38 | **29** / 18 | **44** / 32 | 42 309 / 41 539 | 216 277 / **204 070** | **26** / 17 / 2 |

« Meilleur » suit la règle d'Escouade face à grep : plus de rappel, sinon une précision au moins égale pour moins de tokens réels. La colonne « Tout » du côté Escouade ne compte que les 45 sondes jouées par les deux. Sur les 54, Escouade garde 47 % (IDX-3).

**Sans objet** pour tokensave : 9 sondes.

- `wt-search-renamed`, `wt-callers-renamed`, `wt-callers-old-name` : un index par dossier, pas de delta de worktree.
- `fresh-now`, `fresh-after-gap` : la copie n'est jamais écrite.
- `vue-search-plugintile`, `vue-callers-hueOf`, `vue-context-tile-en` : la réponse n'est que dans des `.vue`, qu'il ne lit pas.
- `svelte-callers-planbanner` : un composant Svelte n'est pas un symbole chez lui. Il répond « No symbol named 'PlanBanner' » et ne voit pas `<PlanBanner>` dans un gabarit. Il lit pourtant les fonctions des `.svelte`.

### context : la question, puis les mots du code

Rappel@10, puis entre parenthèses le rang du premier bon résultat. Trois passes, chez les deux :

- la question telle quelle ;
- la tâche dite avec les mots du code (`codeWords`, le mode « mots du code » du banc) ;
- la question telle quelle, avec les mots du code en `keywords`. C'est l'usage que tokensave recommande.

Seules les sondes où l'un des deux trouve quelque chose sont listées ; les 17 autres sont à 0 partout.

| Sonde | Question Escouade | Question tokensave | Mots du code Escouade | Mots du code tokensave | + keywords Escouade | + keywords tokensave |
|---|---|---|---|---|---|---|
| rs-notion-gap-fr | 0 | 0 | 0 | 100 (4) | 0 | 0 |
| rs-notion-gap-en | 100 (9) | 100 (5) | 0 | 100 (4) | 100 (9) | 100 (5) |
| rs-context-hint-en | 0 | 0 | 50 (3) | 0 | 0 | 0 |
| ts-context-tooltip-en | 100 (3) | 100 (2) | 0 | 0 | 100 (3) | 100 (2) |
| svelte-context-banner-fr | 0 | 0 | 100 (1) | 0 | 0 | 0 |
| md-context-allumer-fr | 0 | 100 (1) | 0 | 0 | 0 | 0 |
| tache-zoom-borne | 100 (2) | 100 (2) | 100 (2) | 100 (2) | 100 (2) | 100 (1) |
| tache-xterm-couleurs | 0 | 0 | 100 (1) | 100 (1) | 0 | 0 |
| tache-lance-processus | 0 | 0 | 100 (9) | 100 (7) | 100 (9) | 100 (5) |
| tache-store-controles | 0 | 0 | 100 (1) | 100 (1) | 0 | 0 |
| **« tâche » (17)** | **6** | **6** | **24** | **24** | **12** | **12** |
| **autres fr/en (10)** | 20 | 30 | 15 | 20 | 20 | 20 |

- **tokensave ne fait pas mieux avec les mots du code** : 24 % sur les « tâche », comme Escouade. Dire la tâche avec les mots du code quadruple le rappel chez les deux.
- **`keywords` ajoutés à une question française aident deux fois moins** que les mêmes mots en guise de question (12 % contre 24 %, chez les deux) : les trois points d'entrée se remplissent encore des mots de la paraphrase.
- Hors « tâche », tokensave gagne une question (`md-context-allumer-fr`, ci-dessous) et une passe « mots du code » (`rs-notion-gap-fr`). Escouade gagne deux passes « mots du code » (`svelte-context-banner-fr`, `rs-context-hint-en`).

### Suffisance des « tâche »

| | Escouade | tokensave |
|---|---|---|
| Lignes nécessaires déjà dans la réponse | 1 % | 1 % |
| Un appel suffit | 0 / 17 | 0 / 17 |
| Appels (tokens) de plus pour tout avoir | 66 (32 189) | 68 (36 153) |

Égalité. Le mur des 17 « tâche » est le même chez tokensave : une paraphrase française ne touche pas les noms du code. Ce n'est donc pas un défaut de notre portage. La vraie mesure de `keywords` reste IDX-4 (de vrais agents).

### Relu à la main

1. **`cas-forget`** (appelants de `forget`, `path` = `plugins/runtime.rs`) : Escouade 100 %, tokensave 13 %, au rang 13.
   - Sans `path`, tokensave répond les trois premiers `forget` de sa recherche (OpenCode, `sidechain`, `MemorySection.svelte`) et range `Runtime::forget` parmi les autres.
   - Même qualifié (`--symbol Runtime::forget`, essayé à la main), il ne trouve qu'1 appelant sur 7 (`runtime.rs:410`, un `self.forget`). Il ne résout pas `core.plugin_runtime.forget(...)` (`commands.rs:328`…) : un receveur typé par le champ d'une structure.
   - Escouade les résout depuis IDX-2 (`codeindex/receivers.rs`) : là, notre portage va plus loin que tokensave.
2. **`rs-homonyme-load`** (`load` de `workflowfiles.rs`) : Escouade 100 %, tokensave 20 %, au rang 45. La même cause sans `path` : ses trois groupes sont d'autres `load`, et leurs appelants dans `planfiles_tests.rs` passent devant. Les 6 sondes d'appelants jouées vont toutes à Escouade.
3. **`rs-impact-hint-file`** (impact du fichier `codeindex/hint.rs`) : Escouade 80 %, tokensave 20 %.
   - `tokensave_impact` reçoit un chemin comme `symbol` et le résout en nœud `file`. Ce nœud n'a pas de dépendants : la réponse ne contient que le fichier.
   - Escouade, pour un fichier, prend les appelants de tous ses symboles et les importeurs. Il trouve `wiring.rs:129`, `core.rs:5741` et `discussion.rs:373`.
4. **`ts-impact-definitions-file`** (impact de `src/lib/editor/definitions.ts`) : les deux à 100 %, mais tokensave gagne en 3 résultats (le fichier, l'import d'`EditorView.svelte`, le test) et 1 349 tokens réels, contre 7 071.
   - La réponse d'Escouade est pleine de faux appelants : `ConversationOverlay.svelte:32` (`text.slice(-max)`), `sdk/plugin/src/index.ts:68`, `board.ts:261`… La résolution par le nom relie ces `.slice()` de chaînes au `const slice` privé de `definitions.ts:141`. De même, `rule("bash", …)`, une fermeture locale d'`engine/opencode/mod.rs:104`, est relié à `hint.rs:243`.
   - **tokensave fait les mêmes faux liens** : `tokensave_callers --symbol rule` rend `opencode/mod.rs:106`, et `slice` a 72 « appelants ». Ce n'est donc pas un point mal porté.
   - C'est notre choix d'étendre l'impact d'un fichier aux appelants de chacun de ses symboles (`calls.rs`, `impact`) qui fait entrer ce bruit dans la réponse.
5. **`md-context-allumer-fr`** (« comment allumer l'index du code sur tous les projets ») : tokensave 100 %, au rang 1, Escouade 0 %.
   - Mêmes termes retenus (`comment`, `allumer`, `tous`, `projets`), mais tokensave met en tête le titre « Sur tous les projets » de `website/content/docs/fr/code-index.md` (meilleur score 24,2). Escouade y met `discussion.rs::instructions` (36,1), dont le corps contient les mêmes mots français.
   - L'en-tête existe bien dans notre index (`index_search` le trouve). Le classement diverge.
6. **`svelte-search-planbanner`** et **`rs-search-saving`** (recherche) : les deux trouvent, mais au rang 1 chez Escouade, au rang 8 et 9 chez tokensave. L'agent lit donc 5 à 7 fois plus de tokens : 3 883 contre 534, et 3 131 contre 571.
   - tokensave n'a pas de symbole pour le composant `PlanBanner.svelte`, et les blocs `describe PlanBanner` de ses tests passent devant.
   - Il n'a pas non plus de filtre `kind` : `Saving` la structure est noyée dans les `saving` des fonctions TS.
7. **`rs-callees-ensure_fresh`** (appelés) : même rappel (86 %), mais une précision de 55 % contre 100 %. `tokensave_callees` descend par défaut à 3 niveaux, et ajoute `store.rs:115`, `scan.rs:239`, `scan.rs:291`, `sharing_tests.rs:60`. Les deux ratent `store.rs:65`.
8. **`svelte-context-banner-fr`** et **`rs-notion-gap-fr`**, en mots du code : chacun gagne une passe.
   - Escouade trouve le composant `PlanBanner` au rang 1, un symbole que tokensave n'a pas.
   - tokensave trouve `Service::ensure_fresh` au rang 4 ; Escouade ne le trouve pas (voir IDX-2, le bonus d'IDX-1 retiré).
9. **Les « tâche » que tokensave « gagne »** (10 sur 17, dont `tache-compte-limite` : 4 608 tokens réels contre 8 559) : les deux ont 0 % de rappel. tokensave ne les gagne qu'en se trompant pour moins cher : il choisit d'autres symboles, plus petits, et ses blocs de code sont plus courts. Pour la même raison, ses réponses `context` coûtent 25 % de moins.

### Ce qui vient du portage, à suivre

Aucun écart mesuré ne vient d'une règle de tokensave qu'on aurait mal recopiée. Les écarts viennent de ce que chacun indexe, et de nos ajouts : `path`, receveurs typés, composants Svelte, impact d'un fichier. Un point reste à éclaircir, pour un ticket de suite :

- **Pour `context`, 26 sondes sur 27 ont d'autres points d'entrée que tokensave, avec les mêmes termes.**
  - Côté tokensave, `src/context/builder.rs` et sa recherche ; côté Escouade, `src-tauri/src/codeindex/context.rs` et `ranking.rs`.
  - La première cause est le contenu de l'index : 52 361 nœuds contre 35 799 symboles (blocs de test, `use`, nœuds de fichier, en-têtes classés en `module`).
  - Mais `md-context-allumer-fr` montre un écart de classement sur des candidats que les deux index ont. Chez nous, une fonction dont le corps contient les mots passe en tête (36,1), devant l'en-tête Markdown. Chez lui, l'en-tête passe en tête (24,2).
  - À comparer candidat par candidat, colonne par colonne, sans rien corriger ici.

### Ce que tokensave a écrit hors de la copie

- **`~/.tokensave/`**, qui n'existait pas avant :
  - `config.toml` et `config.toml.lock` ;
  - `state.toml` ;
  - `benchmark.toml` et `benchmark.toml.lock` (son banc du matériel, au premier `init`) ;
  - `global.db` (le registre de ses projets) ;
  - `memory.mmap`.

  Juste avant de le supprimer, `config.toml` disait toujours `upload_enabled = false`, et `state.toml` disait `pending_upload = 0`, `last_upload_at = 0`, `last_flush_attempt_at = 0` et `last_version_check_at = 0` : rien n'a été envoyé, ni au compteur mondial, ni à GitHub. Le dossier a été supprimé à la fin.
- **Pour la compilation**, par cargo et non par tokensave :
  - `C:\tmp\tokensave-target` ;
  - les crates téléchargées dans `~/.cargo/registry` ;
  - deux dépôts git de grammaires dans `~/.cargo/git` (`tokensave-large-treesitters`, `tree-sitter-fortran`), supprimés à la fin.
- Rien dans `%LOCALAPPDATA%`, `%APPDATA%` ou `%TEMP%`. Aucun réglage d'agent touché, aucun hook git.
- **Pour rejouer** : `~/.tokensave` est recréé au premier lancement avec le compteur **allumé**. Il faut relancer `tokensave disable-upload-counter` avant toute autre commande, puis supprimer `~/.tokensave` à la fin. Le banc lance `tokensave init` dans la copie seulement si elle n'a pas encore d'index : pour chronométrer une indexation neuve, effacer d'abord `C:\tmp\tokensave-snapshot\.tokensave`.

## Avec de vrais agents (ESC-299, IDX-4)

Mesures du 2026-10-30. La question de l'utilisateur : l'index donne-t-il à un vrai agent le code dont il a besoin, pour qu'il n'ait pas à le chercher, à voir qu'il en manque, puis à re-grep ? Les sondes mesurent les outils seuls. Ici, de vrais Claude Code sans fenêtre font 20 tâches courtes dans quatre modes, et on compte ce qu'ils dépensent vraiment.

**En une phrase** : face à grep seul, l'index d'Escouade avec son hook fait dépenser **8 % de moins** à un vrai agent Haiku, à justesse égale (39 sur 40). Le gain est de 13 % sur les questions, et c'est une perte de 14 % sur les petits changements. tokensave fait dépenser **18 % de plus**, avec 37 justes sur 40. Escouade fait donc mieux que tokensave, mais son gain reste de l'ordre du bruit : deux passages d'une même tâche diffèrent de 28 % en médiane.

### Le banc

```sh
cd src-tauri
cargo build --release --example codeindex_agents
./target/release/examples/codeindex_agents.exe check                      # chaque vérification, sur sa bonne et sa mauvaise réponse
./target/release/examples/codeindex_agents.exe check --mode tokensave     # idem, sur la copie sans .git
./target/release/examples/codeindex_agents.exe run --tokensave C:/tmp/tokensave-target/release/tokensave.exe
./target/release/examples/codeindex_agents.exe report --runs ../docs/codeindex-agents-runs.jsonl   # les tableaux de ce ticket, relus
```

- **Les tâches** (`docs/codeindex-agents.json`, commit `303fb5f2`, celui des sondes) :
  - 15 questions, prises des 17 sondes « tâche » (`tache-xterm-couleurs` et `tache-taille-terminal` mises de côté) ;
  - 5 petits changements : le pas de zoom 225 %, la boucle de sauvegarde à 250 ms, la marge de reprise à une minute, la locale « fra », le noir ANSI des terminaux (ESC-80).
- **Les vérifications** sont mécaniques :
  - une question est juste quand sa réponse cite le symbole attendu (l'une des variantes écrites, sans compter la casse ni les `\` d'un chemin) ;
  - un changement est juste quand le texte voulu est **dans le symbole attendu** (de sa première ligne à l'accolade qui le ferme) et que l'ancien n'y est plus.
  - Chaque tâche a une bonne et une mauvaise réponse. `check` les essaie sur la copie : 20 sur 20 prennent la bonne et refusent la mauvaise, sur la copie git comme sur celle de tokensave.
  - Les tests de `probes/agents.rs` refont celles des questions.
- **Les modes** :
  - (a) `grep` : Grep, Glob et Read seuls ;
  - (b) `index-hook` : en plus, les six outils de l'index, la règle des consignes de l'agent et le hook d'ESC-253 ;
  - (c) `index-only` : l'index, Glob et Read, sans Grep ;
  - (d) `tokensave` : comme (b), mais avec le serveur MCP de tokensave et les règles qu'écrit son `install` (`CANONICAL_RULES_MARKDOWN` de 580936b, sans le paragraphe sur les agents Explore), sans hook.
  - Dans tous les modes, Edit et Write servent aux changements, et l'agent n'a jamais de terminal.
- **Les agents** :
  - `claude -p --output-format stream-json`, sur `claude-haiku-5-5` en effort `medium`, deux passages par tâche et par mode : 160 runs ;
  - le compte par défaut du poste, tel quel ;
  - chaque lancement a ses propres options, écrites dans le dossier du run : `--settings`, `--mcp-config --strict-mcp-config`, `--setting-sources project` (sans les plugins ni les hooks de l'utilisateur), `--tools`, `--allowedTools`, `--permission-mode dontAsk`, `--no-session-persistence`.
  - Aucun fichier de connexion n'est touché ni lu, aucun réglage global n'est changé. Les variables de la session parente (`CLAUDE*`, `ESCOUADE_*`) sont retirées par leur nom, sans en lire la valeur.
- **Les copies** :
  - chaque mode a la sienne, dans `%TEMP%\escouade-codeindex-agents\<mode>`, hors de tout worktree, remise au commit après chaque run ;
  - pour tokensave, une copie sans `.git` (`git archive`), remise en état contre une copie intacte, puis `tokensave sync` après un changement.
- **L'index d'Escouade** est servi par un serveur MCP stdio du banc (`probes/agents_mcp.rs`, testé). Il lit la copie et jamais l'app. Il sert les mêmes schémas et les mêmes réponses que le serveur de l'app : les calculs sont sortis de `mcp/index.rs` (`search_on`, `context_on`…) et partagés, sans changer leur comportement. Il compte aussi la ligne des métriques.
- **tokensave**, avec les règles d'ESC-304, plus son `serve` que ce mode demande :
  - `disable-upload-counter` d'abord, puis le banc vérifie `upload_enabled = false` ;
  - `TOKENSAVE_UPDATE_CHECK=off` ;
  - `init --no-git-hook` dans sa copie seulement ;
  - jamais `install`, jamais de hook.
  - À la fin, `~/.tokensave` (qui n'existait pas) a été supprimé. Il contenait `config.toml`, `state.toml`, `benchmark.toml`, `global.db`, deux `.mmap` et `servers/` (vide). `state.toml` montre qu'aucun envoi n'a eu lieu : `last_upload_at = 0`, `last_flush_attempt_at = 0`, `last_version_check_at = 0`, `installed_agents = []`, et 293 896 tokens restés en attente (`pending_upload`). Aucun réglage d'agent ne cite tokensave (`~/.claude/settings.json`, `.claude.json`, `.codex/config.toml`, `.gemini/settings.json`, `.copilot/config.json`).
- **Ce qui est compté**, run par run (`docs/codeindex-agents-results.md`, et les 160 runs bruts dans `docs/codeindex-agents-runs.jsonl`) :
  - la réussite ;
  - les appels par outil ;
  - les re-greps (Grep après le premier appel à l'index), les Glob et Read après lui, les relectures (un Read d'un fichier déjà lu) ;
  - les tokens (entrée, cache écrit, cache lu, sortie), ceux des réponses de chaque outil, le coût selon Claude Code (`total_cost_usd`), le temps (`duration_ms`) et les tours.

### Le coût, mesuré d'abord

- **Mesuré** sur 3 tâches (`q-reprise-limite`, `q-ticket-termine`, `c-sauvegarde-250`), dans les 4 modes : 12 runs, 0,065 $, 1,48 M tokens (presque tous lus en cache), 90 s. Le quota de la semaine est resté à 41 %, la fenêtre de 5 h est passée de 19 à 20 % (cette session comprise).
- **Estimé** pour le tout (20 × 4 × 2 = 160 runs) : ≈ 0,9 $ et 20 M tokens, soit moins de 3 % du quota de la semaine. C'est sous les 8 % de l'accord de l'utilisateur (2026-10-30) et le quota était sous 70 % : le reste a été joué sans attendre. Le coût a été écrit sur la carte du ticket (`report_progress`).
- **Réel** : 0,675 $ et 18,2 M tokens pour les 160 runs, en ≈ 20 min (les quatre modes en parallèle). Le quota de la semaine est à 40 % après (la fenêtre glisse) : le banc ne s'y voit pas au point près.

### Les totaux, par mode (40 runs chacun)

| | grep | index + hook | index seul | tokensave |
|---|---|---|---|---|
| Justes | 39 | 39 | 39 | 37 |
| · questions (30) / changements (10) | 29 / 10 | 29 / 10 | 29 / 10 | 27 / 10 |
| Appels d'outils | 260 | 255 | 324 | 301 |
| · Grep | 147 | 71 | 18 (essayés : l'outil n'existe pas) | 77 |
| · Read / Glob / Edit | 97 / 2 / 14 | 47 / 0 / 15 | 25 / 3 / 12 | 20 / 1 / 14 |
| · outils de l'index | — | 122 (search 47, body 28, context 23, callers 17, callees 7) | 266 (search 128, body 64, callers 37, context 27, callees 10) | 189 (read 52, search 49, context 36, body 26, callers 23, callees 2) |
| Runs qui ont appelé l'index | — | 37 | 40 | 39 |
| · puis ont cherché ou lu quand même | — | 29 | 23 | 31 |
| Re-greps après l'index | — | 59 | 13 | 76 |
| Glob / Read après l'index | — | 0 / 43 | 3 / 25 | 1 / 19 |
| Relectures (Read d'un fichier déjà lu) | 32 | 14 | 9 | 4 |
| Hook : recherches vues / conseillées | — | 71 / 1 | — | — |
| Tokens des réponses des outils | 206 k | 168 k | 192 k | 237 k |
| · dont l'index | — | 86 k | 174 k | 145 k |
| Tokens : cache écrit | 495 k | 410 k | 461 k | 533 k |
| · cache lu | 2 941 k | 3 704 k | 4 206 k | 5 162 k |
| · sortie | 70,8 k | 64,5 k | 62,6 k | 71,3 k |
| · total | 3 508 k | 4 179 k (+19 %) | 4 729 k (+35 %) | 5 767 k (+64 %) |
| **Coût** (Claude Code, $) | **0,164** | **0,151 (−8 %)** | **0,166 (+1 %)** | **0,194 (+18 %)** |
| · questions | 0,131 | 0,113 (−13 %) | 0,127 (−3 %) | 0,137 (+5 %) |
| · changements | 0,033 | 0,038 (+14 %) | 0,039 (+16 %) | 0,057 (+71 %) |
| Temps (somme, s) | 604 | 599 | 561 | 630 |
| Tours | 300 | 295 | 364 | 341 |

- **Ce que l'agent dépense vraiment, c'est le coût, plus que le total des tokens.** Le total est fait aux neuf dixièmes de cache lu (0,1 fois le prix de l'entrée), relu à chaque tour. Les réponses de l'index restent dans le contexte, et chaque tour de plus les relit : le total monte de 19 % avec le hook. Le coût, lui, baisse de 8 % : moins de cache écrit (410 k contre 495 k, car les Grep rendent de gros blocs) et moins de sortie.
- **L'index remplace la moitié des recherches, pas toutes** : avec le hook, 71 Grep et 47 Read au lieu de 147 et 97, et des réponses d'outils plus légères (168 k contre 206 k). Mais 29 des 37 runs qui l'appellent cherchent ou lisent encore après, avec 59 re-greps.
- **Sans Grep, l'agent fait plus d'appels** (324, dont 128 `index_search`) pour le même résultat et le même coût que grep (+1 %). L'index ne remplace pas grep pour Haiku : il le contourne à coups de recherches.
- **Le hook ne sert presque à rien avec Haiku** : il a vu 71 recherches et n'en a conseillé qu'une. Haiku écrit des regex à alternatives (`usage_limit|rate_limit|resume_at`), et le hook se tait sur une regex, comme voulu (« dans le doute, c'est un texte »). Le gain d'`index-hook` vient de la règle des consignes et des outils, pas du hook.
- **Les changements coûtent plus avec l'index, dans les trois modes.** Edit de Claude Code exige un Read du fichier avant : l'agent doit lire, quoi que l'index lui ait rendu. L'appel à l'index s'ajoute donc au chemin de grep au lieu de le remplacer.

### Où l'index fait gagner ou perdre

Le coût par tâche (deux passages), face à grep :

| Tâche | grep | index + hook | index seul | tokensave |
|---|---|---|---|---|
| q-modeles-harness | 2/2 · 13 appels · 0,006 $ | 2/2 · 8 · −25 % | 2/2 · 7 · −49 % | 2/2 · 9 · −21 % |
| q-compte-limite | 1/2 · 28 · 0,017 $ | 2/2 · 12 · −46 % | 1/2 · 16 · −48 % | 2/2 · 11 · −47 % |
| q-mcp-codex | 2/2 · 17 · 0,011 $ | 2/2 · 17 · −15 % | 2/2 · 24 · +19 % | 1/2 · 17 · −22 % |
| q-branche-premier-message | 2/2 · 20 · 0,013 $ | 2/2 · 9 · −30 % | 2/2 · 17 · −20 % | 2/2 · 18 · +0 % |
| q-reprise-limite | 2/2 · 11 · 0,009 $ | 2/2 · 13 · −24 % | 2/2 · 13 · +24 % | 2/2 · 13 · +63 % |
| q-zoom-borne | 2/2 · 11 · 0,006 $ | 2/2 · 9 · −14 % | 2/2 · 10 · −35 % | 2/2 · 9 · −18 % |
| q-lance-processus | 2/2 · 14 · 0,010 $ | 2/2 · 23 · +47 % | 2/2 · 27 · +54 % | 2/2 · 12 · −26 % |
| q-ticket-termine | 2/2 · 16 · 0,011 $ | 2/2 · 14 · −12 % | 2/2 · 18 · +22 % | 2/2 · 12 · −20 % |
| q-langue-defaut | 2/2 · 9 · 0,006 $ | 1/2 · 10 · −22 % | 2/2 · 21 · +50 % | 0/2 · 7 · −36 % |
| q-permission-plugin | 2/2 · 12 · 0,007 $ | 2/2 · 9 · −46 % | 2/2 · 15 · −23 % | 2/2 · 34 · +136 % |
| q-store-controles | 2/2 · 15 · 0,009 $ | 2/2 · 10 · −7 % | 2/2 · 13 · −16 % | 2/2 · 15 · +40 % |
| q-sauvegarde-etat | 2/2 · 8 · 0,005 $ | 2/2 · 18 · +86 % | 2/2 · 36 · +147 % | 2/2 · 20 · +67 % |
| q-worktree-cree | 2/2 · 14 · 0,010 $ | 2/2 · 20 · +3 % | 2/2 · 16 · −37 % | 2/2 · 17 · +10 % |
| q-quota-compte | 2/2 · 7 · 0,007 $ | 2/2 · 9 · −3 % | 2/2 · 10 · −15 % | 2/2 · 17 · +46 % |
| q-cree-agent | 2/2 · 8 · 0,005 $ | 2/2 · 3 · −46 % | 2/2 · 3 · −49 % | 2/2 · 8 · −9 % |
| c-zoom-225 | 2/2 · 13 · 0,006 $ | 2/2 · 18 · +23 % | 2/2 · 15 · +26 % | 2/2 · 16 · +38 % |
| c-sauvegarde-250 | 2/2 · 15 · 0,008 $ | 2/2 · 12 · +5 % | 2/2 · 24 · +53 % | 2/2 · 30 · +192 % |
| c-marge-reprise | 2/2 · 11 · 0,009 $ | 2/2 · 18 · +25 % | 2/2 · 17 · −29 % | 2/2 · 13 · +19 % |
| c-locale-fra | 2/2 · 11 · 0,007 $ | 2/2 · 14 · −6 % | 2/2 · 10 · +2 % | 2/2 · 16 · +53 % |
| c-ansi-noir | 2/2 · 7 · 0,003 $ | 2/2 · 9 · +30 % | 2/2 · 12 · +66 % | 2/2 · 7 · +30 % |

- **L'index gagne quand un nom du code tombe tout de suite** :
  - `create_agent`, en 3 appels au lieu de 8 ;
  - `plugin_set_permission` et `list_models`, −25 à −46 % ;
  - quand grep s'égare : `q-compte-limite`, où grep fait 28 appels et se trompe une fois sur deux, alors que l'index + hook trouve `switch_on_limit` deux fois sur deux pour moitié prix ;
  - quand le nom à trouver est loin des mots de la question : `q-branche-premier-message`, −30 %.
- **Il perd quand la réponse est un détail dans une longue fonction** :
  - `q-sauvegarde-etat` : la boucle de `Core::start` qui regarde `dirty`. Grep trouve `dirty` en 8 appels, l'index fait chercher l'agent de symbole en symbole, +86 à +147 % ;
  - `q-lance-processus` : beaucoup de candidats nommés `spawn` ou `start_process*`, +47 % ;
  - les changements, toujours lus avant d'être édités (voir plus haut).
- **tokensave** gagne sur `q-lance-processus` (−26 %, il lit moins) et perd sur :
  - `q-permission-plugin` (+136 %, 34 appels : l'agent a enchaîné `tokensave_read` et appelé un outil fantôme, `tokensave_grep_placeholder_never`) ;
  - les changements (+71 %) : `tokensave_read`, puis Read quand même pour pouvoir éditer.

### Les tâches où l'agent a dû compléter l'index

| Tâche | index + hook | index seul | tokensave |
|---|---|---|---|
| Changements : c-zoom-225, c-marge-reprise, c-locale-fra | 2/2 chacune | 2/2 chacune | 2/2 chacune |
| c-sauvegarde-250 | 1/2 | 2/2 | 2/2 |
| c-ansi-noir | 0/2 | 2/2 | 0/2 |
| q-worktree-cree, q-sauvegarde-etat | 2/2 | 2/2 | 2/2 |
| q-compte-limite, q-mcp-codex, q-lance-processus | 2/2 | 1/2 | 2/2, 2/2, 1/2 |
| q-modeles-harness, q-zoom-borne, q-store-controles | 2/2 | 1/2 | 1/2, 2/2, 2/2 |
| q-reprise-limite | 2/2 | 0/2 | 2/2 |
| q-ticket-termine | 1/2 | 2/2 | 1/2 |
| q-branche-premier-message, q-quota-compte | 1/2 | 0/2 | 2/2, 1/2 |
| q-langue-defaut | 1/2 | 1/2 | 2/2 |
| q-permission-plugin | 0/2 | 0/2 | 1/2 |
| q-cree-agent | 0/2 | 0/2 | 0/2 |
| **Runs complétés / runs qui ont appelé l'index** | **29 / 37** | **23 / 40** | **31 / 39** |

« Compléter », c'est un Grep, un Glob ou un Read après le premier appel à l'index.

- Avec l'index d'Escouade, l'agent répond **sur l'index seul** dans 8 runs sur 37 avec le hook, et 17 sur 40 sans Grep. Ailleurs, il vérifie ou complète.
- C'est ce que disaient les sondes de suffisance d'IDX-3 : un appel ne rend pas tout le code nécessaire (1 % des lignes), et l'agent va le chercher. La seule tâche que l'index règle partout sans complément est `q-cree-agent`, un nom du code qui est presque la question.

### Ce qui a été relu à la main

- Les 8 réponses refusées ont été relues. 6 sont fausses :
  - `switch_if_over` deux fois, pour `q-compte-limite` : c'est le passage au seuil, pas celui à la limite ;
  - `escouade_launch`, pour `q-mcp-codex` : il passe des `-c` à Codex, sans écrire sa config ;
  - pour `q-langue-defaut`, trois fois (index + hook une fois, tokensave deux fois), la valeur de départ du front (`langOfTag(navigator.language)`), que les réglages remplacent ensuite. La langue du système par défaut est `i18n::system` côté Rust.
- 2 étaient justes, et leur vérification a été élargie après lecture :
  - `checks.ts — checkArchive` (grep, passage 1), que `runChecks` appelle ;
  - `refresh_usage` (tokensave, passage 2), qui relit le quota de chaque compte.
  - Le champ `widened` de la tâche le dit. `report` refait le compte des questions avec les vérifications du fichier, et liste ces deux runs à part (« Checked again »).
- Avant cet élargissement, grep et tokensave avaient chacun une réponse juste de moins (38 et 36).
- **Le bruit** : deux passages d'une même tâche dans un même mode diffèrent de 28 % en médiane. Sur 40 runs, un écart de quelques pour cent entre deux modes n'est pas sûr ; le −8 % d'Escouade est au bord, le +18 % de tokensave et le +14 à +71 % des changements le dépassent.

## Changements à plusieurs endroits (IDX-6, ESC-306)

Mesures du 30 au 31 octobre 2026. IDX-4 n'avait mesuré que des questions et des changements à un seul endroit, avec Haiku. Ici, de vrais Claude Code font **10 changements qui touchent chacun 5 endroits ou plus dans 2 fichiers ou plus**, avec Sonnet 5.5, Haiku 5.5 et, en complément, Opus 5.5. C'est le cas où `index_callers` et `index_impact` devraient le plus servir.

**En deux phrases** : sur ces changements, l'index ne fait pas faire moins d'oublis. Les 130 runs finissent tous sans endroit oublié, parce que la compilation et les tests les montrent quel que soit le mode. Côté coût, l'index + hook fait économiser **23 % à Haiku**, ne change rien à **Sonnet (+1 %, sous le bruit de 16 %)** et coûte **12 % de plus à Opus** (un seul passage sur 5 tâches, indicatif). Plus le modèle est fort, moins l'index l'aide : avec les modèles que l'utilisateur fait tourner, il ne vaut pas la peine pour le coût.

### Le banc

```sh
cd src-tauri
cargo build --release --example codeindex_agents
X=./target/release/examples/codeindex_agents.exe
B="--bench ../docs/codeindex-agents-multi.json --root C:/tmp/esc306/root"
$X check $B                                  # chaque patch, puis le même privé d'un fichier
$X run $B --out C:/tmp/esc306/out --modes grep,index-hook,tokensave --tokensave C:/tmp/tokensave-target/release/tokensave.exe
$X recheck $B --out C:/tmp/esc306/out --why homonym   # revérifie un run depuis ce que l'agent a écrit
$X report --bench ../docs/codeindex-agents-multi.json --out C:/tmp/esc306/out
```

- **Les tâches** (`docs/codeindex-agents-multi.json`, commit `9632cde9`), dites comme un ticket, sans la liste des endroits :

  | Tâche | Genre | Endroits | Fichiers |
  | --- | --- | --- | --- |
  | `plugins-forget` | renommer `Runtime::forget` en `unload` (15 autres `forget` dans le code) | 16 | 3 |
  | `drafts-files` | renommer `DataDir::drafts_files`, un nom rare | 19 | 7 |
  | `checkout-of` | renommer `git::checkout_of` (homonyme : `Core::checkout_of` de `repos.rs`) | 19 | 5 |
  | `account-skip` | ajouter un paramètre `skip` à `account_for_new` et suivre ses 13 appels | 13 | 6 |
  | `commit-root` | ajouter un champ sérialisé à `CommitScope` : Rust, `types.ts`, la fenêtre, fr/en, tests | 9 | 8 |
  | `i18n-unverified` | renommer une clé i18n de 5 composants Svelte (deux autres clés `unverified`) | 8 | 7 |
  | `fact-docs-gaps` | renommer un fait du protocole : cœur, deux schémas, SDK, doc fr/en | 21 | 9 |
  | `app-env-struct` | remplacer un tuple par une structure dans une signature et suivre les erreurs | 8 | 6 |
  | `slugify-gone` | supprimer `core::slugify` et appeler `slug_within(…, 40)` à sa place | 21 | 4 |
  | `missing-account-ts` | renommer une fonction TypeScript de 7 fichiers | 18 | 7 |

  « Endroits » : ce qu'une copie neuve n'a pas encore (chaque texte attendu, chaque ancien nom à faire partir).

- **La vérification** est mécanique, dans `probes/multi.rs` :
  - chaque endroit attendu (un texte, tant de fois dans un fichier) ;
  - aucun ancien nom dans ses fichiers (le CHANGELOG n'en fait jamais partie) ;
  - les homonymes intacts (`kept` : `Core::checkout_of`, les autres `forget`, les chaînes `slugify` des tests) ;
  - les commandes du projet : `cargo check --lib --tests`, les tests ciblés (`cargo test --lib <filtre>`), `npm run check`, `npx vitest run <fichier>`, `node sdk/plugin/scripts/generate.mjs --check`.
  - **Essayée à la main** : `check` applique la bonne réponse (`docs/codeindex-agents-multi/<tâche>.patch`, écrite à la main), puis la même privée d'un fichier. Les 10 prennent la bonne et refusent la mauvaise. Le premier essai a refusé deux de mes bonnes réponses : un `root` déplacé avant son `clone` dans `commit-root` (la compilation l'a vu), et un compte faux dans `slugify-gone`.
- **Les modes**, comme IDX-4, plus un terminal :
  - (a) `grep` : Grep, Glob, Read, Edit, Write ;
  - (b) `index-hook` : en plus, les outils de l'index, la règle des consignes et le hook d'ESC-253 ;
  - (c) `tokensave` : son serveur MCP et ses règles à la place de ceux d'Escouade.
  - **Le terminal ne sert qu'aux vérifications** : Bash est permis pour `cargo check`, `cargo test`, `npm run check`, `npx vitest run` et le générateur du SDK, une commande seule. Claude Code laisse passer de lui-même les commandes en lecture (`ls`, `grep`), quelles que soient ses règles. Un hook (`snap`) refuse donc toute autre commande, et note avant chaque vérification ce qui manque encore à la copie. Les commandes refusées sont comptées à part.
- **Les agents** :
  - `claude -p` sur `claude-sonnet-5-5` et `claude-haiku-5-5` en effort `medium`, deux passages par tâche, mode et modèle : 120 runs ;
  - plus `claude-opus-5-5`, ajouté à la demande de l'utilisateur, sur 5 tâches, en `grep` et `index-hook`, un passage : 10 runs ;
  - le compte par défaut du poste, avec les mêmes options qu'IDX-4 : aucun fichier de connexion touché, aucun réglage global changé.
- **Les couloirs** : un par mode et par modèle (`grep-sonnet`…). Chacun a sa copie au commit, sa cible cargo préchauffée hors de la copie, et une copie de `node_modules` (vite ne sert aucun fichier hors du projet, pas même à travers une jonction). Les six couloirs tournent en parallèle ; les vérifications finales passent une à une.
- **tokensave**, avec les règles d'ESC-304 et d'IDX-4 :
  - `disable-upload-counter` d'abord, et `upload_enabled = false` vérifié ;
  - `TOKENSAVE_UPDATE_CHECK=off` ;
  - une copie sans `.git` ;
  - jamais `install`, jamais de hook.
  - À la fin, `~/.tokensave` (absent au départ) contenait `config.toml`, `state.toml`, `benchmark.toml`, `global.db`, deux `.mmap` et `servers/` (vide). `state.toml` disait `last_upload_at = 0`, `last_flush_attempt_at = 0`, `last_version_check_at = 0`, `installed_agents = []`, et 73 852 tokens en attente. Il a été supprimé. Aucun réglage d'agent ne cite tokensave (`~/.claude/settings.json`, `.claude.json`, `.codex/config.toml`, `.gemini/settings.json`, `.copilot/config.json`).
- **L'index d'avant ESC-307** : la passe a commencé avant qu'ESC-307 arrive sur main. Le mode index + hook a donc encore la ligne « Économie… » dans chaque réponse de l'index (quelques dizaines de tokens par appel, au détriment de l'index), et les consignes qui promettaient « moins de tokens que grep ». Opus a été joué sur le même banc, pour que ses chiffres se comparent.
- **Ce qui est compté**, run par run (`docs/codeindex-agents-multi-results.md`, et les 130 runs bruts dans `docs/codeindex-agents-multi-runs.jsonl`) :
  - la réussite ;
  - les endroits manquants à la **première vérification**, et à la fin ;
  - les vérifications (et celles qui ont échoué), les cycles « vérifier → corriger » (une vérification suivie d'un Edit ou d'un Write) et les commandes refusées ;
  - comme IDX-4 : les appels par outil, les re-greps, les relectures, les tokens, le coût (`total_cost_usd`), le temps et les tours.

### Le coût, mesuré d'abord

- **Pilote** : `plugins-forget` et `commit-root`, 3 modes × 2 modèles, un passage, soit 12 runs. Il a coûté 1,59 $ (Sonnet 1,31 $, Haiku 0,28 $) et 9,8 M tokens. Le quota de la semaine est passé de 43 à 44 % (cette session comprise), celui de 5 h de 11 à 15 %.
- **Estimé** pour les 120 runs : ≈ 16 $, soit ≈ 2 % du quota de la semaine. C'est sous les 8 % de l'accord du 2026-10-30, et le quota était sous 70 % : la passe a été lancée sans attendre. Le coût a été écrit sur la carte du ticket.
- **Opus** : mesuré sur son premier run de chaque mode (0,31 $ en grep, 0,40 $ en index + hook), puis estimé à ≈ 6 $ pour 10 runs, sous les 3 % demandés. Réel : 4,47 $.
- **Réel** : 12,31 $ et 40,1 M tokens pour les 130 runs gardés, plus 0,30 $ pour les trois runs écartés (voir plus bas). Le quota de la semaine est à 45 % à la fin (43 % au départ, cette session comprise).
- **La passe a été arrêtée à la main** à 119 runs sur 120, à la demande de l'utilisateur, pour lancer Opus. Deux runs ont été rejoués seuls :
  - `missing-account-ts` (index + hook, Haiku, passage 2), coupé par l'arrêt ;
  - `slugify-gone` (index + hook, Haiku, passage 2), interrompu à la main par l'utilisateur pendant la passe. Son « timed out after 1800 s » n'était pas un échec de l'agent.

### Sonnet et Haiku, par mode (20 runs chacun)

| | Sonnet grep | Sonnet index + hook | Sonnet tokensave | Haiku grep | Haiku index + hook | Haiku tokensave |
| --- | --- | --- | --- | --- | --- | --- |
| Justes | 20 | 20 | 20 | 20 | 20 | 20 |
| Coût | 2,23 $ | 2,25 $ | 2,46 $ | 0,33 $ | 0,26 $ | 0,32 $ |
| · face à grep | — | **+1 %** | +10 % | — | **−23 %** | −4 % |
| Tokens (cache lu compris) | 2,9 M | 3,7 M | 4,6 M | 8,4 M | 7,4 M | 8,7 M |
| Appels d'outils | 457 | 456 | 382 | 619 | 588 | 529 |
| · Grep / Read | 73 / 82 | 61 / 80 | 50 / 51 | 83 / 205 | 76 / 171 | 93 / 101 |
| · outils de l'index | — | 23 | 97 | — | 32 | 123 |
| Re-greps après l'index | — | 53 | 42 | — | 44 | 58 |
| Relectures | 27 | 23 | 7 | 70 | 50 | 37 |
| Endroits manquants à la 1re vérification | 6 | 4 | 19 (\*) | 0 | 8 | 6 |
| Endroits manquants à la fin | 0 | 0 | 0 | 0 | 0 | 0 |
| Vérifications (échouées) | 41 (5) | 36 (2) | 39 (3) | 78 (8) | 65 (9) | 55 (5) |
| Cycles vérifier → corriger | 10 | 9 | 10 | 5 | 4 | 2 |
| Commandes refusées | 9 | 7 | 13 | 4 | 2 | 1 |
| Hook : recherches vues / conseillées | — | 104 / 20 | — | — | 143 / 16 | — |
| Temps cumulé | 57 min | 53 min | 57 min | 121 min | 97 min | 92 min |
| Tours | 477 | 476 | 402 | 639 | 608 | 549 |

(\*) Les 19 viennent d'un seul run : un `sed -i` refusé, que Sonnet a cru passé avant de lancer `cargo check` (relu plus bas). Ce n'est pas un oubli.

- **Le bruit** : deux passages d'une même tâche diffèrent de 16 % en médiane, chez Sonnet comme chez Haiku. Le +1 % de Sonnet n'est donc pas un écart. Le −23 % de Haiku le dépasse ; le +10 % de tokensave sur Sonnet est au bord.
- **Les oublis** : peu d'endroits manquent à la première vérification, avec ou sans index (Sonnet 6 contre 4, Haiku 0 contre 8, sur 374 endroits par mode). Ce qui manque, c'est d'abord ce qu'aucun outil de code ne relie : les fixtures d'un test TypeScript, un schéma JSON, la copie du schéma sur le site. La compilation, `npm run check` ou le générateur du SDK le montrent, et l'agent corrige. Aucun run ne finit avec un endroit oublié.
- **Sonnet se sert peu de l'index** : 23 appels en 20 runs (`index_callers` 10, `index_search` 7, `index_body` 6), presque toujours suivis d'un Grep (53 re-greps). Il passe par Grep pour avoir les lignes à éditer.

### Où l'index aide, où il ne sert à rien

Le coût face à grep, tâche par tâche (deux passages chacune) :

| Tâche | Sonnet index + hook | Sonnet tokensave | Haiku index + hook | Haiku tokensave |
| --- | --- | --- | --- | --- |
| `plugins-forget` | −10 % | +90 % | −8 % | +11 % |
| `drafts-files` | −6 % | +6 % | −35 % | −44 % |
| `checkout-of` | +29 % | −10 % | +18 % | +3 % |
| `account-skip` | −9 % | +32 % | −37 % | −27 % |
| `commit-root` | −3 % | +6 % | −44 % | 0 % |
| `i18n-unverified` | −14 % | +5 % | +6 % | +6 % |
| `fact-docs-gaps` | +59 % | +22 % | −14 % | −25 % |
| `app-env-struct` | −5 % | +16 % | −5 % | +4 % |
| `slugify-gone` | −7 % | −17 % | +32 % | +3 % |
| `missing-account-ts` | +4 % | −36 % | +5 % | +31 % |

- **Il aide Haiku** sur les changements Rust aux appels nombreux (`account-skip` −37 %, `drafts-files` −35 %) et sur `commit-root` (−44 %). Haiku y relit moins (50 relectures contre 70).
- **Il ne sert à rien** quand ce qui change est un texte et pas un symbole. `index_search docs.gaps` ne trouve pas un nom de fait dans un JSON : `fact-docs-gaps` coûte 59 % de plus à Sonnet avec l'index. Une clé i18n ou une fonction TypeScript se trouvent aussi bien par Grep.
- **Les homonymes** n'ont trompé personne, dans aucun mode : aucun run n'a touché `Core::checkout_of`, les autres `forget` ni les autres clés `unverified`. Sur ces tâches, grep ne mêle pas les homonymes, parce que l'agent lit la ligne avant de l'éditer.
- **tokensave** fait éditer par ses propres outils (`tokensave_str_replace`, `tokensave_multi_str_replace`), sans Read avant. Les agents y relisent donc peu (7 relectures pour Sonnet), mais il coûte 10 % de plus à Sonnet, et 4 % de moins à Haiku.

### Opus (indicatif : un passage, 5 tâches, 10 runs)

Les 5 tâches sont variées :

- `plugins-forget` : un nom à homonymes ;
- `account-skip` : un paramètre et ses appels ;
- `commit-root` : un champ propagé de Rust jusqu'aux textes ;
- `fact-docs-gaps` : un renommage hors du code (JSON, SDK, doc) ;
- `missing-account-ts` : le front seul.

| Sur ces 5 tâches | Opus grep | Opus index + hook | Sonnet grep | Sonnet index + hook | Haiku grep | Haiku index + hook |
| --- | --- | --- | --- | --- | --- | --- |
| Runs (justes) | 5 (5) | 5 (5) | 10 (10) | 10 (10) | 10 (10) | 10 (10) |
| Coût | 2,11 $ | 2,36 $ | 1,34 $ | 1,39 $ | 0,24 $ | 0,16 $ |
| · face à grep | — | **+12 %** | — | +3 % | — | −33 % |
| Endroits manquants à la 1re vérification | 4 | 4 | 6 | 4 | 0 | 8 |
| Cycles vérifier → corriger | 1 | 2 | 8 | 8 | 5 | 3 |
| Outils de l'index / re-greps | — | 9 / 10 | — | 11 / 30 | — | 16 / 28 |

- Opus fait tout juste, avec moins de cycles que Sonnet.
- Il appelle l'index 9 fois en 5 runs, et refait un Grep après.
- La tendance se confirme : l'index aide Haiku (−33 %), il est neutre pour Sonnet (+3 %) et il coûte à Opus (+12 %).
- Avec un seul passage, l'écart d'Opus reste sous le bruit d'une tâche : c'est une tendance, pas une mesure.

### Ce qui a été relu à la main

- `plugins-forget`, index + hook, Haiku, passage 2, refusé pour « homonyme touché ». La 8ᵉ chaîne `plugin_runtime.forget(` de `codeindex_links_tests.rs` n'est pas une fixture : elle est dans le test ignoré `on_this_repository_runtime_forget_has_its_seven_callers`, qui compte sur le dépôt les appelants réels de `Runtime::forget`. La renommer avec la méthode est juste : ma vérification était trop stricte.
  - Elle a été élargie : 7 fixtures gardées au lieu de 8 (champ `widened`).
  - Les 6 runs refusés pour cela ont été revérifiés en entier par `recheck`. Il rejoue sur une copie neuve les Edit, les Write et les éditions de tokensave du run, puis lance toutes les commandes. Les 6 sont justes, et `report` les liste à part.
- `plugins-forget`, grep, Sonnet, pilote. Sa réponse est exactement le bon patch, mais `cargo test` avait échoué : 10 tests de `plugins_runtime_tests`, qui lancent des processus, flanchent quand six couloirs compilent à la fois.
  - Depuis, le banc vérifie un run à la fois, avec une seconde chance.
  - Les deux runs du pilote touchés ont été rejoués.
- `drafts-files`, tokensave, Sonnet, passage 2 : un `sed -i` sur 7 fichiers, refusé par le hook. Sonnet lance alors `cargo check` sans avoir rien changé, d'où les « 19 endroits manquants ». Puis viennent 10 `tokensave_str_replace` et un Grep de contrôle, et le changement est juste.
- `fact-docs-gaps`, index + hook, Sonnet, passage 1 :
  - `index_search docs.gaps` d'abord (inutile : c'est une chaîne), puis Grep ;
  - il édite à la main `protocol.ts`, que le générateur du SDK réécrit depuis le schéma ;
  - les deux schémas JSON lui manquent jusqu'à ce que le générateur le dise. Il lui faut 2 cycles.
- `commit-root`, grep, Sonnet, pilote : le Rust, le type, la fenêtre et les textes sont faits, mais les fixtures du test manquent, et `npm run check` les montre. Un `vitest … | grep` est refusé : il relance sans le tube.
- `checkout-of`, index + hook, Sonnet, passage 1 : `index_callers checkout_of`, puis un Grep (que le hook lui conseillait), et 11 Edit sans erreur. Il écrit qu'il n'a pas touché `Core::checkout_of` de `repos.rs`, « une autre fonction ».
- `slugify-gone`, index + hook, Haiku, passage 2 (rejoué) : la vérification finale a échoué sur `link.exe` (LNK1104, un fichier encore ouvert dans la cible du couloir), pas sur le code. `recheck` le trouve juste.

### Pour décider

- **Économie** : sur un ticket qui touche beaucoup d'endroits, l'index d'Escouade fait économiser 23 % à un vrai agent Haiku, et rien à Sonnet (+1 %). Il coûte 12 % de plus à Opus (indicatif). Sur les changements à un endroit d'IDX-4, Haiku perdait 14 %.
- **Oublis** : il n'en fait pas faire moins. Les oublis viennent des fichiers qu'aucun outil de code ne relie (schémas, fixtures, copies), et la compilation les rattrape de toute façon.
- **tokensave** ne fait pas mieux : +10 % avec Sonnet, −4 % avec Haiku.
- **La conclusion** : tant que l'utilisateur travaille surtout avec Sonnet et Opus, l'index ne vaut pas la peine pour le coût. Il ne vaut la peine qu'avec Haiku. Il pourrait gagner là où il perd le plus, s'il savait chercher les chaînes (noms de faits, clés).
