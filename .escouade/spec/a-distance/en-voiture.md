---
title: "Escouade en voiture (brouillon, ESC-696)"
subject: "a-distance"
sources: ["docs/CAR.md (ESC-696 : l'étude, ses sources officielles lues le 2026-11-27, les choix C1 à C8, le découpage CAR-1 à CAR-6)","docs/MOBILE.md (M3 le verrou, M5 ce qui part en notification, MO-6 les notifications, le plugin natif)","docs/OVERSEER.md (l'autorité du chef)","docs/REMOTE.md (l'API distante, overseer_send et overseer_view servis)","Demande de l'utilisateur du 2026-11-27, relayée par le chef : « Est-ce qu'il y a un monde où l'app mobile Android est compatible Android Auto en mode overseer ? », puis « Étude pour la 2.4, avec Apple aussi »","developer.android.com : Extend messaging notifications to Android Auto ; Build templated messaging experiences ; Car app quality ; Distribute to cars ; Connection API ; Test using the Desktop Head Unit (lus le 2026-11-27)","developer.apple.com : CarPlay Developer Guide (octobre 2026) ; Requesting CarPlay entitlements ; SiriKit Messaging ; Implementing communication notifications ; Using the CarPlay simulator (lus le 2026-11-27)"]
tickets: ["ESC-696","ESC-572"]
decisions: []
compared: null
draft: true
---

> **Brouillon, rien n'est tranché.** Étude d'ESC-696 pour la 2.4 (`docs/CAR.md`). Les choix C1 à C8 attendent l'utilisateur dans la boîte « À toi » ; cette page dit ce que l'étude recommande, pas ce que fait le produit.

## Ce que c'est

Dans la voiture, le chef d'escouade (l'Overseer) parle à l'utilisateur : ses messages sont lus à voix haute par l'assistant de la voiture, l'utilisateur répond en dictant. Pas d'écran de l'app ni de Kanban : Android Auto et CarPlay n'acceptent que des catégories fixées. Le chef y est **un contact de messagerie**.

## Comment (recommandé)

- **Android Auto** : la messagerie par notifications (`MessagingStyle`, réponse par `RemoteInput`, « marquer comme lu », `automotive_app_desc.xml` avec `notification`). La messagerie à modèles de la Car App Library (`ConversationItem`, Car API 7) reste en bêta, limitée au test interne et fermé : pas dans la 2.4. Android Automotive OS : hors de la 2.4.
- **CarPlay** : la catégorie de la communication (SiriKit : envoyer, chercher et lire des messages ; l'annonce des notifications par Siri), demandée à Apple ; la conversation à la voix (iOS 26.4) en recours.
- **Une seule conversation**, celle du chef, qui résume les agents arrêtés, les nouvelles et la boîte « À toi ». Le chef écrit pour l'oreille : une à trois phrases, ni tableau, ni code, ni chemin ; une garde du cœur retire ce qui ne se lit pas.
- **Le cœur sait que l'utilisateur conduit** par le téléphone (`CarConnection` sous Android, la route audio et la scène CarPlay sous iOS), qui le lui dit ; l'état s'éteint seul.

## La sécurité

- **Un mode voiture éteint par défaut**, par espace. Le push reste celui de M5, **sans texte** ; seulement pendant la projection, le natif va chercher le message du chef par le tunnel et le pose dans la notification, masquée sur l'écran verrouillé, retirée en fin de projection. **Jamais sous `servers.contentlessNotifications`**.
- **Aucun feu vert depuis la voiture** : publier, signer, supprimer, autoriser une commande, approuver un plan, valider un ticket qui fusionne (et, recommandé, lancer). Un message dicté en voiture est **marqué** (`origin: car`, posé par le natif) ; le cœur refuse ces gestes à un tour du chef qu'il a déclenché, et le chef les dépose dans « À toi », pour le téléphone, sous verrou. Aucune consigne durable ne s'ajoute depuis la voiture.
- Les clés restent dans le natif (Keystore, Trousseau partagé avec les extensions) ; jamais la WebView.

## Le découpage (estimé)

CAR-1 le cœur (4 h), CAR-2 le Rust du téléphone (2 h), CAR-3 Android Auto (5 h), CAR-4 CarPlay (5 h), CAR-5 UnifiedPush en option (1 h), CAR-6 la publication (1,5 h) : environ 17,5 h d'agent, de l'ordre de 11 h de bout en bout, **après MO-6** (ESC-572). Restent à l'utilisateur : ses choix, la demande d'autorisation CarPlay à Apple, le facteur de forme Android Auto dans la Play Console (au test fermé, la revue ne bloque pas), et l'essai en voiture.
