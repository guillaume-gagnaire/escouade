---
number: 338
id: "reseau-prive-rp-1-les-variables-et-ssh-t"
title: "Réseau privé RP-1 : les variables et ssh, terminaux et lancements toujours par la porte"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-597"
pages: ["a-distance/atteindre-un-reseau-prive","fenetre/reseau","git-fichiers/fetch-automatique"]
tickets: ["ESC-597"]
documents: ["docs/PRIVATE-NETWORK.md"]
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":41,"removed":4},{"page":"fenetre/reseau","added":5,"removed":0},{"page":"git-fichiers/fetch-automatique","added":2,"removed":0}]
accepted_at: "2026-11-26T12:53:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'étude (§ 4.3, R6) dit que la porte prend la place du proxy réglé pour les agents, les lancements, les terminaux et les git du cœur, et que le cœur pose GIT_SSH_COMMAND et écrit core.sshCommand dans les worktrees. Restaient : les terminaux et lancements quand « le proxy aussi pour les terminaux » est décoché, la commande ssh de l'utilisateur, où écrire core.sshCommand, et escouade-server via sans porte dans son environnement.

## Décision

Avec la porte, les terminaux, les lancements et les étapes des worktrees la reçoivent que « le proxy aussi pour les terminaux » soit coché ou non (elle est la seule route vers le réseau privé). GIT_SSH_COMMAND garde la commande ssh de l'utilisateur (core.sshCommand du dépôt, sinon GIT_SSH_COMMAND du serveur, sinon ssh) et y ajoute -o ProxyCommand="<escouade-server> via %h %p". core.sshCommand est écrit au démarrage d'un agent dans la configuration du dépôt de son worktree (partagée par ses worktrees), la commande de l'utilisateur gardée dedans, et rien n'est réécrit s'il passe déjà par la porte. Les git du cœur qui joignent un dépôt distant dans un projet reçoivent les mêmes variables (jeton git du projet), une variable qu'il leur donnait déjà restant. escouade-server via sans proxy dans son environnement se connecte directement.

## Conséquences

Un git lancé sans les variables (un terminal, un éditeur) passe par escouade-server via et sort directement : la porte reste une consigne (SEC-02). Le core.sshCommand écrit reste dans le dépôt si la porte s'éteint, sans effet gênant (via sort en direct). Le fetch de fond d'un serveur avec la porte prend le proxy de la porte, à la différence du poste (Fetch automatique).
