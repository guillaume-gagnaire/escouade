---
number: null
id: "un-agent-peut-demander-la-suppression-de"
title: "Un agent peut demander la suppression des agents qu'il a créés ; l'utilisateur confirme chacune dans la fenêtre"
date: "2026-11-16"
state: "proposed"
origin: "brainstorm"
origin_ref: ""
pages: ["harness/serveur-mcp","fenetre/barre-laterale"]
tickets: []
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/serveur-mcp","added":5,"removed":4},{"page":"fenetre/barre-laterale","added":2,"removed":1}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Un agent orchestrateur crée des agents (create_agent) pour des sous-tâches, puis doit les laisser à l'utilisateur, qui les supprime à la main un par un. La page « Serveur MCP » excluait tout outil qui supprime un agent : un agent poussé par un contenu piégé pourrait s'en servir pour monter en droits.

## Décision

Un outil MCP delete_agent, réservé aux agents d'Escouade, demande la suppression d'un agent que l'appelant a créé par create_agent. Le créateur est gardé sur l'agent (AgentMeta.created_by). L'outil ne supprime rien : il ouvre dans la fenêtre la modale « Supprimer l'agent » de la barre latérale, sous le nom du créateur, sa case du worktree réglée comme demandé et modifiable. Plusieurs demandes passent une à une, la suivante quand la précédente est confirmée ou annulée. L'outil répond tout de suite, avant l'utilisateur ; acceptée, l'agent n'est plus dans list_agents. Refusé pour soi-même, pour un agent que l'appelant n'a pas créé, et à Claude hors Escouade.

## Conséquences

L'exclusion « aucun outil ne supprime un agent » reste vraie pour l'outil : seul l'utilisateur supprime. Un agent piégé peut au plus faire paraître des confirmations pour ses propres agents. Les agents créés avant ce changement n'ont pas de créateur : aucun agent ne peut demander leur suppression. Les demandes attendent la vue d'un projet et se perdent si la fenêtre se recharge (l'agent peut redemander). L'agent demandeur n'est pas prévenu d'un refus : il le voit dans list_agents.
