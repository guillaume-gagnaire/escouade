---
number: 321
id: "pages-des-agents-p7-p8-fonctionnalites-e"
title: "Pages des agents P7, P8 : Fonctionnalités et Lancement et navigateurs ; la page reste sur le poste en 2.3"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-661"
pages: ["harness/pages-des-agents","projets/parties-du-projet","fenetre/fonctionnalites","a-distance/api-distante"]
tickets: ["ESC-661"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-26T00:19:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-661 : quelle partie et quel interrupteur portent les pages, et ce qu'en voient le téléphone et une fenêtre connectée à un serveur.

## Décision

Choisi par l'utilisateur le 2026-11-25 (demande a-36, recommandations acceptées). P7 : un interrupteur « Pages des agents » dans Réglages › Fonctionnalités (avec le serveur MCP), et la partie « Lancement et navigateurs » de chaque projet (outils dans `parts::of_tool`) ; écartée : une partie à elle. P8 : en 2.3, la page reste sur le poste (commandes `page_*` du poste seulement) et sa demande va partout, avec « Une page l'accompagne, à voir sur ton poste » ; l'aperçu en image (AP-10) et la page servie à une fenêtre connectée à un serveur (AP-11) viennent après. Écarté : tout dans la 2.3. Rien ne part au serveur d'équipe.

## Conséquences

AP-2, AP-8 ; la 2.3 tient sans travail sur l'API distante au-delà d'un champ `page` dans les demandes.
