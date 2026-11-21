---
number: 233
id: "app-mobile-m3-ce-qui-demande-le-verrou-b"
title: "App mobile, M3 : ce qui demande le verrou biométrique (proposé : chaque réponse qui fait agir)"
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
accepted_at: "2026-11-20T13:56:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un téléphone se prête, se perd ou reste déverrouillé sur une table. REMOTE.md prévoit un verrou biométrique « possible » avant de répondre à une autorisation ; il faut dire lesquelles des réponses du téléphone le demandent (docs/MOBILE.md § 3.9, M3).

## Décision

À trancher par l'utilisateur. Proposé : le verrou du système (empreinte ou visage, le code de l'appareil en recours) avant chaque réponse qui fait agir un agent ou la machine : « Autoriser » une commande, « Approuver le plan », « Oui » à un feu vert, « Valider » un ticket (un merge suit), « Lancer » un ticket ; une fois par réponse, sans fenêtre de grâce. Sans verrou : refuser, répondre à une question, choisir une option d'un choix, envoyer un message, « Plus tard », renvoyer un ticket. Pas d'appairage sans verrou d'écran. « Verrouiller à l'ouverture » en option, éteint par défaut. Autres options : tout sous verrou ; seulement les autorisations.

## Conséquences

Personne ne fait lancer une commande avec un téléphone pris déverrouillé ; un choix de design ou une réponse à une question restent d'un toucher. Le plugin de biométrie de Capacitor est communautaire (@capgo/capacitor-native-biometric, MOBILE-STUDY.md § 5) ; MO-4 l'intègre.
