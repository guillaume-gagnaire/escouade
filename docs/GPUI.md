# Passer de Tauri à GPUI, sans webview : l'étude (ESC-365)

Étude du 2026-11-07, pour décider. **Aucun code de l'app ne change** : le livrable est ce document, le prototype jetable de `spikes/gpui/` (jamais livré, hors de l'espace de travail de l'app) et ses mesures. Les décisions à prendre par l'utilisateur sont à la fin.

**Le contexte.** « Phoenix » est le nom interne de la réécriture du client de bureau de Discord : Electron remplacé par un client Rust sur **GPUI**, le framework d'interface de Zed, dessiné par le GPU, sans webview pour l'interface ; la webview du système reste pour les contenus embarqués. La question : que coûterait et que rapporterait le même chemin pour Escouade, aujourd'hui en Tauri 2 avec un front Svelte ?

**Comment lire les sources.** Chaque affirmation dit d'où elle vient :

- **[vérifié]** : lu dans le code (sources des crates téléchargées par cargo, dépôt de Zed, dépôt d'Escouade), obtenu d'une API (crates.io, GitHub) ou exécuté ici, le 2026-11-07, sauf autre date dite ;
- **[lu]** : vient d'un article, d'une annonce ou d'un résumé de recherche, non vérifié dans le code ni par une mesure.

Les sources numérotées sont au §9.

## En bref

**La recommandation, en trois phrases.** Rester sur Tauri et viser ses deux points faibles mesurés, la mémoire (celle de la webview au repos, et surtout celle du cœur en usage réel, 1 085 Mo dans le process principal de l'instance de l'utilisateur) et le débit des terminaux, pour 6 tickets et une à deux semaines. GPUI tient ses promesses sur ce qu'il dessine (interface au repos 16 Mo contre 147 à 187 Mo, un gros flux de terminal environ deux fois plus vite pour dix fois moins de CPU, une interface jamais bloquée plus de 135 ms, quand celle d'aujourd'hui se fige jusqu'à 1,5 s sous dix flux), mais le chemin coûte une réécriture de 97 000 lignes de front et des 331 tests e2e, sur un framework pré-1.0 que Zed ne publie plus officiellement depuis un an, et le gain réel en usage serait d'environ un tiers de la mémoire de l'interface, pas un ordre de grandeur, le cœur restant ce qu'il est. Rouvrir la question si GPUI se stabilise (publication officielle régulière ou 1.0), si les cibles de Tauri ne sont pas tenues, si le téléphone se fait en app native, ou si WebKitGTK reste un frein sous Linux.

**Les mesures** (§5, Windows 11, poste chargé par d'autres agents ; app 2.1.2 installée, release ; prototype GPUI release) :

| | App d'aujourd'hui (Tauri, WebView2) | Prototype GPUI |
| --- | --- | --- |
| Fenêtre peinte après le lancement, médiane de 10 | 1 438 ms (935 à 7 790) | 861 ms (353 à 1 733) |
| Interface au repos, un terminal ouvert (ensemble de travail privé) | 187 Mo (cœur 31 Mo, webview 156 Mo) | 16 Mo |
| Interface au repos, dix terminaux | 195 Mo | 18,5 Mo |
| 100 000 lignes (11,2 Mo) dans un terminal : jusqu'à la fin reçue | 1 154 à 2 688 ms | 575 à 956 ms |
| … plus long blocage de l'interface pendant ce flux | 191 à 299 ms | 10 à 12 ms |
| … CPU de l'interface | 5,3 à 6,5 s | 0,8 à 1,8 s |
| … mémoire de l'interface après le flux | 342 à 471 Mo | 48 Mo |
| Dix terminaux à 20 000 lignes chacun, à la fois : fin reçue | 2 186 à 3 559 ms | 965 à 2 055 ms |
| … plus long blocage | 279 à 1 517 ms | 30 à 134 ms |
| … CPU de l'interface | 11,2 à 11,7 s | 0,75 à 1,0 s |
| … mémoire de l'interface après | 835 à 1 080 Mo | 318 Mo |

Le prototype n'a ni cœur, ni agents, ni conversation : il mesure ce que coûte **dessiner** (la fenêtre, une colonne de Kanban, des terminaux), pas l'app entière. En usage réel, l'instance de l'utilisateur (52 process) pèse 1 822 Mo pour l'interface, dont **1 085 Mo pour le seul process principal** (le cœur, qui resterait en GPUI) et 737 Mo pour la webview (§5.5).

## 0. Ce qu'il y a à porter

Recompté le 2026-11-07 sur `main` (aea195e1) **[vérifié]** ; les chiffres du ticket sont justes.

| Quoi | Combien | Comment compté |
| --- | --- | --- |
| Composants Svelte | 183 fichiers, 57 406 lignes (balisage, script et style) | `find src -name '*.svelte'` |
| TypeScript hors tests | 228 fichiers, 39 730 lignes | `src/**/*.ts` sans `*.test.ts` |
| Tests unitaires du front | 230 fichiers `*.test.ts` | Vitest, jsdom |
| Commandes Tauri | 331 `#[tauri::command]` | `src-tauri/src` ; `src/lib/ipc.ts` en appelle 304 par `invoke` |
| Événements vers l'interface | 41 sortes d'`UiEvent` (`src-tauri/src/model.rs`), par un seul `Channel` | |
| e2e Playwright | 106 fichiers, 331 tests | `tests/e2e/*.spec.ts`, par le protocole DevTools de WebView2 |
| Fichiers du cœur qui citent `tauri::` | 77 sur 424 (hors tests), dont 68 `tauri::Runtime`, 20 `AppHandle`, 91 `tauri::async_runtime`, 51 `tauri::test` | `src-tauri/src`, 335 282 lignes de Rust |
| Bibliothèques web | xterm.js 5.5 (+ webgl, fit, search, unicode11, web-links), CodeMirror 6 (17 paquets, dont `@codemirror/merge`), marked 16, shiki 3, DOMPurify | `package.json` |
| Plugins Tauri | `deep-link`, `dialog`, `opener`, `process`, `single-instance`, `updater`, `window-state` ; `notification` sous macOS seulement (Windows : `tauri-winrt-notification`) ; le tray par la feature `tray-icon` de `tauri` | `src-tauri/Cargo.toml` |

Les plus gros morceaux : `settings/` (31 composants, 8 227 lignes), `modals/` (5 138), `docs/` (4 224), `board/` (le Kanban, 3 758), `plugins/` (3 314), `editor/` (3 137), `conv/` (3 069), et `Sidebar.svelte` seul (2 425).

## 1. GPUI aujourd'hui

### 1.1 La licence

- **GPUI est sous Apache-2.0** : `gpui` sur crates.io (S1), `crates/gpui` et `crates/gpui_windows` sur `main` de Zed (S6), et les instantanés `gpui-pre-*` (S2) **[vérifié]**.
- **Le reste de Zed est sous GPL-3.0-or-later** : `editor`, `multi_buffer`, `language`, `terminal`, `terminal_view`, `markdown`, `ui` (S6) **[vérifié]**. Escouade étant elle-même sous GPL-3.0-or-later (`package.json`), **les reprendre serait permis**. Ce n'est pas la licence qui l'empêche, c'est leur forme (§2, l'éditeur).
- `gpui-component` (Longbridge) est sous Apache-2.0 (S7) **[vérifié]**.

### 1.2 La maturité

- **Pré-1.0, et ruptures annoncées.** Le README de `crates/gpui` sur `main` : « GPUI is still in active development as we work on the Zed code editor, and is still pre-1.0. There will often be breaking changes between versions. You'll also need to use the latest version of stable Rust. » (S5) **[vérifié]**.
- **Le Rust récent est une vraie exigence** : `gpui-pre` 0.3.11 ne compile pas avec Rust 1.94 (`std::hint::cold_path` instable), il compile avec 1.98.1 (S4, `spikes/gpui/build.log`) **[vérifié]**. L'app déclare `rust-version = "1.88"` : passer à GPUI, c'est suivre le stable de près.
- **Les ruptures se voient** : entre `gpui` 0.2.2 et `gpui-pre` 0.3.11, la plateforme est sortie du crate (`gpui_platform`, `gpui_windows`, `gpui_macos`, `gpui_linux`, `gpui_wgpu`, `gpui_web`) ; `Application::new()` devient `gpui_platform::application()` (S2, S4) **[vérifié]**. `gpui-component` a publié 0.6.6, 0.7.0 à 0.7.3, 0.8.0, 0.8.1 et 0.9.0 entre le 2026-09-21 et le 2026-11-02, chacune épinglée à un instantané exact de GPUI (`gpui-pre = "=0.3.11"`) (S7) **[vérifié]**.

### 1.3 La distribution

Trois façons d'avoir GPUI, aucune confortable **[vérifié]** (S1, S2, S3) :

| | Ce que c'est | Date | Pour | Contre |
| --- | --- | --- | --- | --- |
| `gpui` officiel | publié par Zed (mikayla-maki) | **0.2.2 du 2025-10-22**, rien depuis | stable, connu | un an de retard sur Zed, sans `gpui_platform` séparé |
| dépendance git de Zed | `gpui = { git = "https://github.com/zed-industries/zed", rev = "…" }` | ce qu'on veut | à jour, la source | tout le dépôt de Zed à cloner (93 941 étoiles, poussé chaque jour) ; aucune version, aucun changelog de GPUI |
| `gpui-pre` | instantanés **hebdomadaires** de Zed republiés par huacnlee (Longbridge) : 0.3.11 = « snapshot of zed@f816e1f » | 0.3.7 du 2026-09-28 à 0.3.11 du 2026-11-02 ; f816e1f est du 2026-11-01 | sur crates.io, ce que suit `gpui-component` | un tiers republie le code de Zed : s'il s'arrête, on revient au git |

Le prototype prend `gpui-pre` 0.3.11 et `gpui-pre-platform` 0.3.11.

### 1.4 Les systèmes

Lu dans les sources de `gpui-pre-*` 0.3.11 (S4) **[vérifié]** :

- **Windows** : Win32, rendu **DirectX 11** (`directx_renderer.rs`, `directx_devices.rs`, aucun `D3D12`), texte par DirectWrite (`direct_write.rs`), notifications toast (`system_notifications.rs`), presse-papiers et fichiers déposés (`clipboard.rs`, `DragQueryFileW`), boîtes de dialogue, jump list, IME (`events.rs`, 12 appels `Imm*`), accessibilité par `accesskit_windows`.
- **macOS** : Metal (`gpui-pre-macos`), `font-kit` pour le texte.
- **Linux et FreeBSD** : rendu par `gpui-pre-wgpu` (wgpu 30, Vulkan ou OpenGL ES : `Backends::VULKAN | Backends::GL` dans `wgpu_context.rs`), fenêtres **Wayland** et **X11** (`gpui-pre-linux/src/linux/wayland`, `x11`), portails XDG, `accesskit_unix`.
- **Le web** : `gpui-pre-web` (wasm32, WebGPU ou WebGL) existe ; l'accessibilité y passe par le DOM (S4, `_accessibility.rs`). Nouveau, et non essayé ici ; `gpui-component` dit y faire tourner sa galerie **[lu]**.

Zed est sorti en stable sous Windows le 2025-10-15, avec DirectX 11 après un essai de Vulkan abandonné pour des soucis de compatibilité (S9) **[lu]**.

**Sous Windows, le prototype compile et tourne** (§5) **[vérifié]**. macOS et Linux n'ont pas été essayés.

### 1.5 Autour de GPUI

- **`gpui-component` 0.9.0** (Longbridge, dépôt `longbridge/gpui-kit`, 19 749 étoiles, poussé le 2026-11-07) (S7) **[vérifié]** pour la présence des modules dans les sources : éditeur de code (`input/editor.rs`, tree-sitter, LSP), vue de diff (`diff/`, conflits compris), texte riche et markdown sélectionnables (`text/`, `selectable`, `selection_format`), coloration (`highlighter/`, tree-sitter ou syntect), dock, onglets, tableaux, listes virtuelles, menus, notifications, barre de titre, réglages, graphiques (`chart/`, `plot/`). **[lu]** dans son README : « Stable performance at 200K lines », « 120 FPS », « UI Integration Testing: render real components in headless windows », « Production Ready: used to build Longbridge Pro ». Non mesuré ici.
- **`gpui-webview` 0.9.0** (même dépôt, sur Wry) : une webview dans une fenêtre GPUI, ce que fait Discord pour ses contenus embarqués (S8) **[vérifié]** (existence et description).
- **Plus petits** : `gpui-router`, `gpui-updater` (tiers), `gpui-testing-library`, `gpui-devtools` ; **`tauri-plugin-gpui`** 0.1.0 (« Render GPUI views inside ordinary Tauri windows », 24 téléchargements) : l'inverse d'un hybride, trop jeune pour s'y appuyer (S8) **[vérifié]**.

### 1.6 Discord, ce qu'on en sait

**[lu]** seulement (S10, articles du 2026-11-05 environ) : le CTO, Stanislav Vishnevskiy, a confirmé la réécriture en Rust et le choix de GPUI plutôt que Tauri ; une capture d'une version de travail sous Windows 11 montre environ 290 Mo, connectée à deux comptes ; aucune date de sortie, aucune mesure indépendante. La source primaire (son message sur X) n'a pas été lue directement.

## 2. Pièce par pièce

« Équivalent » veut dire : il existe dans GPUI ou dans une crate autour, vu dans les sources **[vérifié]**, sauf mention. « À faire » : rien d'existant, à écrire nous-mêmes.

| Pièce | Aujourd'hui | En GPUI | Verdict |
| --- | --- | --- | --- |
| Coque, mise en page, thème | Svelte, CSS (`src/app.css`, jetons), Inter et JetBrains Mono | `div()` en flexbox (taffy), jetons à reprendre en Rust, polices chargées par `text_system().add_fonts` | à refaire, sans difficulté technique ; tout le CSS est à traduire |
| Kanban, glisser-déposer | `board/` (6 composants, 3 758 lignes), glisser HTML5 | `on_drag`, `on_drop`, `drag_over` (exemple `drag_drop.rs`) ; **fait dans le prototype**, une colonne qu'on réordonne | équivalent |
| Terminal | xterm.js 5.5 (WebGL), PTY par `portable-pty` côté Rust | `alacritty_terminal` 0.26.0 (Apache-2.0, 2026-04-06) pour l'émulation ; le rendu est à écrire (Zed le fait dans `terminal_view`, GPL, lié à son workspace). **Fait dans le prototype** : PTY réel (`portable-pty`, comme l'app), couleurs, 10 000 lignes d'historique | équivalent pour l'émulation ; rendu, sélection, recherche, liens, défilement à écrire (~1 500 à 3 000 lignes, d'après le prototype, 608 lignes avec le Kanban et les mesures) |
| Vue des harness (« Vue Claude Code »…) | un terminal xterm.js + les éléments de `view/translate` | même terminal ; les éléments de vue sont des données, rendues par nos composants | équivalent |
| Éditeur | CodeMirror 6, 17 paquets | l'éditeur de Zed **n'est pas une bibliothèque** : `crates/editor` tire 107 dépendances du workspace de Zed (`multi_buffer`, `language`, `project`…), GPL. Reste l'éditeur de `gpui-component` (tree-sitter, LSP) | à reprendre de `gpui-component` ; moins riche que CodeMirror (extensions, langages par paquet) |
| Diffs | `@codemirror/merge`, `DiffView`, `FileDiff` | `gpui-component::diff` (côte à côte, conflits) | équivalent probable, non essayé |
| Markdown | marked + DOMPurify, `{@html}` (`conv/Markdown.svelte`) | `gpui-component::text` (Markdown et HTML), ou `pulldown-cmark` et nos éléments | équivalent ; plus de HTML à assainir |
| Coloration | shiki 3 (grammaires TextMate) | tree-sitter ou syntect (`gpui-component::highlighter`) | équivalent, rendu un peu différent (thèmes à refaire) |
| IME (japonais, chinois…) | celui du navigateur | `InputHandler` de GPUI, `Imm*` sous Windows ; Zed s'en sert | équivalent, pour les champs qui implémentent `InputHandler` (l'éditeur de `gpui-component` le fait) ; non essayé ici |
| Accessibilité | le DOM, rôles ARIA | AccessKit intégré à GPUI (`_accessibility.rs`, `window/a11y.rs`, `accesskit_windows`), **mais chaque élément doit recevoir son rôle et son nom** (`.role()`, `a11y_*`) ; `gpui-component` dit le faire pour ses composants **[lu]**. Un fork note que Zed lui-même n'annote presque rien (S12) **[lu]** | possible, à faire partout nous-mêmes ; c'est aussi ce qui permettrait des tests pilotés par UI Automation (§4.3) |
| Sélection et copier-coller | natifs du navigateur, partout | pas de sélection de texte libre dans GPUI : chaque vue la gère (`gpui-component::text` le fait pour le markdown, l'éditeur pour le code) ; presse-papiers `read_from_clipboard`, `write_to_clipboard` | à faire vue par vue ; risque de régression sur la conversation |
| Fenêtres multiples | Tauri (fenêtre « Pages », inspecteur) | `cx.open_window`, plusieurs fenêtres (exemple `move_entity_between_windows.rs`) | équivalent |
| Page d'un lancement (ESC-311) | une webview enfant par agent (`devpage.rs`), isolée | **une webview reste nécessaire** : `gpui-webview` (Wry) | webview gardée, comme Discord (§4.2) |
| Tray | `tauri` feature `tray-icon` | rien dans GPUI ; `tray-icon` 0.26.1 (tauri-apps, 2026-10-07) s'utilise sans Tauri | à brancher |
| Notifications | `tauri-winrt-notification` (Windows), `tauri-plugin-notification` (macOS) | `show_system_notification` de GPUI (toasts sous Windows ; macOS ; sous Linux par l'interface D-Bus des notifications XDG, via `notify-rust`) : mieux qu'aujourd'hui, où Linux n'a pas de notification (`docs/REMOTE.md`) | équivalent |
| Deep link `escouade://` | `tauri-plugin-deep-link` + installeurs | `register_url_scheme`, `on_open_urls` ; l'enregistrement par l'installeur reste à faire | équivalent partiel |
| Instance unique | `tauri-plugin-single-instance` | rien ; un verrou et un tube nommé à écrire (le paquet `single-instance` date de 2021) | à faire (~1 ticket) |
| État des fenêtres | `tauri-plugin-window-state` | `WindowBounds` à sauver et rendre nous-mêmes | à faire (petit) |
| Sélecteurs de fichiers | `tauri-plugin-dialog` | `prompt_for_paths`, `prompt_for_new_path` | équivalent |
| Ouvrir un lien, un fichier | `tauri-plugin-opener` | `open_url` ; « afficher dans l'Explorateur » à écrire | équivalent partiel |
| Redémarrer | `tauri-plugin-process` | `restart` de GPUI | équivalent |
| Graphe Git, graphiques, diagramme de workflow | SVG dans Svelte | `canvas`, chemins (`path_builder.rs`), SVG par `resvg` ; graphiques de `gpui-component` | à refaire |
| Thèmes des plugins | jetons CSS filtrés (`plugin-themes.ts`) | les mêmes jetons, lus en Rust | équivalent |
| Langues | `src/lib/i18n/` (TS) et `i18n.rs` | tout en Rust | à porter |

## 3. Ce que Tauri fait pour nous et qu'il faudrait refaire

**[vérifié]** dans `src-tauri/tauri.conf.json`, `tauri.linux.conf.json`, `tauri.macos.conf.json` et `.github/workflows/release.yml` :

- **Le bundler.** Windows : NSIS par utilisateur, deux langues, crochets d'installation (`windows/installer-hooks.nsh`). macOS : app universelle et dmg. Linux : AppImage et deb (modèle `.desktop`). Les fichiers d'onnxruntime par système (`tauri.ort-<os>.conf.json`). À la place : `cargo-packager` (CrabNebula : NSIS, WiX, dmg, AppImage, deb ; **0.11.8, dernière version le 2025-11-27**) (S8), ou nos propres scripts NSIS, `hdiutil` et `appimagetool`. GPUI n'apporte rien ici.
- **La mise à jour intégrée.** `tauri-plugin-updater`, `latest.json` sur les releases GitHub, artefacts signés par sa clé, installation silencieuse sous Windows. À la place : `cargo-packager-updater` (0.2.3 du 2025-07-21) ou un updater à nous, qui garde le format de `latest.json` pour que les versions Tauri déjà installées passent à la première version GPUI (le dernier saut se ferait encore par l'updater de Tauri).
- **La signature.** Azure Artifact Signing branché par `tauri-action` (`AZURE_*`) et `scripts/windows-signing.mjs` ; signature et notarisation Apple par les secrets `APPLE_*`. À la place : `signtool` avec la dlib d'Azure, `codesign` et `notarytool`, dans le workflow. Faisable, à refaire et à essayer sur une vraie release.
- **Les 331 commandes et les 41 événements.** Ils deviennent des appels directs (le front et le cœur dans le même process Rust, sans JSON) et des abonnements GPUI (`cx.observe`, `cx.subscribe`, entités). C'est un gain de simplicité, mais tout `ipc.ts` et les `Channel` des terminaux sont à reprendre.
- **Le cœur dépend de Tauri.** 77 fichiers, `Core<R: Runtime>`, `AppHandle` pour la version, les notifications, le clignotement, le tray, la mise à jour et le redémarrage ; les tests du cœur tournent sur `tauri::test::mock_app` (51 usages). C'est exactement la fondation 1 de `docs/REMOTE.md` (« un trait `Host` remplace l'`AppHandle` ») : à faire de toute façon pour le mode serveur, utile aux trois chemins.
- **Les devtools** de la page (inspecteur) : GPUI a un inspecteur à lui (`inspector.rs`, feature `inspector`), moins riche.

## 4. Les dépendances cachées sur le web

### 4.1 Le mode serveur et le téléphone

`docs/REMOTE.md` (2026-10-25) est une étude ; **rien n'est construit** : pas de module `remote` ni d'API WebSocket dans `src-tauri/src`, les commits « remote » parlent du remote control de claude.ai **[vérifié]**.

- **Le mode serveur** ne dépend pas du web : le client de bureau parle au cœur distant par une API (requêtes JSON, événements, flux des terminaux). Un client GPUI s'y branche aussi bien qu'un client Svelte ; mieux même, puisqu'il partage les types Rust du cœur.
- **Le téléphone** en dépend : l'option A de REMOTE.md, celle proposée, est « l'interface web d'Escouade servie dans le tunnel », la même interface Svelte. **Tout GPUI supprime cette interface web.** Il resterait : garder un front web réduit pour le téléphone (deux interfaces à tenir), essayer `gpui-web` (wasm, WebGPU, expérimental), ou faire l'option B (app mobile native). Le choix du téléphone pèse donc sur celui de GPUI (décision à la fin).

### 4.2 Ce que les plugins affichent

**[vérifié]** dans `docs/PLUGINS.md` §1.9, §5.1 et §8.30 :

- **Aucun plugin n'apporte de HTML.** Pas de panneau, de page ni de fenêtre de plugin ; « pas de vue libre : l'iframe isolée reste à essayer ». Les réglages et secrets déclarés sont rendus par nos composants, les thèmes sont des jetons de couleur, les « Vue X » passent par `view/translate`, qui rend des **données** (`typed`, `spoke`, `asks`, `runs`…) que nos composants dessinent.
- La logique des plugins tourne en WASM dans le cœur, hors de toute webview (§5.1 : « elle n'existe ni dans le mode serveur ni dans le banc en ligne de commande »).
- **Donc rien à casser côté plugins** : la porte se ferme seulement sur une future « vue libre » en iframe. En GPUI, une vue de plugin serait une interface décrite en données (un schéma de composants) ou un panneau `gpui-webview` isolé.
- **La Page d'un lancement** (ESC-311, `devpage.rs`) affiche l'app que l'agent développe : c'est du web par nature. Elle garderait une webview (`gpui-webview`), comme Discord garde la sienne pour les vidéos et les Activities. Le code d'aujourd'hui montre déjà la difficulté : une webview native se dessine **par-dessus** la fenêtre, si bien que la page se cache chaque fois qu'un menu, une liste ou la palette passe dessus (`DevPage.svelte`, ESC-311). Ce serait pareil sous GPUI, et pour chaque panneau web d'un hybride (§6.2).

### 4.3 Les e2e

Les 106 fichiers Playwright pilotent la page de WebView2 par le protocole DevTools (`tests/e2e/fixture.ts`) : rôles ARIA, textes, `page.evaluate`. **Sans DOM, aucun ne survit.** Ce qui les remplacerait :

1. **Les tests d'intégration de GPUI**, sans fenêtre : `TestAppContext`, `VisualTestContext`, plateforme de test (`platform/test`), touches et clics simulés, assertions sur les entités ; `visual_test.rs` pour des captures **[vérifié]** (présents dans `gpui-pre`). Rapides (pas de process, pas de WebView2), c'est là qu'iraient la plupart des 331 tests. `gpui-component` dit y tester ses composants, accessibilité comprise **[lu]**.
2. **Une poignée de tests de fumée sur l'exe construit**, pilotés par l'arbre d'accessibilité (UI Automation sous Windows, via AccessKit) : démarrer, ajouter un projet, ouvrir un terminal, envoyer un message. Ils n'existent que si les éléments ont leurs rôles et leurs noms (§2, accessibilité) : un argument de plus pour annoter tout.
3. Les faux CLI (`tests/fixtures/fake-*.mjs`) et les dossiers de données jetables se gardent tels quels : ils parlent au cœur, pas au front.

Coût : réécrire les 331 tests, et les 230 fichiers de tests unitaires du front, en Rust (§6).

## 5. Le gain mesuré

### 5.1 Comment

- **Machine** : Windows 11 Pro 10.0.26200, le poste de développement, **chargé** pendant toutes les mesures par d'autres agents d'Escouade (une cinquantaine de process de l'instance de l'utilisateur) : les écarts entre essais sont larges, les médianes et les ordres de grandeur comptent plus que les chiffres exacts.
- **App d'aujourd'hui** : Escouade **2.1.2 installée** (`%LOCALAPPDATA%\Escouade\escouade.exe`, release, 94 Mo), lancée par la fixture des e2e sur un dossier de données jetable (`ESCOUADE_DATA_DIR` : ni instance unique ni état de fenêtre de l'utilisateur), fenêtre visible. Banc : `spikes/gpui/bench/tauri.spec.ts`, hors des e2e de l'app.
- **Prototype** : `spikes/gpui/`, release, Rust 1.98.1, `gpui-pre` 0.3.11, `alacritty_terminal` 0.26, `portable-pty` 0.9 ; exe de 11 Mo. Bancs : `bench/startup-spike.mjs`, `bench/spike.ps1`, `bench/spike-ten.ps1`.
- **Les mêmes mesures des deux côtés** :
  - *fenêtre* : de `Date.now()` juste avant le lancement du process (Node `spawn`) à la première peinture (le `first-contentful-paint` de la page ; la frame qui suit le premier rendu pour GPUI) ;
  - *mémoire* : `bench/mem.ps1`, ensemble de travail privé (ce que montre le Gestionnaire des tâches) et octets privés, du process principal et de ses `msedgewebview2` (« interface »), puis de tout l'arbre (shells compris) ;
  - *gros flux* : la même commande écrite au PowerShell du terminal, `& 'gpui-spike.exe' --flood 100000; Write-Output ('FLOOD'+'-DONE')` (100 000 lignes de ~110 octets avec des couleurs SGR, 11,2 Mo), par `term_write` pour l'app, à son PTY pour le prototype ; temps de l'écriture à la sentinelle parsée (la page l'a reçue / l'émulateur l'a lue), puis à la frame suivante ; le plus long retard d'une minuterie de 20 ms (page) ou de 8 ms (boucle de GPUI) pendant le flux ; le CPU des process de l'interface ;
  - 10 000 lignes d'historique des deux côtés (`src/lib/terminals.ts`, `scrolling_history` du prototype).
- **Ce que le prototype n'a pas** : le cœur, les agents, les conversations, le serveur MCP, les plugins. Il compare **le coût de dessiner**. Le process principal de l'app (30 Mo au repos) reste dans tous les chemins.

### 5.2 Le démarrage

Médiane basse (la 5e valeur sur 10, la même règle des deux côtés) ; le premier lancement de l'app est sur un dossier de données neuf.

| | Médiane | Min | Max | Lancements |
| --- | --- | --- | --- | --- |
| App 2.1.2 (fenêtre → first-contentful-paint) | 1 438 ms | 935 ms | 7 790 ms | 10 |
| Prototype (fenêtre → première frame) | 861 ms | 353 ms | 1 733 ms | 10 |

Le prototype passe 220 à 690 ms dans `main` avant sa première frame (DirectX, polices, ouverture du PTY), le reste est le chargement du process. L'app montre ensuite son contenu réel (projets, agents) ; le prototype n'a rien à charger. Gain : environ 0,5 s sur un démarrage, pas décisif.

### 5.3 La mémoire au repos

Ensemble de travail privé, en Mo :

| | App : principal | App : webview | **App : interface** | **Prototype** |
| --- | --- | --- | --- | --- |
| Juste lancée | 30,1 | 117,2 | **147,3** | — |
| Un projet, un terminal | 30,8 | 155,9 | **186,7** | **16,0** (octets privés 89) |
| Dix terminaux | 31,8 | 163,6 | **195,4** | **18,5** (octets privés 110) |

Les shells eux-mêmes ne comptent pas ici : dix PowerShell pèsent ~315 Mo (prototype, Windows PowerShell 5.1) à ~495 Mo (app, PowerShell 7) dans l'arbre.

### 5.4 Le gros flux

Un terminal, 100 000 lignes, trois essais par passage. L'app a fait deux passages : le premier avec ce seul terminal ouvert (`tauri.jsonl`), le second dans le dernier de dix terminaux ouverts, celui qu'on voit (`tauri-dix-ouverts.jsonl`) ; le prototype, un seul terminal (`spike.jsonl`).

| | Fin reçue | Frame suivante | Plus long blocage | CPU interface | Interface après |
| --- | --- | --- | --- | --- | --- |
| App 2.1.2, un terminal | 2 688 / 2 434 / 1 888 ms | 2 806 / 2 548 / 2 007 ms | 257 / 297 / 299 ms | 6,4 / 6,5 / 6,1 s | 383 / 396 / 458 Mo |
| App 2.1.2, dix ouverts | 1 327 / 1 185 / 1 154 ms | 1 399 / 1 304 / 1 264 ms | 191 / 199 / 193 ms | 6,1 / 5,6 / 5,3 s | 343 / 392 / 471 Mo (principal 33 Mo) |
| Prototype | 719 / 575 / 956 ms | 778 / 625 / 1 001 ms | 12 / 10 / 10 ms | 1,8 / 0,8 / 1,4 s | 48 Mo |

La mémoire de l'app monte à chaque flux (l'historique d'xterm.js se remplit, la webview grossit) et ne redescend pas pendant la mesure.

Dix terminaux à la fois, 20 000 lignes chacun (22,4 Mo en tout), trois essais :

| | Fin reçue (le dernier) | Plus long blocage | CPU interface | Interface après |
| --- | --- | --- | --- | --- |
| App 2.1.2 | 2 395 / 3 559 / 2 186 ms | 749 / 279 / 1 517 ms | 11,2 / 11,7 / 11,4 s | 835 / 884 / 1 080 Mo (principal 40 à 44 Mo) |
| Prototype | 965 / 1 798 / 2 055 ms | 134 / 30 / 30 ms | 0,75 / 1,0 / 1,0 s | 318 Mo |

Avec dix terminaux en flux, la page de l'app se fige jusqu'à 1,5 s et sa mémoire atteint 1 Go ; le prototype reste sous 135 ms et 318 Mo. Le shell diffère un peu (l'app ouvre PowerShell 7, `pwsh` ; le prototype Windows PowerShell 5.1) : le flux est écrit par un exe natif sur la console, sans pipeline, ce qui le rend peu sensible au shell.

Le prototype rend 17 à 40 frames pendant un flux (un rendu quand un terminal a changé, sondé toutes les 8 ms, que le minuteur de Windows étire) ; ce n'est pas optimisé, l'émulateur étant verrouillé pendant le parse de chaque lecture de 64 Ko. Les 318 Mo après dix flux sont l'historique d'`alacritty_terminal` : 10 000 lignes × 120 colonnes × 24 octets par cellule, par terminal, soit ~29 Mo chacun ; xterm.js garde moins par cellule. À régler par un historique plus court ou compressé, dans les deux chemins.

### 5.5 En usage réel

L'instance de l'utilisateur, au moment de l'étude (52 process, des agents au travail, `bench/mem.ps1 -Root 20472`) **[vérifié]** : interface 1 822 Mo, dont **1 085 Mo pour `escouade.exe` seul** et 737 Mo pour ses six `msedgewebview2` (dont un de 637 Mo, le renderer). Tout l'arbre : 3,5 Go (agents, shells, serveurs compris).

Ce qu'en dit l'étude : en usage, **la plus grosse part est le cœur**, que GPUI ne touche pas. Un client GPUI garderait le cœur et remplacerait les 737 Mo de webview par quelques dizaines de Mo, plus l'historique des terminaux (~30 Mo par terminal plein) : 1 822 Mo deviendraient de l'ordre de 1 150 à 1 400 Mo, un tiers de moins environ, pas un ordre de grandeur. Comprendre ce que tient le cœur (conversations, tampons des vues TUI de 4 Mo chacun, instances WASM, modèles de voix…) rapporte autant, quel que soit le chemin.

### 5.6 Le prototype, en images

Captures dans `spikes/gpui/results/` :

- `spike-1-terminal.png` : la colonne « En cours » (12 cartes, glisser-déposer) et un PowerShell sur un vrai PTY ;
- `spike-apres-flux-2.png` : la fin des 100 000 lignes colorées, puis la sentinelle et l'invite ;
- `spike-flux-1.png` : pendant le flux ;
- `spike-10-terminaux.png`, `spike-10-terminaux-apres-flux.png` : dix PowerShell, au repos et après dix flux à la fois ;
- `tauri-1-terminal.png`, `tauri-apres-flux.png`, `tauri-10-terminaux.png` : l'app d'aujourd'hui dans les mêmes états (le dixième terminal affiché, les autres dans la liste « Terminaux »).

Ce que le prototype a appris, au-delà des chiffres :

- **ConPTY attend une réponse** à sa demande de position du curseur (`ESC[6n`) avant d'afficher quoi que ce soit : l'`EventListener` d'`alacritty_terminal` doit relayer `PtyWrite` au PTY. Sans cela, le terminal reste vide.
- La compilation à froid du prototype en release, dans un dossier cible neuf, prend **115 s pour 304 crates** sur ce poste (sources déjà téléchargées) ; une modification du prototype seul, 4 s. La compilation release de l'app prend 20 à 25 min (LTO, `codegen-units = 1`, mémoire `int0-reference-bench`) : GPUI n'y ajouterait que peu.
- Une première version des volets laissait GPUI couper les lignes de 120 colonnes dans un volet étroit, qui se chevauchaient ; corrigé par `whitespace_nowrap()`. Les mesures des dix flux ont été faites avant la correction ; un essai après (`spike-ten-capture.jsonl`, celui de la capture) donne 1 102 ms, 9 ms de blocage, 0,7 s de CPU, dans la fourchette.
- L'API est agréable pour ce qui est ici (`div()` façon Tailwind, `on_drag`/`on_drop`, `StyledText::with_highlights`), mais tout est à écrire : pas de sélection de texte, pas de défilement de l'historique dans le prototype, pas de saisie au-delà des touches simples.

## 6. Les trois chemins

Les estimations comptent des tickets de la taille de ceux d'Escouade (un agent, un à deux jours, revu) et des semaines de calendrier avec 4 à 6 agents en parallèle, comme pour le redesign 2.0 (22 tickets du 2026-10-23 au 2026-10-27) ; un portage complet est plus lourd qu'un redesign (tout le comportement à refaire et à tester), d'où des fourchettes larges.

### 6.1 A : tout GPUI

Une seule pile en Rust, le front comme le cœur ; la webview ne reste que pour la Page d'un lancement (`gpui-webview`).

| Étape | Tickets | Contenu |
| --- | --- | --- |
| 0. Choisir la base | 1 | `gpui-pre` épinglé ou git de Zed ; `gpui-component` ou composants à nous ; règle d'accessibilité (chaque élément annoté) |
| 1. Socle | 6 | cœur sans Tauri (trait `Host`, `Core` sans `R: Runtime`, tests sans `mock_app`) ; les 331 commandes en appels directs et les 41 événements en entités ; coque GPUI, thème (jetons d'`app.css` et des plugins), polices, i18n ; composants de base ; banc de tests GPUI ; paquets, mise à jour, signature et notarisation (à faire tôt : rien ne sort sans eux) |
| 2. Écrans | 38 à 45 | rail et barre (2), barre latérale (2), Kanban (3), conversation et markdown (4), composer, IME, mentions, pièces jointes, dictée (2), terminaux et vues des harness (3), éditeur (3), diffs (2), Git, PR et CI (4), réglages (5), modales (3), plugins et store (2), docs (2), assistant (1), statistiques et workflow (3), vue d'ensemble et barre d'état (2), Page d'un lancement (1), fenêtres multiples (1) |
| 3. Plateforme | 5 | tray (`tray-icon`), instance unique, deep link et installeurs, état des fenêtres, notifications |
| 4. Tests | 8 | les 331 e2e en tests GPUI par lots, une dizaine de tests de fumée par UI Automation, les 230 fichiers Vitest |
| 5. Bascule | 2 | parité, bêta, dernier saut par l'updater de Tauri, retrait de Svelte et de Tauri |
| **Total** | **60 à 67** | **10 à 14 semaines**, sans nouvelle fonctionnalité pendant ce temps (ou chacune faite deux fois) |

**Ce qu'on perd en route** : l'interface web pour le téléphone (§4.1) ; CodeMirror et ses extensions (l'éditeur de `gpui-component` en fait moins) ; la sélection de texte « gratuite » partout ; l'inspecteur de Chrome pour déboguer l'interface ; les 331 e2e existants et leur histoire ; l'appui d'une API stable (GPUI casse entre versions, §1.2) ; le Rust 1.88 minimal.

**Ce qu'on gagne** : la mémoire et la fluidité du §5 ; une seule langue et un seul process ; plus de WebKitGTK sous Linux (le plantage de `text-wrap: pretty` du 2026-10-29) ni de différences entre trois moteurs ; des tests d'interface sans navigateur.

### 6.2 B : hybride, à la manière de Discord

GPUI pour la coque, le Kanban, les terminaux et la conversation ; la webview gardée pour des panneaux (réglages, docs, store, éditeur et diffs), plus la Page d'un lancement.

| Étape | Tickets | Contenu |
| --- | --- | --- |
| Socle | 6 | comme A |
| Pont web | 3 | `gpui-webview` ; refaire pour ces panneaux ce que Tauri donne (protocole de l'app, `invoke`, `Channel`) ; thème partagé entre CSS et GPUI |
| Écrans GPUI | 20 à 24 | coque, barre latérale, Kanban, conversation, composer, terminaux, vues, Git, vue d'ensemble |
| Panneaux web | 4 | les adapter à vivre dans une webview enfant (taille, focus, raccourcis, menus) |
| Plateforme | 5 | comme A |
| Tests | 6 | tests GPUI pour sa part, Playwright gardé pour les panneaux web (par le protocole DevTools de la webview enfant) |
| **Total** | **44 à 48** | **8 à 11 semaines** |

**Ce qu'on perd** : deux piles d'interface à jamais (Svelte et GPUI), deux thèmes, deux façons de tester ; une webview native se dessine par-dessus GPUI, donc aucun menu, liste ou palette de GPUI ne peut passer sur un panneau web : chaque panneau se cache quand un menu s'ouvre (ce que fait déjà `DevPage.svelte`) ou vit dans sa propre fenêtre ; le focus et les raccourcis passent mal la frontière ; dès qu'un panneau web s'ouvre, les process WebView2 reviennent (117 Mo au repos, §5.3), et le gain mémoire avec eux, sauf à détruire la webview à chaque fermeture.

**Ce qu'on gagne** : les terminaux et le Kanban du §5, plus tôt que A et avec moins de risque sur l'éditeur.

Discord peut le faire parce que ses contenus web (vidéos, Activities) sont des zones à part, plein cadre ; nos panneaux sont au milieu des menus, de la palette et des dialogues. L'hybride nous coûterait presque autant que A pour un résultat moins net.

### 6.3 C : rester sur Tauri et viser ses points faibles

| Ticket | Contenu | Cible proposée |
| --- | --- | --- |
| C1. La mémoire du cœur | mesurer ce que tient `escouade.exe` en usage (1 085 Mo, §5.5) : conversations gardées, tampons des vues TUI (4 Mo chacun), instances WASM par plugin et par vue, modèles de voix, caches ; puis réduire ce qui se réduit | par agent ouvert, un budget dit et tenu |
| C2. Le débit des terminaux | contrôle de flux entre le PTY et xterm.js (le rappel d'écriture d'xterm, des seuils haut et bas), regroupement des sorties côté Rust, pas de rendu d'un terminal caché | 100 000 lignes en moins de 1 s, la page jamais bloquée plus de 50 ms |
| C3. La mémoire de la webview | `MemoryUsageTargetLevel` bas de WebView2 quand la fenêtre est cachée ou dans le tray ; historique réduit des terminaux cachés ; vues lourdes déchargées | interface au repos sous 120 Mo |
| C4. Le démarrage | ce que fait la page avant sa première peinture ; chargement paresseux des vues | fenêtre sous 1 s en médiane |
| C5. Le banc | `spikes/gpui/bench/tauri.spec.ts` déplacé dans `scripts/bench/`, joint à `bench:int0`, rejoué avant et après C1 à C4 | |
| C6. Le cœur sans Tauri | le trait `Host` de `docs/REMOTE.md` : utile au mode serveur, et la moitié du socle de A ou B si on y revient | |
| **Total** | **6 tickets, 1 à 2 semaines** | |

**Ce qu'on perd** : rien de ce qui existe ; on garde WebView2, WKWebView et WebKitGTK et leurs différences, et le plafond de la webview (le flux reste plus lent que dans GPUI, même amélioré).

**Ce qu'on gagne** : une bonne part du gain là où il est le plus gros en usage (le cœur), les terminaux fluides, et C6 qui prépare à la fois le mode serveur et un éventuel A plus tard.

## 7. La recommandation

**C, maintenant ; A, peut-être plus tard ; B, non.**

- Le gain de GPUI est réel et mesuré sur ce qu'il dessine : interface au repos 10 fois plus légère, gros flux deux fois plus rapide pour dix fois moins de CPU, plus de page figée. Mais en usage réel le cœur pèse plus que la webview, et il reste.
- Le coût est une réécriture de tout le front et de tous ses tests, sur un framework pré-1.0 dont la seule distribution à jour est republiée chaque semaine par un tiers, sans accessibilité ni sélection de texte « offertes », et en perdant l'interface web prévue pour le téléphone.
- C prend une à deux semaines, vise les mêmes douleurs, et C6 rend le retour vers A moins cher.

**Ce qui ferait changer d'avis** :

1. GPUI publié officiellement et régulièrement par Zed, ou une 1.0 ; ou `gpui-component` en 1.0 sur une GPUI publiée.
2. C2 et C3 ne tiennent pas leurs cibles : la webview reste le goulot des terminaux ou de la mémoire.
3. Le téléphone fait en app native (option B de REMOTE.md) : plus besoin d'une interface web, l'argument du §4.1 tombe.
4. WebKitGTK continue de bloquer Linux (plantages, rendu) au point de coûter plus qu'un portage.
5. Discord sort Phoenix et des mesures indépendantes confirment le gain sur une app de cette taille.

Si l'un d'eux arrive, refaire le prototype sur la GPUI du moment, y ajouter la conversation (markdown sélectionnable, gros historique) et l'éditeur, et rouvrir cette étude.

## 8. Décisions à prendre

- **Le chemin** : A (tout GPUI), B (hybride) ou C (rester sur Tauri et viser ses points faibles). Recommandé : C.
- **Les cibles de C**, si C : 100 000 lignes en moins de 1 s et jamais plus de 50 ms de blocage ; interface au repos sous 120 Mo ; fenêtre sous 1 s ; un budget mémoire par agent pour le cœur. À garder, changer ou compléter.
- **Le téléphone** : interface web servie dans le tunnel (option A de REMOTE.md) ou app native (option B). Le premier lie Escouade à un front web ; le second rouvre la porte à GPUI.
- **Quand revoir GPUI** : sur l'un des déclencheurs du §7, ou à date fixe (par exemple au début de 2027).
- **Le prototype** : garder `spikes/gpui/` comme point de départ d'un futur essai, ou le retirer une fois l'étude lue.

## 9. Sources

Toutes consultées le 2026-11-07, sauf autre date.

- **S1** crates.io, API, `gpui` : 0.2.2 publiée le 2025-10-22 par mikayla-maki (Zed), Apache-2.0 ; 0.2.0 du 2025-10-09, 0.1.0 du 2022-06-23. <https://crates.io/crates/gpui> **[vérifié]**
- **S2** crates.io, API, `gpui-pre` et `gpui-pre-{platform,windows,macos,linux,wgpu,web}` 0.3.11, publiées le 2026-11-02 par huacnlee, « gpui-pre snapshot of zed@f816e1f », Apache-2.0, dépôt `zed-industries/zed`. <https://crates.io/crates/gpui-pre> **[vérifié]**
- **S3** GitHub, commit `f816e1f9263e3a8ef63c4baaa02e1441ce1c6038` de Zed, 2026-11-01 (« gpui: Render macOS window screenshots offscreen ») ; dépôt `zed-industries/zed`, 93 941 étoiles, poussé le 2026-11-07. **[vérifié]**
- **S4** les sources de `gpui-pre` 0.3.11 et de ses crates de plateforme, téléchargées par cargo (`~/.cargo/registry/src/…/gpui-pre-*-0.3.11`) : `src/platform.rs`, `src/_accessibility.rs`, `src/window/a11y.rs`, `src/elements/div.rs`, `examples/`, `gpui-pre-windows/src/{directx_renderer,direct_write,events,clipboard,system_notifications}.rs`, `gpui-pre-platform/src`, `gpui-pre-linux/src/linux` ; la compilation du prototype (Rust 1.94.0 refusé, 1.98.1 accepté). **[vérifié]**
- **S5** `crates/gpui/README.md` sur `main` de Zed. <https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md> **[vérifié]**
- **S6** les `Cargo.toml` de `crates/{gpui,gpui_windows,editor,multi_buffer,language,terminal,terminal_view,markdown,ui}` sur `main` de Zed ; le `Cargo.toml` racine (`alacritty_terminal` d'un fork de Zed, `accesskit` 0.24, `accesskit_windows` 0.34). **[vérifié]**
- **S7** `gpui-component` 0.9.0 (crates.io, 2026-11-02, Apache-2.0), son archive lue (README, `src/`), ses dépendances (`gpui-pre = "=0.3.11"`, `gpui-base` 0.9.0) ; dépôt `longbridge/gpui-kit`, 19 749 étoiles, poussé le 2026-11-07. **[vérifié]** pour la version, les dépendances et les modules ; **[lu]** pour les promesses du README.
- **S8** crates.io : `gpui-webview` 0.9.0 (2026-11-02), `tauri-plugin-gpui` 0.1.0 (2026-10-21, 24 téléchargements), `alacritty_terminal` 0.26.0 (2026-04-06), `cargo-packager` 0.11.8 (2025-11-27), `cargo-packager-updater` 0.2.3 (2025-07-21), `tray-icon` 0.26.1 (2026-10-07), `single-instance` 0.3.3 (2021-12-16), `notify-rust` 4.18.2 (2026-10-07). **[vérifié]**
- **S9** Zed, « Windows When? Windows Now », 2025-10-15, <https://zed.dev/blog/zed-for-windows-is-here>, et sa reprise par Neowin : DirectX 11 et DirectWrite, Vulkan essayé puis abandonné. **[lu]**, par des résumés de recherche, page non ouverte.
- **S10** sur Phoenix, articles du 2026-11-05 environ : Windows Report, « Discord Is Ditching Electron for Rust, Cutting RAM Usage to Just 290MB », <https://windowsreport.com/discord-is-ditching-electron-for-rust-cutting-ram-usage-to-just-290mb/> ; WindowsForum, « Discord Phoenix: Rust and GPUI Replace Electron on Windows, but 290MB RAM Claims Need Testing », <https://windowsforum.com/news/discord-phoenix-rust-and-gpui-replace-electron-on-windows-but-290mb-ram-claims-need-testing.450993/> ; tbreak, <https://tbreak.com/discord-rust-rebuild-electron/>. **[lu]**, par des résumés de recherche ; le message du CTO sur X n'a pas été lu.
- **S11** un billet du 2026-10-01 disant `gpui` 0.2.2 « roughly 11 months behind Zed main », <https://baeseokjae.github.io/posts/native-app-for-coding-agents-rust-gpui/>. **[lu]** ; recoupé par S1 et S2.
- **S12** `open-gpui` 0.1.0, un fork de GPUI, dont un commentaire dit que Zed ne pose presque aucune API d'accessibilité sur ses éléments, <https://docs.rs/crate/open-gpui/0.1.0/source/src/window/a11y.rs>. **[lu]**, non vérifié dans Zed.
- Dans ce dépôt : `docs/REMOTE.md` (2026-10-25), `docs/PLUGINS.md` §1.9, §5.1, §8.30, `src-tauri/src/devpage.rs` et `src/components/DevPage.svelte` (ESC-311), `src-tauri/tauri*.conf.json`, `.github/workflows/release.yml`, `tests/e2e/fixture.ts`, `src/lib/terminals.ts`. **[vérifié]**
- Les mesures : `spikes/gpui/results/*.jsonl` et les captures, produites par `spikes/gpui/bench/`. **[vérifié]**
