# Confidentialité de l’app mobile

L’app mobile d’Escouade parle à tes propres machines, et à personne d’autre, sauf pour réveiller ton téléphone quand un agent t’attend, si tu l’acceptes. Cette page dit ce qu’elle envoie, à qui, et comment l’arrêter.

## En bref

- L’app n’a **ni compte, ni statistique, ni publicité, ni suivi**. Elle ne contient aucun outil d’analyse ni de rapport de plantage.
- Tout ce que tu lis et envoies (les demandes de tes agents, tes réponses, tes messages, tes photos, tes dictées) passe **directement entre le téléphone et ton poste ou ton serveur**, par un tunnel WireGuard chiffré. L’éditeur d’Escouade ne le reçoit jamais.
- **La seule chose qui passe par l’éditeur** est la notification push, et seulement si tu l’acceptes à l’appairage : un réveil scellé, dont le texte est chiffré de bout en bout. Le relais ne peut pas le lire.

## Ce qui reste sur le téléphone

- **Les clés de chaque appairage** (la clé du tunnel, le jeton de l’espace, la clé des notifications) : dans le Keystore d’Android ou le Trousseau d’iOS, liés à l’appareil, exclus des sauvegardes.
- **Ce que chaque espace a dit en dernier**, pour le montrer hors ligne, et tes brouillons. Ni conversation ni fichier n’est gardé sur le disque.
- **Les permissions du système**, demandées au moment où elles servent : l’appareil photo (lire le QR code d’appairage, joindre une photo), le micro et la reconnaissance vocale (la dictée, faite par le système du téléphone), la biométrie (le verrou avant une réponse qui fait agir un agent), les notifications.

## Ce qui va à tes machines

Tes réponses, tes messages, tes photos et le texte de tes dictées vont au poste ou au serveur Escouade que tu as appairé, par le tunnel. C’est lui qui les garde, comme tout ce que fait Escouade : voir la [confidentialité d’Escouade](/privacy/).

## Les notifications

Quand tu acceptes les notifications à l’appairage :

| Qui | Ce qu’il reçoit | Pourquoi |
|---|---|---|
| Ton poste ou ton serveur | Le texte de la notification, qu’il scelle avec la clé de l’appairage | Il sait ce qui t’attend |
| Le relais de l’éditeur (`push.escouade.dev`) | Un réveil scellé, un numéro d’appairage tiré au hasard, le jeton de push du téléphone et sa plateforme | Passer le réveil à Google ou à Apple |
| Google (Firebase Cloud Messaging, sous Android) | Le jeton de push, l’identifiant d’installation de Firebase, l’identifiant et la version de l’app, ce qu’il faut de l’appareil pour la remise, et les métadonnées du message (taille, heure, priorité) | Remettre le réveil au téléphone |
| Apple (APNs, sous iOS) | Le jeton de push et les métadonnées du message | Remettre le réveil au téléphone |

Personne d’autre que ton téléphone ne lit le texte. De Firebase, l’app n’utilise que la remise des messages (FCM) : ni Analytics, ni Crashlytics, ni base de données, et rien ne part chez Google avant que tu aies accepté les notifications. Sous iOS, l’app ne contient aucun SDK de Firebase.

## Arrêter et effacer

- **Couper les notifications** : dans les réglages de l’app ou du téléphone. Sans elles, rien ne passe par l’éditeur, Google ou Apple.
- **Révoquer un téléphone** depuis ton poste (Réglages › Téléphone) : son tunnel, son jeton et son inscription au relais sont effacés. Retirer un espace depuis l’app fait de même.
- **Désinstaller l’app** efface ce qu’elle garde sur le téléphone.

## Tes droits et le contact

Le relais ne garde que la table des appairages qui ont accepté les notifications (le numéro tiré au hasard, la plateforme, le jeton de push), effacée à la révocation. Pour toute question sur tes données, ou pour exercer tes droits (accès, effacement, opposition), écris à l’adresse donnée dans les [mentions légales](/legal/). Tu peux aussi saisir la CNIL.
