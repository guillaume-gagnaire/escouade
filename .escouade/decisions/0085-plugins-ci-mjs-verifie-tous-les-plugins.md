---
number: 85
id: "plugins-ci-mjs-verifie-tous-les-plugins"
title: "plugins/ci.mjs vérifie tous les plugins, les crates compris, et --changed passe aux pull requests"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-340"
pages: ["plugins/plugins-d-exemple"]
tickets: ["ESC-340"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/plugins-d-exemple","added":3,"removed":0}]
accepted_at: "2026-11-05T09:30:30+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

M0 (ESC-339) a mis les plugins livrés dans plugins/<court>/, mais ci.mjs les sautait. Leurs manifestes disent 0.0.0 (la version vient de l'app, par manifest_text) et leurs crates 1.0.0 : une comparaison stricte serait rouge sur tous, alors que §8.6.1 prévoit des versions propres seulement plus tard. Un plugin servi par escouade-plugins n'a pas non plus de banc possible.

## Décision

ci.mjs vérifie tous les plugins de plugins/ : LICENSE, CHANGELOG, validate (--builtin pour ceux de l'app), et pour un crate cargo test et sa version face au manifeste. Le 0.0.0 n'est admis que pour un plugin au programme de l'app ; dès run.wasm, l'égalité est stricte. Le banc ne joue que run.js et run.wasm. --changed prend les dossiers touchés et leurs dépendants par path ; tout passe quand bouge ce qui juge ou construit les plugins, ou quand git ne sait pas comparer. La CI Linux lance tout sur main et --changed sur une pull request ; le job Windows ne le lance pas.

## Conséquences

La comparaison des versions n'attrape rien tant qu'aucun plugin n'est en run.wasm : chaque portage doit donner sa version au manifeste et au crate. Le job Linux rejoue les tests des crates déjà passés par nextest. Le job reproductible de §8.6.4 reste à poser quand un plugin sera en run.wasm.
