---
number: 344
id: "serveur-d-equipe-chaque-ligne-comptee-je"
title: "Serveur d'équipe : chaque ligne comptée, jeton d'init de 24 h, révoqués hors du plafond, dernier administrateur gardé, journaux sans id"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-642"
pages: ["serveur-equipe/serveur"]
tickets: ["ESC-642"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/serveur","added":5,"removed":4}]
accepted_at: "2026-11-26T19:40:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-625) a relevé SR2-04 (moyenne) et SR2-10 à SR2-13 (basses). Un membre pouvait faire grossir la base sans limite hors du quota, par les listes d'accès (doublons acceptés), les prises de tickets et les compteurs de projets que le serveur ne connaissait pas. Les erreurs S3 et celles de la base pouvaient écrire des ids dans le journal. Le jeton d'init n'expirait jamais. Les appareils révoqués comptaient dans les 500 de l'équipe. Le dernier administrateur pouvait être révoqué.

## Décision

1) Un projet est connu du serveur dès sa première écriture, quelle qu'elle soit (entrée, accès, prise, numéro), et compte alors dans les 10 000 projets de l'équipe. Au-delà, un projet inconnu est refusé (403 too_many). On ne refuse pas une prise ou un numéro sur un projet sans entrée, parce que le client numérote un ticket avant d'envoyer la création du projet. Une nouvelle prise compte 128 octets (ROW_BYTES) dans le quota. Une liste d'accès compte les octets de ses appareils, et chaque appareil n'y figure qu'une fois (400 duplicate_device). Le compteur et l'accès sont une ligne par projet, donc bornés par les 10 000. 2) Une erreur de la base ne passe au journal que par sa sorte : le code SQLite, ou le SQLSTATE de PostgreSQL. Une erreur S3, que par sa sorte ou son statut, jamais par l'URL. 3) Le jeton d'init dure 24 h (TEAM_TOKEN_TTL) et reste à usage unique ; init le dit à sa création. Dans une base d'avant, un jeton non servi reçoit 24 h comptées depuis sa création. 4) Les appareils révoqués restent listés mais ne comptent plus dans le plafond d'appareils. 5) La révocation du dernier administrateur non révoqué est refusée (409 last_admin, avec sa raison), en SQLite comme en PostgreSQL.

## Conséquences

Le quota d'une équipe borne tout ce que ses appareils font grossir. Une équipe existante ne voit pas compter les prises et les listes d'accès faites avant, seulement celles d'après. Pour révoquer son propre appareil, le dernier administrateur doit d'abord en nommer un autre. Un jeton d'init vieux de plus de 24 h ne sert plus : il faut relancer init.
