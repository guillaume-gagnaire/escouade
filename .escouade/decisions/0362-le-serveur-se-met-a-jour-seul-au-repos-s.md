---
number: 362
id: "le-serveur-se-met-a-jour-seul-au-repos-s"
title: "Le serveur se met à jour seul, au repos, sur son canal, et le poste peut le lui demander"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-702"
pages: ["a-distance/mode-serveur","a-distance/api-distante"]
tickets: ["ESC-702"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/mode-serveur","added":27,"removed":10},{"page":"a-distance/api-distante","added":9,"removed":8}]
accepted_at: "2026-11-28T07:21:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur a demandé le 2026-11-27, pour les bêtas de la 2.3 en mode serveur : « Faudra gérer évidemment l'auto update du binaire serveur ». Jusqu'ici, escouade-server ne se mettait à jour que par `escouade-server update`, lancé à la main sur le serveur. Le client, refusé à cause de sa version, ne faisait que dire la commande. Et l'app se met à jour toute seule toutes les 5 minutes : un serveur qui reste en arrière bloque son client.

## Décision

1. escouade-server cherche une version 2 minutes après son démarrage, puis toutes les 15 minutes. Il cherche aussi tout de suite quand un de ses réglages change, quand un client plus récent est refusé avec un jeton valide, et quand un administrateur le demande. Il passe par les fonctions de `update` (version visée, téléchargement, signature liée au nom de l'archive, pose avec les .old), sans chemin à part.
2. Ses réglages sont ceux de l'app, sur le serveur : « Installer les mises à jour automatiquement » (`autoUpdate`) est allumé par défaut, comme dans l'app ; « Recevoir les versions bêta » (`betaUpdates`) est éteint par défaut. Une bêta installée (`install.sh --version X.Y.Z-beta.N`), ou `update --beta`, met le serveur sur le canal bêta. `escouade-server config automatic on|off / channel stable|beta` les change, et le client aussi, depuis Réglages › Général de l'espace. Les stratégies `updates.automatic` et `updates.channel`, du poste ou d'une équipe, les bornent comme pour l'app.
3. Sous systemd sous Linux, il pose la version, puis ne redémarre qu'au repos (`updates::busy`), et systemd le relance. Il ne prend jamais une version plus ancienne que la sienne. Une signature refusée n'est jamais retéléchargée d'elle-même. Une version qui n'a pas pris au redémarrage n'est plus réessayée. Sans systemd, l'automatique éteint ou hors de Linux, il dit seulement la version et la commande. Dans l'image Docker, rien n'est remplacé : il dit qu'une nouvelle image existe, avec son `docker pull`.
4. « Mettre à jour le serveur » : sur le refus de version, le poste envoie `updateServer` (jeton, version) à la place de la poignée de main. Connecté, il appelle `server_update_start`. Les deux sont réservés à un appareil de la portée poste, jamais un téléphone, et passent après une confirmation. Une erreur se dit une fois, sans nouvelle demande. Dans l'image, l'écran donne la commande `docker pull`.
5. Un build de développement ne lit aucune vraie release de lui-même, seulement `ESCOUADE_SERVER_UPDATE_URL`, comme l'app (ESC-543).

## Conséquences

Un serveur se tient à jour sans geste de l'utilisateur, et l'écart de version avec un client qui vient de se mettre à jour reste court. Il ne coupe jamais un tour, une question ou une tâche de fond. Il coûte une lecture de latest.json (quelques centaines d'octets) par quart d'heure et par serveur. Sur un serveur, « Installer les mises à jour automatiquement » et « Recevoir les versions bêta » de Réglages › Général deviennent ceux du serveur. Les mises à jour du poste ne s'y montrent plus quand la fenêtre est sur le serveur. Un administrateur qui veut garder la main éteint l'automatique (`config automatic off`), ou une stratégie le fait pour lui. Le téléphone n'y a pas accès.
