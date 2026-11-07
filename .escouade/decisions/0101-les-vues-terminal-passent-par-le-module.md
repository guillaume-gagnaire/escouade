---
number: 101
id: "les-vues-terminal-passent-par-le-module"
title: "Les vues terminal passent par le module de leur harness, dans une instance à part"
date: "2026-11-06"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-352"
pages: ["plugins/harness-en-process","plugins/tous-les-plugins-en-wasm","plugins/codex-en-plugin","plugins/copilot-cli-en-plugin","plugins/antigravity-en-plugin","plugins/claude-code-en-plugin","plugins/opencode-en-plugin"]
tickets: ["ESC-352"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-en-process","added":9,"removed":0},{"page":"plugins/tous-les-plugins-en-wasm","added":3,"removed":0},{"page":"plugins/codex-en-plugin","added":1,"removed":1},{"page":"plugins/copilot-cli-en-plugin","added":1,"removed":1},{"page":"plugins/antigravity-en-plugin","added":1,"removed":1},{"page":"plugins/claude-code-en-plugin","added":2,"removed":2},{"page":"plugins/opencode-en-plugin","added":1,"removed":1}]
accepted_at: "2026-11-07T01:44:06+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le cœur liait les crates des plugins de harness pour traduire ses « Vue X » : un correctif publié au store ne corrigeait pas la vue, restée à la version de l'app. L'étude ESC-330 (docs/PLUGINS.md §8.2) a retenu une méthode du contrat, `view/translate`, le cœur lisant le fichier de la session et le plugin le traduisant. Restait sa forme : les vues lisent chaque ligne sous le verrou de l'agent, chacune voyant ce que la précédente a fait de lui, et une fin de tour se fait sous ce même verrou, depuis partout. Un appel asynchrone au runtime aurait demandé de refaire tout le code des vues, et le fil du module sert aussi `fs/*`, qui peut vouloir ce verrou. Par ailleurs le lancement des interfaces (`terminal/launch`) n'est pas servi : le cœur a encore besoin de mots que seules ces crates tenaient.

## Décision

Le cœur garde le PTY, le fichier de l'interface, ses hooks et l'état de l'agent ; le plugin dit ce que les lignes veulent dire, par `view/translate`, une ligne à la fois, ce que seul le cœur sait passant par des notes (`end`, `hook`, `frame`). Les vues tournent dans une instance à part du même module, sur son fil, sans `escouade.call`, le cœur attendant sa réponse par le même tour. Un module qui ne répond pas est dit une fois ; ses lignes passent ensuite sans lui 30 s, puis un seul essai. `src-tauri/Cargo.toml` ne dépend plus d'aucun dossier de `plugins/`. Le cœur garde sa copie des mots dont il a besoin pour lancer les interfaces et lire un compte, tant que `terminal/launch` n'est pas servi.

## Conséquences

Une correction publiée au store corrige la vue. Plugin coupé ou désinstallé, le terminal marche mais la conversation ne le suit plus, et le dit une fois : avant, la vue marchait plugin coupé. Les lignes passées pendant une panne ne sont pas relues, et un tour en cours se ferme sans son plugin. Un plugin a une instance de plus, sur son propre fil, quand une vue tourne. `escouade-plugin-protocol` 0.1.1 et `escouade-acp` 0.1.2 sont publiées : Antigravity, Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI et Mistral compilent depuis crates.io, avec de nouvelles empreintes, comme OpenCode, redevenu un `cdylib` seul. Une règle du lancement de l'interface corrigée au store ne corrige pas encore la copie du cœur.
