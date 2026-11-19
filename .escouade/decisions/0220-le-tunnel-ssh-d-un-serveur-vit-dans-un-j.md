---
number: 220
id: "le-tunnel-ssh-d-un-serveur-vit-dans-un-j"
title: "Le tunnel SSH d'un serveur vit dans un job et s'arrête avec ce qui l'a ouvert"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-541"
pages: ["a-distance/se-connecter-a-un-serveur"]
tickets: ["ESC-541"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/se-connecter-a-un-serveur","added":2,"removed":1}]
accepted_at: "2026-11-19T17:50:12+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un faux ssh lancé par un test a survécu à nextest (ESC-541) et a bloqué 55 minutes une chaîne qui lisait sa sortie par un tube. Dans le produit, l'arrêt du tunnel passait par une tâche asynchrone. Une app qui sort juste après (`RunEvent::Exit`) pouvait donc laisser `ssh` vivant. La sortie pour une mise à jour sous Windows (`std::process::exit`, sans `shutdown`) l'y laissait toujours. Ce que `ssh` lance (un ProxyCommand) n'était jamais arrêté avec lui. Un `Servers` abandonné sans arrêt gardait aussi son tunnel, car le superviseur le tenait sur le runtime global.

## Décision

`ssh` (le tunnel, un relais de port comme la commande `escouade-server pair`) tourne dans un job : un Job object avec KILL_ON_JOB_CLOSE sous Windows, un groupe de process ailleurs. Arrêter le tunnel termine le job tout de suite, et `stop` ne rend la main qu'après. Sous Windows, il attend en plus une seconde au plus que le job soit vide. Le superviseur garde le tunnel en cours là où son propriétaire peut l'arrêter lui-même. `Servers` arrête ses tunnels et ses relais quand il disparaît, et `shutdown` les arrête avant de rendre la main. Sous Windows, le système arrête le job quand l'app finit, de quelque façon que ce soit.

## Conséquences

Aucun tunnel, relais ni enfant de ceux-ci ne survit à l'app ni à son propriétaire. Il reste une limite hors Windows : un process de l'app tué par SIGKILL ou planté laisse son groupe en vie, car aucun mécanisme du système ne l'arrête avec son parent. Les tunnels de plusieurs espaces (ESC-539) devront passer par `Tunnel` pour suivre la même règle.
