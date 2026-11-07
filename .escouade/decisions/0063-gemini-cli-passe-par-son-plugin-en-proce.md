---
number: 63
id: "gemini-cli-passe-par-son-plugin-en-proce"
title: "Gemini CLI passe par son plugin, en process"
date: "2026-11-03"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-282"
pages: ["plugins/gemini-cli-en-plugin","plugins/harness-acp-decrits","plugins/livres-avec-escouade","plugins/cursor-cli-en-plugin"]
tickets: ["ESC-282"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/gemini-cli-en-plugin","added":22,"removed":0},{"page":"plugins/harness-acp-decrits","added":3,"removed":0},{"page":"plugins/livres-avec-escouade","added":1,"removed":0},{"page":"plugins/cursor-cli-en-plugin","added":2,"removed":0}]
accepted_at: "2026-11-04T04:50:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-10.7 (ESC-282) sort Gemini CLI du cœur, après Cursor CLI et Mistral Vibe, sur le client ACP générique. Il lui faut des réglages et des règles écrits par le cœur avant le lancement et un argument de plus, --policy, que la table des harness livrés ne savait pas ajouter.

## Décision

La conversation des agents de Gemini CLI passe par le plugin de première main dev.escouade.gemini-cli (escouade-plugins gemini, extension gemini du client de crates/escouade-acp). Le moteur Gemini reste au cœur pour les comptes, les modèles d'un compte, gemini -p, la vue terminal par hooks et l'écriture de son GEMINI_CLI_HOME. La table BUILTIN_HARNESSES grandit pour tous : Prepare reçoit le dossier de l'agent, son dossier de travail, la clôture, la lecture seule et les variables, et répond les arguments ajoutés au lancement. Le client d'avant du cœur (engine/gemini/process.rs) est retiré ; les modèles d'un compte se lisent par un petit échange ACP jetable.

## Conséquences

Rien ne change pour l'utilisateur, sauf qu'en « Accepter les modifications » un git add/commit du worktree demande désormais (comme pour Cursor et Vibe). Coupé, un agent de Gemini le dit au lancement et garde sa session. Pour tout harness questionsInText, un bloc qui n'est pas une question est montré et l'arrêt d'un tour tenu par une question le finit. real_gemini reste écrit et ignoré, faute de compte. OpenCode n'ajoutera qu'une ligne à la table.
