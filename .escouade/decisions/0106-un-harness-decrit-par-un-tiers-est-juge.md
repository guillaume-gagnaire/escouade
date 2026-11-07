---
number: 106
id: "un-harness-decrit-par-un-tiers-est-juge"
title: "Un harness décrit par un tiers est jugé sur son auteur, pas sur le plugin qui le sert"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-361"
pages: ["plugins/harness-acp-decrits"]
tickets: ["ESC-361"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":7,"removed":0}]
accepted_at: "2026-11-07T06:24:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un harness décrit (`contributes.harnesses[].acp`) tourne sur `dev.escouade.acp` : le cœur jugeait ce plugin de l'app, pas l'auteur du manifeste. Un manifeste tiers pouvait donc se dire clôturé (`fenceOnPermissions`), poser `NODE_OPTIONS`, `LD_PRELOAD` ou `PATH` que `proc/*` refuse à un plugin à module, nommer un chemin dans `locate.names`, et faire tourner des `versionArgs` que l'installation ne montrait pas.

## Décision

Une seule règle de première main : un id `dev.escouade.*` (`builtin::first_hand`), réservé par le manifeste et installé signé. Pour un auteur qui ne l'est pas : `fenceOnPermissions` est effacé dans `ProcessEngine::described` ; ses variables (`env`, `accountVar`, `keyVar`, et le lancement d'un harness tiers à module) sont jugées par la liste de `proc/*`, et l'agent ne démarre pas en nommant la variable ; `locate.names` ne garde que des noms nus. Pour tout harness décrit, l'installation montre chaque nom du programme, ses variables, et `versionArgs` s'il n'est pas `--version`. `check_launch` continue de juger le plugin qui sert : la clôture des `fs/*` est tenue par le code de l'app, y compris pour un tiers en `"fs": true`.

## Conséquences

Un tiers en `"fs": false` n'entre plus dans un projet privé ; un tiers en `"fs": true` y entre toujours. Une variable refusée ne se voit qu'au lancement, tant que `escouade-plugin check` et le store ne la jugent pas. La première main ne change pas : Goose, Cursor CLI, Gemini CLI et Mistral Vibe gardent leur parole, et peuvent gagner des lignes `harness:` à l'affichage. La liste de `proc/*` reste courte (`PYTHONPATH`, `RUBYOPT` et `JAVA_TOOL_OPTIONS` passent) : l'élargir vaudra pour les deux.
