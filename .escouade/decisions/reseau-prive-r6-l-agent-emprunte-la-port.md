---
number: null
id: "reseau-prive-r6-l-agent-emprunte-la-port"
title: "Réseau privé, R6 : l'agent emprunte la porte par le proxy de son environnement et le ProxyCommand de ssh"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-551"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-551", "ESC-587"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: ""
accepted_by: ""
---

## Contexte

L'agent ne doit rien savoir du connecteur : il tape git clone, npm install, npm test. Comment ses connexions passent-elles par la porte du serveur (docs/PRIVATE-NETWORK.md § 4.3, R6) ?

## Décision

D'abord deux entrées : la porte (un proxy HTTP CONNECT et SOCKS5 par nom, sur 127.0.0.1, un jeton par agent) devient le seul proxy des agents, des lancements, des terminaux et des git du cœur (HTTPS_PROXY, HTTP_PROXY, ALL_PROXY), le proxy de l'espace passant derrière elle ; ssh passe par ProxyCommand="escouade-server via %h %p" (GIT_SSH_COMMAND, core.sshCommand des worktrees). Ensuite, dans l'image seulement, des noms en boucle locale pour ce qui ignore les proxys (Java).

## Conséquences

Les trois cas (git en SSH, registre npm, API des tests) passent sans rien demander au système. Maven et Gradle veulent leur config de proxy ou les noms en boucle locale. Les API des modèles passent aussi par la porte, qui les sort directement : elle est sur le chemin de chaque agent.
