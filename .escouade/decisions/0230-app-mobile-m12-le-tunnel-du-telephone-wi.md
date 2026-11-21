---
number: 230
id: "app-mobile-m12-le-tunnel-du-telephone-wi"
title: "App mobile, M12 : le tunnel du téléphone, WireGuard intégré et Tailscale quand il est là (proposé)"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-549"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-549"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-20T13:56:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le téléphone n'a pas SSH comme le poste ; REMOTE.md retient WireGuard intégré (boringtun, smoltcp), sans VPN du système, et propose Tailscale en plus quand il est installé. Le pair WireGuard du cœur n'est pas encore construit (docs/MOBILE.md M12, MO-2).

## Décision

À trancher par l'utilisateur. Proposé : WireGuard intégré au cœur (poste et escouade-server) et à l'app, seul le trafic de l'app dans le tunnel, l'API joignable dans le tunnel seulement, derrière le jeton d'appareil ; l'adresse Tailscale de la machine quand Escouade la détecte et que l'utilisateur l'a sur son téléphone ; pas de SSH sur le téléphone. Autres options : WireGuard intégré seul ; l'app WireGuard officielle à part (aucun code de tunnel dans l'app, mais un VPN du système à allumer).

## Conséquences

MO-2 (le tunnel côté cœur : port UDP, UPnP et NAT-PMP, CGNAT, adresse publique) et MO-3 (le côté téléphone, en Rust) ; sur le poste, une redirection de port dans la box quand UPnP ne la fait pas ; sur un VPS, un port UDP dans le pare-feu.
