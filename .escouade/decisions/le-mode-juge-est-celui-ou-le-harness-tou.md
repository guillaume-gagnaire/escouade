---
number: null
id: "le-mode-juge-est-celui-ou-le-harness-tou"
title: "Le mode jugé est celui où le harness tourne vraiment"
date: "2026-11-16"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-489"
pages: ["harness/comptes","plugins/harness-acp-decrits"]
tickets: ["ESC-489","ESC-484","ESC-425"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":12,"removed":0},{"page":"plugins/harness-acp-decrits","added":7,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-484 a montré qu'un harness décrit par ACP qui ne déclare qu'un mode, Bypass (Unharnessed), démarrait quand même sans « Autoriser le mode Bypass » : ses nouveaux agents recevaient « Demander », un mode qu'il n'a pas, et le cœur ne jugeait que le mode de l'agent. Le harness tournait donc dans son seul mode réel, Bypass, sous l'étiquette « Demander ». Il fallait poser ce qu'Escouade fait d'un mode que le harness n'a pas, et d'un manifeste sans mode.

## Décision

Le mode jugé est celui où le harness tourne vraiment, sur tous les chemins (nouvel agent, outil create_agent, ticket et pilote auto, start_ticket, démarrage et reprise, reprise programmée, changement de mode ou de compte, passation, discussion, mode du Kanban de .escouade.yml). Le mode demandé s'il l'a. Un harness à un seul mode tourne dans ce mode : il devient celui de l'agent, montré et jugé (à un seul mode Bypass, il ne démarre pas sans « Autoriser le mode Bypass », et le refus dit d'autoriser le mode ou de choisir un autre harness). Plusieurs modes sans celui demandé : le plus proche qui n'autorise pas plus, dit dans la conversation ; aucun d'aussi prudent : refusé. Un manifeste qui ne déclare aucun mode qu'Escouade connaisse est tenu pour Bypass, et son installation le dit. La passation garde la règle du plus proche qui n'autorise jamais plus et refuse un Bypass non permis. Un agent du Kanban ou d'un outil est refusé avant que rien ne soit fait (le ticket se bloque avec la raison) ; celui de la fenêtre est créé avec son vrai mode et son démarrage dit pourquoi il ne part pas.

## Conséquences

Aucun changement pour les harness d'Escouade (Claude Code, Codex, Copilot CLI, OpenCode, Cursor, Gemini, Mistral Vibe, Antigravity) : leurs modes de nouveaux agents restent les mêmes. Un harness de plugin à un seul mode Bypass, ou sans mode déclaré, demande « Autoriser le mode Bypass ». Un manifeste qui ment sur ses modes ne se détecte pas : ce qu'il déclare est ce qui est jugé. La stratégie de poste et d'équipe permissions.allowBypass de release/2.3 devra juger ce même mode (ticket de suite).
