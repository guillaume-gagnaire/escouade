---
number: 260
id: "les-fonctionnalites-experimentales-de-cl"
title: "Les fonctionnalités expérimentales de Claude Code passent à la vue terminal seule, et un agent en Code Mode reste dans son terminal"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-553"
pages: ["harness/claude-code","plugins/claude-code-en-plugin"]
tickets: ["ESC-553"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/claude-code","added":32,"removed":0},{"page":"plugins/claude-code-en-plugin","added":7,"removed":4}]
accepted_at: "2026-11-21T20:25:59+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Claude Code a sorti le Code Mode (`CLAUDE_CODE_EXPERIMENTAL_CODE_MODE`, 2.1.323) : Claude écrit du JavaScript qui fait ses appels d'outils. L'utilisateur a demandé comment l'activer dans Escouade, sinon un réglage du plugin Claude Code pour les drapeaux expérimentaux. Le Code Mode ne marche que dans une session `claude` interactive : les essais réels sur la 2.1.329 montrent que `-p` en stream-json l'ignore sans rien dire, que la reprise en stream-json d'une session commencée en Code Mode est refusée (« Failed to resume session: This session was started in Code Mode (experimental), which isn't available because for now it only works in the interactive terminal… »), avec ou sans la variable, que l'interface ne la reprend qu'avec la variable, et que `--remote-control` l'empêche de démarrer. Or les agents d'Escouade tournent en stream-json, et la vue terminal reprend la même session.

## Décision

Le plugin Claude Code (2.2.8) a un groupe « Fonctionnalités expérimentales » : Code Mode et Agent Teams (interrupteurs, avec leur version et leurs surfaces), et « Autres drapeaux », borné aux noms `CLAUDE_CODE_EXPERIMENTAL_*` (chacun à 1, jamais une autre variable ni une valeur). Ils se règlent pour tous les projets, puis par projet (portée `project` : comme pour tous, activé, désactivé ; une liste qui remplace), jamais par le `.escouade.yml` d'un dépôt. Ils ne vont que dans le bloc `env` du fichier `--settings` privé de la vue terminal, jamais dans la conversation, une discussion ni un agent de ticket. Le Code Mode n'est pas passé, et la conversation dit pourquoi, avec le remote control, un compte routé par une passerelle ou un fournisseur cloud, une version trop ancienne ou un modèle qui ne le prend pas. Un agent dont le transcript dit `repl-mode` est marqué (`codeMode`) : sa conversation le dit, « Vue Escouade » est refusée, son flux stream-json n'est jamais démarré (reprise automatique, ticket, message), et son interface le reprend toujours avec le Code Mode et sans remote control. Le cœur passe à `terminal/launch` les réglages du plugin pour le projet, la version du harness, le nom de ce qui route le compte et `terminalOnly` (protocole 0.1.4, champs facultatifs).

## Conséquences

L'utilisateur essaie le Code Mode sans toucher à son `~/.claude/settings.json`, projet par projet, et un agent passé en Code Mode ne se casse plus en revenant dans la conversation. Un tel agent reste pour de bon dans la vue terminal : la conversation riche d'Escouade ne le pilote plus. Les permissions ne changent pas : les appels faits par le code arrivent aux hooks et aux règles sous leur nom habituel (vérifié sur la 2.1.329), la clôture d'un projet privé (règles `deny`) tient pour eux, et l'appel `REPL` lui-même s'annonce « Lancer du code : <description> ? ». Le plugin se compile toujours contre le protocole 0.1.2 publié (il lit les champs nouveaux dans les paramètres bruts) ; sur une Escouade d'avant, rien n'est passé. Le store reçoit la 2.2.8 du plugin quand le chef la publie.
