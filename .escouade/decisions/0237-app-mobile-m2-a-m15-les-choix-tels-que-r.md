---
number: 237
id: "app-mobile-m2-a-m15-les-choix-tels-que-r"
title: "App mobile, M2, M4, M6, M9, M13, M14 : les choix tels que recommandés (choisis par l'utilisateur)"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-549"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-549"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":6,"removed":5}]
accepted_at: "2026-11-21T08:58:59+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La conception d'ESC-549 (docs/MOBILE.md § 7) laissait M2 à M15 à trancher. Une seule question a été posée dans la boîte « À toi » (a-11) : « Les autres choix (M2 à M15) tels que recommandés ? » ; l'utilisateur a répondu oui le 2026-11-20. Les choix structurants ont déjà chacun leur décision acceptée (M1 227, M10 228, M11 229, M12 230, M15 231, M16 232, M3 233, M5 234, M7 235, M8 236) ; celle-ci tient les six autres, qui n'en avaient pas.

## Décision

Choisis par l'utilisateur, tels que recommandés : M2 la navigation de B (trois onglets À toi, Agents, Kanban ; des puces d'espace ; le « + » de l'en-tête pour créer ; le retour du système partout) ; M4 toucher « Autoriser », actif seulement quand la commande a été vue jusqu'à sa dernière ligne, puis le verrou du système ; M6 une notification par agent arrêté, groupées par espace (thread-id sous iOS, un groupe sous Android), un résumé pour la boîte, rien quand l'app est ouverte sur la demande ; M9 la dictée du système, sur l'appareil, l'audio ne part jamais au cœur ; M13 hors ligne, le téléphone ne garde que le dernier coup d'œil de chaque espace et les brouillons, jamais une conversation, une commande ni un fichier ; M14 l'aperçu du serveur de test d'un agent après la 2.3.

## Conséquences

Tous les choix de l'app mobile sont faits : les tickets MO-1 à MO-11 peuvent s'écrire sur docs/MOBILE.md § 8. Restent à l'utilisateur : les deux essais avant MO-1 (a-10), le projet Firebase réduit à FCM, la clé APNs, les comptes et les fiches des stores.
