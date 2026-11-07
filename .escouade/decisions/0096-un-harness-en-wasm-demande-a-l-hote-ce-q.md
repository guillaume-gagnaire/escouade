---
number: 96
id: "un-harness-en-wasm-demande-a-l-hote-ce-q"
title: "Un harness en WASM demande à l'hôte ce qu'il ne peut pas lire : fs/gitProtected, et le service ACP passe dans escouade-acp"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-346"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/codex-en-plugin","plugins/copilot-cli-en-plugin","plugins/cursor-cli-en-plugin","plugins/gemini-cli-en-plugin"]
tickets: ["ESC-346"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":10,"removed":0},{"page":"plugins/codex-en-plugin","added":2,"removed":0},{"page":"plugins/copilot-cli-en-plugin","added":2,"removed":0},{"page":"plugins/cursor-cli-en-plugin","added":2,"removed":0},{"page":"plugins/gemini-cli-en-plugin","added":2,"removed":0}]
accepted_at: "2026-11-06T08:49:46+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Porter Codex, Copilot CLI, Cursor CLI et Gemini CLI en run.wasm (P3) a montré ce que le prototype n'avait pas vu. La garde qui accepte git add et git commit sans carte lit .git et lance git rev-parse, ce qu'un module ne peut pas : il aurait demandé à chaque commit, sans erreur. Cursor et Gemini dépendent de plugins/acp par un chemin, qu'un dossier seul ne compile pas. Un chemin d'agent lu par fs/read exige l'agent, que turn::read_file n'envoie pas. Et Codex et Copilot sont à la fois membres du workspace de plugins/ et dépendances du cœur, avec un cdylib, donc sans empreinte dans leurs noms de fichiers : dans un même dossier de compilation, l'app reprenait une rlib bâtie contre un autre serde_json.

## Décision

L'hôte gagne la méthode fs/gitProtected (par escouade.call) : pour le dossier d'un agent de ce plugin, avec la racine agentRepo déclarée, elle dit si git ne peut rien lancer de l'agent ; refusée, ou sans réponse de git en une seconde, elle dit non et la carte s'affiche. Le service du protocole ACP entre dans la crate escouade-acp (escouade_acp::plugin), avec agent_file pour lire un fichier d'agent ; plugins/acp n'est plus qu'une façade. escouade-acp passe à 0.1.1 ; le protocole ne change pas. Un agent sans dossier de compte reçoit celui de son harness (~/.gemini et non ~). Les modules du registre se précompilent à leur première activation, comme ceux de l'amorce. Copilot perd la crate url, dont les macros sortaient de rust_native.json. Le workspace de plugins/ range ses compilations intermédiaires à part (build-dir, src-tauri/target/plugins).

## Conséquences

Les quatre manifestes déclarent ce qu'ils lisent (agentRepo pour Codex et Copilot, agentHome pour Codex et Gemini) : la fenêtre d'installation le dit, et l'utilisateur peut le retirer, au prix d'une carte de plus. Ils demandent aussi limits.memoryBytes et limits.turnSeconds, repris d'ESC-347. fs/gitProtected est une méthode publique, ouverte aux tiers aux mêmes conditions, qui ne rend qu'un booléen. Les Cargo.lock et les empreintes des quatre plugins attendent la publication d'escouade-acp 0.1.1 par le chef ; d'ici là plugins/ci.mjs ne passe pas sur eux. Une adresse non ASCII dans une permission de Copilot vaut pour une fois, et non pour la session. INT-0 reste à remesurer sur un poste vide pour Cursor à son rythme, et l'écart au 99e centile au plus vite de Copilot, Gemini et Codex n'est pas expliqué.
