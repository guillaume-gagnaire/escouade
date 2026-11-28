---
number: null
id: "la-cloison-des-pages-des-agents-une-orig"
title: "La cloison des pages des agents : une origine par page partout, un cul-de-sac du réseau, un filtre de WebView2 et un script de garde au-delà de la CSP"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-667"
pages: ["kanban/pages-des-agents","harness/pages-des-agents"]
tickets: ["ESC-667","ESC-661"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/pages-des-agents","added":31,"removed":10},{"page":"harness/pages-des-agents","added":4,"removed":4}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'étude des pages des agents (docs/AGENT-PAGES.md § 8) prévoyait une même origine pour toutes les pages sous Windows (http://escouade-page.localhost/), une CSP en 'self', `incognito` partout, et comptait sur la CSP, on_navigation et le retrait des préchargements du HTML servi. La sonde d'AP-1 (agentpage_probe, une vraie WebView2) a montré, malgré une CSP appliquée par l'en-tête et par le <meta> : une règle de spéculation et une préconnexion ajoutées par script sortent ; une navigation refusée par on_navigation part quand même ; avec un filtre de requêtes seul, WebView2 se connecte encore à l'adresse refusée (préconnexion, après une résolution DNS) ; eval (et new Function avec les devtools permis) passe. Sous Windows, 'self' laisserait un WebSocket joindre ws://escouade-page…localhost, le port 80 de la machine. Sous Linux, une WebKitGTK `incognito` reçoit de wry un contexte à elle où Tauri n'enregistre pas le schéma : la page ne se charge pas.

## Décision

1. Une origine par page sur tous les systèmes : escouade-page://p-12.localhost/ (WebView2 : http://escouade-page.p-12.localhost/), en plus du contrôle par étiquette. 2. La CSP écrit l'origine de la page en toutes lettres, jamais 'self'. 3. Sous Windows et Linux, les webviews des pages ne joignent le réseau que par un proxy d'Escouade sur la boucle locale qui coupe toute connexion ; sous Windows, elles ont leur propre navigateur WebView2 (dossier agent-pages-webview, --proxy-server avec <-loopback>, WebRTC jamais hors proxy, sans préchargement DNS), et un filtre WebResourceRequested sur * répond 403 à toute requête qui n'est pas la page, la page n'étant servie qu'une fois ce filtre posé ; quand WEBVIEW2_USER_DATA_FOLDER est imposé (les e2e), le filtre seul. macOS sans proxy (macOS 14 et une option de Tauri requises). 4. Un script de garde d'Escouade, dans chaque cadre et avant ceux de la page : WebRTC retiré, eval et Function refusés, les préchargements ajoutés par script désarmés. 5. Sous Linux, pas d'incognito : un dossier de données neuf à chaque ouverture, effacé à la fermeture.

## Conséquences

Rien ne sort d'une page sous WebView2 et sous WebKitGTK 2.52 (sonde ; e2e Windows sans requête ni datagramme). Les pages coûtent sous Windows un process de navigateur WebView2 à part (partagé par toutes) et sous Linux un contexte WebKit par page ouverte. Une page qui se sert de new Function ou d'eval (un moteur de gabarits) ne marche pas : c'est P4. macOS repose sur la CSP, la garde de navigation et le script : la sonde en CI le dira, un proxy pour macOS 14+ resterait à faire si elle trouve une fuite. AP-2 à AP-9 partent de cette cloison, et les plugins d'interface de la 2.4 aussi.
