---
number: 219
id: "le-cache-des-compilations-rust-des-plugi"
title: "Le cache des compilations Rust des plugins est borné : 7 jours, 5 Go"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-523"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-523"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":8,"removed":0}]
accepted_at: "2026-11-19T16:32:15+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Chaque compilation d'un plugin run.wasm (build, test, dev, fetch, recompilation d'une revue, tests du dépôt) posait une copie du dossier du plugin avec son target sous <données>/cache/plugins-rust/, et rien ne la retirait : 926 copies, 83 Go en onze jours sur le poste du chef (2026-11-17). Les tests du dépôt téléchargeaient aussi la chaîne d'outils dans le vrai ~/.escouade.

## Décision

À la fin de chaque compilation, les copies qui n'ont pas servi depuis 7 jours partent, puis les moins récemment utilisées au-delà de 5 Go. ESCOUADE_RUST_CACHE_DAYS et ESCOUADE_RUST_CACHE_GB changent ces bornes. Une copie tenue par une compilation n'est jamais retirée : elle est prise sous son verrou sans attendre, puis renommée avant d'être supprimée. La chaîne d'outils d'une autre version de Rust est retirée quand la version change. cargo-home reste borné par le ramasse-miettes de cargo. `escouade-plugin cache` dit où est le cache et ce qu'il pèse, et `cache clear` le vide. Sous nextest, un test sans ESCOUADE_DATA_DIR prend un dossier partagé dans le dossier de cache du système (escouade-test-data), jamais ~/.escouade. La fenêtre n'a pas d'écran pour cela.

## Conséquences

Le disque d'un auteur de plugin ne se remplit plus sans qu'il le sache. Une copie retirée coûte une compilation complète la fois suivante. Un poste qui lance les tests télécharge une fois la chaîne d'outils (654 Mo) dans escouade-test-data, sous le dossier de cache du système (%LOCALAPPDATA%, ~/.cache, ~/Library/Caches). Le dossier temporaire ne convient pas : un process de test l'a à lui et le vide après lui. `cargo test` sans nextest n'a pas ce filet. Le store n'a rien à régler.
