---
number: 229
id: "app-mobile-m11-les-deux-plateformes-cons"
title: "App mobile, M11 : les deux plateformes construites ensemble, le test fermé de Google Play lancé d'abord (proposé)"
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

Le compte Apple existe (la signature de macOS) ; Google Play demande un compte et, pour un compte personnel récent, un test fermé de 12 testeurs pendant 14 jours avant la production (MOBILE-STUDY.md § 6). Il faut dire l'ordre (docs/MOBILE.md M11).

## Décision

À trancher par l'utilisateur. Proposé : la CI construit Android (AAB, sous Linux) et iOS (IPA, sur les Mac) dès MO-1 ; le test fermé de Google Play est lancé le premier, parce que ses 14 jours sont le plus long délai ; TestFlight en interne en même temps ; la sortie publique des deux ensemble. Autres options : Android seul d'abord ; iOS d'abord.

## Conséquences

L'utilisateur crée le compte Google Play Console tôt et recrute 12 testeurs ; MO-8 envoie aux deux stores.
