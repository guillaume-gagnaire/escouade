---
number: null
id: "un-onglet-de-navigateurs-est-un-vrai-nav"
title: "Un onglet de « Navigateurs » est un vrai navigateur : on s'y connecte à un site, sa fenêtre de connexion comprise"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-695"
pages: ["kanban/page-du-lancement","kanban/navigateurs-pilotes"]
tickets: ["ESC-695"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/page-du-lancement","added":16,"removed":3},{"page":"kanban/navigateurs-pilotes","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Constat de l'utilisateur (2026-11-27) : « mon agent ouvre un navigateur, ça ouvre l'écran de connexion Claude par exemple dans un Chrome, et ça veut pas se connecter dans le navigateur Escouade ». Un onglet ne gardait que l'origine de son adresse de départ : toute autre origine partait au navigateur du système, et une fenêtre ouverte par la page (window.open) était refusée puis envoyée dehors. Or une connexion passe presque toujours par une autre origine (fournisseur d'identité, « Continuer avec Google » qui ouvre une fenêtre, retour par redirection) : elle se faisait dans Chrome, avec la session de Chrome, et l'onglet restait à l'écran de connexion, alors que la session d'un onglet est gardée par projet.

## Décision

1. La navigation de la page (liens, redirections, formulaires, adresse tapée ou collée dans la barre) reste dans l'onglet vers toute origine http(s) ; la barre montre toujours l'adresse en cours. Restent refusés (nulle part, avec un message) : les adresses d'Escouade, file:, data:, javascript:, blob: et tout autre schéma en navigation principale, et ce poste à un port qui n'est ni celui de l'adresse de départ de l'onglet ni un port que déclare le projet (jamais celui du serveur MCP). mailto: et tel: partent au système. Ce qu'ouvre un agent (open_browser, C3) ne change pas.
2. Une fenêtre ouverte par la page s'ouvre dans Escouade, sur la même session que l'onglet (même environnement WebView2, vue liée WebKitGTK, configuration WKWebView), avec window.opener et postMessage ; son titre commence par son origine ; sa taille est bornée et sa place est celle d'Escouade (jamais celle que dit la page) ; au-dessus de la fenêtre d'Escouade, jamais au-dessus d'une autre application ; trois au plus par onglet ; elle se ferme quand la page ou l'ouvreur la ferme, et avec son onglet. Aucune commande ne lui répond, aucun agent ne la lit ni ne la pilote ; une page d'agent n'en ouvre aucune.
3. Un lien de connexion reçu par courriel se colle dans la barre et s'y ouvre ; « Ouvrir dans le navigateur du système » reste.
4. Comme un onglet change désormais d'origine de lui-même, ce qu'un agent lit suit l'origine du moment : read_browser et capture_browser revérifient l'origine du document après coup, et browser_console ne rend que les entrées venues sur une origine du lancement ou permise (C4).

## Conséquences

On se connecte à claude.ai, à Google ou à un site par lien de courriel dans un onglet, et la session reste celle du projet. Une page restée dans l'onglet peut aller sur n'importe quel site : la sécurité tient par la cloison (aucune commande, stockage du projet), par le refus de ce poste hors des ports déclarés, et par C4 appliqué à l'origine du moment (lecture revérifiée, console filtrée par origine). Limites connues : sous WebView2, la requête d'une navigation refusée part quand même (sa réponse n'est jamais chargée) ; sous macOS, non revérifié sur un Mac (WKWebView n'ouvre une fenêtre que sur un geste de l'utilisateur). Un correctif de sûreté en vient : la position que donne la page à sa fenêtre n'est plus utilisée (une valeur négative faisait planter Escouade). L'essai réel (claude.ai, Google, lien de courriel) est à l'utilisateur.
