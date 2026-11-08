---
number: 120
id: "la-page-plugins-des-reglages-devient-le"
title: "La page Plugins des réglages devient le tableau des plugins installés et proposés"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-384"
pages: ["fenetre/reglages"]
tickets: ["ESC-384"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":14,"removed":2}]
accepted_at: "2026-11-08T04:18:54+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Dans la fenêtre des réglages de la 2.2 (direction A), la page Plugins n'était qu'un lien vers la vue Plugins, alors que deux textes de l'app disaient « Réglages › Plugins le rallume » et que rien ne disait quel plugin sert quel réglage. L'amorce d'ESC-373 propose aussi des plugins qu'on ne voyait que dans la vue.

## Décision

Réglages › Plugins liste les plugins installés : ce que chacun ajoute et où cela se règle, sur combien de projets il est actif, « Vérifié », « Réglages » (sa page dans la vue, onglet Réglages) et son interrupteur dans Escouade. Elle liste aussi les plugins de l'amorce pas encore installés, installés d'un clic depuis l'amorce, sans réseau ni fenêtre, et porte « N'installer que des versions vérifiées » et « Ouvrir la vue Plugins ». La vue Plugins reste l'endroit où l'on installe depuis le store, un dossier ou un dépôt, où l'on active un plugin projet par projet, retire une permission et lit un journal. Les lignes « Ce qu'il ajoute » nomment les pages de la 2.2 : Général › Couleurs, Services connectés, Comptes et harness.

## Conséquences

« Réglages › Plugins le rallume » dit vrai : l'interrupteur est sur la ligne du plugin. Aucune clé enregistrée ne change (pluginsVerifiedOnly reste où il était). Depuis les réglages, « Installer » prend toujours la version de l'amorce, même quand le store en a une plus récente : celle-ci se propose ensuite comme une mise à jour dans la vue.
