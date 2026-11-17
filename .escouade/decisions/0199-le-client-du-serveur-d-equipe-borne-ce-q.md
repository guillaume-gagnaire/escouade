---
number: 199
id: "le-client-du-serveur-d-equipe-borne-ce-q"
title: "Le client du serveur d'équipe borne ce qu'il lit, et le dossier proposé au clonage n'est jamais caché"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-519"
pages: ["serveur-equipe/synchronisation","serveur-equipe/projets-equipe"]
tickets: ["ESC-519"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/synchronisation","added":2,"removed":0},{"page":"serveur-equipe/projets-equipe","added":2,"removed":1}]
accepted_at: "2026-11-17T16:20:43+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-18 (ESC-519) a fuzzé ce que le client et le serveur d'équipe lisent sans l'avoir vérifié, et relu ce que le serveur accepte. Deux défauts sont sortis : le client lisait une réponse du serveur sans borne (le corps HTTP entier, un message du direct jusqu'aux 64 Mio de tungstenite), alors que le serveur est hors de confiance (§3.1) ; et le nom de dossier que l'assistant propose pour cloner un projet de la liste reprenait la fin de l'adresse telle quelle, si bien qu'une adresse « …/.ssh » proposait le dossier caché « .ssh » du dossier parent (trouvé par la cible « catalog »).

## Décision

Le client refuse une réponse du serveur de plus de 32 Mio (dès la longueur annoncée, sinon dès qu'elle la dépasse) et ferme la connexion en direct sur un message de plus de 4 Mio. Le nom de dossier proposé pour un clone, quand il vient de l'adresse, perd ses points et ses tirets de tête (« projet » s'il ne reste rien).

## Conséquences

Un serveur d'équipe qui ment ne fait plus allouer l'appareil sans borne ; une page d'un flux (8 Mio au serveur par défaut), un fichier (25 Mio) ou un instantané passent toujours. Un projet de la liste dont le dépôt est « .github » se propose « github » : le membre peut toujours taper le nom qu'il veut. Rien ne change pour l'utilisateur tant que le serveur est honnête ; aucune ligne de CHANGELOG, le serveur d'équipe n'étant pas encore publié.
