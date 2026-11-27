---
number: null
id: "un-seul-pilotage-pour-toutes-les-lignes"
title: "Un seul pilotage pour toutes les lignes de « Navigateurs » : drive_browser, sa cible, la règle de la main, un seul canal"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-666"
pages: ["kanban/navigateurs-pilotes","harness/pages-des-agents"]
tickets: ["ESC-666","ESC-663","ESC-661"]
documents: []
supersedes: [324]
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":70,"removed":0},{"page":"harness/pages-des-agents","added":24,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Deux études conçoivent ensemble ce qu'un agent montre dans une ligne de « Navigateurs » : ESC-661 (`docs/AGENT-PAGES.md`, la page de l'agent, mode 1) et ESC-663 (`docs/AGENT-BROWSERS.md`, un vrai onglet, mode 2). La décision 324 posait sept outils et un seul outil de pilotage, et la décision 316 laissait à AP-0 la forme de la cible (`page: "p-12"` ou `browser: "page:p-12"`). Il restait des écarts entre les deux études : l'ancien nom `drive_page`, des listes d'actions différentes pour une page d'agent, la place de l'interrupteur, sans Fonctionnalités en 2.2. Le 2026-11-26, l'utilisateur a voulu que tout ce qui touche aux navigateurs sorte en 2.2, de `main`. ESC-666 (AP-0) apporte les deux études sur `main` et les met d'accord.

## Décision

Sept outils du serveur MCP, du poste seulement : `list_browsers`, `open_browser`, `drive_browser`, `capture_browser`, `read_browser`, `browser_console`, `close_browser`. S'y ajoutent les outils des pages d'ESC-661 (`show_page`, `get_page`, `close_page`).

**Un seul outil de pilotage, `drive_browser`**, pour toutes les lignes : la page d'un agent comme un vrai onglet. Il n'y a pas de `drive_page`, et une action par appel.

**La cible est `browser`, la clé de la ligne**, celle que rend `list_browsers`, sans champ `page` à part :
- l'id de l'agent pour la page de son test ;
- `run:<projet>:<commande>` ;
- `browser:<id>` ;
- `page:p-12` pour une page d'agent.

L'élément se vise par `target` : `#id`, un sélecteur simple, `text=`, ou une clé de modèle.

**Deux paliers d'actions** :
- montrer : `scroll_to`, `scroll`, `highlight`, `clear`, `width`, `wait` ; `reload`, `back` et `forward` sur un onglet ; `version` sur une page ;
- agir : `tab`, `click`, `type`, `select`, `press`, `navigate`.

Ce qui est permis :
- sur une page d'agent, montrer et `tab`, rien d'autre ;
- sur le lancement du projet, tout (le mot de passe avec accord, décision 326) ;
- hors du lancement, montrer sans demander, agir avec accord (décision 322).

Les options `wait_user` (décision 323) et `look` valent partout. Sur une page d'agent, `capture_browser` et `browser_console` servent, mais pas `read_browser`.

**La règle de la main**, la même pour toutes les lignes :
- pas de pilotage d'une ligne non affichée ;
- le premier geste `isTrusted` de l'utilisateur, ou « Reprendre la main », suspend le pilotage, et l'outil le dit sans rien faire ;
- « La rendre » rouvre le pilotage, et « Ne plus laisser piloter » le ferme ;
- les gestes sont relevés par sondage dans le monde isolé ;
- 4 gestes par seconde et 600 par heure, par ligne.

**Une seule bande de pilotage**, native, hors de la webview, pour les deux modes. La page d'agent y ajoute sa mention de confiance. Chaque geste est une rangée numérotée, et un repère du rail sur une page (direction A).

**Un seul canal** :
- du cœur vers la webview, des scripts fixes d'Escouade paramétrés en JSON, dans le monde isolé `escouade-agent`, par la poignée native : DevTools en process sous Windows (liste fermée de méthodes), `run_javascript_in_world` sous Linux, `WKContentWorld` sous macOS. Jamais `webview.eval`, jamais un script d'agent ;
- de la webview vers le cœur, rien pour un onglet ; pour une page d'agent, seulement `POST _/choose`, une proposition que l'utilisateur confirme.

**Où ça se coupe** :
- en 2.2, deux interrupteurs de Réglages › Agents, sous « Les agents peuvent piloter Escouade », allumés par défaut : « Agents et navigateurs » et « Pages des agents » ;
- en 2.3, les mêmes clés passent dans Réglages › Fonctionnalités, et la partie « Lancement et navigateurs » porte les outils.

Écarté : `drive_page` ou un outil de pilotage par mode, et une cible `page: "p-12"` à part.

## Conséquences

Les deux études disent la même chose, mot pour mot : § 10 de `docs/AGENT-PAGES.md` et § 9 de `docs/AGENT-BROWSERS.md`.

La liste finale des tickets est `docs/AGENT-BROWSERS-TICKETS.md` : 50 h d'agent en 2.2, de `main`, en trois lots.
- **Lot 1, lire** : AB-1 à AB-5, en tête.
- **Lot 2, le socle des pages** : AP-1 à AP-5 et AP-7, en même temps.
- **Lot 3, piloter** : AB-6 à AB-12, AP-9 et DOC.

AP-6 est fondu dans AB-7 et AB-8 (une seule mise en œuvre du pilotage et de la bande). AP-8 et AB-13 ne font plus qu'un, DOC.

AB-R (2 h 30, 2.3 seulement) branche le tout sur les parties, Fonctionnalités, `desk-only`, le téléphone, le serveur, l'Overseer et l'équipe.

Sous WebKit, clics et frappe restent synthétiques, et la console passe par un relais marqué `from: "page"`. macOS est vérifié par la CI Mac d'AB-5.
