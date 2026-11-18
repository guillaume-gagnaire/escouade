---
number: 212
id: "le-client-du-mode-serveur-passe-par-le-s"
title: "Le client du mode serveur passe par le ssh du système, une fenêtre sur une machine à la fois"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-537"
pages: ["a-distance/se-connecter-a-un-serveur","a-distance/mode-serveur","fenetre/reglages"]
tickets: ["ESC-537"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/se-connecter-a-un-serveur","added":59,"removed":0},{"page":"a-distance/mode-serveur","added":7,"removed":6},{"page":"fenetre/reglages","added":2,"removed":0}]
accepted_at: "2026-11-18T18:45:36+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le mode serveur (MS-1, ESC-514) a fait escouade-server et son appairage ; il manquait le client : « Se connecter à un serveur » (MS-3 de docs/SERVER-UX.md). Il fallait choisir comment le poste joint le serveur, où garder le jeton, comment la fenêtre passe d'un cœur à l'autre, et ce que fait l'interface quand la connexion tombe. L'utilisateur a tranché le 2026-11-18 : S1 = B « les piles » (pour les espaces multiples), S4, S10, S11, S15 et S16 à leur réponse par défaut.

## Décision

- Le poste joint le serveur par le ssh du système (OpenSSH de System32 sous Windows) : ssh -N -L 127.0.0.1:<port libre>:127.0.0.1:<port du serveur> <hôte>, avec la config, l'agent et les known_hosts de l'utilisateur. BatchMode=yes et StrictHostKeyChecking=yes sont imposés sur la ligne de commande : Escouade n'accepte jamais une clé d'hôte ni ne tape de mot de passe ; une clé inconnue est un état que l'utilisateur tranche par ssh <hôte>. Aucune clé n'est lue ni copiée.
- Les hôtes proposés viennent de ~/.ssh/config (Include, ProxyJump, première valeur gagnante, sans motifs ni hôtes sans HostName).
- Le jeton d'appareil est gardé au trousseau du poste, une entrée par serveur ; sans trousseau (Linux), dans un fichier 0600 des données de l'app. Jamais dans les réglages ni le journal.
- L'appairage : le code tapé, ou « le lancer d'ici par SSH » (escouade-server pair lancé par le même hôte), les deux proposés (S4).
- Une fenêtre est sur une machine à la fois ; passer d'une machine à l'autre recharge la fenêtre sur l'autre cœur, qui lui dit où se connecter (server_link). L'app rouvre sur la dernière machine.
- Pendant une coupure, l'envoi est suspendu et le brouillon gardé, rien ne part en file (S10) ; le tunnel est relancé aux délais de la fenêtre, une clé ou un compte refusé l'arrête.
- Une autre version est refusée et dit le côté à mettre à jour (S11) ; « Retirer » révoque ce poste sur le serveur s'il est joignable (S15) ; servers.allowed refuse un serveur avant toute connexion (S16).
- La machine se dit dès qu'un serveur est connu : barre de titre, barre d'état (tunnel et latence), rail des projets (direction B : l'espace ouvert en tête, l'autre sous le « + »), pied de la barre latérale, en-tête d'un terminal, « Nouveau projet ». Réglages › Serveurs liste les serveurs et leur état.
- Le serveur donne en plus : la machine dans le welcome, ping/pong, la version dans le refus d'un jeton, la raison d'un code refusé, une trace datée des appareils révoqués (sans jeton), et unpair.

## Conséquences

- La fenêtre permet désormais ws://127.0.0.1:* dans sa CSP en release (et rien d'autre en WebSocket).
- Treize commandes du poste (desk) s'ajoutent à la liste unique (server_*, servers_*).
- Pas de palette de commandes dans l'app : l'entrée est dans Réglages › Serveurs et dans le menu du « + » (dès qu'un serveur est connu ou un projet fermé) ; le tray n'en a pas encore.
- Sur un serveur, « Nouveau projet » cache le sélecteur de dossier du poste (le chemin se tape) jusqu'au navigateur des dossiers du serveur (MS-4). La mise à jour du serveur depuis le client reste à faire : le dialogue de version dit quelle version installer.
- Plusieurs serveurs à la fois, leurs piles et la vue « Partout » restent à MS-5.
