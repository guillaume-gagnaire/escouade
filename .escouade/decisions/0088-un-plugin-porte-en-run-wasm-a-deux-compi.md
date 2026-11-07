---
number: 88
id: "un-plugin-porte-en-run-wasm-a-deux-compi"
title: "Un plugin porté en run.wasm a deux compilations : un workspace plugins/ contre crates/, et son propre Cargo.lock contre crates.io"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-344"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/livres-avec-escouade","kanban/integrations"]
tickets: ["ESC-344"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":11,"removed":0},{"page":"plugins/livres-avec-escouade","added":4,"removed":0},{"page":"kanban/integrations","added":4,"removed":0}]
accepted_at: "2026-11-05T15:04:24+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le store compile le dossier d'un plugin seul. Un plugin porté ne peut donc plus porter workspace = "../../src-tauri", et il lui faut son propre Cargo.lock, qui fixe les crates publiées (point ouvert de docs/PLUGINS.md §8.16). Or un membre de workspace ne peut pas avoir de lock à lui, et le dépôt doit compiler ces plugins contre crates/ pour les tests de l'app et le build de développement. Passer le patch par --config aurait réécrit le lock du plugin ou interdit --locked.

## Décision

Les plugins portés quittent le workspace de src-tauri pour un second workspace, plugins/Cargo.toml, qui a le [patch.crates-io], le profil d'un module, son Cargo.lock, son rust-toolchain.toml (copie gardée égale par un test) et le target de src-tauri. Chaque plugin porté garde dans son dossier un Cargo.lock sur les crates publiées, écrit par scripts/plugin-lock.sh : cargo l'ignore dans le workspace, escouade-plugin build et le store le lisent dans le dossier seul. L'empreinte qui compte est celle du module compilé contre crates.io.

En attendant l'amorce (Z2), un plugin du registre en run.wasm se lance, en build de développement et en test seulement, depuis <target>/plugins-wasm/<id-court>/ (scripts/plugins-wasm.sh). Ces builds laissent aussi un plugin en WASM joindre un faux service par son adresse (Fetcher::fake, Bases::faked). Jira déclare {account}, pour joindre le site du compte comme il le faisait en process.

## Conséquences

Chaque ticket P ajoute son plugin aux members de plugins/Cargo.toml et lance plugin-lock.sh. Le dépôt teste le kit de crates/, le store compile le kit publié : un changement de crates/ n'atteint un plugin qu'après une publication et un nouveau lock. Une version de l'app sortie de main avant Z2 n'aurait pas Linear, Trello, Jira ni ClickUp, et release.yml ne l'arrête pas. La CI construit les modules avant les tests de l'app, et les garde dans l'archive nextest et l'artefact de l'app. L'installation de Jira montre une permission de plus (« Se connecte aux instances que tu lui donnes »).
