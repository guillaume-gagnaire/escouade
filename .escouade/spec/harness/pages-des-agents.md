---
title: "Les pages des agents"
subject: "harness"
sources: ["docs/AGENT-PAGES.md (ESC-661, étude du 2026-11-25 ; apportée sur main et mise d'accord par ESC-666 le 2026-11-26)","docs/AGENT-BROWSERS-TICKETS.md (ESC-666 : la liste finale des tickets)","design/agent-pages/README.md (direction A choisie ; captures a-*)","Demande a-36, réponse de l'utilisateur du 2026-11-25 : A « Les repères », recommandations P1 à P10 acceptées","L'utilisateur, 2026-11-26 (par l'Overseer) : tout ce qui touche aux navigateurs sort en 2.2","Message d'un utilisateur du 2026-11-24 (une donnée) ; l'utilisateur, 2026-11-25 : « le cas trop cool »","docs/AGENT-BROWSERS.md (ESC-663 : drive_browser, monde isolé)","src-tauri/src/devpage.rs, src-tauri/src/mcp/asks.rs (ce qui existe)","src-tauri/src/agentpage.rs (ESC-667 : la cloison)","docs/TODO-2.4.md (plugins d'interface)"]
tickets: ["ESC-661","ESC-666","ESC-667"]
decisions: ["pages-des-agents-p1-p9-p10-les-outils-mc",318,319,320,"pages-des-agents-p7-p8-un-interrupteur-d",317,"un-seul-pilotage-pour-toutes-les-lignes","la-cloison-des-pages-des-agents-une-orig"]
compared: null
---

# Les pages des agents

> **Tranché ; la cloison est construite (AP-1, ESC-667), le reste pas encore.** L'utilisateur a choisi le 2026-11-25 (demande a-36) la direction A « Les repères » et les recommandations P1 à P10 de l'étude ESC-661. Tout ce qui touche aux navigateurs sort en 2.2 (l'utilisateur, 2026-11-26) : les tickets partent de `main`, dans l'ordre de la liste finale `docs/AGENT-BROWSERS-TICKETS.md` (ESC-666). Ce que seule la 2.3 a est dit « en 2.3 ». Le détail est dans `docs/AGENT-PAGES.md`.

## Ce que c'est

Un agent, quel que soit son harness, montre à l'utilisateur une page HTML dans Escouade par le serveur MCP : une revue de fin de lot ou d'avant fusion avec ses choix, un schéma, une comparaison d'options, sa propre page, ou la page à soi que l'utilisateur a réglée une fois. Elle s'ouvre dans une ligne de « Navigateurs », à côté de la conversation de l'agent, qui la pilote en expliquant (défiler, mettre en évidence, changer de largeur) ; l'utilisateur reprend la main quand il veut et tranche dans la page. C'est le premier des deux modes d'ouverture ; le second, un vrai onglet de navigateur, est dans *Les navigateurs pilotés par les agents*. Rien ne sort du poste. Les artefacts de Claude Code (publiés sur claude.ai) ne changent pas.

## L'outil MCP

- `show_page` : `title`, `template` (`review`, `diagram`, `compare`, `html`, `custom`), `data` (le JSON d'un modèle, 256 Ko) ou `html` / `file` (un fichier du dossier de travail et ses fichiers relatifs), `page` (une nouvelle version d'une page), `ask` (la demande de la boîte qu'elle porte), `open` (`beside`, `quiet`). Rend `p-<n>`, la version, la demande `a-<n>`.
- Le pilotage passe par `drive_browser`, l'outil unique des lignes de « Navigateurs » (ESC-663). Une page se vise par `browser: "page:p-12"`, la clé de sa ligne, sans champ `page` à part (fixé par ESC-666). Elle accepte les gestes de montrer (`scroll_to`, `scroll`, `highlight`, `clear`, `width`, `wait`), plus `tab` (un onglet déclaré) et `version` ; jamais `click`, `type`, `select`, `press` ni `navigate`. Les scripts sont des scripts fixes d'Escouade, exécutés dans un **monde isolé** de la page (DevTools de WebView2, `run_javascript_in_world` de WebKitGTK, `inContentWorld` de WKWebView), jamais `webview.eval` ni un script de l'agent. Le pilotage est suspendu dès que l'utilisateur reprend la main. La règle de la main, la bande et le canal sont les mêmes pour tous les modes (*Les navigateurs pilotés par les agents*, « Un seul pilotage »).
- `get_page`, `close_page`. Pas de lecture de ce que l'utilisateur fait dans la page : `read_browser` y est refusé ; `capture_browser` et `browser_console` s'y servent (une page n'a pas encore les tampons de lecture des navigateurs d'AB-1 : à décider par AB-2).
- Une page a ses versions (10 gardées), copiées dans `<données>/pages/` : elle survit au worktree, vit avec son agent puis le délai des fichiers de la boîte. Bornes : 20 pages par agent, 2 Mo de HTML, 10 Mo et 64 fichiers par version.
- Qui : les agents et les discussions ; l'Overseer en 2.3 ; Claude hors Escouade seulement avec une demande. Projet privé : ses pages ne sont connues que de ses agents.
- Sa trace : une carte dans la conversation (titre, modèle, version, état de sa demande, « Afficher », le nombre de repères), et une rangée numérotée par geste de pilotage.

## La vue (direction A « Les repères »)

La conversation et la page côte à côte. Chaque geste de l'agent est un repère numéroté : une rangée « ③ Met en évidence le critère 3 » dans la conversation et le même ③ sur un rail d'Escouade le long de la page, où se voit la partie à l'écran ; un repère cliqué ramène la page où l'agent l'avait mise. Au-dessus de la page, la bande de pilotage, la même que pour un onglet, avec la mention de confiance (« Écrite par pagination · hors réseau, sans accès à Escouade ni à tes fichiers ») : « pagination pilote · Reprendre la main », puis « Tu as la main · La rendre ». Sous la page, la bande du choix. La ligne de « Navigateurs » porte l'étiquette « choix » tant que sa demande attend. Maquettes : `design/agent-pages/` (`?d=a`).

## Le retour des choix

Les choix d'une page sont une demande de la boîte « À toi » (`ask` de `show_page`, le même dépôt que `ask_user`). Un clic dans la page propose une option par un seul canal (`escouade-page://p-<n>.localhost/_/choose`, schéma fermé, validé par le cœur, servi à la seule webview de la page) ; rien ne part sans le geste de l'utilisateur dans la bande d'Escouade sous la page, qui montre le libellé de la demande lu dans le cœur. Ensuite, le circuit de la boîte tel quel : 10 s pour annuler, un message à l'agent, rien d'autre. Un feu vert ne se propose jamais depuis la page. Un onglet (l'app développée, un site) n'a aucun canal.

## Les modèles et la page à soi

Les modèles `review`, `diagram` (Mermaid embarqué, sans réseau) et `compare` (les planches de la boîte) sont des pages d'Escouade remplies des données de l'agent, rendues en texte. La page à soi (`custom`) se règle sur le poste ou est proposée par `pages.custom` du `.escouade.yml` ; le poste l'emporte, et en 2.3 l'équipe peut l'imposer ou la recommander. Elle reçoit `window.escouade` (titre, agent, ticket, thème, langue, données, demande, `choose`).

## Sécurité

Construite par AP-1 (ESC-667) et détaillée, système par système, dans *Les pages des agents : la cloison* (`kanban/pages-des-agents`). En bref : une webview à part (`agentpage-*`, refusée par la garde des commandes ; la capacité ne nomme que `main`), une origine par page (`escouade-page://p-12.localhost/`) servie à sa seule webview depuis sa copie ; une CSP en en-tête et en `<meta>` qui écrit l'origine de la page en toutes lettres (pas `'self'`), sans `unsafe-eval`, sans cadre ni formulaire ; navigation hors de sa page refusée, une adresse du web gardée pour la bande et ouverte sur un geste seulement ; rien qui dure d'une ouverture à l'autre ; permissions et téléchargements refusés ; un script de garde d'Escouade dans chaque cadre (WebRTC, `eval` et `Function`, préchargements ajoutés par script) ; sous Windows et Linux, un cul-de-sac du réseau (et sous Windows un filtre de toutes les requêtes) ; ce qu'Escouade exécute dans la page, dans un monde isolé ; devtools sur un geste de l'utilisateur. La sonde `agentpage_probe` et l'e2e `agentpage.spec.ts` ouvrent une page piège : rien n'en sort.

## Place dans l'app

- **En 2.2** : un interrupteur « Pages des agents » dans Réglages › Agents, sous « Les agents peuvent piloter Escouade » (il dépend du serveur MCP), allumé par défaut, à côté d'« Agents et navigateurs ». Coupé, `show_page`, `get_page` et `close_page` quittent la liste des outils et `drive_browser` refuse les lignes `page:`. Rien n'est à distance.
- **En 2.3** : le même interrupteur passe dans Réglages › Fonctionnalités, sous la même clé, et la partie « Lancement et navigateurs » d'un projet porte les outils. Le téléphone et une fenêtre connectée à un serveur voient la demande dans « À toi », pas la page ; les commandes `page_*` sont du poste seulement ; rien ne part au serveur d'équipe.

## Pour la 2.4

La cloison et le canal servent aux plugins d'interface (panneaux d'un plugin) ; il leur faudra des requêtes et réponses, des abonnements, des permissions du manifeste, un stockage, les jetons du thème (`docs/AGENT-PAGES.md` § 11).
