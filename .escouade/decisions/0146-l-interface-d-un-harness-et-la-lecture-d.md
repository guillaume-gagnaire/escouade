---
number: 146
id: "l-interface-d-un-harness-et-la-lecture-d"
title: "L'interface d'un harness et la lecture d'un compte passent par son plugin ; le cœur rend ses copies"
date: "2026-11-12"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-359"
pages: ["plugins/harness-en-process"]
tickets: ["ESC-359"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-en-process","added":11,"removed":1}]
accepted_at: "2026-11-12T09:35:11+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Après P8 (ESC-352), le cœur lançait encore lui-même les interfaces des « Vue X » et lisait le quota et les modèles d'un compte, sur environ 2 200 lignes copiées des plugins. Une règle de lancement corrigée au store ne corrigeait donc pas l'interface, et il fallait une version de l'app. La méthode `terminal/launch` existait au contrat, mais aucun plugin ne la servait. Il fallait aussi dire comment le quota et les modèles passent par le plugin : par un process du harness, ou par `proc/*` depuis le module.

## Décision

Chaque plugin de harness sert `terminal/launch`, que le cœur vérifie comme le lancement d'un agent (`check_launch`) avant de garder le PTY ; les fichiers d'une interface vont sous le dossier de l'agent ou à côté de son fichier privé. Ce qu'un compte dit par un process à lui (ses modèles, son quota, ce qu'une interface doit lire avant de partir, la copie d'une session) passe par `account/read` et `account/lines` : le plugin décrit le process et le dialogue, et le cœur le lance comme un agent (le programme qu'il a trouvé, les variables du compte, son Job object). Pas de `proc/*`, qui ne donne pas les secrets d'un compte et ne connaît ni le programme trouvé ni WSL. Restent au cœur, et c'est dit : la connexion d'un compte de Codex, le menu des modèles de Cursor CLI, Mistral Vibe et Gemini CLI (le cœur le fait aussi pour leurs agents qui tournent), les grilles de prix (le coût reste au cœur) et les textes rédigés de Claude Code, Codex, Copilot CLI et OpenCode. Plugin coupé ou désinstallé, l'interface ne s'ouvre pas, et la fenêtre dit pourquoi.

## Conséquences

Les huit copies listées sortent du cœur, et `Engine::escouade_launch` aussi. Le contrat grandit : `escouade-plugin-protocol` 0.1.2 et `escouade-acp` 0.1.4 sont à publier par l'utilisateur ; d'ici là, la CI joue les modules du dépôt (`AWAITING_PUBLICATION`). Neuf plugins sont à republier et signer : Claude Code 2.2.5, Codex 2.2.4, Copilot CLI 2.2.4, Antigravity CLI 2.2.6, OpenCode 2.2.4, Cursor CLI 2.2.4, Mistral 2.2.4, Gemini CLI 2.2.4 et ACP 2.2.4. Une interface ne s'ouvre plus sans son plugin.
