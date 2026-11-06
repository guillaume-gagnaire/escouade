---
number: 97
id: "acp-et-claude-code-en-module-ce-qu-un-ag"
title: "ACP et Claude Code en module : ce qu'un agent atteint par le client, et des limites d'instance déclarées"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-347"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/harness-acp-decrits"]
tickets: ["ESC-347"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":10,"removed":0},{"page":"plugins/harness-acp-decrits","added":7,"removed":0}]
accepted_at: "2026-11-06T11:24:44+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le portage de `dev.escouade.acp` et `dev.escouade.claude-code` en `run.wasm` (P3b) pose deux questions que la recette de P1 ne tranchait pas. Le client ACP natif ouvrait tout chemin absolu hors de la clôture, et un module n'a pas de disque : il passe par `fs/*`, limité à des racines déclarées. Et une instance sert tous les agents d'un harness : un message de 18 Mio de fichiers, 24 Mio en base64, envoyé puis renvoyé par `--replay-user-messages`, fait tomber un module à 64 Mio de mémoire, essayé, et prend 1,1 s sur les 2 s d'un tour.

## Décision

1. Par le client (`fs/read_text_file`, `fs/write_text_file`), un agent ACP lit dans `agent` et `agentRepo` et écrit dans `agent`, 4 Mio par fichier. `agentRepo` reste en lecture : rien n'écrit dans le dépôt principal depuis un worktree. Hors de ces racines, le refus est un `-32000`.
2. Le plugin Claude Code ne déclare aucune permission. Ce que Claude Code garde d'une connexion n'est lu que par le cœur (`src-tauri/src/engine/claude/signin.rs`), sans passer par `escouade.call` ni `fs/*` : le plugin ne l'a jamais lu pour lui-même.
3. Un plugin en WASM peut demander `limits.memoryBytes` (256 Mio au plus) et `limits.turnSeconds` (10 s au plus), jamais moins que le défaut de l'hôte. Un tour qui tient une décision garde ses 500 ms. Ce n'est pas une permission : la fenêtre d'installation ne le dit pas.

## Conséquences

Un harness décrit qui lisait hors de son dossier par Escouade, par exemple un dossier temporaire, reçoit un refus et doit lire lui-même : Goose n'est pas touché (`"fs": false`), et le CHANGELOG le dit. Un fichier de plus de 4 Mio ne passe plus par le client.
`escouade-acp` passe en `0.1.1`, qui doit être publiée avant que le plugin ACP compile seul dans son dossier.
Une instance qui a monté en mémoire n'en rend rien avant son arrêt : le pic d'un gros PDF reste. Un tour bloqué peut tenir 10 s un harness qui a demandé ce plafond, sans retenir l'agent d'une décision.
Les autres harness portés auront le même besoin de limites.
