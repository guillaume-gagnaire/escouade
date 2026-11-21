---
number: 271
id: "overseer-d-equipe-o4-tout-membre-lui-par"
title: "Overseer d'équipe (O4) : tout membre lui parle dans un fil commun signé, seuls les administrateurs donnent des consignes"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/overseer-equipe"]
tickets: ["ESC-580","ESC-588","ESC-615","ESC-618"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-22 (ESC-580, docs/TEAM-OVERSEER.md §6). L'Overseer d'une personne n'obéit qu'à elle : ses messages marqués, des consignes qui citent ses mots. Une équipe a plusieurs maîtres, dont un peut être un poste volé, et le serveur ne doit pouvoir ni forger ni glisser un ordre. Choix O4, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

O4-A, la recommandation. Un fil commun par équipe (ou par projet), flux chiffré par la clé de l'équipe ou du projet, chaque message signé par l'appareil de qui l'écrit et vérifié au registre ; ses réponses et ses actions y sont écrites, avec « Annuler » pour l'auteur de la demande ou un administrateur ; pas de fil privé au départ. Tout membre lui demande, dans les droits du rôle de service et de ce membre ; un lecteur lit. Seul un administrateur donne une consigne durable, avec la garde d'aujourd'hui (mots cités, négations, 24 h), administrateur au moment du message et au moment de l'action ; son auteur ou tout administrateur la retire. Seul le fil commande : un ticket, un document, un souvenir ou une réponse d'agent sont des données, même écrits par un administrateur. Écartés : B (tout membre, consignes comprises), C (les administrateurs seuls).

## Conséquences

Environ 3 h d'agent (OE-4) : le flux, la vérification, l'écran « Overseer d'équipe » dans l'espace de l'équipe, et par l'API distante pour le téléphone. La voix et l'écran restent sur le poste de qui lui parle.
