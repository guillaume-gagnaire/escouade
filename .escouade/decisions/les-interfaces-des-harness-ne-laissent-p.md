---
number: null
id: "les-interfaces-des-harness-ne-laissent-p"
title: "Les interfaces des harness ne laissent passer que ce qu'elles tiennent : vue Copilot CLI sans écriture d'office, refusée sous clôture ; escouade-hook par son chemin entier"
date: "2026-11-12"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-430"
pages: ["plugins/copilot-cli-en-plugin","plugins/antigravity-en-plugin","plugins/harness-en-process"]
tickets: ["ESC-430"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/copilot-cli-en-plugin","added":11,"removed":2},{"page":"plugins/harness-en-process","added":5,"removed":0},{"page":"plugins/antigravity-en-plugin","added":7,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La relecture de Copilot CLI 2.2.5 et d'Antigravity CLI 2.2.7 (2026-11-12) a trouvé : en Édits auto et Auto, la vue Copilot permettait `--allow-tool=write`, qui couvre le `.git` du worktree (un hook puis un `git commit` sans question) ; elle se disait tenue en clôture (`fence_by_client`) et en lecture seule sans rien qui les tienne ; sous Windows, les hooks d'Antigravity, de Vibe et de Gemini appelaient `escouade-hook` par son nom via `cmd`, qui cherche d'abord dans son dossier courant. Le cœur de la 2.2 avait les mêmes défauts. L'aide de Copilot CLI 1.0.98 ne dit pas qu'un refus `write(<dossier>)` couvre les fichiers sous le dossier, et l'essai de `--secret-env-vars` montre que Copilot ne développe plus `${ESCOUADE_MCP_TOKEN}` quand cette variable y est.

## Décision

1) Vue Copilot CLI : aucune écriture sans question en Édits auto et Auto (choix de l'utilisateur, faute de pouvoir prouver la couverture d'un refus par chemin) ; `--deny-tool=write(…)` sur `.git`, `.copilot`, `.github/copilot`, `.github/hooks` et le dossier des fichiers joints dans tous les modes ; `--deny-tool=write` et `--deny-tool=shell` pour une discussion ; la vue d'un agent clôturé est refusée et le dit, plutôt que tenue (aucune sorte de règle de Copilot ne refuse une lecture). Le process envoie à la carte une écriture dans la configuration de Copilot comme dans `.git`. 2) Le jeton de la vue Copilot n'est que dans le fichier privé du cœur, qui le porte en clair ; `ESCOUADE_MCP_TOKEN` est dans `--secret-env-vars`. 3) Toutes les interfaces appellent `escouade-hook` par son chemin entier (sous Windows avec des `/`, sans guillemets ; par son nom seulement quand une ligne ne tient pas le chemin) et posent `NoDefaultCurrentDirectoryInExePath=1` sous Windows. 4) À la fin d'une interface, le cœur ôte le jeton des fichiers que son lancement a écrits.

## Conséquences

Dès qu'un projet privé existe, aucun agent n'a de vue Copilot CLI ; en Édits auto et Auto, chaque écriture y demande. escouade-acp passe en 0.1.5 : Gemini CLI et Mistral attendent sa publication sur crates.io (AWAITING_PUBLICATION) avant leurs verrous. Copilot CLI 2.2.6, Antigravity CLI 2.2.8, Gemini CLI et Mistral 2.2.6. Le même correctif est porté dans le cœur de la 2.2 (ticket/esc-430-2.2). À essayer en vrai : agy et copilot connectés sous Windows (hooks, refus, vue refusée).
