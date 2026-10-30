---
title: "Les plugins d'exemple"
subject: "plugins"
sources: ["docs/TODO-2.1.md (PLG-10)","docs/PLUGINS.md","plugin-examples/README.md"]
tickets: ["ESC-182"]
decisions: [4]
compared: null
---

# Les plugins d'exemple

Quatre plugins complets, à copier pour en écrire un, et qui servent aussi aux tests d'Escouade (banc, règles d'autorisation, revue du store).

## Où ils vivent

Dans le dépôt d'Escouade, dossier `plugin-examples/`, un sous-dossier par plugin, chacun lisible et installable seul, sous licence MIT. Il n'y a pas de dépôt à part : comme pour les plugins de première main, aucun nouveau dépôt GitHub n'est créé sans l'accord de l'utilisateur. Le nom `escouade-plugins` est réservé à l'exécutable des plugins de première main (INT-4) ; si les exemples sortent un jour en dépôt public, il s'appellera `escouade-plugin-examples`.

## Les quatre exemples

| Dossier | Ce qu'il montre | Genre |
| --- | --- | --- |
| `high-contrast` | Un thème sombre et clair (jetons de la fenêtre, couleurs d'état), et une palette de projets | Déclaratif |
| `discord-webhook` | Deux webhooks : un ticket passé « À tester », un agent qui attend (`agent.status` `waiting`) ; l'adresse est un secret | Déclaratif |
| `slack` | Les mêmes faits postés par `chat.postMessage` dans le salon de chaque projet : celui de `channels.json`, sinon `escouade-<nom du projet>` ; le jeton du bot (`xoxb-…`) lu au trousseau (`secrets/get`) à chaque fait ; sans jeton, ou avec un jeton qui n'est pas d'un bot, rien ne part et le journal le dit | Programme TypeScript |
| `team-rules` | Les commandes des agents jugées selon `rules.json` (un fichier du dépôt du plugin) : une ligne qui contient une commande refusée est refusée, même après `&&`, `;` ou `|` ; une ligne faite seulement de commandes permises est autorisée ; le reste, et tout ce qui n'est pas une commande, va à l'utilisateur | Programme TypeScript |

Les programmes sont écrits avec `@escouade-dev/plugin` ; leur `dist/plugin.mjs` est compilé et versionné, car Escouade lance un plugin tel que son dépôt le contient.

Gemini CLI n'est pas un exemple (il est dans le cœur, ESC-274) ; l'exemple d'un harness ACP décrit viendra avec PLG-6, et celui d'une source tierce avec ce point (2.2).

## Ce qui les tient

- `plugin-examples/ci.mjs`, joué en CI Linux après la construction de `escouade-plugin` : pour chaque exemple en TypeScript, ses tests (faux Escouade du SDK, faux Slack), son typage, et son `dist/` recompilé puis comparé à celui du dépôt ; pour tous, `escouade-plugin validate` et le banc de conformité.
- Les tests du cœur : le banc sur les quatre exemples, l'exemple « Règles de l'équipe » qui refuse `rm -rf`, le manifeste Discord joué contre un faux serveur HTTP.
- La revue locale du store relit les exemples sans y trouver de piste hors de leurs permissions : leurs appels réseau sont écrits en clair (`fetch(`), et aucun `package-lock.json` n'est versionné (esbuild figé dans `package.json`).
- La page « Les plugins d'exemple » du site (`plugins/examples`, fr et en) montre leur code entier ; un test du site le compare aux fichiers.
