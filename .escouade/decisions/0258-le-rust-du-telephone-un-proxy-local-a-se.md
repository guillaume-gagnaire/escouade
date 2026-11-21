---
number: 258
id: "le-rust-du-telephone-un-proxy-local-a-se"
title: "Le Rust du téléphone : un proxy local à secret, et le format v1 des notifications scellées avec sa garde contre le rejeu"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-568"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-568"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":11,"removed":4}]
accepted_at: "2026-11-21T14:35:12+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MO-3 (ESC-568) fait le côté téléphone du tunnel (M12) et l'ouverture des notifications scellées (M7, § 3.11 de docs/MOBILE.md), dont le format restait à définir pour que MO-6 scelle côté cœur ce que le téléphone ouvre. Deux questions de sécurité se posaient en chemin. (1) La WebView ne parle pas à la pile TCP/IP du tunnel : il lui faut un proxy local sur 127.0.0.1, or sur un téléphone toute app joint 127.0.0.1, et passerait donc par le tunnel jusqu'à l'API. (2) Une notification scellée doit être refusée si elle est altérée, sous une autre clé, ou rejouée, alors que FCM et APNs peuvent livrer en retard et dans le désordre, et que l'horloge du téléphone peut être fausse.

## Décision

1. Le proxy local ne transmet une connexion que si sa première requête nomme, en premier segment du chemin, un secret de session (32 octets au hasard, tirés à chaque démarrage du tunnel, connus de la seule WebView par le plugin), et l'en retire avant le tunnel (GET /<secret>/remote devient GET /remote) ; sinon une 403 et rien n'entre dans le tunnel. Le jeton d'appareil reste demandé derrière.
2. Les notifications scellées, version 1 : base64url sans remplissage de version (1) ‖ nonce (24 octets au hasard) ‖ XChaCha20-Poly1305 ; la clé est tirée de la clé de l'appairage (32 octets) par HKDF-SHA256 (sans sel, info escouade-mobile/notification/v1), jamais la clé de l'appairage elle-même ; l'AAD est « escouade-notification » suivi de la version ; le JSON porte seq (dès 1, croissant par appairage), at, kind, title, body et target facultatif ; 3 072 caractères au plus.
3. Le rejeu se refuse par numéro, pas par l'heure : le téléphone garde pour chaque appairage le plus grand seq ouvert et une fenêtre des 64 au-dessous (comme WireGuard et IPsec) ; déjà vu = rejouée, sous la fenêtre = trop ancienne. La garde ne bouge qu'après l'étiquette vérifiée. Son état n'est pas un secret, l'app le garde ; le cœur garde le compteur avec l'appareil et ne le réutilise jamais. « at » ne sert qu'à l'affichage.
4. La crate escouade-mobile est un paquet à part (son verrou, sa chaîne d'outils), hors de l'espace de travail de l'app ; deux jobs de CI à elle (Linux : tests, Clippy, cargo deny, Android et l'AAR ; Mac : XCFramework et plugin pour le simulateur), lancés quand elle, le plugin ou leurs scripts changent. Le cœur prendra le scellement dans la même crate, sans sa feature tunnel (MO-6).

## Conséquences

MO-4 donne l'adresse du proxy (secret compris) à la WebView et doit permettre le HTTP en clair vers 127.0.0.1 sous Android ; l'API du cœur verra l'origine de la WebView (https://localhost, capacitor://localhost) et l'hôte 127.0.0.1:<port>. MO-6 scelle par escouade_mobile::notification::seal, garde un seq par appairage (persistant, jamais réutilisé) et fait garder l'état de la garde par le service de notification d'Android et l'extension d'iOS (préférences, groupe d'apps). Un changement du format passe par une version 2 : la version 1 reste lisible tant que des téléphones l'attendent. La CI de l'app n'est pas ralentie ; un Mac de plus tourne quand la crate ou le plugin changent.
