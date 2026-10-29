---
number: 2
title: "Le store annonce sur Discord : salon privé des admins (fr), salon public fr et en"
date: "2026-10-30"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-295"
pages: ["store/annonces-discord","store/revue-et-signature"]
tickets: ["ESC-295"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/annonces-discord","added":16,"removed":0},{"page":"store/revue-et-signature","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur (2026-10-29) veut être prévenu des revues à faire et que les nouveautés du store se voient, sur le Discord d'Escouade : un salon pour les admins seuls, le reste en public. Le contenu vient d'auteurs tiers (hostile) et une adresse de webhook est un secret.

## Décision

Trois webhooks facultatifs (STORE_DISCORD_ADMIN_WEBHOOK, STORE_DISCORD_PUBLIC_WEBHOOK_FR, STORE_DISCORD_PUBLIC_WEBHOOK_EN). Admins en français : version à revoir (avec /review et la commande plugin-review), signalement (motif et nombre seulement), nom trop proche, étiquette déplacée, retrait/badge retiré. Public fr et en : nouveau plugin (« Non vérifié »), nouvelle version, « Vérifié par Escouade », plugin retiré (raison publique). Une tâche pg-boss par message, Retry-After respecté, reprises espacées ; 5 nouveautés par auteur, jour (UTC) et salon public, puis un résumé à minuit. allowed_mentions vide, Markdown échappé, liens écrits cassés, longueurs bornées, liens vers le store seulement ; un retrait --quiet ne va pas au public ; un plugin retiré de la liste par son auteur n'est pas annoncé.

## Conséquences

Une table discord_quota (migration 0008) ; l'utilisateur crée les salons et les webhooks, le chef d'escouade les pose dans Coolify (store/README.md). Sans variable, rien n'est envoyé et le démarrage le dit. Les messages sont relus dans la base au moment d'envoyer : un événement dont l'objet a disparu n'est pas annoncé.
