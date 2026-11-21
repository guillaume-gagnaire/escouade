---
number: 241
id: "les-points-reserves-du-manifeste-ne-s-an"
title: "Les points réservés du manifeste ne s'annoncent plus, validate les dit sans refuser"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-524"
pages: ["plugins/vue-d-ensemble"]
tickets: ["ESC-524","ESC-501"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/vue-d-ensemble","added":7,"removed":2}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

`menus`, `statusBar`, `badges` et `projectTemplates` sont lus mais rien ne les sert. Pourtant la fenêtre les comptait dans « Ce qu'il ajoute », rangeait le plugin dans « Outils » et les citait dans Réglages › Plugins : un auteur tiers (issues 38 à 42) et l'utilisateur croyaient qu'ils marchaient. L'étude du contrat des plugins (ESC-501, T7, choix n° 9) recommande de ne plus les annoncer tout de suite, dans une 2.2.x.

## Décision

La fenêtre (fenêtre d'installation depuis un dossier ou le store, page du plugin, Réglages › Plugins) ne compte ni ne montre plus ces quatre points et ne range plus un plugin d'après eux ; un plugin qui ne déclare qu'eux n'ajoute rien. `escouade-plugin validate` et le contrôle du manifeste du store (lecteur partagé, corpus commun) disent chacun d'eux, et `escouadeYml`, « réservé, sans effet dans cette version » : un avertissement, la version passe. Le contrat ne change pas. `escouadeYml` sort de la documentation. `description`, `default`, `options` d'un réglage et l'ordre du manifeste sont documentés.

## Conséquences

Aucun manifeste publié n'est refusé ; aucun plugin de la première main ne change de rangement ni de contributions montrées (aucun ne déclare ces points). Concevoir `statusBar` reste une étude à part ; la portée `project` (T4) remplacera `escouadeYml` dans la 2.3.
