---
number: 141
id: "rechercher-des-mises-a-jour-relit-le-sto"
title: "« Rechercher des mises à jour » relit le store avant de chercher"
date: "2026-11-09"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-405"
pages: ["store/store-dans-l-app"]
tickets: ["ESC-405"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":4,"removed":1}]
accepted_at: "2026-11-09T17:52:40+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Retour de bêta 2.2 (2026-11-09) : Claude Code 2.2.4 était publié au store, mais « Chercher maintenant » ne le proposait pas. La recherche comparait aux offres en mémoire, lues seulement au démarrage puis toutes les 6 heures.

## Décision

La recherche des mises à jour lancée par l'utilisateur relit d'abord le catalogue et la liste de révocation du store (par Réglages › Réseau), puis cherche partout (étiquettes Git, dossiers, store, amorce). Une lecture en cours, ou finie depuis moins de 30 s, est reprise : des clics proches ne font qu'une lecture. Store injoignable : la recherche se fait sur les offres gardées, et la page le dit, « Store injoignable : offres du jj/mm hh:mm ». Sinon, elle dit « Store lu le jj/mm hh:mm ». Le bouton dit « Recherche… » et tourne pendant ce temps. La relecture du démarrage et des 6 heures ne change pas.

## Conséquences

Une version publiée est proposée au clic. Un clic coûte au plus une lecture du catalogue (un 304 s'il n'a pas changé) et de la liste toutes les 30 s. La commande plugins_check_updates rend { plugins, store } au lieu de la seule liste.
