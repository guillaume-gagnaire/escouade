---
number: null
id: "la-page-a-soi-le-poste-l-emporte-pages-c"
title: "La page à soi : le poste l'emporte, pages.custom attend l'accord du poste et ne sort jamais du dépôt"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-677"
pages: ["harness/pages-des-agents","projets/escouade-yml"]
tickets: ["ESC-677"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":20,"removed":12},{"page":"projets/escouade-yml","added":10,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AP-7 (ESC-677) construit la page à soi des pages des agents (show_page, template custom). L'étude ESC-661 (docs/AGENT-PAGES.md § 7.4) disait qu'une page du dépôt n'avait pas besoin d'un accord comme une commande, puisqu'elle ne fait rien tourner hors de la cloison. Le ticket AP-7, écrit par l'Overseer, demande au contraire que pages.custom ne s'applique qu'après l'accord du poste, comme les autres réglages du fichier : un .escouade.yml vient d'un dépôt, donc d'un tiers possible.

## Décision

1. La page à soi vient d'abord du poste (« Page à soi », Réglages › Projets › Lancement, groupe « Pages des agents » ; gardée sur la machine, jamais dans le fichier), sinon de pages.custom de .escouade.yml, sinon de rien : le poste l'emporte toujours. 2. pages.custom compte avec les commandes du fichier (même empreinte, même « Faire confiance à ces commandes ») : rien n'en est appliqué avant l'accord, une valeur changée le redemande, l'approuvée reste en place d'ici là. 3. pages.custom est un .html relatif au dépôt : absolu, schéma ou « .. » refusé à la lecture ; à l'usage, le fichier est ouvert dans le dossier du projet, liens suivis, et refusé s'il en sort. 4. window.escouade ne porte que ce qu'un modèle reçoit (page, version, modèle, titre, nom de l'agent, ticket, nom du projet, thème, langue, données, demande nulle jusqu'à AP-4), gelé, sans chemin ni jeton. 5. Sans page réglée (ou celle du dépôt en attente), show_page(custom) est refusé en disant de prendre review, diagram, compare ou html.

## Conséquences

Un dépôt cloné ne fait servir aucune page à l'insu de l'utilisateur, au prix d'un accord de plus quand il en propose une. Ce qu'en disait l'étude (« pas besoin d'un accord ») est remplacé. Le schéma de .escouade.yml gagne pages.custom. En 2.3, AB-R ajoute pages.custom imposé ou recommandé par l'équipe, sous la même règle du poste.
