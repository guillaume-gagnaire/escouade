---
number: 52
id: "la-conversation-de-cursor-cli-passe-par"
title: "La conversation de Cursor CLI passe par son plugin de première main, le reste de son moteur au cœur"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-222"
pages: []
tickets: ["ESC-222"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/cursor-cli-en-plugin","added":16,"removed":0},{"page":"plugins/harness-acp-decrits","added":4,"removed":0}]
accepted_at: "2026-11-02T05:10:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte



## Décision

Le plugin dev.escouade.cursor-cli (escouade-plugins cursor, sur le client de crates/escouade-acp et le service de dev.escouade.acp) tient la conversation : lancement décrit, traduction, permissions, questions, plan, effort, refus. Le moteur Cursor reste au registre du cœur pour ce que le contrat ne dit pas encore : capacités, comptes, modèles d'un compte, textes rédigés, vue terminal. Le contrat gagne relay et byKey (engine/launch), content (agent/sendMessage), keyVar et closeGraceMs (harness), fenceOnPermissions (acp) pour garder le comportement d'avant ; les ids des éléments portent la fin de la clé de leur process. Plugin coupé : l'agent le dit à son lancement et garde sa session.

## Conséquences
