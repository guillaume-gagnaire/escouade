---
number: 335
id: "equipe-couts-bornes-et-signes-parole-d-u"
title: "Équipe : coûts bornés et signés, parole d'un collègue en donnée, passages vers la machine sous règle, mode « auto » sous accord"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-649"
pages: ["serveur-equipe/couts-equipe","serveur-equipe/memoire-equipe","serveur-equipe/passer-un-agent","serveur-equipe/fichier-equipe"]
tickets: ["ESC-649","ESC-626"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/couts-equipe","added":14,"removed":3},{"page":"serveur-equipe/memoire-equipe","added":10,"removed":5},{"page":"serveur-equipe/passer-un-agent","added":11,"removed":1},{"page":"serveur-equipe/fichier-equipe","added":6,"removed":1}]
accepted_at: "2026-11-26T12:51:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-626, constats SR3-03 à SR3-08) a montré qu'un simple membre pouvait remplir ou cacher les budgets de l'équipe et mettre sa dépense au nom d'un autre, poser une consigne chez les agents des autres par un souvenir écrit à la main, faire exécuter ce qu'il voulait à la machine de l'équipe en lui passant un agent (et écraser une branche locale), et qu'un administrateur imposait « auto » à tous sans quorum ni accord ; un maxLoops ou un NaN venus d'un membre n'avaient pas de borne.

## Décision

1) Les totaux de coûts sont refusés en entier à la lecture hors bornes (coût fini, positif, ≤ 10 000 $ par jour et par appareil ; ≤ 10 milliards de tokens par jour ; ≤ 62 jours, 256 lignes, 1 024 tickets ; jour en date) ou s'ils nomment un autre membre que leur signataire ; la file du poste refuse de même, un résumé non fini est refusé. 2) Chez un membre, un souvenir d'un autre membre que seul son auteur a gardé est une donnée de tiers (marquée, avec son auteur), jusqu'à ce qu'un autre humain (un administrateur) le garde. 3) La machine de l'équipe ne prend d'elle-même un agent passé que selon `handoff.machineTakes` du fichier d'équipe (`admins` d'office, `members`, `nobody`) ; le reste attend « Prendre » dans sa boîte « À toi ». La branche d'un passage doit être un nom de branche et ne jamais écraser une branche locale autre que celle du ticket ; elle se récupère d'un remote du dépôt, jamais d'une adresse de l'offre, avec `--`. 4) `permissions.defaultMode: auto` venu de l'équipe se tient comme une clé sensible (quorum, accord de chaque poste), dit une fois par empreinte dans « À toi ». 5) Le maxLoops d'un ticket d'équipe est ramené à 3, 5 ou 8.

## Conséquences

Un administrateur qui veut la machine pour tous les membres l'écrit dans le fichier d'équipe. Une mémoire d'équipe gardée par son seul auteur n'est plus une consigne chez les autres : l'administrateur la garde pour qu'elle le devienne. Les postes gardent leur mode tant qu'ils n'ont pas accepté « auto ». Limites : sous les bornes, un membre peut encore déclarer une dépense fausse (à son appareil) ; ce que passe un administrateur, la machine l'exécute.
