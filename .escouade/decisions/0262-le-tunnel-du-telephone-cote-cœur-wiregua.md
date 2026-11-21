---
number: 262
id: "le-tunnel-du-telephone-cote-cœur-wiregua"
title: "Le tunnel du téléphone côté cœur : WireGuard en espace utilisateur, l'API dans le tunnel seulement, STUN pour l'adresse publique, servers.phone"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-567"
pages: ["a-distance/tunnel-du-telephone","a-distance/api-distante","fenetre/reglages-imposes","fenetre/reglages","a-distance/mode-serveur"]
tickets: ["ESC-567"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/tunnel-du-telephone","added":38,"removed":0},{"page":"a-distance/api-distante","added":7,"removed":0},{"page":"fenetre/reglages-imposes","added":7,"removed":0},{"page":"fenetre/reglages","added":2,"removed":0},{"page":"a-distance/mode-serveur","added":11,"removed":0}]
accepted_at: "2026-11-21T19:50:31+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MO-2 (docs/MOBILE.md § 8) construit le côté cœur du tunnel du téléphone que M12 (décision 230) a retenu : WireGuard intégré au poste et à escouade-server, plus Tailscale quand il est là. Il fallait trancher comment l'API est servie au téléphone sans rien ouvrir d'autre, comment savoir l'adresse publique et le CGNAT, ce qu'une stratégie du poste coupe, et comment faire l'essai réel en 4G avant l'appairage de MO-4.

## Décision

1. Un pair WireGuard en espace utilisateur (boringtun sans sa feature device, smoltcp en IPv4) sur un port UDP du process, 51820 par défaut ; le cœur est 10.77.0.1/24, chaque téléphone une adresse de .2 à .254 (son AllowedIPs) ; une clé inconnue n'a aucune réponse. 2. L'API est servie dans le tunnel sur 10.77.0.1:80 ; sur le poste, quand l'accès est allumé, l'API n'a aucune socket de la machine (Listen::Nothing). Dans le tunnel, le Host doit être l'adresse du tunnel, l'Origin celle de la fenêtre ou de Capacitor (refusée sur 127.0.0.1), et le jeton d'appareil reste demandé ; un pair appairé à travers le tunnel ne prend plus que le jeton de son appareil, et la révocation de l'appareil retire le pair. 3. La box : UPnP (igd-next) puis NAT-PMP écrit à la main, une heure renouvelée, retirée à l'extinction ; jamais sur un serveur. L'adresse publique par STUN (stun.cloudflare.com puis stun.l.google.com : une requête de liaison), relue toutes les cinq minutes, son changement gardé et dit ; un CGNAT quand l'adresse de la box est partagée ou privée, ou n'est pas l'adresse publique. 4. Tailscale détecté par sa commande ou la route vers 100.100.100.100 : l'API aussi servie en TCP sur son adresse, au même numéro de port, pour une machine du tailnet seulement ; allumé par défaut, coupable. 5. Une stratégie servers.phone (restreindre, éteint seulement) en plus de servers.remoteApi ; l'une ou l'autre éteint un tunnel allumé aussitôt, à la différence des connexions de l'API. 6. L'état dans phone.json (lisible du seul propriétaire), la clé privée du tunnel au trousseau ou dans le fichier des secrets d'un serveur. 7. escouade-server phone (status, on, off, add, remove) : add affiche une config WireGuard et son QR code pour l'app WireGuard officielle, la clé privée du téléphone montrée une fois et gardée nulle part ; c'est le chemin de l'essai en 4G avant MO-4.

## Conséquences

Aucun droit d'administrateur ni pilote ; seul le port UDP est exposé. Une requête STUN part vers Cloudflare ou Google à chaque regard sur le réseau tant que l'accès est allumé (l'adresse IP de la machine, rien d'autre) ; un poste géré coupe tout par servers.phone. MO-3 reprend la pile (stack.rs) côté téléphone, MO-4 fait le QR code d'appairage qui lie le pair à l'appareil. Restent un nom DynDNS, le pare-feu IPv6 de la box ouvert par Escouade, et la veille du poste.
