---
number: 355
id: "escouade-server-ecoute-sur-une-socket-un"
title: "escouade-server écoute sur une socket Unix par défaut pour le service d'install.sh ; le port reste celui de l'image et du proxy"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-655"
pages: ["a-distance/mode-serveur","a-distance/se-connecter-a-un-serveur"]
tickets: ["ESC-655"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:59:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-655, § 4, question B3 (a-47). L'API écoute aujourd'hui sur 127.0.0.1:46480. Sur une machine où d'autres comptes existent, ces comptes atteignent ce port. Ils peuvent aussi le prendre pendant un redémarrage (mise à jour, panne) : le client de bureau, revenu par son tunnel, enverrait alors son jeton à l'usurpateur.

## Décision

escouade-server prend --socket <chemin>. Le dossier de la socket est créé en 0700, au seul utilisateur, et ses dossiers parents sont vérifiés : chacun appartient à l'utilisateur ou à root, et aucun n'est modifiable par d'autres, sauf un dossier collant de root. La socket est tenue par un verrou ; une socket morte est remplacée, une vivante jamais. La socket est en 0600. Le service de l'utilisateur posé par install.sh écoute par défaut sur $XDG_RUNTIME_DIR/escouade/server.sock. L'image Docker, le service avec un proxy et install.sh --port restent sur le port. server-status.json dit la socket, et pair, devices et health la suivent. Le client de bureau ouvre ssh -L 127.0.0.1:<port>:<socket>, avec le chemin lu par SSH dans l'état du serveur, jamais deviné. Si le serveur SSH refuse la redirection vers une socket (AllowStreamLocalForwarding no), le client classe l'erreur et propose de repasser sur le port. Choisi par l'utilisateur le 2026-11-26, avec la recommandation de l'étude.

## Conséquences

Les autres comptes de la machine n'atteignent plus l'API et ne peuvent plus prendre sa place. servers.json garde le chemin de la socket à côté du port. Le dépannage du tunnel (VS Code Remote-SSH, plages réservées de Windows, redirection vers une socket refusée) entre dans la page du site. Ticket BC-5, environ 2 h d'agent.
