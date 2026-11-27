---
number: 349
id: "cloisonnement-windows-w1-un-jeton-restre"
title: "Cloisonnement Windows, W1 : un jeton restreint à faible intégrité pour les agents clôturés des harness d'Escouade, après une porte d'essais réels"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-591"
pages: ["projets/cloisonnement-windows","projets/projets-prives"]
tickets: ["ESC-591"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/cloisonnement-windows","added":35,"removed":0}]
accepted_at: "2026-11-27T23:58:32+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Sous Windows natif, la clôture d'un projet privé n'est qu'une liste de règles de refus donnée au harness : une commande qui construit son chemin autrement lit le dossier (audit de la 2.2.0, SEC-02, point 1). L'étude ESC-591 (docs/WINDOWS-ISOLATION.md) compare AppContainer, un jeton restreint, un compte Windows par ensemble, Windows Sandbox et WSL, et les éprouve dans une maquette jetable. Un jeton restreint qui garde le SID de l'utilisateur ouvre un processus ordinaire de l'utilisateur en écriture (échappée) ; en faible intégrité, il ne l'ouvre plus, refuse le dossier privé et garde git, node, Claude Code et la boucle locale. L'AppContainer coupe la boucle locale (serveur MCP, hooks, proxy local) sans administrateur ; un compte par ensemble demande l'administrateur et une connexion de chaque harness par compte ; Windows Sandbox n'est pas faite pour un agent.

## Décision

Choisi par l'utilisateur le 2026-11-27 (demande a-44, option a, « les recommandations ») : la voie B. Sous Windows natif, chaque agent clôturé d'un harness d'Escouade tourne, avec tout son arbre, sous un jeton restreint dérivé de celui de l'utilisateur, en faible intégrité, portant le SID de son ensemble ; une ACE de refus héritée pour ce SID est posée sur ce qu'il ne doit pas toucher, et l'étiquette « bas » là où il écrit. Rien n'est construit avant la porte WB-1 : de vraies sessions de Claude Code, Codex, OpenCode et Copilot CLI sous ce jeton, sur le poste de l'utilisateur. Si elles n'y tiennent pas, la voie retombe sur WSL (W6) et sur le conseil d'un compte par ensemble. Pourquoi : c'est la seule voie de l'étude qui ferme à la fois les fichiers des autres projets et les processus de l'utilisateur tout en gardant la boucle locale (serveur MCP, proxy local), sans administrateur et sur toutes les éditions ; AppContainer coupe la boucle locale, un compte par ensemble demande l'administrateur et une connexion de chaque harness par compte, Windows Sandbox n'est pas faite pour un agent.

## Conséquences

≈ 8 tickets (WB-1 à WB-8), ≈ 18 h d'agent, ≈ 10 h de bout en bout, plus ≈ 45 min d'essais de l'utilisateur. Sans administrateur, sur toutes les éditions. « Ses fichiers » dirait « Barrière » sous Windows pour les harness d'Escouade. Restent ouverts : le réseau, la lecture des fichiers de l'utilisateur hors des projets, les fichiers que le harness tient lui-même pour tous les projets ; un volume sans ACL ne porte pas la barrière (W3).
