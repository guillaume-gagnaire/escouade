---
number: 94
id: "l-attente-du-transcript-d-antigravity-de"
title: "L'attente du transcript d'Antigravity devient des lignes gardées et un minuteur, avec agent/items et fs/read tail"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-348"
pages: ["plugins/antigravity-en-plugin","plugins/tous-les-plugins-en-wasm","plugins/livres-avec-escouade"]
tickets: ["ESC-348"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/antigravity-en-plugin","added":14,"removed":5},{"page":"plugins/tous-les-plugins-en-wasm","added":6,"removed":0},{"page":"plugins/livres-avec-escouade","added":1,"removed":0}]
accepted_at: "2026-11-06T07:11:45+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le plugin natif d'Antigravity CLI attendait jusqu'à 600 ms dans le fil du plugin, par `sleep`, quand `agy` écrivait son transcript après l'événement qui dit l'étape finie. Un module WASM n'attend pas dans un tour. Le natif lisait aussi le transcript entier, sans limite, alors que `fs/read` refuse un fichier au-delà de 4 Mio, et il lisait le dossier de l'utilisateur, qu'un module ne connaît pas. `agent/items` n'atteignait que les agents d'un process partagé.

## Décision

Le client garde dans l'ordre ce qu'`agy` a dit à partir de l'étape en retard. `agent/frames` répond ce qui passait avant. Un seul minuteur de l'hôte, posé en notification, fait relire le transcript toutes les 40 ms, 15 fois par lot. Ce qui passe alors sort en `agent/items`. La patience épuisée, les lignes passent sans le transcript, comme avant. Le cœur fait passer `agent/items` par la pompe de l'agent, derrière la réponse du lot d'avant, pour tout agent. `fs/read` gagne `tail` : un fichier au-delà de la limite rend sa fin et `cut`, à partir d'un caractère entier. Le module déclare `agentHome` en lecture et `limits.fileBytes` à 4 Mio. `GIT_CONFIG_GLOBAL`, `CARGO_HOME` et `RUSTUP_HOME` sont ajoutés par le cœur (`Agy::spawn`). La crate garde une `rlib` pour la vue terminal du cœur jusqu'à P8.

## Conséquences

Un `agy` lent ne retient plus les autres agents du plugin. Le module ne cherche une étape que dans les 4 derniers Mio du transcript : une étape plus ancienne ne s'y trouve plus, ce qui n'arrive que pour une ligne de plus de 4 Mio. Une ligne de plus dans la fenêtre d'installation (« Lit le dossier de compte de tes agents »). Si l'instance plante pendant l'attente, ce qu'elle gardait est perdu, un `result` compris : le tour reste ouvert jusqu'au message suivant ou à l'arrêt. `turn::read_file` n'envoie pas l'agent : Codex et les autres harness de P3 doivent appeler `host_call` avec lui pour lire une racine d'agent.
