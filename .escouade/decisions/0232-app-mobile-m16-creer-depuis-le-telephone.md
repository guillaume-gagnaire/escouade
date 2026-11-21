---
number: 232
id: "app-mobile-m16-creer-depuis-le-telephone"
title: "App mobile, M16 : créer depuis le téléphone un projet sur un serveur, un agent et un ticket dicté (choisi par l'utilisateur)"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-549"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-549"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":14,"removed":13}]
accepted_at: "2026-11-20T13:56:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La conception d'ESC-549 laissait la création au poste. L'utilisateur, le 2026-11-20 : « Je pilote depuis le téléphone, comment je crée un nouveau projet ? » ; il a choisi l'option recommandée par le chef (docs/MOBILE.md § 3.12, M16).

## Décision

Le téléphone crée, en 2.3 : un projet sur un espace serveur, depuis l'adresse d'un dépôt (cloné sur le serveur, avec ses clés Git) ou un dossier du serveur, par ce que fait déjà le client de bureau (ESC-538 : clone_repo, fs_browse), le clonage suivi ; un agent avec son premier message, tapé ou dicté (create_agent), sans mode Bypass ; un ticket dicté, son titre et ses critères tirés de la dictée et relus avant de créer (ticket_create). Restent au poste : un projet sur ce poste, un dépôt neuf, l'écriture d'un ticket en entier. « Lancer l'agent » et « Créer et lancer » demandent le verrou (M3). Les autres options étaient : rien créer depuis le téléphone ; seulement un ticket dicté.

## Conséquences

Deux tickets de plus : MO-10 (créer un projet et un agent, 3 h) et MO-11 (le ticket dicté, 2 h), menés à côté de MO-4 ; la progression du clonage poussée au client si elle ne l'est pas encore. Un essai réel sur le VPS d'essai (le serveur doit avoir accès au dépôt). Captures b-creer-projet, b-creer-agent, b-ticket-dicte.
