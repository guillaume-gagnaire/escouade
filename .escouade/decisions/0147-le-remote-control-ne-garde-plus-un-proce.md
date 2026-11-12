---
number: 147
id: "le-remote-control-ne-garde-plus-un-proce"
title: "Le remote control ne garde plus un process par agent"
date: "2026-11-12"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-417"
pages: ["fenetre/barre-laterale","fenetre/reglages","harness/process-des-agents"]
tickets: ["ESC-417"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/barre-laterale","added":13,"removed":6},{"page":"fenetre/reglages","added":4,"removed":4},{"page":"harness/process-des-agents","added":29,"removed":0}]
accepted_at: "2026-11-12T12:29:04+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Issue #26 (macOS M4, 16 Go, Escouade 2.1.5) : avec « Remote control pour tous les agents », chaque démarrage de l'app relançait un process par agent non archivé, même terminé (65 sessions en 4 minutes, environ 26 Go, swap massif, JetsamEvent), process jamais arrêtés pour inactivité. Codex et les comptes sans abonnement claude.ai étaient lancés pour échouer, à chaque lancement. Une app tuée laissait sous macOS et Linux ses agents tourner avec le ppid 1, que le lancement suivant relançait : 1 642 démarrages de claude dans le journal. Chaque agent chargeait aussi tous les serveurs MCP de l'utilisateur.

## Décision

1. Le remote control ne lance que le process des agents actifs, ou actifs depuis moins que le délai d'arrêt des inactifs ; un agent terminé ou inactif garde son remote control et sa session distante, sans process, et son prochain process la rattache. 2. L'arrêt des inactifs vaut aussi pour les agents reliés (sauf travail de fond). 3. Un plafond de process d'agents non actifs gardés par le remote control, 8 par défaut, réglable (« Process gardés au plus ») ; au-delà, les moins récemment actifs s'arrêtent. 4. Rien n'est lancé pour un harness qui ne sait pas se relier (sa capacité, pas son nom) ; un refus définitif d'un compte (pas d'abonnement claude.ai) est retenu dans ses réglages jusqu'à un changement de ses réglages, une reconnexion ou un rallumage à la main réussi. 5. Sous macOS et Linux, Escouade note le groupe de chaque process d'agent (pid, début, programme) dans agent-groups.json et arrête au lancement suivant ceux qui vivent encore à l'identique, jamais un pid repris ; sous Windows le Job (KILL_ON_JOB_CLOSE) suffit. 6. Un réglage « Serveurs MCP des agents : tous les miens / seulement Escouade » ; le second passe --strict-mcp-config à Claude Code, qui ignore alors la configuration de l'utilisateur, le .mcp.json du projet et les connecteurs du compte ; les autres harness n'ont pas d'équivalent.

## Conséquences

Un agent terminé n'est joignable depuis claude.ai qu'après son prochain message (ou ticket, ou reprise) : ce qu'on écrit à sa session sur claude.ai n'arrive pas tant qu'il n'a pas de process. La mémoire et le nombre de process restent bornés quel que soit le nombre d'agents. Le réglage MCP « seulement Escouade » retire aux agents les outils MCP de l'utilisateur et du projet : c'est un choix, d'où le défaut « tous les miens ». Le correctif est à reprendre dans la 2.2 par cherry-pick ; le flag --strict-mcp-config de l'interface en vue Claude Code dépend du lancement des interfaces par leur plugin (ESC-359).
