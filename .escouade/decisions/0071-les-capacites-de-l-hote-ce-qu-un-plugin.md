---
number: 71
id: "les-capacites-de-l-hote-ce-qu-un-plugin"
title: "Les capacités de l'hôte : ce qu'un plugin fait hors du protocole est déclaré, appliqué et dit"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-330"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-330"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-04T21:57:33+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un module WASM ne peut ni lancer un programme, ni ouvrir une socket, ni lire un fichier. L'inventaire du §8.1 de docs/PLUGINS.md relève ce que les plugins livrés font hors du protocole : le réseau de tous, le flux SSE et le serveur local d'OpenCode, le WebSocket et les corps binaires de la voix, les fichiers d'ACP, de Codex et d'Antigravity, les minuteurs des hébergeurs, et un seul plugin qui lance un programme, isola. Quatre règles de `net/fetch` empêchent aussi la première main d'y passer.

## Décision

1. Le modèle : une instance par plugin, sur un fil qui attend un seul canal d'événements, un tour par lot d'événements. Rien n'attend dans un tour. `net/fetch` tourne à côté du fil, et un appel lent ne retient plus le suivant.
2. Deux portes : les lignes JSON-RPC pour ce qui dure, un appel synchrone (`escouade.call`) pour ce qu'une machine à états doit avoir avant de répondre : les fichiers.
3. `net/fetch` est revu : `{account}` au manifeste donne l'hôte que l'utilisateur a saisi pour ce compte, réseau local et http compris pour lui seul ; le proxy et le TLS de Réglages › Réseau sont suivis (avec un proxy, seul le nom est jugé : cela revoit la décision 32) ; `bodyBase64` ; des limites déclarées, plafonnées par l'hôte à 120 s et 32 Mio.
4. Nouvelles capacités : `net/stream` avec un crédit de lecture ; `ws/*`, `wss` seulement ; `local:shared` pour le seul serveur qu'un `Launch::shared` a annoncé ; `timer/*`, droit de base borné à 10 ms et 64 minuteurs ; `files`, des racines nommées (`plugin`, `agent`, `agentRepo`, `agentHome`, `temp`), jamais un chemin libre, la clôture des projets privés appliquée par l'hôte ; `proc/*` sur `permissions.commands`, le programme trouvé par le cœur, sans shell, dans le Job object du plugin.
5. Chaque capacité a son texte fr et en dans la fenêtre d'installation (§8.4). Ce qui n'est pas déclaré répond `-32003`.
6. Restent au cœur : le lancement des CLI, le PTY, `agent/frames`, `Launch::files`, le coffre, les jetons des CLI, la connexion des comptes, `signin.rs` pour ses lectures, le MCP, la voix locale.

## Conséquences

Le proxy cesse d'être contourné, au prix du jugement des adresses derrière lui. `proc/*` reste la capacité la plus forte : le module est isolé, le programme qu'il lance ne l'est pas ; une entrée en `*` sur un programme généraliste est refusée à un tiers vérifié, sauf justification relue. Le prototype n'a essayé que `net/fetch` et `fs/read` : le flux, le WebSocket, les minuteurs et `proc/*` sont à prouver dans leurs tickets ; le délai de la voix en direct est à mesurer. Antigravity doit réécrire son attente du transcript.
