---
number: 57
id: "mistral-vibe-passe-par-son-plugin-en-pro"
title: "Mistral Vibe passe par son plugin, en process"
date: "2026-11-02"
state: "superseded"
origin: "ticket"
origin_ref: "ESC-281"
pages: ["plugins/mistral-vibe-en-plugin","plugins/livres-avec-escouade","plugins/harness-acp-decrits","plugins/cursor-cli-en-plugin"]
tickets: ["ESC-281"]
documents: []
supersedes: []
superseded_by: 62
changes: [{"page":"plugins/mistral-vibe-en-plugin","added":21,"removed":0},{"page":"plugins/livres-avec-escouade","added":1,"removed":0},{"page":"plugins/harness-acp-decrits","added":4,"removed":0},{"page":"plugins/cursor-cli-en-plugin","added":2,"removed":0}]
accepted_at: "2026-11-02T10:01:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-10.6 (ESC-281) sort Mistral Vibe du cœur, après Cursor CLI, sur le client ACP générique de PLG-6. docs/PLUGINS.md §4.13 prévoyait que Vibe entre dans le plugin « Mistral » de la voix avec un compte partagé ; le ticket demande de garder ses comptes par clé tels quels.

## Décision

La conversation des agents de Mistral Vibe passe par le plugin de première main dev.escouade.mistral-vibe (escouade-plugins vibe, extension vibe du client de crates/escouade-acp). Le moteur Vibe reste au cœur pour les comptes par clé, les modèles d'un compte, vibe -p, la vue terminal et l'écriture de son VIBE_HOME (confiance, denylist d'un projet privé). Les harness livrés se branchent par une table (BUILTIN_HARNESSES : plugin, native, prepare). Le contrat gagne l'élément session. Le regroupement dans dev.escouade.mistral et le compte partagé avec la voix se font dans un ticket à part avant la release 2.2 (choix du chef du 2026-11-02).

## Conséquences

Rien ne change pour l'utilisateur, sauf qu'en « Accepter les modifications » un git add/commit du worktree demande désormais (comme pour Cursor depuis INT-10.1). Coupé, un agent de Vibe le dit au lancement et garde sa session. Gemini CLI n'ajoutera qu'une ligne à la table. L'identifiant dev.escouade.mistral-vibe ne doit sortir dans aucune version publiée.
