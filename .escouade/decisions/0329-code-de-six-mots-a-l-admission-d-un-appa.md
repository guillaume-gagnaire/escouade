---
number: 329
id: "code-de-six-mots-a-l-admission-d-un-appa"
title: "Code de six mots à l'admission d'un appareil"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-634"
pages: ["serveur-equipe/entree"]
tickets: ["ESC-634","ESC-633","ESC-625"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/entree","added":21,"removed":10}]
accepted_at: "2026-11-26T02:09:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-625, SR2-08, haute) : ESC-633 ôte le secret du lien des pages du serveur, mais un lien qui fuit ailleurs (courriel transféré, messagerie, capture d'écran), ou un serveur qui l'aurait lu, laisse encore entrer qui l'a à la place de l'invité : l'appareil de l'administrateur admettait de lui-même toute demande dont la preuve du secret passait, à chaque lecture. Le serveur aurait alors les clés de l'équipe.

## Décision

Aucun appareil n'entre dans une équipe sans que la personne qui invite confirme six mots montrés aux deux écrans. Les six mots sont tirés de l'identifiant de l'invitation et du KeyPackage MLS de l'invité tel qu'il est parti, signé par sa feuille ; l'appareil de l'invité les montre (« Code de cet appareil »), celui de qui invite les calcule lui-même après avoir vérifié la signature et la preuve du secret (« Demandes d'entrée »). « Les mots sont les mêmes » fait entrer, avec les mots montrés ; « Les mots diffèrent » refuse la demande, annule l'invitation, coupe du serveur l'appareil qui a demandé (quand qui refuse en est administrateur) et prévient les administrateurs par une alerte scellée dans le flux de l'équipe. L'appareil en ligne n'admet plus rien de lui-même ; la confirmation est un geste sur l'ordinateur, refusé à distance (API distante, téléphone), absent du MCP et de l'Overseer d'équipe. Cela vaut pour un nouveau membre comme pour un autre appareil d'un membre (QR code, code court). La récupération, où les deux appareils sont sur la même machine, lit elle-même les mots du nouvel appareil.

## Conséquences

Un lien qui fuit ne donne plus qu'une demande que la personne qui invite refuse ; le serveur ne peut mettre un autre KeyPackage à la place de celui de l'invité sans que les mots diffèrent. En contrepartie, une admission demande que les deux personnes se parlent (de vive voix, en personne) au moment où l'invité entre : l'administrateur ne fait plus entrer un invité en son absence. Les tests et e2e qui comptaient sur l'admission automatique passent par la confirmation. Une demande ancienne en attente au moment de la mise à jour n'entre pas seule : elle apparaît avec ses six mots.
