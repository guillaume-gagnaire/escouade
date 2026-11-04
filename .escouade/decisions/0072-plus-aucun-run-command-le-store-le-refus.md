---
number: 72
id: "plus-aucun-run-command-le-store-le-refus"
title: "Plus aucun run.command : le store le refuse, l'app ne le lance plus"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-330"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-330"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-04T21:57:33+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le §5.7 de docs/PLUGINS.md gardait les binaires tiers attestés. La décision de l'utilisateur du 2026-11-04 veut que tous les plugins soient compilés en WASM et qu'il ne reste aucun programme natif par système. Aucun binaire tiers n'est publié au store aujourd'hui : rien n'est cassé.

## Décision

1. Le store refuse tout `run.command`, `run.windows`, `run.macos` et `run.linux` à la soumission, avec la raison. Le contrôle n° 10 perd ses binaires par système et ses attestations ; `gh attestation verify` sort de la revue.
2. L'app ne lance plus aucun `run.command`, quelle que soit l'origine, dossier local et développement compris. Le manifeste reste lisible, et la fiche dit pourquoi le plugin ne démarre pas.
3. Un plugin qui a besoin d'un programme le nomme par `permissions.commands` (`proc/*`) : c'est le programme de l'utilisateur, lancé par le cœur, pas un binaire livré par le plugin.
4. Les harness décrits (Goose) ne changent pas : le cœur lance le CLI, `dev.escouade.acp` le sert.
5. Les versions publiées qui se lancent par Node (`slack` 1.1.0, `team-rules` 1.1.1) restent pour les apps d'avant, bornées par `escouade.max`.
6. Le transport par process de `runtime.rs` part en dernier, avec `escouade-plugins`.

## Conséquences

Un auteur en Go ou en C n'a pas de voie en 2.2 : `run.wasm` est Rust au début. Le faux plugin des tests (`tests/fixtures/fake-plugin.mjs`) et le banc par process sont à reprendre. Le §5.7 est remplacé.
