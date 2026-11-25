---
number: 324
id: "les-agents-pilotent-les-navigateurs-par"
title: "Les agents lisent puis pilotent les navigateurs par sept outils MCP, dans un monde isolé"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-663"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-663","ESC-661"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":91,"removed":0}]
accepted_at: "2026-11-26T00:48:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur veut (2026-11-24 et 25) que les agents, par le MCP, ouvrent un navigateur de la barre latérale à côté de leur conversation et le pilotent en expliquant : c'est le cas phare. Aujourd'hui, aucune commande ne touche les navigateurs (ESC-657). ESC-661 conçoit en même temps les pages des agents, pilotables elles aussi. La sonde d'ESC-663, sur une vraie WebView2 et une vraie WebKitGTK, montre deux choses : un script exécuté dans le monde de la page (`webview.eval`, `evaluate_script_with_callback`) est faussé par une page qui réécrit ses globales ; un monde isolé, lui, lit et clique juste.

## Décision

Sept outils du serveur MCP, du poste seulement, dans la partie « Lancement et navigateurs » : `list_browsers`, `open_browser` (à côté ou en dessous de la conversation de l'appelant, ou la ligne qui attend), `drive_browser` (un seul outil de pilotage pour toutes les lignes de « Navigateurs », pages des agents d'ESC-661 comprises), `capture_browser`, `read_browser`, `browser_console` et `close_browser`.

Le cœur n'exécute que des scripts fixes d'Escouade, paramétrés en JSON. Ils tournent dans un monde isolé de la page, par la poignée native de la webview : protocole DevTools dans le process sous Windows, `run_javascript_in_world` sous Linux, `WKContentWorld` sous macOS. Le résultat revient par le callback natif. Jamais `webview.eval`, jamais un script d'agent, aucun canal de la page vers Escouade. L'utilisateur reprend la main d'un geste : une bande native, un curseur fantôme et les gestes numérotés dans la conversation.

## Conséquences

En 2.2, de `main` (décision de l'utilisateur du 2026-11-26), puis reversé dans la 2.3 : environ 30 h 30 d'agent en deux lots.

- **Le lot 1** (AB-1 à AB-5, environ 12 h 30) lit les navigateurs : `list_browsers`, `read_browser` (le HTML, borné, par sélecteur), `capture_browser` et `browser_console`. Il part en tête.
- **Le lot 2** (AB-6 à AB-13, environ 18 h) pilote, après ESC-666 (AP-0).

AB-R (1 h 30, 2.3 seulement) branche le tout sur la partie « Lancement et navigateurs », Fonctionnalités, `desk-only`, le téléphone, le mode serveur et le ticket d'équipe.

Sous WebKit, clics et frappe sont synthétiques (`isTrusted` à `false`), la console passe par un relais de la page marqué `from: "page"`, et l'arbre d'accessibilité est calculé par le script. macOS est vérifié par la CI Mac d'AB-5.
