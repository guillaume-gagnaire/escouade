---
number: 81
id: "les-fichiers-et-les-programmes-d-un-plug"
title: "Les fichiers et les programmes d'un plugin en WASM : permissions.files, proc/* et leurs deux portes"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-333"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-333"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":18,"removed":2}]
accepted_at: "2026-11-05T06:47:21+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-333 (A3 du plan, docs/PLUGINS.md §8.4.6 et §8.4.7) branche les fichiers et les programmes sur l'hôte WASM. L'étude laissait plusieurs points ouverts : où déclarer les racines, comment `run.js` y accède alors qu'il n'a pas l'import `escouade.call`, ce qu'est le « Job object du plugin » pour une instance sans process, et comment borner la sortie d'un programme.

## Décision

1. Les racines se déclarent sous `permissions.files`, comme une permission ordinaire avec un mot (`files:agent:write`) qu'on peut retirer ; `plugin` est en lecture seule.
2. `escouade.call` sert `run.wasm`. `run.js` passe par une ligne JSON-RPC (`p.fs`, `p.runProgram`), avec le même code, les mêmes refus et le même journal : escouade-js ne change pas.
3. Une réponse trop grande pour `escouade.call` est gardée avec sa requête : une écriture n'est jamais refaite.
4. Un Job object par programme, tenu par l'instance : tout meurt avec elle ou à `proc/kill`.
5. Le crédit de sortie est implicite : l'hôte ne lit plus au-delà de 256 Kio qui n'ont pas atteint l'entrée du module, sans `proc/credit`.
6. Pas de `.cmd` ni de `.bat` sous Windows, puisqu'il n'y a pas de shell.
7. La fenêtre dit une ligne par commande, pour pouvoir en retirer une : « Lance cette commande sur ton ordinateur », et non une ligne pour toutes.
8. Seuls les plugins de harness reçoivent un agent, par `agent/attach` ; un autre plugin a `plugin`, `temp` et `proc/spawn` avec un `project`.

## Conséquences

`run.js` n'a pas d'appel synchrone : ses lectures sont des promesses, ce que son API est déjà. Un outil installé par npm (un `.cmd`) n'est pas lançable par `proc/spawn`. `agentHome` manque tant qu'un agent tourne sur le compte par défaut : P3 devra donner le dossier par défaut de chaque harness. Changer le SDK reconstruit les bundles de slack et de team-rules, dont les empreintes changent à leur prochaine publication. Un plugin d'une autre sorte qui voudrait le dossier d'un agent demandera d'étendre `agent/attach`.
