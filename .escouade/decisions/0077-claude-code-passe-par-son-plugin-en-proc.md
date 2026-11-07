---
number: 77
id: "claude-code-passe-par-son-plugin-en-proc"
title: "Claude Code passe par son plugin, en process, et le cœur garde l'état de l'agent et les comptes"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-226"
pages: ["plugins/claude-code-en-plugin","plugins/harness-en-process","plugins/livres-avec-escouade"]
tickets: ["ESC-226"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/claude-code-en-plugin","added":13,"removed":0},{"page":"plugins/harness-en-process","added":11,"removed":0},{"page":"plugins/livres-avec-escouade","added":1,"removed":0}]
accepted_at: "2026-11-05T03:17:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-10.5 sort du cœur le harness le plus utilisé, celui de presque tous les e2e. Sa traduction était écrite à même l'état de l'agent (plan, demandes, fichiers joints), et son client répondait lui-même aux requêtes de contrôle, attendues par le cœur. La revue du 2026-10-29 fait partir signin.rs et usage.rs avec lui, mais une partie de ces fichiers décide pour le cœur (relancer un agent) ou sert tous les harness (les fenêtres de quota).

## Décision

Le plugin de première main dev.escouade.claude-code tient le client (stream-json et protocole de contrôle), la traduction, localcmd et la lecture d'une connexion. Le client et la traduction sont des machines à états, sans process, fil, réseau, fichier ni minuteur ; les lectures sont isolées dans signin.rs. La traduction ne garde que l'état du flux (blocs, compteurs par modèle) ; le plan, les demandes, la conversation et la décision d'interruption restent au cœur, qui applique des éléments. Le moteur Claude reste au registre : capacités, comptes, modèles d'un compte, claude -p, vue terminal, déclaration du serveur MCP. La vue lit son transcript par la traduction du plugin, prise comme une bibliothèque. De signin.rs et usage.rs ne restent au cœur que ce qui décide (relancer un agent, RefreshGate, le quota au point d'accès OAuth avec les réglages réseau de l'app) et les fenêtres de quota de tous les harness. Le contrat grandit pour tous : réponse du harness attendue (Asked, Item::Answer) sans retenir les appels suivants, agent/stopTask, agent/taskOutput, agent/remote, clôture tenue par le fichier de réglages que le cœur écrit, éléments conversation, message, tasks, call, toolResult, replayed, ask, result, limit, overage, remote, told, answer, said. Un lot d'éléments s'applique en plusieurs fois, chaque carte et chaque fin de tour à part. Un plugin relancé par un changement de langue ou de réseau retrouve ses agents avant son prochain appel.

## Conséquences

Aucun changement pour l'utilisateur. Claude Code coupé dans la vue Plugins, ses agents ne démarrent plus et le disent, leur session gardée ; coupé pendant un tour, l'agent s'arrête en le disant ; les textes rédigés, le quota et la vue terminal continuent. Le cœur dépend de la crate du plugin. Le changement de compte au seuil entre deux vrais comptes ne s'essaie pas sur ce poste : il reste testé sur les faux. Un plantage du plugin perd l'état du flux : un texte en cours peut être repris sous une nouvelle ligne, et un appel attendu (un changement de mode) finit en délai dépassé. Au plus vite, le long tour dépasse le seuil de 5 ms au 99e centile (+22 ms, comme OpenCode et Mistral Vibe) ; au rythme réel, il ne bouge pas, et le CPU d'un tour de 2 min prend 0,9 s de plus. Le plugin sera porté en WASM ensuite (ESC-330).
