---
number: 83
id: "le-kit-rust-sort-sur-crates-io-en-0-1-0"
title: "Le kit Rust sort sur crates.io en 0.1.0 sous MIT, les plugins en dépendent par version"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-341"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-341"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":6,"removed":0}]
accepted_at: "2026-11-05T09:07:14+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le store ne lit que le dossier d'un plugin : un plugin Rust ne peut pas dépendre de `crates/` par un chemin (docs/PLUGINS.md §8.6.3). Les trois crates étaient en 1.0.0 et en GPL-3.0-or-later, alors que la décision 4 du 2026-10-25 met le SDK et le schéma sous MIT. Une dizaine de lignes de `forge.rs` viennent de la PR #17 de Florian Gagnaire, et le dépôt n'a pas de CLA. Une version publiée ne s'efface pas.

## Décision

`escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` sortent en `0.1.0`, sous licence MIT, choisie par l'utilisateur le 2026-11-05. Son accord est demandé à Florian Gagnaire par l'issue #22 (https://github.com/guillaume-gagnaire/escouade/issues/22) : le chef ne publie qu'après son « oui » en commentaire de #22. Les plugins de la première main restent en GPL-3.0-or-later et dépendent des crates par version ; `[patch.crates-io]` dans `src-tauri/Cargo.toml` les ramène sur `crates/`. Entre elles, les crates gardent `path` et `version`, car l'empaquetage n'est pas atteint par le patch et `cargo publish` retire le `path`. Le cœur, qui n'est pas publié, garde ses `path`.

## Conséquences

Un auteur tiers en Rust garde le choix de sa licence, et son `run.wasm` n'est pas tiré vers la GPL. Un changement cassant du kit passe à `0.2.0`, avec l'exigence des plugins, dans le même commit, sinon cargo laisse le patch de côté. La publication attend le « oui » de Florian Gagnaire en commentaire de #22. Avant que le store compile un plugin de la première main, il reste à lui retirer `workspace = "../../src-tauri"` et à lui donner son `Cargo.lock`, ce qui demande les crates publiées (docs/PLUGINS.md §8.16).
