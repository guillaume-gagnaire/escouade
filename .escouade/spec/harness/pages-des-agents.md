---
title: "Les pages des agents"
subject: "harness"
sources: ["docs/AGENT-PAGES.md (ESC-661, étude du 2026-11-25)","design/agent-pages/README.md (direction A choisie ; captures a-*)","Demande a-36, réponse de l'utilisateur du 2026-11-25 : A « Les repères », recommandations P1 à P10 acceptées","Message d'un utilisateur du 2026-11-24 (une donnée) ; l'utilisateur, 2026-11-25 : « le cas trop cool »","docs/AGENT-BROWSERS.md (ESC-663 : drive_browser, monde isolé)","src-tauri/src/devpage.rs, src-tauri/src/mcp/asks.rs (ce qui existe)","docs/TODO-2.4.md (plugins d'interface)"]
tickets: ["ESC-661"]
decisions: [316,318,319,320,321,317]
compared: null
---

# Les pages des agents

> **Tranché, pas encore construit.** L'utilisateur a choisi le 2026-11-25 (demande a-36) la direction A « Les repères » et les recommandations P1 à P10 de l'étude ESC-661 ; les tickets AP-0 à AP-9 la construisent dans la 2.3. Le détail est dans `docs/AGENT-PAGES.md`.

## Ce que c'est

Un agent, quel que soit son harness, montre à l'utilisateur une page HTML dans Escouade par le serveur MCP : une revue de fin de lot ou d'avant fusion avec ses choix, un schéma, une comparaison d'options, sa propre page, ou la page à soi que l'utilisateur a réglée une fois. Elle s'ouvre dans une ligne de « Navigateurs », à côté de la conversation de l'agent, qui la pilote en expliquant (défiler, mettre en évidence, changer de largeur) ; l'utilisateur reprend la main quand il veut et tranche dans la page. Rien ne sort du poste. Les artefacts de Claude Code (publiés sur claude.ai) ne changent pas.

## L'outil MCP

- `show_page` : `title`, `template` (`review`, `diagram`, `compare`, `html`, `custom`), `data` (le JSON d'un modèle, 256 Ko) ou `html` / `file` (un fichier du dossier de travail et ses fichiers relatifs), `page` (une nouvelle version d'une page), `ask` (la demande de la boîte qu'elle porte), `open` (`beside`, `quiet`). Rend `p-<n>`, la version, la demande `a-<n>`.
- Le pilotage passe par `drive_browser`, l'outil unique des lignes de « Navigateurs » (ESC-663) ; une page se vise par `page: "p-12"` ou `browser: "page:p-12"` (forme fixée en AP-0). Actions : `scroll_to`, `highlight`, `clear`, `width`, `tab`, `version`, par des scripts fixes d'Escouade exécutés dans un **monde isolé** de la page (DevTools de WebView2, `run_javascript_in_world` de WebKitGTK, `inContentWorld` de WKWebView), jamais `webview.eval` ni un script de l'agent ; suspendu dès que l'utilisateur reprend la main.
- `get_page`, `close_page`. Pas de lecture de ce que l'utilisateur fait dans la page.
- Une page a ses versions (10 gardées), copiées dans `<données>/pages/` : elle survit au worktree, vit avec son agent puis le délai des fichiers de la boîte. Bornes : 20 pages par agent, 2 Mo de HTML, 10 Mo et 64 fichiers par version.
- Qui : les agents, l'Overseer, les discussions ; Claude hors Escouade seulement avec une demande. Projet privé : ses pages ne sont connues que de ses agents.
- Sa trace : une carte dans la conversation (titre, modèle, version, état de sa demande, « Afficher », le nombre de repères), et une rangée numérotée par geste de pilotage.

## La vue (direction A « Les repères »)

La conversation et la page côte à côte. Chaque geste de l'agent est un repère numéroté : une rangée « ③ Met en évidence le critère 3 » dans la conversation et le même ③ sur un rail d'Escouade le long de la page, où se voit la partie à l'écran ; un repère cliqué ramène la page où l'agent l'avait mise. Au-dessus de la page, la bande de confiance (« Écrite par pagination · hors réseau, sans accès à Escouade ni à tes fichiers ») ; dans sa barre, « pagination pilote · Reprendre la main », puis « Tu as la main · La rendre » ; sous la page, la bande du choix. La ligne de « Navigateurs » porte l'étiquette « choix » tant que sa demande attend. Maquettes : `design/agent-pages/` (`?d=a`).

## Le retour des choix

Les choix d'une page sont une demande de la boîte « À toi » (`ask` de `show_page`, le même dépôt que `ask_user`). Un clic dans la page propose une option par un seul canal (`escouade-page://<p-n>/_/choose`, schéma fermé, validé par le cœur, servi à la seule webview de la page) ; rien ne part sans le geste de l'utilisateur dans la bande d'Escouade sous la page, qui montre le libellé de la demande lu dans le cœur. Ensuite, le circuit de la boîte tel quel : 10 s pour annuler, un message à l'agent, rien d'autre. Un feu vert ne se propose jamais depuis la page. Une page de lancement (l'app développée) n'a aucun canal.

## Les modèles et la page à soi

Les modèles `review`, `diagram` (Mermaid embarqué, sans réseau) et `compare` (les planches de la boîte) sont des pages d'Escouade remplies des données de l'agent, rendues en texte. La page à soi (`custom`) se règle sur le poste ou est proposée par `pages.custom` du `.escouade.yml` (le poste l'emporte) ; elle reçoit `window.escouade` (titre, agent, ticket, thème, langue, données, demande, `choose`).

## Sécurité

Une webview à part (`agentpage-*`, refusée par `refuses`), sous une origine à elle (`escouade-page://`) servie depuis la copie seule ; CSP stricte (`default-src 'none'`, `connect-src 'self'`, scripts sans `unsafe-eval`, pas de cadre ni de formulaire), aucun réseau sans exception, navigation hors de sa page seulement par une bande d'Escouade sur un geste, `incognito`, permissions et téléchargements refusés, préchargements DNS et `RTCPeerConnection` retirés, ce qu'Escouade exécute dans un monde isolé, devtools sur un geste de l'utilisateur. Ce qui manque à la webview des navigateurs, système par système, est dans `docs/AGENT-PAGES.md` § 8.4.

## Place dans l'app

Réglages › Fonctionnalités : « Pages des agents » (avec le serveur MCP). Dans un projet : la partie « Lancement et navigateurs ». Le téléphone et une fenêtre connectée à un serveur voient la demande dans « À toi », pas la page (en 2.3) ; les commandes `page_*` sont du poste seulement ; rien ne part au serveur d'équipe.

## Pour la 2.4

La cloison et le canal servent aux plugins d'interface (panneaux d'un plugin) ; il leur faudra des requêtes et réponses, des abonnements, des permissions du manifeste, un stockage, les jetons du thème (`docs/AGENT-PAGES.md` § 11).
