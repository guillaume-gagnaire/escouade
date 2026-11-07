---
number: 90
id: "la-documentation-des-plugins-ne-montre-q"
title: "La documentation des plugins ne montre que du code compilé, et le modèle Rust dépend du kit comme un tiers"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-342"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-342"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":7,"removed":0}]
accepted_at: "2026-11-05T15:32:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur a demandé le 2026-11-04 de publier la documentation pour permettre aux développeurs externes de créer des plugins en TS ou en Rust. Chaque appel de l'hôte devait avoir un exemple dans les deux langages, et chaque exemple devait compiler ou passer validate. Copiés à la main dans les pages, ces exemples auraient dérivé : les pages de `plugins/` l'ont déjà montré (ESC-183). Un modèle écrit dans le dépôt pouvait aussi dépendre du kit par `path`, et ne rien prouver de ce qu'un tiers obtient.

## Décision

Une page de `plugins/` ne porte aucun bloc de TypeScript ou de Rust écrit à la main : un bloc est fait de lignes d'un fichier compilé, cité sous son chemin (un modèle, un plugin de `plugins/`, ou les deux plugins de `sdk/doc-examples`, qui ne sont pas publiés). `npm create escouade-plugin --kind rust` écrit un plugin qui dépend de `escouade-plugin-kit` par version, depuis crates.io, avec son propre `Cargo.lock` et un `[workspace]` à lui. La documentation décrit le système final : `run.command` refusé au store, la revue qui recompile un `run.wasm`.

## Conséquences

Changer une ligne d'un exemple oblige à la recopier : le test dit laquelle. Le job de CI des modèles télécharge les crates de crates.io et la chaîne d'outils de Rust. Un `cargo update` peut sortir le `Cargo.lock` du modèle de la liste fermée : un test le tient. `create-escouade-plugin` passe en 0.4.0, mise en attente sur npm à la fusion sur main, à approuver par le chef avant que `--kind rust` existe pour les auteurs. Tant que la revue d'un `run.wasm` (A7) et le réglage du bac à sable en production ne sont pas faits, la page de publication est en avance sur le service.
