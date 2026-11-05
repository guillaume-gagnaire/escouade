---
number: 79
id: "le-kit-rust-compile-en-wasm-le-tour-dans"
title: "Le kit Rust compile en WASM : le tour dans le protocole, la langue dans initialize, reqwest par le kit"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-334"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-334"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":7,"removed":0}]
accepted_at: "2026-11-05T05:15:09+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

A4 (ESC-334, docs/PLUGINS.md §8.10) fait compiler les crates partagées pour wasm32-wasip1 sans que les plugins changent de forme. Un module n'a ni environnement, ni process, ni tokio, ni reqwest. Le prototype d'ESC-330 s'en sortait par des copies de sources et par un faux crate nommé reqwest, ce qui ne tient pas sans fork.

## Décision

1. Le tour (`turn`) vit dans `escouade-plugin-protocol`, pas dans le kit : Codex et les harness n'ont besoin que du protocole. Le kit y ajoute l'exécuteur de ses tâches. Un plugin exporte son tour par une ligne, `module!(run)`, sans effet en natif.
2. `initialize` dit `lang` à tout plugin, tiers compris. C'est un champ de plus du protocole 1, optionnel à la lecture.
3. Un plugin prend son client par `escouade_plugin_kit::http::reqwest` : reqwest en natif, une façade sur `net/fetch` en wasm. Proxy, certificats et délai y sont acceptés sans effet : l'hôte les tient.
4. Un hôte qui refuse un minuteur le dit au journal et le `sleep` finit aussitôt.
5. Tant qu'un plugin est aussi une `rlib` d'`escouade-plugins`, son module se compile par `cargo rustc --crate-type cdylib` (`scripts/plugins-wasm.sh`), sans changer son manifeste.

## Conséquences

Porter un plugin coûte une ligne `use`, `module!(run)` et, pour un harness, `turn::run`. Les réponses du cœur passent par une attente commune aux deux boucles, à la place de `tokio::sync::oneshot`. `prototypes/wasm-all/prepare.mjs` ne marche plus : le prototype se refait depuis le commit de8eba05. Un `sleep` en module attend le côté hôte d'A2 ; `read_file` attend A3. Les flux d'OpenCode restent hors de la façade jusqu'à `net/stream`.
