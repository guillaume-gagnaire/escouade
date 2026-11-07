---
number: 36
id: "le-sdk-parle-les-deux-transports-p-file"
title: "Le SDK parle les deux transports ; p.file() pour les fichiers du plugin"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-321"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-321"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":12,"removed":5}]
accepted_at: "2026-11-01T16:53:10+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

WASM-1 (ESC-320) faisait tourner les bundles du SDK 0.2 en WASM grâce à un prélude qui imitait Node (process.stdout, createInterface de node:readline, un setTimeout qui ne se déclenchait jamais). Le ticket WASM-2 demande que le SDK choisisse lui-même son transport, que le plugin puisse lire un fichier de son dossier au-delà des JSON de la racine, et que le prélude se réduise à ce que le SDK ne fait pas.

## Décision

1) start() du SDK (0.3.0) choisit le transport : l'hôte WASM d'Escouade (globalThis.Javy et __esc.listen/write/trace du prélude) dans l'app, l'entrée et la sortie standard sous Node ; le plugin ne change pas. 2) p.file(chemin) = files/read, servi par l'hôte aux deux transports (process et WASM) avec les mêmes règles : un fichier du dossier du plugin, sous-dossiers compris, 256 Kio, UTF-8 ; refusés (-32003) un chemin absolu, avec .. ou un lecteur, un lien qui sort du dossier, un nom caché (.env, .git) et le manifeste ; -32005 si absent, -32004 trop gros ou pas du texte. 3) Le prélude ne fait plus process.stdout ni createInterface : un bundle du SDK 0.2 ne tourne plus en WASM et son journal dit de le reconstruire ; slack et team-rules ont leur dist reconstruit. 4) Pas de minuteur servi par l'hôte : le SDK n'en a pas besoin (la seconde d'Escouade borne shutdown en WASM), setTimeout n'est pas défini en WASM plutôt qu'un faux qui ne se déclenche jamais. 5) create-escouade-plugin écrit run.js ; le banc d'escouade-plugin joue un run.js par Node jusqu'à WASM-3. 6) schemas/plugin-protocol-1.json décrit net/fetch et files/read (délais 10 s et 5 s, erreurs refused, failed, notFound).

## Conséquences

Un auteur écrit une fois son plugin pour les deux transports, et ses tests Node valent pour l'app. Un plugin run.js bâti avec le SDK 0.2 (aucun n'est publié : la 2.2 ne l'est pas) doit être reconstruit. Les .env et .git d'un dossier de plugin ne sont jamais lisibles par p.file(). Une bibliothèque qui a besoin de setTimeout ne tourne pas en WASM tant que l'hôte ne sert pas de minuteur. escouade.min du gabarit reste 2.1.0 tant que l'app est en 2.1.x : à relever à 2.2.0 à la sortie.
