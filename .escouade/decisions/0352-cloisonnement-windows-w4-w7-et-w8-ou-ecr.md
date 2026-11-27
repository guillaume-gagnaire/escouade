---
number: 352
id: "cloisonnement-windows-w4-w7-et-w8-ou-ecr"
title: "Cloisonnement Windows, W4, W7 et W8 : où écrit l'agent, les terminaux de l'utilisateur hors de la barrière, les ACE dites une fois"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-591"
pages: ["projets/cloisonnement-windows"]
tickets: ["ESC-591"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/cloisonnement-windows","added":16,"removed":20}]
accepted_at: "2026-11-27T23:58:34+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-591 (docs/WINDOWS-ISOLATION.md, § 5.4 et § 6). Sous la barrière (W1, voie B), un agent n'écrit que là où l'étiquette « bas » est posée ; il faut dire où, si les terminaux que l'utilisateur ouvre lui-même y passent, et comment l'utilisateur apprend qu'Escouade pose des règles sur ses dossiers.

## Décision

Choisi par l'utilisateur le 2026-11-27 (demande a-44, option a, « les recommandations »). W4 : l'agent écrit dans son dossier, ses worktrees, un dossier temporaire à lui (TEMP, TMP) et le dossier du compte de son harness, étiquetés « bas », plus des caches à lui par variables (npm_config_cache, DISABLE_AUTOUPDATER pour Claude Code) ; le reste lui est refusé, et sa conversation le dit. W7 : les terminaux que l'utilisateur ouvre lui-même restent hors de la barrière. W8 : les règles posées sur les dossiers sont dites une fois, à l'allumage de « Projet privé », sans accord à part. Pourquoi : étiqueter « bas » les caches de l'utilisateur les ouvrirait à tout processus de faible intégrité du poste ; l'utilisateur n'est pas la menace ; l'allumage du réglage est déjà le geste.

## Conséquences

Un outil qui écrit ailleurs que dans ces dossiers échoue dans un agent (« Accès refusé ») ; WB-4 pose les variables, WB-6 les textes.
