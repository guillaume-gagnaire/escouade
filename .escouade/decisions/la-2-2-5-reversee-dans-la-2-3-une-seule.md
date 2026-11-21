---
number: null
id: "la-2-2-5-reversee-dans-la-2-3-une-seule"
title: "La 2.2.5 reversée dans la 2.3 : une seule borne de la sortie d'un tour, protocole 0.1.6 et kit 0.1.5, profils et réglages par projet servis à distance"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-563"
pages: ["plugins/programmes-js-ts-en-wasm","docs/docs-dans-le-depot","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-563","ESC-542","ESC-530","ESC-534","ESC-544","ESC-482","ESC-520"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":6,"removed":6},{"page":"docs/docs-dans-le-depot","added":1,"removed":1},{"page":"git-fichiers/hebergeurs-de-code","added":4,"removed":4}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La 2.2.5 (v2.2.5, puis ESC-555 et ESC-556) livre sur main le contrat des plugins (profils, réglages par projet, groupes, jokers typés, limite de programmes, secrets en morceaux, pid : ESC-525 à 531), wasmtime 50.0.3, la borne de 256 Mio de la sortie d'un tour (ESC-542) et « Synchroniser » dans l'espace Docs (ESC-534). Elle a publié escouade-plugin-protocol 0.1.4 et escouade-plugin-kit 0.1.4. La 2.3 avait déjà sa propre borne de la sortie d'un tour (la mémoire du module, SEC-11, ESC-470), un protocole 0.1.5 et un kit 0.1.4 d'un autre contenu (ESC-452 à ESC-494), jamais publiés, une liste unique des commandes servies à un client distant, et les docs d'un projet d'équipe sur le serveur (ESC-482), dites dans la barre d'état que la 2.2.5 vide.

## Décision

Une vraie fusion de main (1b6b7957) dans release/2.3. La sortie d'un tour garde la borne de la 2.3, la mémoire du module (64 Mio par défaut, 256 au plus), plus stricte que les 256 Mio de la 2.2.5 ; de la 2.2.5 elle prend la sortie nommée dans le journal et ce qui était écrit pendant le tour, jeté aussitôt. Les crates montent d'un cran au-dessus de ce qui est publié : protocole 0.1.6 et kit 0.1.5, qui portent le contenu de la 2.2.5 et celui de la 2.3 ; le plugin GitHub 2.3.0 les prend. Les profils et réglages par projet d'un plugin (plugin_profile_*, plugin_project_set, plugin_project_write), docs_pull et wake_remote sont servis à un client distant : ils agissent sur le cœur. Un projet dont les docs sont sur le serveur d'équipe voit « Docs sur le serveur d'équipe » dans l'en-tête des espaces des docs, à la place de « Synchroniser ». Un secret de profil gardé dans le fichier reste lu et dit « dans le fichier » quand une stratégie interdit ce fichier, comme les autres secrets d'un plugin (ESC-520).

## Conséquences

Un plugin qui écrit sans fin est arrêté plus tôt que sur la 2.2.5 quand il n'a pas demandé plus de mémoire. Une version publiée ne se republie pas : protocole 0.1.6 et kit 0.1.5 sont à publier avec la 2.3. Reste pour un ticket de suite : faire juger par les stratégies imposées (rangs 5 et 6) les réglages par projet d'un plugin et la réservation d'un compte (ESC-556), qu'aucune clé ne juge encore.
