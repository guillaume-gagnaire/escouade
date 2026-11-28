---
number: 379
id: "escouade-server-sur-une-socket-unix-par"
title: "escouade-server sur une socket Unix par défaut (BC-5) : --socket, le service d'install.sh, le client qui la lit par SSH"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-706"
pages: ["a-distance/mode-serveur","a-distance/se-connecter-a-un-serveur"]
tickets: ["ESC-706","ESC-655"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/mode-serveur","added":31,"removed":18},{"page":"a-distance/se-connecter-a-un-serveur","added":16,"removed":11}]
accepted_at: "2026-11-28T18:35:31+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 355 (B3, ESC-655) a choisi la socket Unix par défaut pour le service d'install.sh, le port restant celui de l'image et du proxy. ESC-706 la construit. Sur une machine où d'autres comptes existent, le port de la boucle locale est joignable par tous, et l'un d'eux peut le prendre pendant un redémarrage : le client revenu par son tunnel lui enverrait son jeton.

## Décision

escouade-server prend --socket <chemin> (ESCOUADE_SERVER_SOCKET), exclusif de --port et --listen, refusé sous Windows ; dossier créé en 0700 et refusé s'il n'est pas à l'utilisateur, ouvert à d'autres ou un lien ; chaque dossier au-dessus à l'utilisateur ou à root, non modifiable par d'autres sauf un dossier collant de root ; un verrou <socket>.lock ; une socket morte remplacée, une vivante gardée ; la socket en 0600, retirée à l'arrêt propre. server-status.json dit la socket ; une commande escouade-server status [--json] la dit au client ; health vérifie qu'elle est là sans s'y connecter ; pair dit la socket et le tunnel à la main. install.sh écoute par défaut sur %t/escouade/server.sock (utilisateur) ou /run/escouade-server/server.sock (système, RuntimeDirectory 0700) ; --port N garde un port ; relancé sans --socket ni --port, il garde l'écoute de l'unité là et le dit. L'image et les exemples Compose et Coolify restent sur le port. Le client de bureau lit la socket par SSH (status --json), jamais devinée, la garde dans servers.json à côté du port, ouvre ssh -N -L 127.0.0.1:<port libre>:<socket> avec les mêmes options sûres, relit l'écoute une fois sur « Pas d'Escouade », et classe la redirection refusée (AllowStreamLocalForwarding no) comme son propre état, Why::SocketForwarding, avec le repli install.sh --port proposé.

## Conséquences

Les autres comptes de la machine n'atteignent plus l'API ni ne prennent sa place. Une lecture SSH de plus à chaque « Connexion » sans port tapé. Une installation d'avant garde son port. Les e2e ne tournant que sous Windows, la socket y est jouée par le faux ssh sur un port ; la vraie socket est essayée par les tests Rust sous Linux et server-install-test.sh sous systemd.
