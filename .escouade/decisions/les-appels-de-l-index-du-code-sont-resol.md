---
title: "Les appels de l'index du code sont résolus comme tokensave, d'un symbole à un symbole"
date: "2026-10-30"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-297"
pages: ["docs/index-du-code"]
tickets: ["ESC-297","ESC-294"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/index-du-code","added":20,"removed":9}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Les sondes d'ESC-294 ont montré que index_callers répondait par le nom seul : « qui appelle Runtime::forget » donnait les appels des 15 forget du projet (7 justes sur 29), path ne servant qu'aux définitions. L'utilisateur a demandé le 2026-10-29 d'implémenter « EXACTEMENT comme tokensave ».

## Décision

La résolution des appels de tokensave (commit 580936b, MIT) est portée : références nommées comme ses extracteurs, receveurs typés (Type::method), stratégies dans son ordre, score_candidate, égalités rangées dans une table des appels ambigus, seuil de 0,6, noms trop courants bloqués entre fichiers, preuve d'import pour Python/JS/TS, relecture partielle par l'ensemble touché, limite des fichiers à 1 Mio. Les liens vont d'un id à un id. index_callers, index_callees et index_impact prennent un id ou un nom répondu définition par définition (max_matches, path), les ambigus à part. Écarts (écrits dans docs/CODEINDEX-PROBES.md) : le delta des worktrees, les receveurs par le type d'un champ en Rust, la suppression des frères d'un appel typé pour tous les langages, les composants Svelte appelables, les stratégies des langages non lus et les receveurs typés de C# non portés, les imports non résolus en liens.

## Conséquences

La base de l'index change de schéma (2) : elle se refait à la première lecture. Les réponses des trois outils changent de forme (matches, more, ambiguous ; les symboles portent leur id). Licence MIT de tokensave dans THIRD-PARTY-NOTICES.md et CREDITS.md.
