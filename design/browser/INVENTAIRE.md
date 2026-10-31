# ESC-311, un navigateur dans Escouade : inventaire et directions

Aperçus rendus par `node design/browser/render.mjs`, par-dessus l'aperçu de l'app (`design/app-redesign/index.html`) :
`a-onglet.png`, `b-panneau.png`, `c-fenetre.png` et `etats.png` (les états, communs aux trois directions).

## Les endroits touchés

| Endroit | Aujourd'hui | Avec le navigateur |
| --- | --- | --- |
| « ▶ Tester » d'un ticket (`test-launch.svelte.ts`, fenêtre « Tester DEM-4 ») | « Ouvert dans le navigateur : http://… » ; `openUrl` à `launch.ready` | la page s'ouvre dans Escouade ; la ligne dit « Ouvert dans Escouade » et garde le lien vers le navigateur du système |
| Lancements de la barre latérale (projet et worktrees) | une ligne par processus, son état | le processus qui a une adresse dit « page ouverte » ; un clic ouvre la page |
| Lancement d'un worktree (isola, recette) | adresse de la recette ouverte au navigateur | même chose, une page par worktree |
| Réglages › Projets | — | « Ouvrir les pages des lancements : Dans Escouade / Dans le navigateur du système » |
| Raccourcis | — | F12 (Windows, Linux), Cmd+Opt+I (macOS) quand la page a le focus ou que sa barre est visible |
| Petite largeur (fenêtre de 1028 px, panneau étroit) | — | la barre se resserre : avancer et l'adresse d'origine passent dans « … » |

## Les états de la page

1. **Le lancement démarre** : la page attend son adresse (point ambre), « web démarre… », « Voir les logs ».
2. **Chargement** : filet de progression sous la barre, point ambre.
3. **La page ne répond pas** : le serveur tourne mais rien ne revient (délai), point rouge, « Recharger », « Voir les logs ».
4. **Le lancement s'est arrêté** (ou a planté, avec son code) : la page reste figée, « Relancer », « Voir les logs ».
5. **Le lancement échoue avant d'être prêt** : comme 4, sans page à figer.
6. **Lien vers une autre origine** : il s'ouvre dans le navigateur du système ; un message le dit.
7. **Plusieurs worktrees en même temps** : une page par worktree, nommée « worktree · processus » ; relancer recharge la même page.
8. **Relance** : la même page se recharge à la même adresse (le chemin courant gardé si le port est le même).
9. **Le worktree est supprimé ou le ticket validé** : sa page se ferme.

Tous ces textes existent en français et en anglais (`src/lib/i18n`).

## Les trois directions

- **A, un onglet de l'agent** (`a-onglet.png`) : « Page » rejoint « Escouade » et « Claude Code » dans le sélecteur de vue de l'agent. La page prend toute la zone centrale ; le lien au worktree est évident (c'est l'agent ouvert). Contre : on ne voit plus la conversation en même temps que la page.
- **B, un onglet du panneau de droite** (`b-panneau.png`) : « Page » à côté de « Non commités », « Historique », « PR et CI » ; la conversation reste à gauche, le panneau s'élargit (58 %). On voit l'agent et son résultat ensemble. Contre : la page est étroite (≈ 640 px sur un écran de 1440), une app de bureau se lit mal.
- **C, une fenêtre à part** (`c-fenetre.png`) : une fenêtre « demo-api · Pages » par projet, un onglet par worktree, qu'on peut mettre sur un second écran. Contre : on quitte la fenêtre d'Escouade (moins qu'avec le navigateur du système, mais quand même).

## Ce que Tauri 2.12 impose (lu dans la source de `tauri-2.12.0`)

- **A et B** demandent une webview **enfant** posée dans la fenêtre d'Escouade (`Window::add_child`), derrière la fonctionnalité **`unstable`** de Tauri. Une webview native se dessine **au-dessus** du DOM : les menus, la palette (Ctrl+K) et les fenêtres d'Escouade qui passent sur la zone de la page doivent la masquer le temps qu'ils sont ouverts.
- **C** utilise `WebviewWindowBuilder`, API stable ; rien ne passe au-dessus.
- **Devtools en version publiée** : fonctionnalité `devtools` de Tauri (sinon `open_devtools` n'existe qu'en debug). Sous macOS elle passe par des API privées de WebKit : pas de Mac App Store, sans effet pour Escouade (Developer ID, notarisée). Sous Windows (WebView2), les devtools s'ouvrent toujours dans leur propre fenêtre ; sous macOS et Linux, l'inspecteur s'attache à la page.
- **Stockage à part** : `data_directory` (Windows, Linux) par projet ; sous macOS `data_store_identifier` (macOS 14 et plus), et en deçà une session `incognito`.
- **Isolation** : la page est chargée en `WebviewUrl::External`, sans capacité Tauri (aucune fenêtre ni webview de ses `capabilities`), `on_navigation` refuse toute autre origine que celle du lancement et l'envoie au navigateur du système, `on_new_window` aussi.

## Le choix (2026-10-31)

**A, avec la fenêtre de C en plus.** « Page » dans le sélecteur de vue de l'agent par défaut ; un bouton de la barre
« Ouvrir dans une fenêtre à part » détache la page dans la fenêtre « demo-api · Pages » (un onglet par worktree,
déplaçable sur un second écran) ; « Ouvrir dans le navigateur du système » reste. Un réglage par projet choisit où
s'ouvrent les pages des lancements : dans l'agent, dans une fenêtre à part, ou dans le navigateur du système. La webview
enfant (fonctionnalité `unstable` de Tauri) est acceptée pour A : elle se masque quand un menu, la palette ou une fenêtre
d'Escouade passe dessus, testé sur les trois systèmes (WebKitGTK sous Linux compris).

## Ajout : pleine largeur, tablette, mobile (2026-10-31)

Aperçus : `a-tablette.png`, `a-mobile.png`, `a-retreci.png` (fenêtre de 1028 px) et leurs versions `-en`.
Trois boutons à côté de Devtools (dans « … » en vue étroite) : pleine largeur, tablette 768 px, mobile 390 px,
centrée sur fond neutre avec un filet, la hauteur de la zone ; la largeur s'affiche à côté, « Rétréci à 692 px »
quand la zone est plus étroite. Gardée par page (« worktree · processus »), dans l'agent comme dans la fenêtre à part.
Raccourci : `Ctrl+Maj+M` (`⇧⌘M`), celui du mode appareil des devtools de Chromium, qui fait le tour des trois.

Ce que voit le site en plus de la largeur (non fait, la largeur suffit aux media queries) :

| | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
| --- | --- | --- | --- |
| User agent mobile | `ICoreWebView2Settings::put_UserAgent`, à chaud | `customUserAgent`, à chaud | `webkit_settings_set_user_agent`, à chaud |
| `devicePixelRatio` | `CallDevToolsProtocolMethod("Emulation.setDeviceMetricsOverride")` | aucune API | aucune API |
| Tactile | `Emulation.setTouchEmulationEnabled` par le même protocole | aucune API | aucune API |

## Ce que Tauri demande (rapport)

- Fonctionnalité `devtools` de `tauri` : sans elle, `Webview::open_devtools` n'existe qu'en debug. Effet : toute webview a
  ses devtools par défaut (`unwrap_or(true)` dans tauri-runtime-wry) ; la fenêtre principale les coupe donc hors debug
  (`window.devtools = Some(cfg!(debug_assertions))`), la barre de « Pages » aussi. Sous macOS, elle passe par des API
  privées de WebKit : refusée par le Mac App Store, sans effet pour Escouade (Developer ID, notarisée). WebView2 ouvre
  toujours ses devtools dans une fenêtre à part, et `is_devtools_open` y répond toujours faux.
- Fonctionnalité `unstable` : `Window::add_child` (webview enfant dans la fenêtre), `Webview::reparent` (passer dans la
  fenêtre « Pages » sans recharger), `WindowBuilder` sans webview.
- Capacités : par `webviews` (`main`, `pagesbar-*`) et non par `windows`, qui couvrirait aussi les webviews enfants de
  la fenêtre principale. Les commandes de l'app ne passent l'ACL que pour une origine distante ; l'origine du serveur
  de dev est « locale » pour Tauri, d'où le refus explicite de toute commande venant d'une webview `devpage-*`
  (`windowthread::off_the_window`).
- Stockage : `data_directory` (Windows, Linux), `data_store_identifier` (macOS 14+), `incognito` avant.

## L'accès des agents à la page (MCP), à faire

Pour qu'un agent vérifie son travail lui-même, des outils MCP d'Escouade sur la page de **son** worktree :

- `page_open(url?)`, `page_reload` : ouvrir ou recharger sa page (sous les garde-fous du pilote auto, sur l'origine du
  lancement seule) ;
- `page_screenshot(width?)` : une capture PNG de la page (WebView2 `CapturePreview`, WKWebView `takeSnapshot`,
  WebKitGTK `webkit_web_view_get_snapshot`), à la largeur choisie ;
- `page_console(since?)` et `page_network(since?)` : les messages de la console et les requêtes ; sous Windows par
  le protocole des devtools (`Runtime.consoleAPICalled`, `Network.*`), ailleurs par un script injecté qui les note
  et les rend par une adresse refusée (comme le raccourci des devtools), WebKit n'ayant pas d'accès au protocole ;
- `page_click(selector)`, `page_type(selector, text)`, `page_text(selector?)` : par `eval` d'un script dans la page,
  le résultat rendu de même.

Ce que ça demande : un canal de retour de la page vers le cœur qui ne soit pas l'IPC de Tauri (adresse refusée ou
serveur local à jeton) ; ne jamais exécuter dans la page un texte de l'agent tel quel (sélecteurs échappés, actions
d'une liste fermée) ; limiter les captures et journaux en taille ; journaliser chaque appel comme les autres outils ;
un projet privé n'expose sa page qu'à ses agents ; l'utilisateur voit quand un agent clique dans la page.
