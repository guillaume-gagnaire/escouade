---
number: 227
id: "un-ticket-peut-avoir-sa-branche-cible-et"
title: "Un ticket peut avoir sa branche cible, et la validation refuse une branche d'une autre ligne"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-552"
pages: ["kanban/tickets","kanban/validation","kanban/agent-d-un-ticket","harness/boite-a-toi"]
tickets: ["ESC-552","ESC-531"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/tickets","added":8,"removed":0},{"page":"kanban/validation","added":8,"removed":0},{"page":"kanban/agent-d-un-ticket","added":4,"removed":0},{"page":"harness/boite-a-toi","added":4,"removed":0}]
accepted_at: "2026-11-20T15:17:02+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le 2026-11-20, trois tickets en revue ont été validés depuis la boîte « À toi ». Le Kanban d'Escouade vise release/2.3, mais ESC-531 était un ticket de la 2.2 : sa branche partait de main. La validation a fusionné release/2.3 dans ticket/esc-531 (des centaines de fichiers en conflit), puis a confié le conflit à l'agent du ticket. Rien ne permettait à un ticket de dire sa cible, et rien n'empêchait de fusionner la cible dans une branche partie d'une autre ligne.

## Décision

1. Un ticket a une branche cible facultative (`Ticket::target`), vide pour celle du Kanban. Elle se pose dans sa fenêtre (« Cible »), par `create_ticket` et `update_ticket` du serveur MCP, et par « Choisir la cible de ce ticket » (`ticket_set_target`, toute colonne sauf « Terminé », jamais pendant une validation, la base du worktree de son agent avec). Elle sert de bout en bout : le worktree en part, la validation y fusionne, la PR la vise, le pilote auto démarre le ticket dessus (bloqué si elle n'existe plus), le retard se compte sur elle (la base du worktree). La carte et la ligne de la boîte « À toi » disent « vers main » quand elle diffère de celle du Kanban.

2. La garde, avant toute étape d'une validation qui fusionne ou propose une PR (avant les tests, le commit et le merge), et avant « L'agent résout » : la branche du ticket ne descend pas de sa cible quand elle porte des commits que la cible n'a pas et qu'une autre branche a aussi. Le critère, `git::other_line` : C = `git rev-list <branche> ^<cible>` ; pour chaque branche locale ou distante O, sauf la branche du ticket, la cible et leurs copies distantes, et celles dont le nom commence par le préfixe des branches des tickets ou des agents, les commits de C que O a aussi (`|C| − |rev-list <branche> ^<cible> ^O|`). Si une O en a, la validation s'arrête : le ticket reste « À tester », bloqué avec « « ticket/esc-531 » ne part pas de sa cible release/2.3 : elle porte 12 commits de main que release/2.3 n'a pas. Rien n'est fusionné : choisis la cible de ce ticket. », rien n'est fusionné dans la cible ni dans la branche, rien n'est dit à l'agent. La branche trouvée (celle qui en a le plus, une locale d'abord, puis le nom le plus court) est gardée (`Ticket::other_line`) et proposée en premier par « Choisir la cible de ce ticket » (carte) et « Viser main » (boîte).

Pourquoi ce critère plutôt que `merge-base --is-ancestor` : Escouade ne garde pas le commit d'où une branche est partie, et une branche reprise n'en a pas ; l'ancêtre commun ne dit pas de quelle ligne viennent les commits. Les commits partagés avec une autre branche le disent sans rien enregistrer, et nomment la ligne à proposer. Une branche qui contient la branche du ticket entière compte aussi : un ticket parti de main sans commit à lui est main.

## Conséquences

Un correctif de la 2.2 se fait depuis un Kanban qui vise la 2.3, sans changer la cible du Kanban. Une validation coûte quelques `git rev-list` de plus (deux par branche candidate). Limite connue : une copie que l'utilisateur fait de la branche d'un ticket sous un autre nom (hors des préfixes des tickets et des agents), puis dépassée par de nouveaux commits, passerait pour une autre ligne : la validation s'arrête et le dit, et choisir la cible du Kanban ne lève pas l'arrêt ; il faut supprimer la copie. Un projet de plusieurs dépôts n'a pas de cible par ticket et n'a pas la garde (chaque dépôt garde sa base).
