---
number: 369
id: "une-tache-programmee-par-un-agent-attend"
title: "Une tâche programmée par un agent attend l'accord de l'utilisateur ; un outil ne fait que proposer le retrait d'un souvenir d'équipe validé"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-647"
pages: ["kanban/taches-programmees","serveur-equipe/memoire-equipe","harness/boite-a-toi"]
tickets: ["ESC-647","ESC-627"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/taches-programmees","added":12,"removed":3},{"page":"serveur-equipe/memoire-equipe","added":5,"removed":1},{"page":"harness/boite-a-toi","added":5,"removed":0}]
accepted_at: "2026-11-26T16:05:38+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-627) relève deux asymétries. SR4-04 (moyenne) : un agent programmait, dans tout projet non privé, une tâche qui relance un agent à chaque minute, sans question, sans trace de son auteur, et qui lui survit : une persistance. SR4-05 (basse) : un agent, ou un client MCP extérieur, effaçait d'un coup pour toute l'équipe un souvenir validé par un humain, alors qu'une écriture d'agent n'est qu'une proposition.

## Décision

1. Une tâche programmée créée ou modifiée par un agent (l'Overseer compris) naît en attente (`awaiting`) : ses échéances passent sans ticket jusqu'à l'accord de l'utilisateur, donné dans « À toi » (vue « Tâche programmée à accepter » : projet, rythme, consigne, critères ; sa clé porte l'empreinte de la tâche, une tâche changée entre-temps refuse l'accord sur l'ancienne) ou par « Accepter » sur sa ligne. Elle garde son auteur (agent, projet, ticket), montré dans la liste et rendu par `list_scheduled_tasks`. Aucun agent ne prend en charge ni ne coche cette vue. 2. Un agent ne crée ni ne modifie une tâche que dans son propre projet ; l'Overseer garde les projets qu'il voit, ses tâches attendant l'accord comme les autres. 3. Un agent ne donne pas un rythme dont deux échéances sont à moins de 15 minutes (mesuré sur les 500 prochaines). L'utilisateur (fenêtre, Claude Code hors d'Escouade) n'est pas borné. 4. `forget_memory` sur un souvenir d'équipe validé par un humain (écrit à la main, ou gardé à la relecture) pose une demande de retrait signée (`removalAsked`) au lieu d'une pierre tombale ; le souvenir reste lu par tous jusqu'à ce qu'un membre règle la demande dans Réglages › Projets › Équipe › Mémoire partagée (« Retirer » ou « Garder »), geste de la fenêtre seule. Un outil oublie encore d'un coup ses propres propositions et, la relecture éteinte, ce qu'un agent a écrit ; une mémoire hors équipe reste effaçable.

## Conséquences

Un agent qui programme une tâche doit attendre l'utilisateur, comme pour un ticket d'un collègue (ESC-645) ; le chef d'escouade ne peut plus poser une routine seul. Le plancher de 15 minutes est plus souple que l'heure proposée par le rapport, l'accord de l'utilisateur couvrant le reste. Les tâches d'avant restent comme elles sont (sans auteur, non en attente). Côté équipe, un nouveau champ du journal (`removalAsked`), ignoré par les postes d'avant : un poste en 2.2 ne voit pas la demande mais garde le souvenir. Nouvelles commandes de fenêtre `scheduled_accept` et `team_memory_decide_removal`.
