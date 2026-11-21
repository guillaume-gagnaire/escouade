---
number: 259
id: "un-process-qui-tourne-quand-son-projet-d"
title: "Un process qui tourne quand son projet devient privé est jugé comme un démarrage, harness non activé compris"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-582"
pages: ["projets/projets-prives","plugins/vue-d-ensemble"]
tickets: ["ESC-582","ESC-404","ESC-497"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/projets-prives","added":10,"removed":1},{"page":"plugins/vue-d-ensemble","added":2,"removed":2}]
accepted_at: "2026-11-21T16:56:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La CI de release/2.3 tombait sur macOS (run 44893172996, harness_scope_tests::an_agent_of_a_private_project_waits_for_its_harness_to_be_turned_on). Le démarrage anticipé d'un agent jugeait son harness (allumé : le projet n'était pas encore privé), puis attendait la version du harness et le jeton du compte ; le projet devenait privé pendant cette attente, la clôture était calculée privée, et le process démarrait. Ensuite, `start_process_in` rendait ce process vivant sans juger le harness pour le projet (`check_harness_in` ne venait qu'après), et `refence` ne le relançait pas, sa clôture étant déjà la bonne. Un agent pouvait ainsi garder son process sur un harness que l'utilisateur n'avait pas activé pour ce projet privé. Plus largement, rendre un projet privé ne touchait pas aux process qui tournaient.

## Décision

1) Le harness non allumé pour le projet (coupé, ou pas encore activé dans un projet privé) devient une raison de ne pas continuer pour un process qui tourne (`run_refusal`), à côté des stratégies et de « Autoriser le mode Bypass » (ESC-497). Un process qui tourne est donc jugé comme un démarrage avant de recevoir un message : refusé, il s'arrête aussitôt au repos, et c'est dit. Au travail, il est jugé de nouveau à la fin de son tour (`settle_locked_mode`). 2) Quand un projet devient privé, que ce soit par la fenêtre, par `.escouade.yml` ou par une stratégie qui l'impose, les process et interfaces de ses agents au repos dont le harness n'y est pas allumé s'arrêtent aussitôt, avec la phrase « … n’est pas activé pour ce projet privé … L’agent s’arrête. ». Ceux au travail finissent leur tour, puis s'arrêtent avec « … L’agent s’arrête à la fin de son tour. ». Le tour n'est pas coupé : l'utilisateur a rendu le projet privé, il n'a pas coupé le harness. Couper le harness pour le projet garde sa règle d'ESC-404, qui interrompt le tour.

## Conséquences

Aucun process ne tourne sur un harness que l'utilisateur n'a pas activé pour un projet privé, et la conversation dit pourquoi. Un agent au repos perd son process au moment du passage en privé, alors qu'avant il le gardait jusqu'à son tour suivant, où la clôture le relançait. La session est gardée et « Activer » le fait repartir. Un test rejoue la course du Mac à coup sûr : un point d'arrêt de test (`start_hold`) placé entre le jugement du harness et la lecture de la clôture. Le même trou existe sur main (2.2).
