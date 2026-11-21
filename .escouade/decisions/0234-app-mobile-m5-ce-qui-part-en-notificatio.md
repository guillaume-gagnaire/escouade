---
number: 234
id: "app-mobile-m5-ce-qui-part-en-notificatio"
title: "App mobile, M5 : ce qui part en notification, et ce qui n'y part jamais (proposé : jamais le texte d'une commande)"
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

L'app fermée ne sait rien sans notification. Même chiffrée de bout en bout (le relais ne voit qu'un réveil), une notification se lit sur l'écran verrouillé, par-dessus l'épaule. Il faut dire ce qu'elle porte (docs/MOBILE.md § 3.8, M5 et M6).

## Décision

À trancher par l'utilisateur. Proposé : part en notification un agent arrêté (« tests-e2e attend une autorisation », son projet, son espace), le résumé de la boîte « À toi » (la sorte et l'auteur, au plus toutes les 30 min, comme INBOX.md § 11), un ticket « À tester », une CI rouge sur un ticket. Ne part jamais : le texte d'une commande, le titre ou le texte d'une demande, le texte d'un message ou d'une question, un chemin, un nom de fichier joint, un nom de branche. Avec servers.contentlessNotifications : « Un agent attend ta réponse » seulement. Aucune action sur la notification (pas d'« Autoriser » depuis l'écran verrouillé) ; la toucher rouvre la demande. Regroupement (M6) : une notification par agent arrêté, groupées par espace, un résumé pour la boîte. Autre option : le titre des demandes dans le corps.

## Conséquences

Rien de ce qu'un agent écrit ne se lit sans déverrouiller ; le geste reste dans l'app, après la lecture de la commande. MO-6 (les notifications) l'applique côté cœur, avant le scellement.
