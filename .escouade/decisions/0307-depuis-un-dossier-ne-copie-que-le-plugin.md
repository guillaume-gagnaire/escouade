---
number: 307
id: "depuis-un-dossier-ne-copie-que-le-plugin"
title: "« Depuis un dossier… » ne copie que le plugin et prend le module construit de packs/"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-652"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-652"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":9,"removed":0}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Issue #43 (2026-11-22, Escouade 2.2.5 sous WSL, plugin Rust/WASM) : « Depuis un dossier… » copiait le dossier entier (2,5 Go avec target/ et tous les packs/), puis attendait un plugin.wasm à la racine alors qu'escouade-plugin build range le module dans packs/<id>-<version>.wasm, que « Charger un dossier… » prend déjà. Le refus ne disait pas quoi faire, et le journal nommait plugin.wasm même quand ce fichier n'existait pas.

## Décision

La copie d'un dossier laisse de côté .git/, target/, node_modules/ (où qu'ils soient) et ce qu'ignore Git quand le dossier est dans un dépôt (git ls-files, core.fsmonitor coupé ; hors dépôt, la liste fixe seule), et ne garde de packs/ que le module de la version du manifeste. Ce que le manifeste nomme (icône, bundle, module) reste, .git jamais ; un lien n'est jamais suivi. Un run.wasm copié prend packs/<id>-<version>.wasm, sinon plugin.wasm, comme un dossier en développement ; le module est dans la copie, donc sous l'empreinte montrée, et la fenêtre d'installation dit lequel tournera. Sans module, l'inspection refuse en disant de lancer escouade-plugin build ou de passer par En développement › Charger un dossier… (fr et en). Le journal nomme le fichier réellement chargé.

## Conséquences

Une installation depuis un dossier est petite et marche sur un plugin Rust construit, sans copier un module à la main. Un fichier ignoré par Git qui n'est pas nommé par le manifeste n'est plus installé : un plugin qui en dépendrait doit le nommer ou ne pas l'ignorer. L'aperçu d'installation gagne un champ module (Preview.module). Correctif 2.2.7, parti de main, à reverser dans release/2.3.
