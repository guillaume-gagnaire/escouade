---
number: 73
id: "run-wasm-le-rust-des-plugins-compile-par"
title: "run.wasm : le Rust des plugins compilé par le store en wasm32-wasip1, reproductible"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-330"
pages: []
tickets: ["ESC-330"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-04T21:57:33+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte



## Décision



## Conséquences

Un module coûte 0,46 à 0,64 Mo, 12 s de compilation à froid, 50 à 93 ms de précompilation. Le §5.6 et le §5.7 de docs/PLUGINS.md sont revus. L'image du store grossit d'environ 680 Mo, et la limite de sa compilation monte à 2 Gio et 5 minutes par plugin. macOS n'a pas été essayé : un job de CI doit le voir. Changer de version de rustc change toutes les empreintes : tous les modules sont à recompiler et à resigner. Un `run.wasm` n'est ouvert qu'à Rust au début.
