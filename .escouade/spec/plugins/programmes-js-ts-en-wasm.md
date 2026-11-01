---
title: "Les programmes JS/TS des plugins, en WASM sans Node"
subject: "plugins"
sources: ["docs/PLUGINS.md §5 (ESC-319, ESC-320, ESC-321)","prototypes/wasm-plugins/","Demande de l'utilisateur du 2026-11-01 : « je veux pas rendre node obligatoire… chaque plugin ts qui est soumis sera buildé en wasm »","Revue d'ESC-319 par le chef (2026-11-01) : l'adresse résolue","src-tauri/src/plugins/wasm.rs","src-tauri/src/plugins/files.rs","sdk/plugin/src/index.ts","schemas/plugin-protocol-1.json"]
tickets: ["ESC-319","ESC-320","ESC-321"]
decisions: [29,32,36]
compared: null
---

Un plugin dont le programme est écrit en JavaScript ou en TypeScript tourne **dans Escouade**, en WASM, sans que Node soit installé chez l'utilisateur. Décidé le 2026-11-01 (ESC-319) ; construit par les tickets WASM-1 à WASM-7, dans l'ordre : le runtime dans l'app (WASM-1, ESC-320) et le SDK sur les deux transports (WASM-2, ESC-321) sont faits. L'étude, les mesures et le prototype : `docs/PLUGINS.md` §5, `prototypes/wasm-plugins/`.

## Le manifeste

- Un programme en JS/TS se déclare par `"run": { "js": "dist/plugin.mjs" }` : le bundle commité par l'auteur, sans `node_modules`, un fichier du dossier du plugin (un chemin relatif, sans `..`). Un manifeste a `run.command` ou `run.js`, jamais les deux : les deux lecteurs du manifeste (l'app et le store) le refusent, avec la raison en français et en anglais (« `run.js` ne va pas avec `run.command` : un programme est une commande ou un bundle JS/TS, pas les deux. »).
- Le `.wasm` n'est jamais commité : le store le fait, ou l'app pour un plugin installé depuis un dossier ou un dépôt git.

## Ce qui tourne

- Le bundle est compilé par la Javy épinglée (aujourd'hui 9.1.0, téléchargée par l'app à son premier besoin et vérifiée par le SHA-256 de son archive) avec le plugin Javy d'Escouade (`escouade-js`, QuickJS), après le prélude d'Escouade, en mode déterministe : le même bundle donne le même `.wasm` à l'octet, sous Windows, macOS ou Linux. `escouade-js` est construit par la CI, son SHA-256 noté, et livré **dans l'exécutable de l'app** ; une app construite sans lui ne lance aucun plugin `run.js` et dit pourquoi.
- L'app compile et précompile le module **à l'installation**, et le garde dans son cache ; elle le recharge de là à chaque démarrage du plugin, et le précompile à nouveau après une mise à jour de l'app (les précompilés d'une autre version sont retirés au démarrage). Elle le fait tourner par wasmtime dans son propre process : une instance par plugin, sur un fil à elle, qui garde son état d'un appel à l'autre comme le process d'avant.
- Le protocole ne change pas : les mêmes méthodes et les mêmes délais (`initialize`, `events/fact`, `permissions/decide` en 500 ms, `secrets/get`, `settings/get`, `log`), portés par l'entrée et la sortie standard du module ; le même journal, le même budget de redémarrages. S'y ajoutent `net/fetch` (en WASM seulement) et `files/read` (aux deux transports), décrits avec leurs délais et leurs erreurs dans `schemas/plugin-protocol-1.json`.
- Le poids ajouté à l'exécutable de l'app : environ 15 Mo (wasmtime et Cranelift, et `escouade-js` embarqué), mesuré dans `docs/PLUGINS.md` §5.3.

## Le SDK, sur les deux transports

- `@escouade-dev/plugin` (0.3.0) garde son API : `createPlugin()`, `onEvent`, `decidePermission`, `p.secret`, `p.settings`, `p.log`, `start()`. Un plugin écrit pour lui tourne **sans changement** sous Node (`run.command`, l'entrée et la sortie standard) et dans Escouade en WASM (`run.js`) : `start()` choisit le transport, le plugin ne le sait pas.
- `p.file(chemin)` lit un fichier du dossier du plugin, en texte, un sous-dossier compris (`data/words.txt`), sous Node comme en WASM (`files/read`). `readFileSync(new URL('../rules.json', import.meta.url))` marche toujours pour les JSON de la racine.
- `fetch` reste le `fetch` standard ; en WASM, Escouade le sert (`net/fetch`) avec les règles ci-dessous.
- Le prélude d'Escouade ne fait plus que ce que le SDK ne fait pas (les lignes, `fetch`, `readFileSync`, `URL`, `console`). Un bundle construit avec le SDK d'avant (0.2) ne tourne pas en WASM : le plugin s'arrête et son journal dit de le reconstruire avec le SDK 0.3 ou plus récent.
- Il n'y a pas de minuteur en WASM : `setTimeout` n'y est pas défini (une bibliothèque qui l'appelle échoue avec la raison). Le SDK n'en a pas besoin : en WASM, le délai d'une seconde d'Escouade borne `shutdown`.
- `npm create escouade-plugin` écrit un plugin en `run.js`, sans script `node` à lancer, qui passe `escouade-plugin validate`. En attendant WASM-3, le banc d'`escouade-plugin` (`test`, `dev`) joue un plugin `run.js` par Node ; il faut donc Node à l'auteur, jamais à l'utilisateur.

## Ce que l'hôte sert, et applique

- **Le réseau** : `fetch` passe par Escouade (`net/fetch`), en https seulement, vers un hôte de `permissions.network` ou l'un de ses sous-domaines ; tout autre hôte, une adresse IP écrite dans l'URL, http, une redirection vers ailleurs sont refusés, et le journal le garde. Une redirection n'est jamais suivie : le plugin reçoit la réponse 3xx. 10 s et 1 Mio de réponse au plus.
- **L'adresse résolue** : même pour un hôte déclaré, la connexion est refusée si le nom résout une adresse de la machine ou du réseau local (bouclage, privée, de lien local, non spécifiée, de multidiffusion ou de diffusion ; IPv4 et IPv6, une IPv4 mappée ou traduite en IPv6 jugée comme elle-même). Le jugement porte sur les adresses que la connexion emploie elle-même, pas sur une résolution à part que le DNS pourrait contredire. Le plugin voit un `fetch` qui rejette (« slack.com resolves to 127.0.0.1, an address of this machine or of the local network »), le journal le garde. Pour la même raison, `net/fetch` ne passe pas par le proxy de Réglages › Réseau.
- **Les fichiers** : seulement ceux du dossier du plugin, en lecture ; rien d'écrit. Les fichiers JSON de sa racine (`rules.json`, `channels.json` : 256 Kio chacun, 1 Mio en tout, jamais son manifeste) lui sont donnés au démarrage ; `p.file()` lit les autres, 256 Kio au plus, en UTF-8. Est refusé, et gardé au journal : un chemin absolu, avec `..` ou un lecteur, un lien qui mène hors du dossier, un nom caché (`.env`, `.git`), le manifeste. Un plugin lancé par une commande a les mêmes règles pour `p.file()` (il garde par ailleurs les droits de son process).
- **Les secrets et les réglages** : comme pour un process, seulement les siens ; ses secrets masqués dans son journal.
- **Les limites** : 64 Mio de mémoire, 2 s par tour (500 ms pour une décision), une boucle sans fin arrêtée ; une boucle sans fin, une mémoire qui grossit ou une exception non rattrapée finissent l'instance, qui est refaite avec le budget de redémarrages d'un process (trois fois en cinq minutes, puis arrêtée) ; le journal dit chaque fois pourquoi (« a turn ran past 2000 ms », « out of memory », l'exception).
- Un plugin WASM ne peut ni lancer un programme, ni ouvrir une socket, ni lire l'environnement ou un fichier hors de son dossier, ni charger un module natif ou distant. Un bundle qui importe autre chose que `node:fs` et `node:readline` n'est pas compilé, et le plugin s'arrête avec la raison.
- Réglages › Plugins dit « Tourne dans Escouade, isolé » pour un tel plugin (au lieu de « Lance un programme sur ton ordinateur, avec tes droits »), et ses hôtes comme une permission appliquée (« appliqué par Escouade : il ne joint aucun autre hôte, ni une adresse de ta machine ou de ton réseau local »).

## Le store

- À la soumission, le store compile le bundle dans un conteneur sans réseau, avec les outils épinglés, deux fois : deux empreintes différentes refusent la version. Il publie le `.wasm` et son empreinte (et les versions du compilateur) dans `version.json`, que la signature couvre.
- Un plugin tiers du store ne peut pas avoir pour `run.command` un script ou un interpréteur (`node`, `deno`, `bun`, `python`…) : il se publie en `run.js`. Les binaires tiers attestés restent permis. Installé depuis un dossier ou un dépôt git, un plugin lancé par Node reste permis, non vérifié, et Escouade dit qu'il lui faut Node.
- La revue locale recompile le bundle et compare l'empreinte à celle de `version.json` avant de lire le bundle et `src/`.

## Les outils de l'auteur

- `escouade-plugin dev`, `test`, `validate` et `pack` compilent en local sans Node : la Javy épinglée est téléchargée à son premier besoin et vérifiée par son SHA-256, `escouade-js` est embarqué (WASM-3).

## La première main

- Les plugins `dev.escouade.*` écrits en Rust (harness, sources, hébergeurs) restent des programmes natifs, construits et signés avec l'app.
- `dev.escouade.slack` et `dev.escouade.team-rules` passent en 2.0.0 sur `run.js`, revus, signés et publiés par le chef ; leurs 1.x restent pour les apps d'avant. Leurs `dist/plugin.mjs` sont reconstruits avec le SDK 0.3 (ESC-321) et tournent dans le runtime de l'app.
