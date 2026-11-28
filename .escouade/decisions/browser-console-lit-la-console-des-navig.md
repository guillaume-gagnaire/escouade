---
number: null
id: "browser-console-lit-la-console-des-navig"
title: "browser_console lit la console des navigateurs depuis un curseur ou un instant, même cachés, avec l'accord hors du lancement"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-671"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-671"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":28,"removed":7}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-4 (ESC-671) construit le quatrième outil de lecture du lot 1 (docs/AGENT-BROWSERS.md § 3 bis) sur les tampons d'AB-1 et l'accord d'AB-2. Il fallait fixer la forme de since, ce que vaut un curseur d'une vue d'avant, et si la console d'un navigateur caché se lit.

## Décision

browser_console lit les tampons du navigateur : since est le curseur rendu (« console.réseau »), un instant ISO 8601 ou des millisecondes (nombre ou chiffres) ; levels parmi error, warn, info, log, debug (tous par défaut) ; network vrai par défaut ; limit de 1 à 500 (200), pour chacun. Il rend entries (avec url et line, from), network, cursor et dropped ; adresses sans chaîne de requête. La console d'un navigateur caché se lit (elle ne montre rien de la page). Hors du lancement, le même accord que read_browser, par agent, origine et navigateur. Un curseur plus loin que les tampons (navigateur rouvert) lit les nouveaux depuis leur début.

## Conséquences

Un agent suit la console de son app sans rien perdre ni relire, et sait ce que le tampon a perdu. Le message de l'accord dit qu'il peut refaire read_browser ou browser_console. AB-5 reprend l'outil dans son e2e de lot.
