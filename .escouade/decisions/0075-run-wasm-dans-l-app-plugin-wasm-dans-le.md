---
number: 75
id: "run-wasm-dans-l-app-plugin-wasm-dans-le"
title: "run.wasm dans l'app : plugin.wasm dans le dossier du plugin, un seul hôte, fetch à côté du fil, 16 appels à la fois"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-331"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-331"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":10,"removed":1}]
accepted_at: "2026-11-05T00:21:07+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 73 ouvre run.wasm et l'étude (docs/PLUGINS.md §8.3, §8.5) dit le modèle, mais trois points manquaient pour le faire tourner : où l'app trouve le module, alors que le manifeste nomme un Cargo.toml et que build ne vient qu'avec A5 ; comment borner les fetch une fois qu'ils ne sont plus servis un par un ; et l'étude attendait un `_initialize` que rustc 1.98 n'exporte pas d'un cdylib wasm32-wasip1 (ni le module de test, ni Linear, ni Codex).

## Décision

Le module d'un plugin run.wasm est `plugin.wasm`, à la racine de son dossier. Une installation depuis le store y écrit le module de version.json, à la place de tout ce que l'archive aurait sous ce nom, avant l'empreinte du dossier, et une version sans module n'installe rien. Le précompilé se garde par l'empreinte du module et la version de l'app. Un réacteur est un module sans `_start` qui exporte `escouade-turn` ; `_initialize` est appelé s'il existe et n'est pas exigé. Le fil d'un plugin attend sur le canal où le cœur écrit déjà, la fin d'un fetch y revient comme une ligne, et la fermeture du canal arrête le fil et abandonne les fetch en cours. net/fetch est servi à côté du fil pour run.js comme pour run.wasm, 16 appels au plus à la fois par plugin. escouade.call existe sans méthode branchée.

## Conséquences

Un plugin se teste en dossier local avec un plugin.wasm posé à la main ou par --wasm, en attendant `build` (A5). Le store (A6) doit publier un module pour chaque run.wasm, sans quoi l'app refuse d'installer. La borne de 16 n'est pas dans l'étude : elle remplace la borne de fait d'un appel à la fois, et l'attente de son tour ne compte pas dans les 10 s d'un appel. A3 devra tenir compte qu'une réponse trop grande d'escouade.call fait refaire l'appel, pour tout ce qui écrit. Les tests de parité de Linear et Codex par l'hôte de l'app ne tournent pas en CI tant que leurs modules n'y sont pas construits.
