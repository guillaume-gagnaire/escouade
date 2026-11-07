---
number: 64
id: "opencode-passe-par-son-plugin-le-cœur-la"
title: "OpenCode passe par son plugin : le cœur lance un harness partagé, le plugin en est le client"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-223"
pages: ["plugins/opencode-en-plugin","plugins/harness-en-process","plugins/livres-avec-escouade"]
tickets: ["ESC-223"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/opencode-en-plugin","added":16,"removed":0},{"page":"plugins/harness-en-process","added":6,"removed":0},{"page":"plugins/livres-avec-escouade","added":2,"removed":0}]
accepted_at: "2026-11-04T07:03:49+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-10.2 sort OpenCode du cœur. Contrairement à Cursor CLI et Mistral Vibe, OpenCode n'est pas un flux de lignes par agent : un seul `opencode serve` (0,7 Go) sert les agents d'un compte, en HTTP et SSE. Le contrat d'INT-9 supposait un process par agent, lu par le cœur. Il fallait décider qui lance ce serveur partagé et qui lit son flux, sans code propre à OpenCode dans le cœur.

## Décision

Le contrat grandit pour tous. `Launch::shared` : le plugin décrit le process, le cœur le lance une fois pour les agents du même plugin, programme et variables, le garde dans son Job object et l'arrête avec son dernier agent. Le client HTTP et la traduction vont dans la crate du plugin, qui envoie ses éléments sans être appelé (`agent/items`). `agent/open`, `agent/close`, `agent/letGo`, `agent/compact`, `agent/rename` et `agent/control` portent le cycle de vie. La table `BUILTIN_HARNESSES` reçoit une ligne, sans `native` ni `prepare`.

## Conséquences

Le serveur survit à un plantage du plugin, comme le veut docs/PLUGINS.md §4.2 : un serveur lancé par le plugin mourrait avec lui, hors du Job et de la mesure des ressources. Sans appel en cours pour révéler un plantage, chaque agent regarde l'état de son plugin toutes les 250 ms. La clôture et la lecture seule d'OpenCode sont tenues par le plugin (`fenceByClient`), cru pour un plugin livré seulement. `apply_items` gagne l'alerte de fin de tour et de question, ce qui rétablit aussi celles de Cursor CLI et de Mistral Vibe, absentes depuis INT-10.1. Copilot CLI et les harness suivants peuvent reprendre ce contrat.
