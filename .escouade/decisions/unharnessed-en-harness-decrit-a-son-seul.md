---
number: null
id: "unharnessed-en-harness-decrit-a-son-seul"
title: "Unharnessed en harness décrit, à son seul mode Bypass, sans installeur, sa session reprise par --resume"
date: "2026-11-16"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-491"
pages: ["plugins/harness-acp-decrits"]
tickets: ["ESC-484","ESC-491"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":11,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur a demandé un plugin pour Unharnessed (2026-11-16, ESC-484). Unharnessed ne demande jamais avant d'agir, son noyau Python n'est pas un bac à sable, il n'a aucun mode ACP ni installeur officiel hors d'une extension VS Code publiée pour macOS Apple Silicon seulement. ESC-484 n'avait pas proposé sa décision ; ESC-491, qui porte sa branche, la reprend à jour d'ESC-489 (le mode jugé) et de ce qu'il ajoute au pilote ACP.

## Décision

Le plugin de première main dev.escouade.unharnessed décrit Unharnessed par ACP sans code : un seul mode, bypassPermissions, son sens dit en clair ; ni readOnly, ni install, ni updateCommand, ni modèles déclarés ; sa cellule Python montrée « Python » par acp.tools ; ses comptes par UNHARNESSED_CODING_AGENT_DIR ; sa session reprise par acp.resume (--resume <id>, l'id lu dans _meta.unharnessed.saved.sessionId), ses tours à lui finis par acp.turnEnd (ses phases responseBoundary et terminalQuiescence), son contexte lu par acp.usage. escouade.min vaut 2.2.2 : c'est la version qui refuse ses agents sans « Autoriser le mode Bypass ».

## Conséquences

Sur l'app 2.2.2, le plugin marche, ses agents refusés sans Bypass permis ; acp.resume, turnEnd et usage y sont ignorés (session neuve dite, tour de réveil hors tour, contexte non lu) et prennent effet avec la 2.2.3. Pas de coût ni de jetons d'entrée et de sortie : il ne les dit pas comme tels. Ce que fait Unharnessed d'un --resume vers une session dont le fichier a disparu n'a pas été essayé en vrai (le faux en ouvre une neuve).
