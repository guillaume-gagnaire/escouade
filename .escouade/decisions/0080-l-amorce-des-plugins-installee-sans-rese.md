---
number: 80
id: "l-amorce-des-plugins-installee-sans-rese"
title: "L'amorce des plugins : installée sans réseau par le chemin du store, plugins à part entière, sans mise à jour seule"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-338"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/livres-avec-escouade"]
tickets: ["ESC-338"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":4,"removed":3},{"page":"plugins/livres-avec-escouade","added":12,"removed":0}]
accepted_at: "2026-11-05T06:27:02+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 70 prévoit une amorce dans l'app et la décision de l'utilisateur du 2026-11-04 fait de la première main des plugins à part entière (« pas d'auto update »). Il restait à dire où vit l'amorce, comment on quitte l'origine `builtin` alors que les plugins ne sont pas encore publiés, et ce que devient une version révoquée ou un plugin désinstallé.

## Décision

`plugins/seed.json`, embarqué, nomme les versions par l'empreinte de leur `version.json` ; leurs octets sont dans les ressources de Tauri (`seed/`), avec la liste de révocation du jour. Au démarrage, ce que l'amorce tient et qui n'est pas installé passe par `inspect_store` et `install`, avec les contrôles de `Store::fetch` : origine `store`, marque `seed`. Une version refusée n'installe rien et se dit dans la vue ; une version retirée par `disable` s'installe coupée, sa raison dite ; un sceau retiré n'installe rien. La liste de révocation la plus récente l'emporte, du store ou de l'amorce. Rien ne se met à jour seul : la plus récente du store et de l'amorce est proposée, jamais une plus ancienne. Un désinstallé est noté, jamais remis, et proposé par « Proposés par Escouade ». Un id que `seed.json` nomme quitte `builtin` (`Plugins::given`) ; `seed.json` reste vide jusqu'à Z2. La release échoue sur une version absente, non signée, révoquée ou hors de `escouade.min`/`max`.

## Conséquences

Rien ne change pour l'utilisateur avant Z2. La release dépend du store dès que `seed.json` n'est plus vide : le chef publie d'abord. Une `dev.escouade.*` révoquée sans remplaçant reste coupée jusqu'à une nouvelle version au store ou une nouvelle release. Le temps du premier lancement avec les 23 plugins n'est pas mesuré : `sync_seed` tourne au chargement du cœur. Une entrée `builtin` refusée par l'amorce tourne encore comme avant, jusqu'à Z3.
