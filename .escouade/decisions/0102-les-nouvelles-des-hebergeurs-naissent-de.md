---
number: 102
id: "les-nouvelles-des-hebergeurs-naissent-de"
title: "Les nouvelles des hébergeurs naissent des lectures déjà faites, et se règlent une à une"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-68"
pages: ["fenetre/notifications","fenetre/vue-d-ensemble","kanban/environnements","harness/serveur-mcp","git-fichiers/pull-requests","git-fichiers/vue-pull-requests"]
tickets: ["ESC-68"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/notifications","added":16,"removed":2},{"page":"fenetre/vue-d-ensemble","added":16,"removed":2},{"page":"kanban/environnements","added":3,"removed":1},{"page":"harness/serveur-mcp","added":8,"removed":0},{"page":"git-fichiers/pull-requests","added":1,"removed":0},{"page":"git-fichiers/vue-pull-requests","added":1,"removed":0}]
accepted_at: "2026-11-07T02:47:09+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-68 ajoute huit notifications (CI en échec, changements demandés, PR approuvée, PR mergée, conflit, relecture demandée, déploiement en production réussi, déploiement en échec), un groupe « Pull requests » dans la vue d'ensemble et des outils MCP de lecture des PR. Il n'y a pas de webhook, et une notification ne doit pas interroger l'hébergeur plus qu'aujourd'hui. Or seules les PR des tickets « En revue » et les déploiements se lisent en arrière-plan : celles d'un agent ordinaire se lisent quand il est affiché, et aucune lecture de fond ne dit à qui une relecture est demandée.

## Décision

Une notification est l'écart entre deux lectures déjà faites : aucune lecture de plus n'existe pour elle. Une PR de ticket compare son état gardé, une PR d'agent la liste gardée sur l'agent, et une PR tenue par un ticket n'est dite que par son suivi ; « relecture demandée » vient de la liste du projet, seule lecture qui dit les relecteurs, gardée en mémoire, et sa première lecture ne dit rien. Le texte d'une notification dit un numéro, jamais un titre. Chaque nouvelle a son propre réglage, activé par défaut, et désactivée elle ne fait plus rien, barre des tâches comprise. « Déploiement en échec » garde tout environnement, comme ESC-67 ; « Déploiement en production réussi » ne vaut que pour la production. La fusion et les changements demandés d'un ticket passent de la notification des tickets à la leur. La vue d'ensemble lit les listes des projets à l'ouverture puis chaque minute tant qu'elle est affichée, et les outils MCP passent par la même liste gardée une minute ; à moins d'un dixième de la limite d'appels d'un hébergeur, ce qui peut attendre reprend la dernière liste. Les outils MCP marquent leur réponse comme les outils du store, `<third_party_data>`, et `get_ticket` ne rend aucun texte de tiers.

## Conséquences

Rien ne coûte d'appel de plus à l'arrière-plan : la seule lecture nouvelle est celle de la vue d'ensemble ouverte. En contrepartie, la PR d'un agent ordinaire ne notifie que lorsqu'il est affiché, et une relecture demandée que lorsque la liste du projet est lue (vue « Pull requests », vue d'ensemble, `list_pull_requests`) : l'utilisateur qui n'ouvre rien n'en sait rien. Une lecture de fond des listes réglerait cela au prix d'un appel par projet : c'est à décider à part. Quelqu'un qui avait coupé les notifications de tickets recevra de nouveau les fusions et les déploiements en échec, jusqu'à couper leurs nouvelles cases. Essayé contre le faux GitHub seulement.
