---
number: 328
id: "les-echecs-de-l-api-distante-comptes-par"
title: "Les échecs de l'API distante comptés par source et par portée, le proxy déclaré, l'origine propre de l'app Android par Tailscale"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-631"
pages: ["a-distance/api-distante","a-distance/mode-serveur","a-distance/tunnel-du-telephone"]
tickets: ["ESC-631"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":7,"removed":5},{"page":"a-distance/mode-serveur","added":7,"removed":7},{"page":"a-distance/tunnel-du-telephone","added":4,"removed":4}]
accepted_at: "2026-11-26T01:16:25+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-624) a relevé SR1-04 : le compte des échecs de l'API distante était global, et un inconnu qui joignait l'API (tailnet, réseau Docker partagé, domaine derrière un proxy) fermait la porte au propriétaire et brûlait le code d'appairage. ESC-654 a déjà rendu le bon jeton toujours admis, compté les échecs par adresse et fait qu'un code faux n'annule jamais le code. Restaient : un plafond global des codes faux que n'importe quelle source pouvait atteindre, y compris contre les codes tapés sur la machine ; derrière un proxy, tous les clients d'Internet comptés comme une seule source ; et SR1-09 : par Tailscale, les origines génériques http(s)://localhost admises, qu'une page web d'une autre machine du tailnet peut avoir.

## Décision

1. Le plafond des codes d'appairage faux (120 par minute) se compte à part pour cette machine (la boucle locale : sa fenêtre, escouade-server pair, un client par SSH) et pour ailleurs (tunnel, Tailscale, réseau de --listen, clients d'un proxy) : les codes faux venus d'ailleurs ne refusent jamais un code tapé sur la machine, et la fenêtre le dit (toast) en plus du journal. Le code n'est jamais annulé par un code faux : c'est la forme retenue, plus sûre qu'une annulation par source ou qu'une annulation annoncée, puisqu'un inconnu ne peut ni brûler le code ni bloquer l'utilisateur sur sa machine, à son insu ou non.
2. escouade-server --trust-proxy <adresse> (ESCOUADE_SERVER_TRUST_PROXIES) déclare un proxy : d'une connexion de cette adresse, la dernière adresse de X-Forwarded-For (celle que le proxy écrit) est la source, comptée « ailleurs » ; d'une autre adresse, l'en-tête n'est jamais lu. Une adresse IP par proxy (pas de plage).
3. Par Tailscale, seules les origines de l'app du téléphone (capacitor://localhost sous iOS, https://app.escouade.invalid sous Android) et celle de la fenêtre passent. L'app Android prend ce nom propre (sous .invalid, réservé, RFC 6761) par un crochet capacitor:copy:after qui ne change que la configuration copiée dans le projet Android ; iOS garde capacitor://localhost. Par le tunnel WireGuard, les anciennes origines http(s)://localhost restent admises (chaque pair est un téléphone).

## Conséquences

La force brute d'un code passe de 1 200 à 2 400 essais lus au plus pendant ses 10 minutes (une chance sur 450 millions environ, au lieu de 900 millions) : jugé acceptable pour que la machine garde toujours son appairage. Un proxy posé sur la machine même doit être déclaré, sans quoi ses clients comptent comme cette machine. L'app Android repart d'un stockage local vide (nouvelle origine) ; elle n'est pas publiée. L'essai réel d'un Android par Tailscale reste à faire par l'utilisateur (dans sa boîte « À toi »). La recette Coolify propose ESCOUADE_SERVER_TRUST_PROXIES en commentaire.
