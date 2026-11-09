---
number: 137
id: "commandes-interactives-depuis-le-champ"
title: "Commandes interactives depuis le champ"
date: "2026-11-09"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-401"
pages: ["conversation/commandes-interactives"]
tickets: ["ESC-401"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"conversation/commandes-interactives","added":26,"removed":0}]
accepted_at: "2026-11-09T16:02:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

En stream-json, Claude Code n'exécute pas ses commandes à écran (/login, /permissions, /hooks…) : il répond « isn't available in this environment ». Tapées dans le champ d'un agent, elles ne faisaient rien d'utile. L'utilisateur a demandé le 2026-11-09 de pouvoir gérer ces commandes, /login en tête.

## Décision

Une commande interactive tapée ou choisie dans le menu « / » ne part jamais comme un message. /login ouvre « Se connecter » du compte de l'agent, qui reprend ensuite sur ce compte avec sa session ; /logout demande confirmation puis lance la commande de déconnexion du harness (claude auth logout, codex logout) avec le dossier du compte. /model et /effort seuls ouvrent les menus du champ, /plan passe en mode Plan, /usage ouvre Réglages › Comptes et harness, /remote-control allume le remote control. Les autres basculent l'agent dans la vue Claude Code, où la commande est tapée. /resume, /branch, /fork, /teleport et /exit ne sont pas proposés (l'agent garde sa session). /config et /mcp vont à la vue Claude Code : leurs réglages (thème, serveurs MCP de l'agent) n'ont pas d'équivalent dans Escouade. Le menu « / » marque chacune de l'endroit où elle s'ouvre. Pour les autres harness : /login, /logout, /model et /effort seulement.

## Conséquences

La liste vit dans src/lib/interactive.ts et suit Claude Code 2.1.319 : une version qui ajoute une commande à écran demande de la compléter. Escouade ne lit ni n'écrit jamais la connexion : elle passe par le programme du harness. Les commandes à écran des autres harness (Codex, Copilot…) restent à essayer une à une avant de les aiguiller.
