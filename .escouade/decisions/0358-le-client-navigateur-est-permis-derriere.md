---
number: 358
id: "le-client-navigateur-est-permis-derriere"
title: "Le client navigateur est permis derrière un proxy HTTPS, en https seulement"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-655"
pages: ["a-distance/mode-serveur","a-distance/api-distante"]
tickets: ["ESC-655"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:59:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-655, §§ 4 et 10, question B4 (a-48). Un client navigateur sert surtout un appareil sans ssh. Derrière un proxy TLS (Caddy, Traefik de Coolify), la page doit être acceptée sous le nom du domaine. Le fork d'OshOEz a pour cela une option --origin, qui permet aussi http:// hors de la boucle locale, avec un simple avertissement.

## Décision

Quand le client navigateur est allumé (--browser), chaque nom de --allow-host (ESCOUADE_SERVER_ALLOW_HOSTS) fait accepter la page de https://<nom> : son Origin, et sa WebSocket en wss. http:// est refusé hors de la boucle locale, sans repli : la signature et l'interface passeraient en clair. Pas d'option --origin à part. Le reste ne change pas : le Host contrôlé, --trust-proxy pour compter les échecs par client, et les risques d'un proxy, que la spec dit déjà (l'API sur Internet, à filtrer par adresse). Choisi par l'utilisateur le 2026-11-26, avec la recommandation de l'étude.

## Conséquences

Un appareil sans ssh atteint le serveur par son domaine. Une fuite est bornée par la clé non extractible de l'appareil (décision sur l'identité dans le navigateur). La CSP de la page nomme wss://<nom>. Les tests de refus (http:// d'un nom déclaré, Host rebondi, Origin d'un autre port) entrent dans BC-1, et la documentation du proxy dans BC-8.
