---
number: 365
id: "un-agent-ne-change-pas-de-compte-avant-l"
title: "Un agent ne change pas de compte avant le tour que Claude Code doit sur une tâche de fond finie"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-522"
pages: ["harness/comptes"]
tickets: ["ESC-522"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":2,"removed":0}]
accepted_at: "2026-11-21T18:00:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

« Changer de compte à » déplace un agent libre (au repos, rien en arrière-plan). Entre la fin d'une tâche de fond et le tour que Claude Code ouvre de lui-même sur sa notification, l'agent paraissait libre : une lecture du quota tombée là le déplaçait et tuait ce tour (switching_tests, vu sous charge par le chef ; reproduit à coup sûr avec un tour retardé de 3 s).

## Décision

Une tâche de fond finie après la fin du dernier tour de l'agent, il y a moins d'une minute, retient l'agent sur son compte comme une tâche qui tourne : il passe à la fin du tour que Claude Code ouvre sur elle. Passée la minute sans ce tour, rien n'est plus retenu.

## Conséquences

Le tour sur la tâche n'est plus perdu ; le passage attend au plus une minute de plus dans ce cas. AgentRt garde l'heure de fin de son dernier tour (turn_ended_at, owes_own_turn).
