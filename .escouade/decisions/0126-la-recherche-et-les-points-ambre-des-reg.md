---
number: 126
id: "la-recherche-et-les-points-ambre-des-reg"
title: "La recherche et les points ambre des réglages (R8)"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-386"
pages: ["fenetre/reglages"]
tickets: ["ESC-386"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":23,"removed":9}]
accepted_at: "2026-11-08T12:12:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Dernier lot de la nouvelle fenêtre des réglages (direction A, design/settings-2-2/PROPOSITION.md §2, principes 6 et 7). La fenêtre compte près de 240 contrôles sur 17 pages ; la barre avait un champ « Rechercher un réglage » inerte depuis R2, et rien ne disait qu'une page attendait quelque chose (un harness sans compte, un jeton expiré, un plugin arrêté). Les messages du cœur et du kit nommaient encore les onglets de la 2.0 (« Réglages › Réseau », « Settings › Harness › Attachments », « Réglages › Intégrations »).

## Décision

1. La recherche lit le registre des rangées (une rangée = son libellé, la description trouvée à côté dans le catalogue, sa page, Avancé ou non) et le nom et la description des pages, en français et en anglais quelle que soit la langue de l'interface, casse et accents ignorés. Ses résultats remplacent les entrées de la barre tant que le champ a du texte ; un résultat ouvre sa page (et son projet), déplie Avancé, amène la rangée au milieu et la surligne en ambre. Ctrl+F y mène ; Échap vide le champ, un second ferme la fenêtre. À l'ouverture, le focus reste sur l'entrée de la page à l'écran.
2. Les points ambre sont sur Comptes et harness (harness trouvé sans compte), Services connectés (jeton expiré ou illisible), Plugins (désactivé par Escouade, manifeste invalide, arrêté seul) et sur un projet qu'aucun compte de son harness ne sert (son entrée et sa page Projet). La raison est l'infobulle de l'entrée, une ligne par problème ; le point lui-même est muet pour un lecteur d'écran, l'entrée en a la description, et son nom ne change pas.
3. Un test refuse tout « Réglages › X » / « Settings › X » dont X n'est pas une page, dans les catalogues, les sources de l'app, le cœur et le kit (pas plugins/, refait par ESC-387). Les chemins nomment une page puis son groupe : « Réglages › Avancé › Réseau », « Settings › Advanced › Attachments », « Réglages › Agents › Textes rédigés », « Réglages › Services connectés ».

## Conséquences

La recherche ne connaît que ce que le registre déclare : une rangée ajoutée à une page sans entrée au registre n'est pas trouvée (le test « rien ne se perd » couvre les clés, pas les libellés). Les jetons et les process des plugins sont lus à l'ouverture de la fenêtre (un appel `forges` et un `plugin_run` par plugin en marche). Le kit escouade-plugin-kit change de texte (hosts.rs, http.rs) : à republier sur crates.io par le chef. Les plugins de plugins/ gardent les anciens chemins jusqu'à ESC-387.
