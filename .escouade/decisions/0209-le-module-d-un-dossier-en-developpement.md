---
number: 209
id: "le-module-d-un-dossier-en-developpement"
title: "Le module d'un dossier en développement est le dernier construit dans packs/, et un dossier sans module dit quoi lancer"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-500"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-500"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":9,"removed":0}]
accepted_at: "2026-11-18T12:16:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Issue #43 : « Charger un dossier » d'un plugin Rust exigeait plugin.wasm à la racine du dossier (« refused its module: …/plugin.wasm: No such file or directory »), alors que escouade-plugin build écrit packs/<id>-<version>.wasm ; la doc faisait copier le module à la main. Le SDK publié (0.3.0) avait un validateur plus ancien que l'app, et les documents se contredisaient (PLUGINS-DESIGN §5.4 disait qu'un plugin WASM ne lance aucun programme).

## Décision

Un dossier en développement (Origin::Dev) prend le module packs/<id>-<version>.wasm (le dernier construit pour la version de son manifeste), sinon son plugin.wasm ; rien n'est compilé au chargement ni pris hors du dossier. Sans module, « Charger un dossier » refuse en disant la commande (escouade-plugin build <dossier> ; npm run build pour un run.js sans bundle), et un module retiré ensuite arrête le plugin avec la même phrase. Un plugin installé garde son plugin.wasm seul. Le validateur du manifeste de la bibliothèque partagée (store, SDK) expose ses champs, comparés à plugin-1.json, et chaque manifeste du dépôt est jugé par les deux lecteurs dans un test. docs/PLUGINS.md et les schémas font foi pour un auteur ; PLUGINS-DESIGN.md est le document de conception, daté.

## Conséquences

Plus de geste caché : créer, construire, charger, reconstruire suffit, et c'est joué par un test en Rust et en TS. Compiler au chargement aurait demandé la chaîne Rust privée (650 Mo) à l'ouverture d'un dossier : écarté. @escouade-dev/plugin 0.4.0 et create-escouade-plugin 0.5.0 sont prêts, non publiés (le chef publie). Les issues #38 à #42 (évolutions du contrat) restent à part.
