---
number: null
id: "un-compte-partage-protege-les-sessions-d"
title: "Un compte partagé protège les sessions du projet privé ; la réservation est une isolation de plus"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-556"
pages: ["harness/comptes","projets/projets-prives"]
tickets: ["ESC-556"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":28,"removed":10},{"page":"projets/projets-prives","added":18,"removed":5}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Support Discord du 2026-11-20 : un utilisateur a passé un projet en privé, puis a cliqué « Réserver « Firmin.ai » à ce projet », proposé en couleur d'attente comme le remède à un compte partagé. C'était son seul compte Claude Code : ses autres projets n'en avaient plus, sans qu'on le lui dise. Il jugeait ça « un peu trop hardcore niveau sécurité ». Or la clôture des fichiers (ESC-140) refuse déjà aux agents des autres projets les dossiers de sessions du projet privé sur chaque compte : un compte partagé n'est pas une faute.

## Décision

1) Sur un compte partagé, le niveau de protection d'un projet privé est celui de sa clôture : les agents des autres projets se voient refuser ses dossiers de sessions rangés par le harness (Claude Code : `projects/<chemin encodé>`, mémoire de projet et worktrees compris, un nom de plus de 200 caractères reconnu comme Claude Code le coupe), et ce qu'un harness range par session ou par date (file-history, todos, shell-snapshots de Claude Code ; sessions de Codex ; session-state de Copilot CLI ; storage d'OpenCode) leur est refusé entier. Restent communs, et dits : la connexion, les réglages, la mémoire globale, l'historique des prompts que tient le harness ; Cursor CLI, Mistral Vibe, Antigravity CLI et Gemini CLI n'ont en commun que la connexion ou la clé et le quota (un dossier par agent). Aucun fichier d'identifiants n'est lu. 2) La rangée « Comptes » de la Confidentialité dit le partage comme une information, sans couleur d'attente, et présente la réservation comme une isolation de plus, facultative. 3) Une réservation qui laisserait des projets ouverts sans compte actif de son harness demande d'abord, partout où l'on réserve, en disant ce qui se passera et en proposant « Ajouter un compte réservé… ». 4) Défaire est simple : un projet sans compte parce qu'un compte est réservé ailleurs nomme ce compte et son projet, avec « Partager à nouveau » (conversation, toast, Kanban, Réglages › Comptes et harness) ; un compte réservé a « Ne plus le réserver » sur la page du projet privé.

## Conséquences

Personne ne perd plus son seul compte en un clic ; un projet privé sur un compte partagé reste protégé au niveau de la clôture (garde-fou ou barrière selon le harness), ce qui reste commun étant dit. Le refus « aucun compte ne sert le projet » porte désormais l'id du projet (`no-account:<harness>@<projet>|…`). Le rendu de référence design/app-redesign (#private) montre la rangée neutre. Ce qu'un harness tient lui-même pour tous les projets (historique des prompts rappelé par Claude Code, mémoire globale) reste partagé : seule la réservation le sépare.
