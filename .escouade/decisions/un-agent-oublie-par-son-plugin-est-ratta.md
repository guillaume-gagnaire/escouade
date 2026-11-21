---
number: null
id: "un-agent-oublie-par-son-plugin-est-ratta"
title: "Un agent oublié par son plugin est rattaché par le cœur, et un refus se dit une fois"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-562"
pages: ["plugins/harness-en-process"]
tickets: ["ESC-562"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-en-process","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Sur la capture de l'utilisateur du 2026-11-20, la conversation d'un agent Claude Code n'était plus qu'une suite de « Le plugin de Claude Code n'a pas traduit ce que son harness a dit (no agent …: it was not attached (-32602)) », une ligne par lot, et l'agent était inutilisable. La cause est reproduite par un test du runtime. Un plugin oublié (`forget` : mise à jour, réinstallation, permission, langue) jetait la liste des attach gardés. Le cœur ne rattachait l'agent que s'il trouvait le plugin au repos ; si un autre appelant redémarrait le plugin d'abord, les agents restaient non attachés jusqu'au bout.

## Décision

1) Le runtime garde, à travers un `forget`, les attach et les dossiers des agents qui vivent : l'instance suivante les reçoit avant toute autre chose, qui que ce soit qui la démarre. 2) Le cœur reconnaît la réponse « -32602 no agent <clé>: it was not attached » des plugins de harness de première main, par son code et son texte, sans changer le protocole. Il redonne alors l'agent et refait le même appel une fois : agent/frames, les appels qui écrivent au harness, le quota, le titre et les commandes. 3) Si le plugin refuse l'agent qu'on lui redonne, une seule ligne dit la raison et le geste (envoyer un message, relancer le plugin dans Réglages › Plugins). Le harness s'arrête, sa session est gardée, et le tour finit en erreur sur cette ligne sans la redire. 4) Un attach refusé au redémarrage d'un plugin est signalé au cœur (`on_attach_refused`). Le cœur réessaie, puis le dit sur l'agent comme en 3.

## Conséquences

Les plugins publiés ne changent pas et profitent de la réparation. Un plugin tiers qui veut qu'un agent oublié soit repris répond pour lui le même code et le même texte. Une fin de process qui a déjà été dite, mot pour mot, comme dernière ligne de la conversation n'est plus redite par le cœur ; cela vaut aussi pour un plugin arrêté. Le plugin de démonstration Minimal sait désormais oublier ses agents et refuser un rattachement, pour les tests.
