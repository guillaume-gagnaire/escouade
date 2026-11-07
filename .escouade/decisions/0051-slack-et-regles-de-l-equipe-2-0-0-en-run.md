---
number: 51
id: "slack-et-regles-de-l-equipe-2-0-0-en-run"
title: "Slack et Règles de l'équipe 2.0.0 en run.js ; la version qui sert le WASM se dit par escouade-plugin"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-326"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-326"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":5,"removed":0}]
accepted_at: "2026-11-02T04:55:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les deux plugins de première main étaient lancés par Node. WASM-1 à 5 sont livrés ; il reste à migrer ces deux plugins. Leur escouade.min doit être la version qui sert le WASM (2.2.0), mais l'app n'est bumpée qu'au commit de release : la CI de main aurait échoué à validate.

## Décision

slack et team-rules passent en 2.0.0 sur run.js (scripts Node retirés, escouade.min 2.2.0), code et dist inchangés. escouade-plugin gagne --escouade <version> (validate, test) et dit wasm-since dans --version ; plugins/ci.mjs juge les plugins du dépôt pour cette version, lue de l'outil et non écrite en dur. Étiquettes, soumission, revue et signature restent le geste du chef.

## Conséquences

Plus de Node chez l'utilisateur pour ces deux plugins, et permissions.network appliqué. Les 1.x restent au store pour les apps d'avant. Les fixtures du store qui représentent des plugins à process gardent l'ancienne forme. Les tests qui jouent les plugins réels avec l'app de la version courante demandent la version SERVED_SINCE ou l'adaptent.</consequences>
<parameter name="origin">ticket
