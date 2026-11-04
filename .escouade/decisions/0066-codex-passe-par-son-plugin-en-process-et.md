---
number: 66
id: "codex-passe-par-son-plugin-en-process-et"
title: "Codex passe par son plugin, en process, et sa vue lit sa traduction en bibliothèque"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-225"
pages: ["plugins/codex-en-plugin","plugins/harness-en-process","plugins/livres-avec-escouade"]
tickets: ["ESC-225"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/codex-en-plugin","added":15,"removed":0},{"page":"plugins/harness-en-process","added":7,"removed":0},{"page":"plugins/livres-avec-escouade","added":1,"removed":1}]
accepted_at: "2026-11-04T09:29:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-10.4 sort Codex du cœur. Ce n'est pas un harness ACP : son client JSON-RPC (`codex app-server`) ouvre le fil en plusieurs étapes, tient le tour en cours, et sa traduction était écrite sur l'état de l'agent. La vue Codex lit son rollout par cette même traduction. La grille d'OpenAI (`pricing.rs`) part avec lui (revue du 2026-10-29). OpenCode (INT-10.2) et Copilot CLI (INT-10.3) ont déjà fait grandir le contrat.

## Décision

Le plugin de première main dev.escouade.codex tient le client, la traduction, les mots du protocole et la grille d'OpenAI, en machine à états sans process. Le moteur Codex reste au registre pour ses capacités, les comptes, le quota et les modèles d'un compte sans agent, codex exec et la vue terminal. La vue lit son rollout par la traduction du plugin, prise comme une bibliothèque. Codex se sert du contrat d'OpenCode et de Copilot CLI (opens, UsageAsked, permissionCard, quota, limitRest, compact et rename) et n'y ajoute que ce qui lui manque : turn dans la réponse de compact, entry, activity, took, et une limite de trois minutes à l'ouverture. Il garde git add et git commit acceptés sans demander.

## Conséquences

Aucun changement pour l'utilisateur. Une seule traduction pour la conversation et la vue, au prix d'une dépendance du cœur vers la crate du plugin. Le plugin de Codex lance git et lit le config.toml du compte quand le serveur d'Escouade est refusé : c'est dit dans son README. Après un plantage du plugin en plein tour, les demandes en attente restent répondables par leur id, mais le tour en cours n'est plus connu : un message envoyé avant sa fin peut être refusé par Codex et le dit. Les essais réels restent à rejouer sur une version 0.163 ou plus récente de Codex.
