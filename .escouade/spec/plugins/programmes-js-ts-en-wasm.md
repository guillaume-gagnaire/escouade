---
title: "Les programmes JS/TS des plugins, en WASM sans Node"
subject: "plugins"
sources: ["docs/PLUGINS.md §5 (ESC-319)","prototypes/wasm-plugins/","Demande de l'utilisateur du 2026-11-01 : « je veux pas rendre node obligatoire… chaque plugin ts qui est soumis sera buildé en wasm »"]
tickets: ["ESC-319"]
decisions: ["les-programmes-js-ts-des-plugins-tournen"]
compared: null
---

Un plugin dont le programme est écrit en JavaScript ou en TypeScript tourne **dans Escouade**, en WASM, sans que Node soit installé chez l'utilisateur. Décidé le 2026-11-01 (ESC-319) ; construit par les tickets WASM-1 à WASM-7, dans l'ordre. L'étude, les mesures et le prototype : `docs/PLUGINS.md` §5, `prototypes/wasm-plugins/`.

## Le manifeste

- Un programme en JS/TS se déclare par `"run": { "js": "dist/plugin.mjs" }` : le bundle commité par l'auteur, sans `node_modules`. Un manifeste a `run.command` ou `run.js`, jamais les deux.
- Le `.wasm` n'est jamais commité : le store le fait, ou l'app pour un plugin installé depuis un dossier ou un dépôt git.

## Ce qui tourne

- Le bundle est compilé par la Javy épinglée (aujourd'hui 9.1.0) avec le plugin Javy d'Escouade (`escouade-js`, QuickJS), après le prélude d'Escouade, en mode déterministe : le même bundle donne le même `.wasm` à l'octet, sous Windows, macOS ou Linux.
- L'app le précompile à l'installation (son cache, refait à chaque mise à jour), puis le fait tourner par wasmtime dans son propre process : une instance par plugin, qui garde son état d'un appel à l'autre comme le process d'avant.
- Le protocole ne change pas : les mêmes méthodes et les mêmes délais (`initialize`, `events/fact`, `permissions/decide` en 500 ms, `secrets/get`, `settings/get`, `log`), portés par l'entrée et la sortie standard du module. Le SDK garde son API.

## Ce que l'hôte sert, et applique

- **Le réseau** : `fetch` passe par Escouade (`net/fetch`), en https seulement, vers un hôte de `permissions.network` ou l'un de ses sous-domaines ; tout autre hôte, une adresse IP, http, une redirection vers ailleurs sont refusés, et le journal le garde.
- **Les fichiers** : seulement ceux du dossier du plugin, en lecture (`rules.json`, `channels.json` : les fichiers JSON de sa racine, 256 Kio chacun, 1 Mio en tout, jamais son manifeste) ; rien d'écrit.
- **Les secrets et les réglages** : comme pour un process, seulement les siens.
- **Les limites** : 64 Mio de mémoire, 2 s par tour (500 ms pour une décision), une boucle sans fin arrêtée ; un plantage refait l'instance, avec le budget de redémarrages d'un process.
- Un plugin WASM ne peut ni lancer un programme, ni ouvrir une socket, ni lire l'environnement ou un fichier hors de son dossier, ni charger un module natif ou distant. Un bundle qui importe autre chose que `node:fs` et `node:readline` n'est pas compilé.
- Réglages › Plugins dit « Tourne dans Escouade, isolé » pour un tel plugin, et ses hôtes comme une permission appliquée.

## Le store

- À la soumission, le store compile le bundle dans un conteneur sans réseau, avec les outils épinglés, deux fois : deux empreintes différentes refusent la version. Il publie le `.wasm` et son empreinte (et les versions du compilateur) dans `version.json`, que la signature couvre.
- Un plugin tiers du store ne peut pas avoir pour `run.command` un script ou un interpréteur (`node`, `deno`, `bun`, `python`…) : il se publie en `run.js`. Les binaires tiers attestés restent permis. Installé depuis un dossier ou un dépôt git, un plugin lancé par Node reste permis, non vérifié, et Escouade dit qu'il lui faut Node.
- La revue locale recompile le bundle et compare l'empreinte à celle de `version.json` avant de lire le bundle et `src/`.

## Les outils de l'auteur

- `escouade-plugin dev`, `test`, `validate` et `pack` compilent en local sans Node : la Javy épinglée est téléchargée à son premier besoin et vérifiée par son SHA-256, `escouade-js` est embarqué.

## La première main

- Les plugins `dev.escouade.*` écrits en Rust (harness, sources, hébergeurs) restent des programmes natifs, construits et signés avec l'app.
- `dev.escouade.slack` et `dev.escouade.team-rules` passent en 2.0.0 sur `run.js`, revus, signés et publiés par le chef ; leurs 1.x restent pour les apps d'avant.
