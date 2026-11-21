---
number: 254
id: "un-agent-dont-le-remote-control-est-allu"
title: "Un agent dont le remote control est allumé à la main reste joignable ; « en veille » dit quand un agent relié n'a pas de process"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-544"
pages: ["harness/process-des-agents","fenetre/barre-laterale"]
tickets: ["ESC-544","ESC-417"]
documents: []
supersedes: [147]
superseded_by: null
changes: [{"page":"harness/process-des-agents","added":8,"removed":4},{"page":"fenetre/barre-laterale","added":7,"removed":5}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Signalement du 2026-11-19 (Escouade sous Linux dans WSL) : le remote control d'un agent paraît activé dans Escouade, mais l'app Claude montre sa session déconnectée. Depuis la décision 0147 (ESC-417, 2.2.0), l'arrêt des agents inactifs (30 min par défaut) vaut aussi pour les agents reliés, et au-delà de « Process gardés au plus » les moins récemment actifs s'arrêtent : un agent relié garde son remote control et sa session sans process, et claude.ai le voit déconnecté jusqu'à son prochain message envoyé depuis Escouade, alors qu'Escouade dit encore « activé ». L'utilisateur a tranché le 2026-11-19 : un agent dont le remote control a été activé à la main reste joignable.

## Décision

Remplace les points 1 et 2 de la 0147 pour les agents allumés à la main ; le reste de la 0147 (plafond, harness qui ne savent pas se relier, refus d'un compte, groupes de process, serveurs MCP) tient. 1. Un agent dont le remote control a été allumé à la main (son menu, `RemoteSource::Hand`) n'est jamais arrêté pour inactivité ; il compte dans le plafond « Process gardés au plus » (8 par défaut), au-delà duquel les moins récemment actifs s'arrêtent, quelle que soit leur source. Le remote control d'un réglage (pour tous les agents, ou d'un projet) garde la règle de la 0147 : arrêt après le délai des inactifs. 2. Au démarrage de l'app (et à la réouverture d'un projet), les agents allumés à la main repartent, même terminés ou inactifs depuis longtemps, dans le plafond, et rattachent leur session ; ceux d'un réglage ne repartent que s'ils sont actifs ou l'ont été depuis moins que le délai des inactifs. 3. L'état dit vrai : en vue Escouade, un agent relié sans process dit « En veille : reprend au prochain message » (icône pâlie, infobulle, titre du menu) avec un geste « Réveiller » qui relance son process et rattache sa session, sans changer sa source ; avec un process, son menu dit « Relié ». « Désactiver le remote control » reste.

## Conséquences

Un agent allumé à la main est joignable depuis claude.ai tant qu'Escouade tourne et qu'il est dans le plafond, y compris après un redémarrage. La mémoire reste bornée (issue #26) : au plus « Process gardés au plus » process non actifs, quel que soit le nombre d'agents allumés à la main ; au-delà, l'agent passe « en veille » et le dit. Allumer à la main coûte donc un process gardé ; le réglage « pour tous » reste économe. Deux variables d'environnement (`ESCOUADE_IDLE_STOP_MS`, `ESCOUADE_IDLE_CHECK_EVERY_MS`) raccourcissent l'arrêt des inactifs pour les tests de bout en bout. Correctif de la ligne 2.2 (part de main), à reverser dans release/2.3.
