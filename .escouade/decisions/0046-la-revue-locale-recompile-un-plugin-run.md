---
number: 46
id: "la-revue-locale-recompile-un-plugin-run"
title: "La revue locale recompile un plugin run.js et le joue dans l'hôte WASM, hors ligne"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-324"
pages: ["store/revue-et-signature","plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-324"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":8,"removed":1},{"page":"plugins/programmes-js-ts-en-wasm","added":4,"removed":4}]
accepted_at: "2026-11-02T00:50:19+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-323, le store compile lui-même en WASM le bundle de chaque plugin run.js et nomme ce module dans version.json, que la signature « Vérifié par Escouade » couvre. La revue locale (npm run plugin-review, /review-plugin) lisait le bundle mais ne vérifiait pas que le module servi en est bien la compilation ; et son banc lançait un conteneur Docker ou Podman, inutile pour un module qui tourne déjà dans l'hôte WASM de l'app, et que beaucoup de postes n'ont pas.

## Décision

1. La revue télécharge le module .wasm et le vérifie (taille, SHA-256) contre version.json, puis recompile le bundle de l'archive relue avec l'escouade-plugin du poste (ESCOUADE_PLUGIN, le PATH, ou src-tauri/target du dépôt), par le compilateur du store repris tel quel (un dossier neuf, un environnement à lui). Il doit compiler avec les versions que nomme version.json (compiler : Javy, escouade-js, escouade-plugin). La même empreinte : « ce qui a été lu est ce qui tourne ». Une autre : la revue s'arrête, verdict « Refuser », comme une archive changée. Pas de recompilation (pas d'escouade-plugin, un autre compilateur, un bundle refusé) : la revue continue, mais c'est un bloqueur.
2. Le banc d'un plugin run.js joue le module publié par escouade-plugin test --wasm <module> --offline, dans l'hôte WASM, sans conteneur, dans un environnement sans les variables de la revue. --offline refuse tout fetch, même vers les hôtes de permissions.network, comme le --network=none du conteneur. Le conteneur reste pour un plugin à binaire.
3. sign refuse une version run.js dont la recompilation n'a pas été faite ou n'a pas donné la même empreinte, et relit le wasm du version.json servi pour s'assurer que c'est ce module-là qui a été recompilé.

## Conséquences

Le badge d'un plugin run.js garantit que le module installé est le build du bundle relu. Le chef doit avoir sur son poste l'escouade-plugin de la version qui a compilé au store : quand l'app passe de version, il installe le nouvel escouade-plugin, et les versions compilées avant restent à revoir avec l'ancien (le rapport dit lequel). Plus besoin de Docker ou de Podman pour relire un plugin run.js. escouade-plugin test gagne --wasm et --offline, documentés dans son aide. Une revue faite avant ce ticket sur un plugin run.js ne se signe pas : il faut la relancer.
