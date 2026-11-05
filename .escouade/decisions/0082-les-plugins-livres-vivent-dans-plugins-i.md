---
number: 82
id: "les-plugins-livres-vivent-dans-plugins-i"
title: "Les plugins livrés vivent dans plugins/<id-court>/, et ci.mjs les laisse de côté jusqu'à M1"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-339"
pages: ["plugins/livres-avec-escouade","plugins/vue-d-ensemble"]
tickets: ["ESC-339"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":1,"removed":1},{"page":"plugins/vue-d-ensemble","added":1,"removed":1}]
accepted_at: "2026-11-05T07:41:48+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 70 veut chaque plugin de première main dans plugins/<id-court>/. ESC-339 (M0) fait ce déménagement sans changer de code. Une fois dans plugins/, ces dossiers tombent sous ce qui parcourt ce dossier (plugins/ci.mjs, le test du site), alors qu'ils n'ont pas encore de run.wasm et que leur programme reste escouade-plugins.

## Décision

Le registre des plugins livrés est lu dans les dossiers de plugins/ dont le programme est {escouade}/escouade-plugins, plus la démonstration, qui devient la fixture tests/fixtures/plugins/minimal/. Chacun porte une LICENSE, copie de celle de la racine (GPL-3.0-or-later), et un CHANGELOG.md. plugins/ci.mjs et le test de la page des exemples écartent ces dossiers tant que ce programme est le leur : leurs tests restent ceux de l'app, jusqu'à M1.

## Conséquences

Rien ne change à l'usage : mêmes identifiants, même code ; seuls les liens vers le code depuis le store et les manifestes changent d'adresse. Les liens tree/main/plugins/builtin/… des versions déjà publiées d'Escouade ne répondent plus. M1 retire le filtre de ci.mjs, et l'image du store copie désormais tout plugins/ pour ses manifestes Cargo.
