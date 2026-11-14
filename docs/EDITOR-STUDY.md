# L'éditeur de VS Code dans Escouade : l'étude (ESC-444)

Étude du 2026-11-14, pour décider. La question de l'utilisateur : « Est-ce qu'il y a un monde où on arrive à intégrer l'éditeur de VS Code (arbo + code et tabs) dans notre éditeur ? » La réponse : **oui, ça tourne**, avec ses coûts mesurés ci-dessous.

Le livrable est ce document, la **maquette** de la branche `ticket/esc-444` (commitée, jamais fusionnée) et ses mesures. La branche `study/esc-444`, poussée pour la seule CI de mesure, porte le même code. Aucun comportement du produit ne change : la vue VS Code n'existe que derrière un réglage de développement, sur une branche qui ne sera pas fusionnée. La spec (pages *Éditeur*) n'est donc pas touchée.

**Comment lire les sources.** Chaque affirmation dit d'où elle vient :

- **[vérifié]** : exécuté ici (e2e, banc, essai à la main) ou lu dans le code téléchargé (paquets npm, serveurs de VS Code et de VSCodium), le 2026-11-14 ;
- **[lu]** : vient d'une documentation ou d'un texte de licence, non vérifié par une mesure.

Les sources numérotées sont au §9.

## En bref

⟨EN-BREF⟩

## 1. La maquette

### 1.1 Ce qui tourne

Le réglage de développement `localStorage['escouade.dev.vscodeEditor'] = '1'` (puis la fenêtre rechargée) met la vue VS Code à la place de CodeMirror dans l'éditeur d'un projet, seul ou dans un volet. Tout ce qui suit est vérifié par `tests/e2e/vscode-view.spec.ts` sous WebView2 (2 tests, 26 s et 13 s) **[vérifié]** :

- **La vue de VS Code** : le plan de travail de VS Code 1.138 (`@codingame/monaco-vscode-api` 37.6.1, MIT) avec son explorateur, ses onglets, son éditeur, sa recherche, sa palette, sa barre d'état. Capture : `editor-study/windows-alone.png`.
- **Les fichiers par le cœur** : un fournisseur de système de fichiers de VS Code (`src/lib/vscode/fs-provider.ts`) sur les commandes de l'éditeur d'aujourd'hui (`fs_tree`, `fs_read`, `fs_write`, `fs_create`, `fs_mkdir`, `fs_rename`, `fs_delete`). La page ne lit jamais le disque. Un fichier en CRLF modifié dans VS Code et enregistré par Ctrl+S garde ses CRLF, et l'écriture est refusée si le fichier a changé sur le disque depuis sa lecture (`expectedHash`).
- **Un serveur de langage lancé par le cœur** : `lsp_start` lance `typescript-language-server` ou `rust-analyzer` dans le dossier du projet (un nom de serveur, jamais un programme donné par la page), et relaie ses messages par l'IPC de la fenêtre (`lsp_send`, événement `vscode-study-lsp`, `src-tauri/src/vscodestudy.rs`). `monaco-languageclient` 11.0.3 les relie à la vue (`src/lib/vscode/lsp.ts`). L'erreur de type de `src/app.ts` est soulignée, et listée dans les problèmes. rust-analyzer est branché de la même façon, mais seul TypeScript est essayé par les tests.
- **Le thème d'Escouade** : ses jetons de couleur (`app.css`, en oklch et color-mix) sont lus tels que le navigateur les calcule, rendus en `#rrggbbaa`, et posés sur 74 clés d'un thème de couleurs de VS Code et sur les portées de sa coloration (`src/lib/vscode/theme.ts`). Le thème est enregistré comme une extension, « Escouade », et appliqué par `workbench.colorTheme`.
- **Des extensions d'Open VSX** : Dracula (un thème), Vim (une extension web avec du code, activée sans erreur) et Even Better TOML (une extension de langage) s'installent depuis Open VSX et tournent dans l'hôte d'extensions web de la page (un worker). Le thème Dracula s'applique, puis on revient à celui d'Escouade. Capture : `editor-study/windows-dracula.png`.
- **Une liste d'extensions permises** : le réglage `extensions.allowed` de VS Code. Tokyo Night, hors de la liste, est refusée, puis installée une fois la liste levée.
- **Dans une colonne** : la vue est mise dans un volet à côté d'un agent, échangée de place avec lui, redimensionnée par la poignée, activée par un clic, fermée par son ×, puis rouverte. Ses onglets sont les mêmes à chaque étape, et l'hôte d'extensions n'a démarré qu'une fois. Voir le §2.
- **L'hôte d'extensions Node** : le serveur de VSCodium, lancé par le cœur avec le Node du poste, en boucle locale, avec un jeton. Code Spell Checker et ESLint (avec l'eslint du projet) installés depuis Open VSX y donnent leurs diagnostics. Les fichiers passent toujours par le cœur (44 lectures). Voir le §3.2. Capture : `editor-study/windows-node-host.png`.

### 1.2 Comment c'est branché

| Pièce | Où | Ce qu'elle fait |
| --- | --- | --- |
| Le plan de travail | `src/lib/vscode/workbench.ts` | Démarre une fois par page les services de VS Code (les « service overrides » de 25 paquets de monaco-vscode-api, 12 extensions de langage par défaut). L'espace de travail est un fichier `.code-workspace` en mémoire. Chaque projet ouvert y ajoute son dossier. |
| La vue | `src/components/editor/VscodeView.svelte` | Prête sa place au plan de travail, avec une barre (« Conversation », ce qu'elle est). Relaie à Escouade le fichier demandé (`app.openEditor` : un lien d'une conversation, Ctrl+P). |
| Le choix | `App.svelte`, `panes/Pane.svelte`, `src/lib/vscode/flag.svelte.ts` | `vscodeFlags.editor` : la vue VS Code ou `EditorView` (CodeMirror). |
| Les fichiers | `src/lib/vscode/fs-provider.ts`, `tree.ts` | Le fournisseur de fichiers sur les commandes `fs_*`, en `file:`, ou en `vscode-remote:` avec l'hôte Node. |
| Le langage | `src/lib/vscode/lsp.ts` ; `lsp_start`, `lsp_send`, `lsp_stop` | Le transport de `monaco-languageclient` sur l'IPC. Côté cœur, le cadrage `Content-Length` du protocole. |
| Le thème | `src/lib/vscode/theme.ts` | Les jetons d'Escouade vers un thème de VS Code. |
| L'hôte Node | `vscode_host_start` | `server-main.js` du serveur, lancé avec le Node du poste. |
| Le banc | `src/lib/vscode/study.ts`, `scripts/study/editor-bench.mjs`, `.github/workflows/study-esc-444.yml` | Voir le §4.1. |

Il a fallu changer la CSP de la fenêtre (`tauri.conf.json`) : `'unsafe-eval'`, `blob:` pour les workers et les iframes, les domaines d'Open VSX, et `ws://127.0.0.1:*` pour l'hôte Node. Voir le §3.3.

### 1.3 Ce qui a coincé, et comment

- **Les versions.** `monaco-languageclient` 11.0.3, la dernière, exige `@codingame/monaco-vscode-api` ^37, alors que celle-ci en est à la 39 (VS Code 1.145). La maquette prend donc la 37.6.1, soit VS Code 1.138 (commit `7debcd0e`), et `vscode` et `monaco-editor` en alias npm des paquets de CodinGame **[vérifié]**. Le client de langage a donc deux versions majeures de retard, et chaque montée de version demande de déplacer tout l'ensemble à la fois.
- **Un iframe déplacé dans la page se recharge.** Au premier essai, l'élément du plan de travail était déplacé dans la vue qui le montre. L'iframe de l'hôte d'extensions web s'est rechargé, envoyant deux fois son port, et VS Code l'a arrêté (« UNEXPECTED message », code 81) **[vérifié]**. Il en irait de même pour toute webview de VS Code (aperçu Markdown, vues d'extensions). La maquette garde donc le plan de travail dans une **couche fixe, jamais déplacée**, et le cale à chaque image sur la place de la vue qui le montre. Il est caché quand aucune vue ne le montre. Le mode « views » de monaco-vscode-api, qui accroche chaque partie (explorateur, éditeur) à un élément de la page, déplace lui aussi le DOM des parties (`attachPart` fait `container.append`) **[vérifié]**, et n'y change rien.
- **Les clics passent par la couche, pas par le volet.** Le plan de travail n'est pas dans le volet du DOM : un clic dessus n'activait pas le volet, et son survol ne montrait pas le × du volet. La vue relaie donc ces deux gestes au volet (`Through` dans `workbench.ts`) **[vérifié]**.
- **Les workers.** Le worker de l'hôte d'extensions est lancé depuis un blob de son iframe, où un chemin relatif ne veut rien dire : toutes les adresses des workers sont absolues, et en modules ES (`worker.format: 'es'` dans `vite.config.ts`) **[vérifié]**.
- **Le code des extensions web passe par `eval`.** L'hôte d'extensions web de VS Code charge le code d'une extension puis l'évalue. La CSP d'Escouade l'interdisait (« EvalError »), et la page a dû recevoir `'unsafe-eval'` **[vérifié]**.
- **Open VSX demande confiance.** `workbench.extensions.installExtension` attend la réponse à la question « Do you trust the publisher … ? » (« Trust Publisher & Install ») pour chaque nouvel éditeur. Capture : `editor-study/windows-trust.png` **[vérifié]**.
- **Le serveur de VS Code de Microsoft refuse ce client.** Voir le §3.2.
- **Les serveurs survivaient à l'app.** Le serveur de langage et l'hôte Node, gardés par le cœur, restaient en vie après la sortie de l'app, et tenaient ouverts les tubes du banc. Le cœur les arrête maintenant avec leurs enfants en sortant (`vscodestudy::shutdown`, `taskkill /T` ou groupe de processus). Un produit les mettrait dans un Job, comme les agents (`escouade_model::job`), pour qu'ils meurent même quand l'app est tuée **[vérifié]**.

### 1.4 Lancer la maquette

```sh
git switch ticket/esc-444 && npm ci
npx tauri build --debug --no-bundle
# Les deux e2e (le second seulement si le serveur de VSCodium est donné) :
ESCOUADE_E2E_CDP_PORT=<port> ESCOUADE_STUDY_VSCODE_SERVER=<dossier du REH> npx playwright test tests/e2e/vscode-view.spec.ts
```

À la main : dans les outils de développement de la fenêtre, `localStorage.setItem('escouade.dev.vscodeEditor', '1')` (et `escouade.dev.vscodeNodeHost` pour l'hôte Node, avec `ESCOUADE_STUDY_VSCODE_SERVER` dans l'environnement de l'app), puis recharger la fenêtre et ouvrir l'éditeur d'un projet.

## 2. Dans une colonne, et plusieurs colonnes

**Une colonne, déplaçable, redimensionnable, sans perdre ses onglets : oui** **[vérifié]**. L'éditeur est déjà un contenu de volet de l'écran partagé (`{ kind: 'editor' }`, ESC-392). La vue VS Code s'y met comme CodeMirror, en tirant la puce « Éditeur » de la barre de titre sur la zone de travail. L'e2e la met à côté d'un agent, l'échange de place avec lui, la redimensionne, l'active d'un clic, la ferme par son × et la rouvre. Ses deux onglets ouverts restent les mêmes, et l'hôte d'extensions ne redémarre jamais. C'est la couche fixe du §1.3 qui le permet : le plan de travail ne quitte jamais la page, seule sa place change.

Une limite de l'écran partagé, pas de VS Code : quand le volet de l'agent n'a plus la place d'être coupé en deux, la case « gauche » de la boussole est barrée. Elle l'est même pour un contenu qui ne fait que changer de côté. L'e2e passe donc par la case du centre (échanger).

**Plusieurs colonnes d'éditeur.** monaco-vscode-api démarre ses services une fois par page, et le plan de travail n'a qu'**une** partie éditeur. Le pont qui permettrait d'en ouvrir d'autres, `createAuxiliaryEditorPart`, ouvre une autre fenêtre dans VS Code et n'est pas pris en charge ici (`unsupported` dans les services de monaco-vscode-api) **[vérifié]**. Ce qui reste :

| Façon | Possible ? | Ce qu'on a | Coût |
| --- | --- | --- | --- |
| Les **groupes d'onglets** de VS Code, côte à côte dans la colonne | Oui **[vérifié]** (`splitEditorRight`, capture `windows-pane-split.png`) | Plusieurs colonnes d'onglets dans **un** volet d'Escouade | Rien de plus |
| Un **éditeur de code autonome** dans un autre volet d'Escouade, sur le même modèle | Oui **[vérifié]** : même texte, mêmes diagnostics du serveur de langage, une frappe dans l'un modifie l'onglet de l'autre (capture `windows-second-editor.png`) | Le code, sans les onglets ni l'arborescence de VS Code (ceux d'Escouade à faire) | Un éditeur Monaco de plus : quelques Mo |
| **Plusieurs vues VS Code** à la fois | Non dans une page. Seulement avec une page (un iframe ou une webview) par colonne. | Des plans de travail complets mais séparés : leurs modifications non enregistrées et leurs extensions ne se voient pas. | Un plan de travail entier par colonne, soit 130 à 210 Mo de webview mesurés sous Windows pour le premier (§4), à peu près autant pour chacun, et un hôte d'extensions de plus |
| **Une seule vue, qui suit la colonne active** | Oui (c'est la maquette) | Une colonne VS Code à la fois | Rien de plus |

La réponse pour un produit : une colonne « Éditeur VS Code » par projet, avec ses propres groupes d'onglets côte à côte. Puis, si on veut du code dans un autre volet (à côté d'un agent qui travaille), des éditeurs autonomes sur les mêmes modèles. Pas de second plan de travail.

## 3. Les extensions

### 3.1 Sans Node : les extensions web, dans la page

Elles tournent dans l'hôte d'extensions web de VS Code (un worker, dans un iframe de la page). Ce qui ne demande que le navigateur marche. Ce qui demande Node ne s'installe pas (« not available in Escouade for the Web platform »).

| Extension (Open VSX) | Sorte | Résultat |
| --- | --- | --- |
| Dracula Theme | Thème | **Marche** : installé, appliqué **[vérifié]** |
| Vim | Web, avec du code | **Marche** : activée, sans erreur **[vérifié]** |
| Even Better TOML | Langage, web, avec un serveur de langage en worker | **À moitié** : le langage `toml` est reconnu, mais son serveur (`server-worker.js`, 35 Mo) n'est pas servi par Open VSX (503 sur son point `unpkg`) **[vérifié]** |
| Tokyo Night | Thème | **Marche** (refusée tant que la liste permise l'exclut) **[vérifié]** |
| Prettier | Node | **Ne s'installe pas** sans hôte Node : « not available … for the Web platform » **[vérifié]** |
| Code Spell Checker | Node (sa version web est limitée) | **Ne s'installe pas** sans hôte Node : « not available in this setup » **[vérifié]** |
| GitHub Theme | Thème, avec du code Node | **Ne s'installe pas** sans hôte Node **[vérifié]** |

Deux limites d'Open VSX pour les extensions web, toutes deux vues ici : son point `unpkg` refuse les gros fichiers (le 503 ci-dessus), et sert le JavaScript en `text/plain` avec `X-Content-Type-Options: nosniff`, ce qui fait échouer un `importScripts` de worker **[vérifié]** (`curl -I`). Le remède serait de servir les extensions nous-mêmes. Le cœur téléchargerait le VSIX, vérifierait sa somme et sa signature (§3.3), puis servirait ses fichiers par un protocole à lui, avec les bons types MIME.

Dans les deux modes, une extension qui prend le clavier (Vim) prend aussi les touches d'un éditeur autonome hors du plan de travail **[vérifié]** (l'e2e l'installe en dernier pour cette raison). Il faudrait arbitrer les raccourcis d'Escouade et ceux de VS Code.

### 3.2 Avec Node : l'hôte d'extensions de VS Code, lancé par le cœur

**Le serveur.** L'hôte d'extensions Node de VS Code est son serveur (le « remote extension host », REH), celui de « Remote SSH » ou d'OpenVSCode Server. La vue s'y relie par la prise en charge du serveur distant de monaco-vscode-api (`remoteAuthority`, `connectionToken`). Le cœur le lance avec le Node du poste : `node out/server-main.js --host 127.0.0.1 --port … --connection-token-file … --server-data-dir … --extensions-dir …`.

- **Le serveur de Microsoft refuse ce client.** Celui du même commit que la vue (VS Code 1.138, 195 Mo en zip, 606 Mo décompressé) répond « Unauthorized client refused » à chaque connexion **[vérifié]**. Il embarque `vsda`, un module natif fermé qui fait signer un défi au client, et seul le VS Code de Microsoft sait le signer. Sa licence le dit aussi : on ne peut l'utiliser « with each validly licensed copy of Microsoft Visual Studio Code », et ses limites techniques en font partie **[lu]** (S4).
- **Celui de VSCodium marche.** VSCodium REH (MIT, sans `vsda`) en 1.140.06880, la version la plus proche publiée (VSCodium n'a pas de 1.138, et OpenVSCode Server s'arrête à 1.109), accepte la vue une fois le `commit` de son `product.json` réécrit à celui du client **[vérifié]**. Le client 1.138 et le serveur 1.140 se parlent sans souci dans nos essais, mais rien ne le garantit : un produit construirait son REH depuis Code-OSS au commit exact du client.
- **Son poids** : 82 Mo (Windows, macOS arm64) à 89 Mo (Linux) en archive, 252 Mo décompressé sous Windows. Il embarque son propre Node (v24.21.0, 89 Mo) **[vérifié]**.
- **Les fichiers restent au cœur.** Avec un serveur distant, les extensions Node ne voient que les fichiers `vscode-remote:`. Un espace de travail en `file:` les laissait sans diagnostic **[vérifié]**. L'espace de travail passe donc en `vscode-remote://127.0.0.1:<port>/…`, servi par le même fournisseur de fichiers sur les commandes du cœur. Les extensions, côté serveur, lisent le disque elles-mêmes, comme dans VS Code.
- **Ce qui marche** : Code Spell Checker (installé en 1,5 s) et ESLint 3.0.34 avec l'eslint 9 du projet donnent leurs diagnostics (« "recieve": Misspelled word », « 'inutile' is assigned a value but never used ») **[vérifié]**.
- **Ce qui coince** : les ressources que sert le serveur (grammaires de ses extensions intégrées, icônes, webviews) sont refusées par CORS, puisque la page est sur `tauri.localhost` et le serveur sur `127.0.0.1` **[vérifié]**. Ce sont des erreurs dans la console, sans effet sur les diagnostics. Il faudrait que le cœur serve ces ressources, ou retirer les extensions intégrées du REH, qui doublonnent celles de la page.

**Les versions de Node** **[vérifié]**, avec le REH 1.140 (dont le Node est 24.21.0) :

| Node | Modules natifs du serveur (11, dont node-pty, spdlog, @parcel/watcher, sqlite3) | Le serveur | ESLint et Code Spell Checker |
| --- | --- | --- | --- |
| 24.14.0 (celui du poste) | 11/11 chargés | Démarre | Marchent |
| 22.23.5 | 11/11 chargés | Démarre | Marchent |
| 20.20.2 | 11/11 chargés | **Ne démarre pas** : `node:module` n'exporte pas `registerHooks` | — |

Les modules natifs ne cassent pas : ils sont en **N-API**, dont l'ABI est stable d'une version de Node à l'autre (chargés par `process.dlopen` sous les trois versions). Ce qui casse, c'est l'API JavaScript : le serveur appelle `module.registerHooks`, arrivé avec Node 22.15 et 23.5 **[lu]** (S8). Le minimum est donc Node 22.15. Un module natif compilé pour une seule ABI (hors N-API) casserait, lui, avec une autre version majeure. C'est le cas de certaines extensions, qui publient des VSIX par plateforme : elles visent le Node du serveur de VS Code, pas celui du poste **[lu]**.

**Télécharger le Node exact, ou prendre celui du poste ?** Le REH embarque déjà le Node qu'il veut (89 Mo sous Windows). La voie la plus simple et la plus sûre est de **télécharger le REH entier à la demande**, la première fois qu'on installe une extension Node : environ 85 Mo, avec son Node. Pas besoin de Node sur le poste, et pas de surprise de version. Prendre le Node du poste n'économise que les 89 Mo du binaire, et suppose un Node ≥ 22.15. Si on le garde comme option, il faut vérifier sa version avant de lancer le serveur, et retomber sur le téléchargement en dessous.

### 3.3 La sécurité

- **Boucle locale et jeton** : le serveur écoute sur `127.0.0.1` seulement. Son jeton de 128 bits (aléa du système) est passé par un fichier (`--connection-token-file`), pas sur sa ligne de commande, que tout processus du poste peut lire. Sans le jeton, `/` répond 403 et la connexion WebSocket est refusée. Seul `/version` répond sans jeton, comme dans tout serveur de VS Code **[vérifié]**.
- **Ses données à part** : sans `--server-data-dir` ni `--extensions-dir`, le serveur écrit dans `~/.vscode-server`, le dossier du VS Code de l'utilisateur. Le premier essai l'a créé (puis il a été retiré). Les deux options sont maintenant toujours données, dans le dossier de données d'Escouade **[vérifié]**.
- **Les droits des extensions** : comme dans VS Code, les extensions Node tournent avec les droits de l'utilisateur, sans bac à sable. Les extensions web tournent dans un worker de la page : elles n'ont pas l'IPC d'Escouade, mais elles ont le réseau que la CSP permet.
- **La CSP** : la maquette a dû ajouter `'unsafe-eval'` au script de la page d'Escouade, pour l'hôte d'extensions web. Un produit ne le ferait pas dans la page principale : il servirait l'iframe de l'hôte d'extensions d'une origine à part (un protocole du cœur avec sa propre CSP), comme le fait vscode.dev.
- **L'origine et la signature des extensions** : Open VSX (Eclipse Foundation) publie pour chaque version une signature, une somme SHA-256 et une clé publique, et marque les espaces de noms vérifiés (Vim, Dracula, ESLint : vérifiés) **[vérifié]** (API d'Open VSX). **Rien ne les vérifie dans la maquette.** Les extensions web sont chargées fichier par fichier depuis Open VSX. La vérification des signatures de VS Code passe par `@vscode/vsce-sign`, un module fermé de Microsoft qui n'est ni dans le web, ni dans le REH de VSCodium. Un produit ferait vérifier par le cœur la somme et la signature de chaque VSIX (avec la clé publique d'Open VSX) avant de le servir.
- **La liste d'extensions permises** : `extensions.allowed` de VS Code marche ici, par éditeur ou par extension **[vérifié]**. Une équipe pourrait l'imposer par `.escouade.yml` ou par la politique de son serveur. Le message de refus de VS Code est trompeur (« not available in Escouade for the Web platform »), et serait à remplacer par le nôtre.
- **Une dépendance à surveiller** : monaco-vscode-api 37.6.1 épingle `dompurify` 3.4.15, sous le coup d'un avis critique (corrigé en 3.4.17, la version d'Escouade) **[vérifié]** (`npm audit`).

## 4. Les mesures

### 4.1 Comment

Aucun test ne pilote la page sous WebKitGTK ni sous WKWebView. La page déroule donc elle-même le banc (`src/lib/vscode/study.ts`) quand l'app est lancée avec `ESCOUADE_VSCODE_STUDY`, et note ses phases dans un journal. Les phases : l'app au repos ; un projet ajouté ; son éditeur ouvert (temps jusqu'à l'arborescence affichée) ; repos ; dix fichiers de 160 lignes ouverts l'un après l'autre (temps jusqu'au dixième onglet) ; repos ; extensions d'Open VSX ; capture. À côté, `scripts/study/editor-bench.mjs` relève toutes les 250 à 400 ms la mémoire de l'arbre de processus de l'app : le cœur, les processus de la webview (WebView2 ; WebKitGTK ; sous macOS, ceux de `com.apple.WebKit.*`, enfants de launchd, comptés par leur nom) et les programmes lancés par le cœur (serveurs de langage, hôte Node). La mémoire d'une phase est la moyenne des 3 dernières secondes de son repos. C'est la mémoire résidente (RSS) sous Windows et macOS, et la proportionnelle (PSS, qui compte une fois une page partagée) sous Linux.

Le même banc a tourné sur ce poste (Windows 11, fenêtre cachée, deux essais par mode) et sur les machines de la CI de GitHub (run ⟨RUN⟩ du workflow `study-esc-444.yml`, sur la seule branche `study/esc-444`, deux essais par mode) : `ubuntu-24.04` sous Xvfb, `macos-latest`, `windows-latest`. L'app est construite en debug (`tauri build --debug --no-bundle`), avec le frontend de production : les temps et la mémoire de la page sont ceux d'une release, le cœur est un peu plus lourd. Trois modes : **CodeMirror** (l'éditeur d'aujourd'hui), **VS Code** (la vue, et le serveur TypeScript lancé par le cœur), **VS Code + Node** (la vue et l'hôte Node de VSCodium sur le Node du poste, sans le serveur TypeScript du cœur).

⟨MESURES⟩

## 5. Les trois voies

Trois façons d'avoir « l'éditeur de VS Code » dans Escouade. La première a sa maquette, les deux autres sont comparées sans maquette.

**A. La vue VS Code dans l'éditeur d'Escouade (monaco-vscode-api)**, la maquette. Les services du plan de travail de VS Code tournent dans la page d'Escouade, sans Electron. Les fichiers sont ceux du cœur, les serveurs de langage sont lancés par le cœur, le thème est le nôtre, et les extensions web viennent d'Open VSX. L'hôte Node est facultatif.

**B. VS Code entier dans une vue (code-server ou OpenVSCode Server)** **[lu]** (S9). Un serveur Node local sert toute l'interface web de VS Code, qu'Escouade montrerait dans une webview enfant, comme la page d'un lancement (ESC-419). Il a tout, extensions Node comprises. Mais :

- **Node est obligatoire** : il faut l'embarquer ou le télécharger (le serveur fait 80 à 90 Mo en archive, 250 Mo décompressé, Node compris : ce sont les tailles du REH de VSCodium, du même code ; code-server est du même ordre).
- **Un serveur local à protéger**, qui sert aussi les fichiers : il lit et écrit le disque lui-même, pas par le cœur. Tout ce que le cœur sait faire sur les fichiers (refus si le fichier a changé, garde des worktrees des agents, rafraîchissement de git) serait contourné.
- **Une autre app dans Escouade** : son look, ses menus, ses raccourcis, sa barre de titre. Seuls les réglages de couleurs se partagent.
- **Une version en retard** : OpenVSCode Server en est à 1.109 (VS Code est à 1.145) ; code-server suit de plus près.
- **La mémoire** : celle de la vue de la maquette, plus le serveur Node toujours là (le mode « VS Code + Node » du §4 en donne l'ordre).

Effort : une à deux semaines pour un premier branchement, réutilisant la webview enfant d'ESC-419. Mais l'intégration resterait faible (pas de lien avec les volets, ni avec Ctrl+P, ni avec les liens des conversations sans passer par l'URL du serveur).

**C. Garder CodeMirror et lui ajouter les serveurs de langage.** `@codemirror/lsp-client` 6.3.0 (MIT, de l'auteur de CodeMirror, depuis 2025) apporte la complétion, le survol, les diagnostics, les sauts (définition, déclaration, implémentation, type), les références, le renommage, le formatage et l'aide aux signatures **[vérifié]** (ses exports). Le pont du cœur vers les serveurs de langage est celui de la maquette (`lsp_start`, `lsp_send`), utilisable tel quel. Ce qu'il faudrait refaire nous-mêmes :

- les actions de code et corrections rapides, les jetons sémantiques, les indices en ligne (inlay hints), le plan du document (outline), les symboles du projet, la hiérarchie d'appels : rien de cela n'est dans `@codemirror/lsp-client` **[vérifié]** ;
- la gestion des serveurs : lesquels pour quels langages, les trouver ou les installer, leurs réglages ;
- et jamais d'extensions : ni thèmes, ni Vim, ni ESLint autrement que par son serveur de langage, ni Prettier, ni débogueur.

Il garde le design d'Escouade, sa légèreté (CodeMirror : 460 Mo de webview avec 10 fichiers, contre 670 Mo pour la vue VS Code sous Windows), et tout ce que l'éditeur sait déjà faire (marques des changements, comparaison avec le disque, sources des worktrees, recherche flottante).

| | A. Vue VS Code (monaco-vscode-api) | B. VS Code entier dans une vue | C. CodeMirror + serveurs de langage |
| --- | --- | --- | --- |
| Arborescence, onglets, éditeur de VS Code | Oui | Oui | Non (ceux d'Escouade) |
| Serveurs de langage | Oui, par le cœur, et toute la palette de VS Code (actions de code, sémantique, indices) | Oui, par ses extensions | Oui, par le cœur ; sans actions de code, jetons sémantiques ni indices en ligne |
| Extensions | Web sans Node ; Node avec l'hôte facultatif | Toutes celles d'Open VSX | Aucune |
| Fichiers par le cœur | Oui | Non | Oui |
| Node | Facultatif (hôte Node) | Obligatoire | Non |
| Look | VS Code, aux couleurs d'Escouade | VS Code | Escouade |
| Poids du frontend (brotli) | +3,7 Mo | ~0 (le serveur en plus : 80 à 90 Mo) | Négligeable (moins de 0,1 Mo) |
| Mémoire de la webview, 10 fichiers (Windows) | +210 Mo | + celle d'un VS Code web, et son serveur | ≈ CodeMirror |
| Dans une colonne | Une, avec ses groupes d'onglets ; d'autres en éditeurs autonomes | Une webview par colonne | Autant qu'on veut |
| Sécurité | `unsafe-eval` à isoler, extensions sans signature vérifiée (à faire au cœur) | Un serveur local, toute l'interface | Rien de nouveau |
| Effort (§7) | 6 à 7 semaines, +1,5 semaine pour Node | 1 à 2 semaines, intégration faible | 3 à 4 semaines |

## 6. Les limites : licences, marketplace, Node

**Les licences.**

- `@codingame/monaco-vscode-api` et ses paquets, `monaco-languageclient`, `vscode-languageclient` : MIT ; Code-OSS (le code de VS Code) : MIT ; VSCodium : MIT ; `typescript-language-server` : Apache-2.0 **[vérifié]** (leurs `package.json`). Tout est compatible avec la GPL-3.0-or-later d'Escouade, et se cite dans `THIRD-PARTY-NOTICES.md`.
- **Le serveur de VS Code de Microsoft n'est pas libre** : il ne s'utilise qu'avec une copie de Microsoft Visual Studio Code, et ses limites techniques (`vsda`) font partie de sa licence **[lu]** (S4). Il est donc exclu : seul un REH construit depuis Code-OSS (celui de VSCodium, ou le nôtre) convient.
- **La marque « Visual Studio Code »** n'est pas à nous : la vue se dirait « éditeur » ou « vue de code », pas « VS Code » dans le produit.

**Le marketplace.** Celui de Microsoft (Pylance, C# Dev Kit, Remote, Copilot…) est réservé à ses produits : ses conditions disent que ses offres sont faites pour les produits Visual Studio **[lu]** (S6). Escouade ne peut prendre ses extensions que sur **Open VSX** (Eclipse Foundation), où manquent les extensions fermées de Microsoft (Pylance, C# Dev Kit, la famille Remote, Live Share), et où d'autres sont en retard de version. Pour Python, il faudrait basedpyright ou Pyright à la place de Pylance ; pour C#, csharp-ls ou OmniSharp. Côté web, Open VSX limite aussi ce que la page peut charger (§3.1).

**Node.**

- **Sans Node** (la vue seule) : thèmes, grammaires, extensions web, serveurs de langage lancés par le cœur. C'est ce qu'Escouade garantit à tous.
- **Avec le Node du poste** : seulement à partir de Node 22.15 (`module.registerHooks`), pour le REH 1.140. Les modules natifs du serveur sont en N-API et ne cassent pas. Ceux d'une extension, s'ils ne sont pas en N-API, cassent avec une autre version majeure que celle du serveur.
- **En téléchargeant le REH à la demande** (avec son Node) : pas de dépendance au poste, 82 à 89 Mo la première fois. Recommandé (§3.2).
- Le REH doit avoir **le même commit** que la vue. VSCodium ne publie pas toutes les versions de VS Code (pas de 1.138), et monaco-vscode-api a la sienne. Un produit construirait donc le REH lui-même, depuis Code-OSS, au commit de la version de monaco-vscode-api qu'il embarque, dans sa CI, pour les trois systèmes. C'est le poste le plus lourd de l'hôte Node.

## 7. La recommandation et son effort

**La recommandation : la voie A, la vue VS Code par monaco-vscode-api, sans Node d'abord.** Elle donne ce que l'utilisateur demande (l'arborescence, les onglets et l'éditeur de VS Code, ses serveurs de langage, ses extensions web) en gardant ce qui fait Escouade : les fichiers par le cœur, les serveurs lancés par le cœur, le thème, une colonne de l'espace de travail. La maquette montre que rien de bloquant ne s'y oppose. Ses coûts sont mesurés et raisonnables : +3,7 Mo compressés, +130 à 210 Mo de webview, environ 1 s à la première ouverture (une fois par page), et la tenue sous WebKit (§4).

La voie C (CodeMirror + serveurs de langage) reste la solution de repli, si l'on juge trop cher le poids, la mémoire ou le look de VS Code. Elle partage avec A son premier ticket (le gestionnaire des serveurs de langage du cœur), qui n'est donc jamais perdu. La voie B (VS Code entier dans une vue) n'est pas recommandée : Node obligatoire, un serveur local qui lit le disque sans le cœur, et une autre app dans Escouade.

**Ce que A demande pour devenir un produit** (estimations pour un agent et une revue, sur le modèle des tickets de la 2.2) :

| # | Ticket | Effort |
| --- | --- | --- |
| A1 | Le gestionnaire des serveurs de langage du cœur : lesquels pour quels langages, les trouver, les lancer dans un Job, les relancer, leurs journaux et réglages (commun avec C) | 3 j |
| A2 | Le fournisseur de fichiers complet : fichiers binaires (une commande en octets), changements sur le disque poussés par le cœur, sources des worktrees des agents, conflit d'écriture montré | 3 j |
| A3 | La vue dans l'éditeur et dans les volets : la couche, le focus, l'arbitrage des raccourcis d'Escouade et de VS Code, Ctrl+P, les liens des conversations, la fermeture avec des modifications non enregistrées | 5 j |
| A4 | Le thème suivi en direct (clair, sombre, teinte du projet), les polices, la langue (pack de langue français de monaco-vscode-api) | 2 j |
| A5 | Les extensions par le cœur : le VSIX d'Open VSX téléchargé, sa somme et sa signature vérifiées, ses fichiers servis par un protocole du cœur ; la liste permise par les réglages et `.escouade.yml` | 4 j |
| A6 | L'hôte d'extensions web sur une origine à part, sans `unsafe-eval` dans la page d'Escouade | 3 j |
| A7 | Ce que l'éditeur fait aujourd'hui : marques des changements par rapport à la base, comparer avec le disque, choix de la source (projet ou worktree d'un agent) | 4 j |
| A8 | e2e et budget de mémoire et de temps sur les trois systèmes, dans la CI | 3 j |
| | **Total** | **27 j, soit 6 à 7 semaines** |
| N1 | (Hôte Node) Le REH construit depuis Code-OSS au commit de la vue, pour les six cibles, dans la CI, et publié | 4 j |
| N2 | (Hôte Node) Téléchargé à la demande et vérifié, lancé par le cœur (jeton en fichier, données à part, Job), ses ressources servies par le cœur (CORS) | 3 j |
| | **Avec l'hôte Node** | **+7 j, soit 1,5 semaine** |

Pour comparer : la voie C demande A1 (3 j), le client de `@codemirror/lsp-client` branché (4 j), les actions de code et corrections rapides (3 j), le plan du document et les symboles (3 j), et ses e2e (2 j), soit **15 j, 3 à 4 semaines**, pour un éditeur sans extensions.

**Deux points à régler dès A1.**

- **Se passer de `monaco-languageclient`** : ce n'est qu'une enveloppe d'une trentaine de lignes autour de `vscode-languageclient` (`MonacoLanguageClient`, S2). C'est elle qui retient monaco-vscode-api deux versions majeures en arrière. Le client de langage peut être écrit comme une extension de la page, sur `vscode-languageclient/browser` et le transport de la maquette, ce qui laisse suivre monaco-vscode-api à son rythme (VS Code 1.145 aujourd'hui).
- **Une version de VS Code à la fois** : la vue, ses extensions par défaut et le REH ont le même commit. Chaque montée de monaco-vscode-api (une majeure toutes les trois à cinq semaines en 2026 : de la 30 le 1er avril à la 39 le 6 novembre) est un ticket : la vue, le REH reconstruit, les e2e sur les trois systèmes.

## 8. Décisions à prendre

1. **La voie** : A, la vue VS Code (recommandée) ; C, CodeMirror et les serveurs de langage ; ou B, VS Code entier dans une vue.
2. **Avec A, l'éditeur de CodeMirror** : remplacé par la vue VS Code pour l'éditeur d'un projet (CodeMirror restant pour les petits éditeurs : docs, comparaisons, réglages), ou les deux au choix dans les réglages. Garder les deux coûte de les maintenir tous les deux.
3. **L'hôte Node** : pas d'hôte Node d'abord (recommandé), puis le REH téléchargé à la demande avec son Node (recommandé) ; ou le Node du poste, à partir de 22.15 ; ou jamais d'extension Node.
4. **Les extensions** : Open VSX seulement (le marketplace de Microsoft est exclu), la question de confiance de VS Code par éditeur, et une liste permise qu'une équipe peut imposer (`extensions.allowed`, par `.escouade.yml`).
5. **Plusieurs colonnes** : une colonne VS Code par projet, avec ses groupes d'onglets ; et, si on le veut, des éditeurs autonomes dans d'autres volets (§2).

## 9. Sources

- S1. `@codingame/monaco-vscode-api`, dépôt et démo (`demo/`, `docs/vscode_server.md`), étiquette v37.6.1 : <https://github.com/CodinGame/monaco-vscode-api> (MIT).
- S2. `monaco-languageclient` 11.0.3 : <https://github.com/TypeFox/monaco-languageclient> (MIT) ; ses dépendances (`@codingame/monaco-vscode-api` ^37.6.0) par `npm view`.
- S3. VSCodium 1.140.06880, archives `vscodium-reh-*` et leurs sommes : <https://github.com/VSCodium/vscodium/releases/tag/1.140.06880> (MIT).
- S4. Le serveur de VS Code de Microsoft, `https://update.code.visualstudio.com/commit:7debcd0e2acdea1c52de81bf9ee1620444407dda/server-win32-x64/stable`, et son fichier `LICENSE` (« Microsoft pre-release software license terms »).
- S5. Open VSX, API publique (`/api/<espace>/<extension>`, champs `files.signature`, `files.sha256`, `files.publicKey`, `verified`) : <https://open-vsx.org>.
- S6. Conditions du marketplace de Visual Studio : <https://aka.ms/vsmarketplace-ToU>.
- S7. `@codemirror/lsp-client` 6.3.0 (MIT) : <https://github.com/codemirror/lsp-client>.
- S8. Node.js, `module.registerHooks` (ajouté en v23.5.0 et v22.15.0) : <https://nodejs.org/api/module.html#moduleregisterhooksoptions>.
- S9. OpenVSCode Server (Gitpod, MIT), dernières versions publiées : <https://github.com/gitpod-io/openvscode-server/releases> ; code-server (Coder, MIT) : <https://github.com/coder/code-server>.
