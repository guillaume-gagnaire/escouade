---
number: 152
id: "variables-des-plugins-declarees-versions"
title: "Variables des plugins déclarées, versions vérifiées par défaut, harness tiers bornés"
date: "2026-11-12"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-424"
pages: ["plugins/harness-acp-decrits","plugins/tous-les-plugins-en-wasm","store/store-dans-l-app"]
tickets: ["ESC-424","ESC-418"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":8,"removed":0},{"page":"plugins/tous-les-plugins-en-wasm","added":6,"removed":0},{"page":"store/store-dans-l-app","added":2,"removed":0}]
accepted_at: "2026-11-12T20:03:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de sécurité de 2026-11 (ESC-418) a montré (SEC-05, démontré) qu'un plugin autorisé à lancer `git status` lançait n'importe quelle commande par les variables d'environnement qu'il joint (`GIT_CONFIG_*`, `GIT_SSH_COMMAND`, `PYTHONSTARTUP`, `*_PROXY`, `HOME`…) : l'hôte ne refusait qu'une courte liste. Il a relevé (SEC-06) qu'un plugin tiers s'installait par défaut sans revue, et qu'un harness décrit par un tiers pouvait nommer un interpréteur comme programme et lancer ses `versionArgs` sans geste.

## Décision

1. Un plugin ne pose que les variables que son manifeste déclare (`permissions.env`), montrées à l'installation ; le `acp.env` d'un harness décrit compte comme déclaré, et le cœur pose lui-même `accountVar`, `keyVar` et l'adresse des hooks. 2. Une liste de familles est refusée à un tiers même déclarée (`ESCOUADE_*`, `LD_*`, `DYLD_*`, `GIT_*` sauf auteur et committer, `PYTHON*`, `RUBY*`, `PERL*`, `NODE_*`, `JAVA_TOOL_OPTIONS`, `npm_config_*`, `*_PROXY`, `SSL_*`, `XDG_*`, `HOME`, `USERPROFILE`, `APPDATA`, `PATH`, `SHELL`, `EDITOR`, `PAGER`…), par le lecteur du manifeste (app, `escouade-plugin validate`, store) comme par l'hôte. La première main est tenue à sa déclaration seule (Antigravity CLI pose `HOME` du dossier de son agent) ; une version de la première main d'avant le champ garde ses lancements. 3. « N'installer que des versions vérifiées » est allumé pour une nouvelle installation ; une installation existante garde son choix. 4. Un harness d'un tiers ne nomme comme programme ni un shell ni un interpréteur connu, et son `versionArgs` ne tourne qu'après un geste de l'utilisateur sur ce harness (un agent ou un terminal lancé) dans la session.

## Conséquences

Le format du manifeste grandit (`permissions.env`, codes `variable` et `interpreter`) ; le schéma et la bibliothèque du store suivent. Neuf plugins de première main montent d'un cran (Antigravity CLI 2.2.7, Claude Code 2.2.6, Codex, Copilot CLI, Cursor CLI, Gemini CLI, Mistral et OpenCode 2.2.5, isola 2.2.2) : leur publication revient au chef. Un tiers qui posait une variable doit la déclarer, et ne peut plus poser une des familles. La version d'un harness tiers est inconnue jusqu'au premier agent lancé.
