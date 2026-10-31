---
title: "Les plugins d'exemple"
subject: "plugins"
sources: ["docs/TODO-2.1.md (PLG-10)","docs/PLUGINS.md","plugins/README.md","docs/STORE.md (§3.6, §9.2)","docs/TODO-2.2.md (décision 8)"]
tickets: ["ESC-182","ESC-314"]
decisions: [4,21]
compared: "2026-10-30T05:11:05+01:00"
---

# Les plugins d'exemple

Quatre plugins complets, à copier pour en écrire un, et qui servent aussi aux tests d'Escouade (banc, règles d'autorisation, revue du store). Ce sont des plugins de première main : écrits et tenus par Escouade, publiés sur le store sous ses identifiants `dev.escouade.*`, mais pas livrés avec l'app.

## Où ils vivent

Dans le dépôt d'Escouade, dossier `plugins/`, un sous-dossier par plugin, nommé par la fin de son identifiant (`plugins/slack/` pour `dev.escouade.slack`), comme le seront les plugins de première main (décision 8 de TODO-2.2). Chaque dossier se lit seul, comme le dépôt d'un plugin tiers : son `escouade-plugin.json`, un `README.md` (ce qu'il fait, ses permissions en mots, comment le vérifier), sa `LICENSE` (MIT, avec la ligne `SPDX-License-Identifier: MIT`, puisque la racine du dépôt est sous GPL-3.0), son `CHANGELOG.md` et ses tests. Le `repository` de chaque manifeste est `https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/<id-court>`. Il n'y a pas de dépôt à part ; l'ancien dossier `plugin-examples/` n'existe plus.

## Les quatre plugins

| Dossier | Identifiant | Ce qu'il montre | Genre |
| --- | --- | --- | --- |
| `high-contrast` | `dev.escouade.high-contrast` | Un thème sombre et clair (jetons de la fenêtre, couleurs d'état), et une palette de projets ; le texte au moins à 7:1 sur chaque surface, l'accent et les couleurs d'état à 4,5:1, les bordures à 3:1 | Déclaratif |
| `discord` | `dev.escouade.discord` | Deux webhooks : un ticket passé « À tester », un agent qui attend (`agent.status` `waiting`) ; l'adresse est un secret ; les mentions coupées (`allowed_mentions`) | Déclaratif |
| `slack` | `dev.escouade.slack` | Les mêmes faits postés par `chat.postMessage` dans le salon de chaque projet : celui de `channels.json`, sinon `escouade-<nom du projet>` ; le jeton du bot (`xoxb-…`) lu au trousseau (`secrets/get`) à chaque fait ; sans jeton, ou avec un jeton qui n'est pas d'un bot, rien ne part et le journal le dit | Programme TypeScript |
| `team-rules` | `dev.escouade.team-rules` | Les commandes des agents jugées selon `rules.json` (un fichier du dossier du plugin) : une ligne qui contient une commande refusée est refusée, même après `&&`, `;`, `&` ou `|` ; une ligne faite seulement de commandes permises est autorisée ; une substitution, une redirection ou un échappement, le reste et tout ce qui n'est pas une commande vont à l'utilisateur | Programme TypeScript |

Tous sont en version 1.1.0 (le changement d'identifiant, depuis `dev.exemple.*`). Les programmes sont écrits avec `@escouade-dev/plugin` ; leur `dist/plugin.mjs` est compilé et versionné, car Escouade lance un plugin tel que son dépôt le contient.

Gemini CLI n'est pas un exemple (il est dans le cœur, ESC-274) ; l'exemple d'un harness ACP décrit viendra avec PLG-6, et celui d'une source tierce avec ce point (2.2).

## Leur publication

Ils passent par le store comme ceux d'un tiers : soumis depuis leur dossier du dépôt de l'app, à une étiquette `plugins/<id-court>/v1.2.3` (jamais `v1.2.3`, réservée aux releases de l'app), puis revus et signés par le chef d'escouade. Le store n'accepte leurs identifiants que de ce dépôt (voir « Publier et sécurité »), et l'app ne les installe que signés (voir « Le store dans l'app »). Pour en faire le sien, un auteur copie le dossier et lui donne un identifiant à lui (`io.github.<login>.<plugin>`).

## Ce qui les tient

- `plugins/ci.mjs`, joué en CI Linux après la construction de `escouade-plugin` : pour chacun, ses tests (pour les programmes, le faux Escouade du SDK et un faux Slack, leur typage, et leur `dist/` recompilé puis comparé à celui du dépôt ; pour les déclaratifs, `node --test` sur leur manifeste) ; pour tous, `escouade-plugin validate` et le banc de conformité, avec `--first-party` puisque leurs identifiants sont réservés.
- Les tests du cœur : le banc sur les quatre plugins (et le refus de leur identifiant sans `--first-party`), « Règles de l'équipe » qui refuse `rm -rf`, le manifeste Discord joué contre un faux serveur HTTP.
- L'essai du store (`trial.test.ts`) les soumet depuis leur dossier aux dix contrôles : tous passent pour le dépôt de l'app, l'identifiant est refusé à un fork.
- La revue locale du store relit leurs fichiers sans y trouver de piste hors de leurs permissions : leurs appels réseau sont écrits en clair (`fetch(`), aucun `package-lock.json` n'est versionné (esbuild figé dans `package.json`), et ni l'adresse de leur dépôt dans le manifeste ni leur identifiant `dev.escouade.*` ne comptent comme un hôte ou un dossier d'Escouade.
- La page « Les plugins d'exemple » du site (`plugins/examples`, fr et en) montre leur code entier ; un test du site le compare aux fichiers de `plugins/`.
