---
title: "Claude Code en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.5)","docs/PLUGINS.md §8.27 (ESC-347)","docs/TODO-2.2.md, INT-10.1 à 10.5","Ticket ESC-226","plugins/claude/README.en.md"]
tickets: ["ESC-226","ESC-347","ESC-352","ESC-364","ESC-369"]
decisions: []
compared: null
---

# Claude Code en plugin

La conversation des agents de Claude Code passe par le plugin de première main `dev.escouade.claude-code` (INT-10.5) : un module WASM depuis ESC-347 (`run.wasm`, sa crate `escouade-plugin-claude` dans `plugins/claude/`), avec sa version à lui, joué dans l'hôte de l'app. Il est au registre des plugins livrés et sur le store (sa page pointe vers ce dossier). Pour l'utilisateur, rien ne change : mêmes agents, mêmes comptes, mêmes modèles par compte, même quota et même changement de compte au seuil, même passation, mêmes textes rédigés, même vue terminal.

## Ce que fait le plugin

- Il décrit le lancement (le flux stream-json, le modèle, l'effort et le mode, les consignes ajoutées, la session à reprendre ou copier, les dossiers lus sans question, le serveur d'Escouade par un fichier privé de l'agent) ; le cœur trouve `claude`, vérifie la description et le lance, chaque compte dans son `CLAUDE_CONFIG_DIR`.
- Il pose `initialize` comme première ligne, puis dit au cœur les commandes et les modèles du compte, et que l'agent est prêt.
- Ce que Claude Code écrit lui arrive et repart en éléments de la conversation : textes et raisonnements au fil de l'écriture, appels avec leur diff, sous-agents et tâches de fond, cartes de permission et questions, estimation du tour en cours, fin de tour avec le coût par modèle, quota, limite d'usage, connexion perdue.
- Un changement de modèle, d'effort ou de mode, une interruption, un titre, une tâche arrêtée et le lien à claude.ai passent par le protocole de contrôle : le cœur attend la réponse de Claude Code, comme avant.

Son manifeste ne déclare aucune permission : ni réseau, ni secret, ni fichier, ni commande. L'hôte l'applique. Il demande seulement les plafonds de mémoire et de temps d'un tour (`limits`) : un message peut porter 24 Mio en base64.

## Ce qui reste au cœur

Le moteur `Claude` du registre garde ce qui n'est pas la conversation : ses capacités, les comptes, les modèles d'un compte sans agent, les textes rédigés (`claude -p`), la vue terminal et ses hooks, le serveur MCP d'Escouade. L'état de chaque agent reste au cœur : sa conversation, son plan, ses demandes en attente. Une lecture d'un fichier que l'utilisateur a joint reste acceptée sans carte, par le cœur.

La lecture d'une connexion (ce que Claude Code garde dans son dossier et, sous macOS, son trousseau, l'empreinte d'une connexion) est au cœur seul, dans `src-tauri/src/engine/claude/signin.rs` : le plugin ne voit jamais une connexion. Il ne garde que les mots d'un tour qui l'a perdue, pour les traduire. Le cœur garde aussi ce qui décide : relancer un agent une fois sa connexion bonne, et le quota lu au point d'accès OAuth avec les réglages réseau de l'app.

**La vue Claude Code passe par le module** (`view/translate`, ESC-352) : le cœur lit le transcript et passe ses lignes au plugin, une seule traduction pour les deux vues. Le coût d'un tour de la vue reste calculé par le cœur, sur l'usage de ses messages aux prix publics, et rendu au plugin avec la fin du tour. Voir *Le contrat des harness en process*.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, un agent de Claude Code dit au lancement « Le plugin Claude Code (dev.escouade.claude-code) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness et la conversation le dit (voir *Le contrat des harness en process*). Les textes rédigés et le quota d'un compte ne passent pas par lui. Sa vue terminal marche plugin coupé, mais la conversation ne la suit plus, et le dit.

## Une discussion ne lance rien (ESC-364)

Une discussion perd les outils qui écrivent ou lancent une commande, et ceux qui lui restent sont refusés d'office s'ils arrivent : `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash`, `PowerShell` (le shell ordinaire de Claude Code sous Windows), `Monitor` et `EnterWorktree`, qui crée un worktree. La liste est tenue des deux côtés, le plugin et le cœur, qui ne se lient pas : un test du cœur vérifie qu'elles sont les mêmes.
