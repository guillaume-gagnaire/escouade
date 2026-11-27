---
number: 356
id: "l-identite-d-un-appareil-dans-le-navigat"
title: "L'identité d'un appareil dans le navigateur : une clé WebCrypto non extractible et un défi signé, jamais un jeton lisible"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-655"
pages: ["a-distance/api-distante","a-distance/mode-serveur"]
tickets: ["ESC-655"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:59:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-655, § 2, question B2 (a-46). Le fork d'OshOEz range le jeton de l'appareil dans le localStorage de l'origine (http://127.0.0.1:<port>). Toute page servie plus tard sur ce port, une extension du navigateur ou une faille XSS peut alors le lire et l'emporter. Ce jeton vaut un terminal sur le serveur jusqu'à sa révocation. Le cookie est écarté : un navigateur l'envoie à tous les ports de 127.0.0.1.

## Décision

Dans le navigateur, l'appareil est une clé ECDSA P-256 créée par WebCrypto, non extractible (extractable: false) et gardée dans IndexedDB. À l'appairage, la page envoie sa clé publique avec le code ; le serveur la garde dans remote-devices.json. À chaque connexion, le serveur envoie un défi dans la WebSocket et la page le signe. Aucun jeton n'est jamais lisible par la page, ni dans le localStorage, ni dans un cookie, ni dans un en-tête. Le bouton « Oublier ce serveur » efface la clé et révoque l'appareil. Le client de bureau garde son jeton au trousseau. Choisi par l'utilisateur le 2026-11-26, avec la recommandation de l'étude.

## Conséquences

Une page de la même origine, une extension ou un XSS peut se servir de la clé tant que la page est ouverte, mais jamais l'emporter : une fuite ne dure pas plus que la page. La poignée de main de l'API distante gagne une variante par défi signé (cœur, protocole, tests), et l'appairage prend une clé publique. Coût : environ 1 h 30 d'agent de plus dans BC-2. WebCrypto exige un contexte sûr, ce que sont http://127.0.0.1, localhost et HTTPS.
