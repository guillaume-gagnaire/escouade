---
number: 70
id: "la-premiere-main-dans-plugins-id-court-p"
title: "La première main dans plugins/<id-court>/, publiée au store comme un tiers, avec une amorce dans l'app"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-330"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-330","ESC-226","ESC-227","ESC-328"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-04T21:57:32+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur, le 2026-11-04 : « A la fin, je veux que tous les plugins builtins soient dans /plugins/* (exemple plugins/claude), tous publiés sur le store, vérifiés et signés. » L'app doit pourtant rester utilisable hors ligne et dès son installation. Le store ne lit que le dossier d'un plugin : `path = "../../crates/…"` n'y existe pas. Cela remplace l'origine `builtin` des décisions 30 et revoit la règle « jamais en silence » du §3.2 pour la première main.

## Décision

1. Chaque plugin de première main quitte `plugins/builtin/dev.escouade.<id>/` pour `plugins/<id-court>/`, avec son manifeste, son README, sa LICENSE, son CHANGELOG et ses tests : claude, codex, copilot, cursor, gemini, antigravity, opencode, acp, mistral, openai, groq, elevenlabs, deepgram, github, gitlab, bitbucket, gitea, azure-devops, linear, jira, trello, clickup, isola. Ses identifiants `dev.escouade.*` ne changent pas. `dev.escouade.minimal` va dans `tests/fixtures/plugins/`, non publié.
2. Chacun passe comme un tiers : étiquette `plugins/<id-court>/vX.Y.Z`, soumission, compilation par le store, revue, signature du chef, publication. Sa version est la sienne, à partir de 2.2.0. Les plugins déménagés restent en GPL-3.0-or-later.
3. L'app embarque une amorce : `version.json`, sa signature, l'archive et `plugin.wasm` des versions publiées, nommées par `plugins/seed.json`. Elle l'installe sans réseau par le chemin d'une installation du store. Le chef publie d'abord, la release ensuite ; `release.yml` échoue sur une signature invalide.
4. Ensuite, le store met ces plugins à jour : sans clic si leurs permissions ne grandissent pas, sinon après accord. Un agent en cours garde sa version. On ne redescend jamais de version. Un plugin livré se coupe mais ne se désinstalle pas.
5. Avant de lancer, l'app vérifie : la signature, les empreintes, `dev.escouade.*` signé sinon refusé, la révocation (celle du store, sinon celle de l'amorce), la compatibilité. L'origine `builtin`, `{escouade}/…` et `check_program` disparaissent. `dev.escouade.*` reste réservé au dépôt de l'app.
6. `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` sont publiés sur crates.io ; un plugin en dépend par version, un `[patch.crates-io]` les ramène aux chemins locaux pour le développement.
7. L'ordre : le déménagement, la CI de `plugins/` (`plugins/ci.mjs`), les contrôles du store pour un `run.wasm`, la publication de chacun.

## Conséquences

`escouade-plugins` (34,5 Mo) disparaît, l'amorce pèse 11 à 14 Mo en clair. Un correctif d'un plugin n'attend plus une release. Une release dépend de la publication préalable par le chef : 23 plugins demandent une commande qui enchaîne les gestes. Une révocation frappe aussi la première main. Publier sur crates.io est un geste public et sans retour, à faire par l'utilisateur ou sur son accord. La mise à jour sans clic, crates.io et la licence peuvent se refuser séparément. ESC-226 passe avant le déménagement, ESC-328 prépare l'étape Rust, ESC-227 passe en dernier.
