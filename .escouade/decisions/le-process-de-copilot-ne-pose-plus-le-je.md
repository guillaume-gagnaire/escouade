---
number: null
id: "le-process-de-copilot-ne-pose-plus-le-je"
title: "Le process de Copilot ne pose plus le jeton d'Escouade dans une variable, et seul un dossier entier va en tête du PATH des hooks"
date: "2026-11-13"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-433"
pages: ["plugins/copilot-cli-en-plugin","plugins/antigravity-en-plugin","plugins/harness-en-process"]
tickets: ["ESC-433"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/copilot-cli-en-plugin","added":5,"removed":4},{"page":"plugins/antigravity-en-plugin","added":2,"removed":0},{"page":"plugins/harness-en-process","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Les relectures de Copilot CLI 2.2.6 et d'Antigravity CLI 2.2.8 (2026-11-13) ont laissé cinq remarques. Le process de Copilot posait encore ESCOUADE_MCP_TOKEN, dont ses commandes héritent, alors que le cœur écrit le jeton dans le fichier privé depuis la 2.2.0-beta.12. Le dossier d'escouade-hook était mis en tête du PATH sans vérifier qu'il est absolu : à la racine d'un lecteur, il donnait C:, le dossier courant de ce lecteur. La lecture des modèles d'Antigravity ne vidait pas les clés de la machine. On se demandait aussi s'il fallait retirer ESCOUADE_HOOK_TOKEN aux commandes d'une vue.

## Décision

1. Copilot CLI 2.2.8 : son process ne pose plus ESCOUADE_MCP_TOKEN, qui rejoint les variables de --secret-env-vars, et le manifeste ne la déclare plus. escouade.min reste 2.2.0. Une bêta 1 à 11, que min ne peut pas écarter (il ne compare que les chiffres), perd le serveur d'Escouade dans ses agents Copilot (choix de l'overseer).
2. Seul un dossier entier va en tête du PATH : un lecteur et sa racine, ou un partage, sans « ; » ni « " ». Sinon le PATH reste tel quel. C'est fait dans escouade-acp 0.1.6 (path_first), pour Gemini CLI 2.2.7 et Mistral 2.2.7, et dans Antigravity CLI 2.2.9.
3. Antigravity CLI 2.2.9 : `agy models` vide les clés de la machine que le compte ne donne pas.
4. ESCOUADE_HOOK_TOKEN reste dans l'environnement des vues : Copilot 1.0.98 retire aussi à ses hooks une variable de --secret-env-vars (mesuré), et sans sandbox, ce qu'escouade-hook lit, une commande de l'agent le lit aussi.
5. Les docs ne disent plus qu'un refus tient « Bypass compris » (il ne vise que l'outil d'écriture), et PROTOCOL.md décrit la ligne de hook d'agy.

## Conséquences

escouade-acp 0.1.6 attend sa publication par l'utilisateur ; d'ici là, Gemini CLI et Mistral sont dans AWAITING_PUBLICATION, sans leur Cargo.lock. Copilot CLI 2.2.8 et Antigravity CLI 2.2.9 ont leur verrou et sont à relire et signer. Sur une bêta 2.2.0 antérieure à la beta.12, mettre Escouade à jour.
