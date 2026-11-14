---
number: null
id: "une-mise-a-jour-de-plugin-laisse-sur-la"
title: "Une mise à jour de plugin laisse sur la page ; « Tout mettre à jour » groupe celles qui ne demandent rien de plus"
date: "2026-11-13"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-436"
pages: ["store/store-dans-l-app"]
tickets: ["ESC-436"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":8,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Demande de l'utilisateur du 2026-11-13 : un bouton « Tout mettre à jour » sur la page des mises à jour, et ne plus être envoyé sur la page du plugin après une mise à jour. Avant ce changement, « Tout mettre à jour » n'existait qu'en tête des installés et ouvrait une fenêtre par plugin, l'une après l'autre ; annuler l'une arrêtait toute la file. Chaque mise à jour finie ouvrait la page du plugin. Le cœur ne gardait qu'une lecture en attente pour toute l'app, si bien qu'on ne pouvait pas lire toutes les mises à jour avant de demander l'accord.

## Décision

« Tout mettre à jour · N » est aussi en tête de la page Mises à jour. Une mise à jour laisse sur la page où on était ; seule une installation neuve ouvre la page du plugin. « Tout mettre à jour » lit d'abord toutes les versions. Celles qui n'ajoutent ni permission ni contribution se confirment ensemble, dans une seule fenêtre. Les autres ont chacune leur fenêtre, après, avec ce qu'elles ajoutent marqué. Refuser l'une n'annule pas les autres, et une erreur est dite sur sa ligne sans arrêter les autres. Rien ne s'installe sans un geste. Le cœur garde une lecture en attente par plugin.

## Conséquences

Moins de fenêtres quand rien de nouveau n'est demandé, sans que l'accord se perde : une permission ou une contribution nouvelle a toujours sa propre fenêtre. Une version qu'on ne peut pas comparer, faute de manifeste installé, est traitée comme une version qui demande plus. Plusieurs plugins peuvent maintenant être lus et en attente en même temps (un par id) ; ils sont effacés quand on refuse ou qu'on annule. Le faux store des e2e peut publier sur demande des versions retenues (POST /_fake/release).
