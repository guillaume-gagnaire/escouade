---
number: 29
id: "les-programmes-js-ts-des-plugins-tournen"
title: "Les programmes JS/TS des plugins tournent en WASM sans Node : le store les compile (Javy + escouade-js), l'app les fait tourner par wasmtime"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-319"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-319"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":24,"removed":0}]
accepted_at: "2026-11-01T09:50:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La recommandation D de docs/PLUGINS.md §2.4 (décidée le 2026-10-26, §4.11) servait les programmes des plugins par un process en 2.1 et repoussait un runtime WASM à plus tard. Un plugin en TypeScript (dev.escouade.slack, dev.escouade.team-rules) se lance donc par `node dist/plugin.mjs` : il faut Node 22.18 chez l'utilisateur. Le 2026-11-01, l'utilisateur : « je veux pas rendre node obligatoire, donc soit tu les exécutes dans la partie navigateur de tauri, soit tu prends en compte wasm directement et chaque plugin ts qui est soumis sera buildé en wasm ». ESC-319 a comparé et mesuré Javy, Extism et componentize-js sous Windows (docs/PLUGINS.md §5.2 et §5.3), et fait tourner les deux bundles commités, sans retouche, dans un hôte wasmtime (prototypes/wasm-plugins/, 9 tests).

## Décision

D est révisée : le WASM devient le runtime des programmes en JS/TS, tout de suite. (1) Un programme JS/TS se déclare `run.js` (le bundle commité) ; le store le compile dans un conteneur sans réseau par la Javy épinglée (9.1.0) et le plugin Javy d'Escouade (escouade-js, QuickJS avec l'export escouade-turn), en mode déterministe, deux fois, et publie le .wasm et son empreinte dans version.json, que la signature couvre ; la revue locale recompile et compare. (2) L'app le précompile à l'installation et le fait tourner par wasmtime dans son process, une instance par plugin, le même protocole JSON-RPC porté par l'entrée et la sortie standard du module ; elle applique le réseau (net/fetch servi par l'hôte, seulement https vers permissions.network), les fichiers (ceux du dossier du plugin, en lecture), 64 Mio et le temps par l'époque. (3) La webview est écartée (fenêtre, mode serveur, banc, IPC partagé, trois moteurs, CSP par page). (4) Le run natif reste pour la première main en Rust et pour les binaires tiers attestés ; un script ou un interpréteur (node…) n'est plus une commande d'un plugin tiers du store. (5) Lien statique (un module se suffit), précompilation à l'installation.

## Conséquences

Node n'est plus un prérequis pour aucun plugin du store ; le réseau et les fichiers des plugins JS/TS sont appliqués, plus seulement déclarés. Coût mesuré : environ +13 Mo dans l'exécutable (wasmtime 50, Cranelift, WASI p1), 1,27 Mo par plugin, 152 ms de compilation une fois à l'installation puis 1,9 ms par démarrage, 71 µs par décision (contre 129 µs pour Node). Escouade tient un plugin Javy à lui (escouade-js) et suit Javy ; extism-js et componentize-js ne sont pas retenus (compilations non reproductibles, Node pour componentize-js). Sept tickets, dans l'ordre : runtime dans l'app, SDK, escouade-plugin sans Node, store, revue locale, doc du site fr et en, migration de dev.escouade.slack et dev.escouade.team-rules en 2.0.0 (revues et signées par le chef).
