---
number: 38
id: "escouade-plugin-joue-et-compile-les-plug"
title: "escouade-plugin joue et compile les plugins run.js en WASM, et vérifie sa Javy à chaque usage"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-322"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-322"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":15,"removed":7}]
accepted_at: "2026-11-01T19:39:47+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

WASM-1 et WASM-2 font tourner les plugins `run.js` dans l'app, mais `escouade-plugin` (le banc des auteurs, livré avec l'app) les jouait encore par Node, et rien ne permettait de compiler un bundle comme le fera le store. La Javy téléchargée n'était vérifiée qu'une fois, sur son archive : un programme changé ensuite sur le disque aurait été lancé.

## Décision

1. `escouade-plugin test` et `dev` jouent un plugin `run.js` en WASM, dans l'hôte de l'app (mêmes scénarios qu'un process ; l'instance finie après sa réponse à `shutdown`, comme dans l'app), sans Node. Javy et le cache des modules sont ceux du dossier de données d'Escouade.
2. `escouade-plugin build <dossier> [--out] [--json]` écrit `packs/<id>-<version>.wasm` et son `.sha256`, et en JSON le compilateur (javy, engine = empreinte d'escouade-js, prelude) : ce que le store et la revue appelleront.
3. `validate` refuse un bundle qui ne peut pas tourner en WASM ; le refus (un `import` autre que `node:fs`/`node:readline`, un `export`) est dit en français ou en anglais.
4. La Javy : l'archive vérifiée par le SHA-256 noté dans le code (refusée sinon), puis le programme vérifié à chaque usage contre l'empreinte notée à côté de lui ; une Javy changée n'est jamais lancée et est téléchargée à nouveau.
5. `escouade-plugin --version` dit Javy, escouade-js et le prélude, et sort en erreur sans escouade-js : la release le vérifie sur chaque système.
6. `javy-codegen` est encore en 4.0.0 : la CLI de Javy reste.

## Conséquences

Un auteur sans Node teste et compile un plugin JS/TS ; le store (WASM-4) et la revue locale (WASM-5) appelleront `escouade-plugin build`. Mesuré : la même empreinte deux fois sous Windows et sous Linux (WSL), avec le même escouade-js. Une vérification de ~30 Mo de Javy à chaque compilation (quelques dizaines de ms). Les installations qui avaient déjà la Javy de WASM-1 la téléchargent une fois de plus (pas d'empreinte notée).
