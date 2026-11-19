---
number: 216
id: "distribuer-escouade-server-une-image-doc"
title: "Distribuer escouade-server : une image Docker sans root, install.sh pour systemd, une mise à jour signée par la clé de l'updater"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-515"
pages: ["a-distance/mode-serveur","a-distance/api-distante"]
tickets: ["ESC-515","ESC-537"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/mode-serveur","added":49,"removed":11},{"page":"a-distance/api-distante","added":5,"removed":5}]
accepted_at: "2026-11-19T10:32:16+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MS-1 (ESC-514) a livré le binaire escouade-server, sans moyen de le déployer : ni image, ni service, ni release, ni mise à jour. L'utilisateur a demandé le 2026-11-16 une image Docker prête à l'emploi pour déployer un serveur Escouade ; docs/REMOTE.md (décision 4 du mode serveur) prévoyait un binaire et systemd, puis une image. Il fallait trancher : où vivent les harness et Node dans l'image, comment l'API est atteinte sans être exposée, avec quelle clé se vérifie une mise à jour, quand l'image est publiée, et comment le client apprend que le serveur est à mettre à jour.

## Décision

1. L'image ghcr.io/guillaume-gagnaire/escouade/escouade-server (linux/amd64, linux/arm64) : Debian 13 mince, le programme et l'amorce des plugins, git, openssh-client, curl, tini en PID 1, Node.js 22 et npm dans l'image ; l'utilisateur escouade (uid 10001), sans sudo ; HOME=/data, tout ce qu'elle écrit dans le volume (données, projets, harness et leurs connexions), système de fichiers en lecture seule, aucune capacité. Node est dans l'image (les harness de npm le veulent, une nouvelle image le met à jour) ; les harness sont dans le volume (installés par la commande de leur plugin, ils survivent à une nouvelle image).
2. Par défaut l'API n'écoute que sur 127.0.0.1 du conteneur. Les exemples (deploy/docker-compose.yml, deploy/coolify/docker-compose.yml) mettent ESCOUADE_SERVER_LISTEN=0.0.0.0 et publient le port sur 127.0.0.1 de l'hôte seulement, pour un tunnel SSH. L'accès par un domaine derrière un proxy TLS passe par --allow-host (ESCOUADE_SERVER_ALLOW_HOSTS), qui ajoute ce seul nom au contrôle de Host ; il est en commentaire dans la recette Coolify, avec ses risques. Aucun exemple n'expose l'API sans TLS sur Internet.
3. Les options du serveur ont leurs variables (ESCOUADE_SERVER_DATA, _PORT, _LISTEN, _ALLOW_HOSTS) ; le serveur écrit server-status.json (son état, ce qui le tient occupé), que lisent escouade-server health (le HEALTHCHECK) et escouade-server update.
4. Les archives du serveur sont signées par la clé de l'updater de l'app (format de Tauri) : escouade-server update et install.sh la vérifient avant de rien toucher (install.sh avec minisign, sinon openssl). update attend que le serveur soit au repos (updates::busy, comme le redémarrage automatique de l'app), ou --now ; systemd (Restart=always) le relance. Dans l'image, update refuse : la mise à jour est une nouvelle image.
5. install.sh pose un service de l'utilisateur (loginctl enable-linger) ou du système (--system, utilisateur escouade qui possède le programme) ; idempotent ; --uninstall garde les données, --purge les retire.
6. La release construit chaque architecture sur son runner (Ubuntu 22.04), signe et vérifie l'archive, pousse l'image de l'architecture par son empreinte seule, et n'étiquette l'index (version et latest, beta pour une bêta) qu'après la publication de la release, avec une attestation de provenance au registre.
7. Le refus de version de la poignée de main porte serverUpdate quand le serveur est le plus ancien : la commande escouade-server update --to <version du client>, ou la nouvelle image dans Docker. Le client du mode serveur (ESC-537) le porte dans son Problem::Version et la fenêtre dans connection.refusal.serverUpdate ; l'écran qui s'en sert reste à faire.

## Conséquences

Un serveur se déploie en trois commandes Docker, par Compose ou Coolify, ou par install.sh sans Docker ; il ne s'expose jamais par défaut. L'image pèse 427 Mo décompressée (≈ 150 Mo à télécharger), Node compris. La clé de l'updater signe désormais aussi le serveur : la perdre bloquerait aussi ses mises à jour. Les identifiants des comptes vivent sur le serveur : la doc, install.sh et la FAQ le disent. Le chemin de publication (poussée au registre, index, attestation) ne se prouve qu'au premier tag ; l'essai à blanc couvre l'archive, sa signature et l'image construite. Restent au client : « Se connecter à un serveur », l'écran « Mettre à jour le serveur », une adresse wss:// d'un domaine.
