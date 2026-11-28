---
number: 369
id: "les-instantanes-ne-detruisent-plus-rien"
title: "Les instantanés ne détruisent plus rien : rétention de 30 jours, instantané illisible écarté, retrait par un administrateur"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-637"
pages: ["serveur-equipe/synchronisation","serveur-equipe/serveur"]
tickets: ["ESC-637","ESC-625"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/synchronisation","added":13,"removed":5},{"page":"serveur-equipe/serveur","added":9,"removed":3}]
accepted_at: "2026-11-22T16:46:30+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-625, SR2-01, haute) a montré qu'un instantané de projet était accepté de tout appareil ayant accès au projet, y compris sur les flux de l'équipe (liste des projets, réglages imposés, store privé, clé des coûts), et que le serveur effaçait aussitôt les messages qu'il couvrait. Un seul membre pouvait donc effacer les entrées signées des administrateurs ou toute l'histoire d'un projet, sans retour possible. Le serveur ne lit pas les instantanés : il ne peut pas savoir si un instantané est bon.

## Décision

1. Le serveur refuse tout instantané sur les flux de l'équipe (403), à tous. 2. Poser un instantané n'efface plus rien : les messages couverts restent 30 jours (`snapshot_retention`), puis le balayage les oublie, avec les instantanés d'avant, seulement si aucun appareil n'a signalé l'instantané comme illisible. Le serveur garde plusieurs instantanés par projet tant que la rétention court. 3. Un appareil qui part d'un instantané, ou qui est déjà au-delà, le vérifie. S'il ne s'ouvre pas, il l'écarte sans arrêter l'équipe : il le signale au serveur, note une alerte qui nomme l'appareil qui l'a posé, puis lit l'instantané d'avant ou le journal. 4. Les administrateurs lisent la liste des instantanés signalés et en retirent un (`DELETE /v1/projects/{id}/snapshot?upto=n`) ; le serveur refuse de retirer celui sur lequel le flux a déjà été élagué.

## Conséquences

Le serveur garde jusqu'à 30 jours d'opérations en plus, dans le quota de l'équipe. Un instantané illisible n'est plus une alarme « journal » : c'est une alerte dans la carte de l'équipe, avec « Retirer l'instantané » pour un administrateur et « Masquer » pour un membre. Le format de la base change (table `snapshots`, colonne `projects.pruned`) : une base d'avant est migrée, ses instantanés tenus pour élagués. Un membre qui signale à tort un bon instantané ne fait que retarder l'élagage et prévenir les administrateurs.
