---
number: null
id: "le-plan-d-un-agent-dans-list-background"
title: "Le plan d'un agent dans list_background_work"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-1"
pages: ["harness/serveur-mcp"]
tickets: ["ESC-1"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/serveur-mcp","added":7,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La fenêtre montre au-dessus d'une conversation le plan de l'agent (titre, tâches numérotées, « 3/7 tâches »). Le serveur MCP ne le donnait pas : un client hors de la fenêtre (widget PhoneBox, agent qui suit un collègue) ne voyait que les sous-agents et les tâches de fond.

## Décision

list_background_work gagne un champ plan : { title, done, total, tasks: [{ id, title, status, active?, steps? }] }, tâches dans l'ordre du plan, sans blocked_by ; null sans plan ou plan sans tâche. Un appelant qui n'agit pas sur le projet (règle full / may_act_on) n'en reçoit que { done, total }. get_agent_summary ne change pas (done/total y restent facultatifs, non faits).

## Conséquences

Champ ajouté, rien d'existant ne change : un client d'avant l'ignore. Le pont PhoneBox peut afficher l'avancement d'un agent. La description de l'outil et docs/PROTOCOL.md le disent.
