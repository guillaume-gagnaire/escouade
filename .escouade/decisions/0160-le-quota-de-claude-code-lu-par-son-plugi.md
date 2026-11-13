---
number: 160
id: "le-quota-de-claude-code-lu-par-son-plugi"
title: "Le quota de Claude Code lu par son plugin, le cœur ne parle plus à api.anthropic.com"
date: "2026-11-13"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-441"
pages: ["harness/comptes"]
tickets: ["ESC-441"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":4,"removed":4}]
accepted_at: "2026-11-13T22:51:24+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le quota d'un compte Claude Code sans agent était lu par le cœur lui-même : `GET https://api.anthropic.com/api/oauth/usage` avec le jeton OAuth lu dans la connexion du compte (usage.rs, `read_oauth`, `fetch_oauth`). Les autres harness passent par leur plugin (`account/read` de `quota`, ESC-359). Deux voies : (1) un process de Claude Code du compte lancé par le cœur, à qui le plugin fait faire `initialize` puis `get_usage` ; (2) le plugin appelle l'endpoint, le cœur lui passant le jeton (`permissions.network` vers api.anthropic.com). En cours de ticket, l'utilisateur a aussi demandé de sortir du cœur les commandes d'installation des 8 harness (`const INSTALL` des façades), que les manifestes des plugins déclarent déjà.

## Décision

Option 1. Le plugin dev.escouade.claude-code (2.2.7) sert `account/read` de `quota` : un `claude` du compte sans tour ni session (`--no-session-persistence --strict-mcp-config`), `initialize` puis `get_usage`, son `rate_limits` rendu tel quel et lu par le cœur comme celui d'un agent (`limits` absent = relevé `Seen`). Le cœur garde le rythme (un compte actif au plus toutes les 5 min, `oauth_due`) et le savoir de la connexion : absente → « Pas connecté » sans process, relue à la lecture suivante ; expirée → « Connexion expirée » sans process. L'endpoint, `fetch_oauth`, `read_oauth`, `api_from_env` et `ESCOUADE_USAGE_API` sont retirés ; `Credentials::Token(String)` devient `Credentials::InDate`, sans jeton. Mesures (Claude Code 2.1.323, 2026-11-13) : environ 1 s par lecture (initialize 0,8 s, get_usage 0,2 s), un seul appel à l'endpoint par process, aucun quand un autre process du compte a lu dans la minute (`cachedUsageUtilization` du `.claude.json` du compte). Les commandes d'installation des 8 harness ne sont plus copiées au cœur : `Engine::install` les lit dans le manifeste du plugin (`plugins::harness::install_of`) ; un manifeste qui n'en a pas donne une commande vide, « Claude Code est introuvable » ne nommant alors que `npm install -g @anthropic-ai/claude-code`, et OpenCode 2 « Installe OpenCode 1. » sans commande.

## Conséquences

Sécurité : le jeton OAuth ne sort plus de Claude Code. Le cœur ne le garde plus en mémoire et n'appelle plus Anthropic ; aucun plugin ne le reçoit ni n'a de permission réseau vers api.anthropic.com. L'option 2 aurait remis à un module WASM, même signé de première main, un jeton qui vaut l'abonnement de l'utilisateur sur toute l'API (pas seulement le quota), exposé à une version compromise ou à un bogue de journalisation, pour gagner 0,8 s sur une lecture faite au plus toutes les 5 minutes. Coût : un process de Claude Code d'environ 1 s par compte actif sans agent, au plus toutes les 5 minutes. Sur un 429, une réponse sans `limits` ne fait jamais baisser une fenêtre et une erreur laisse les dernières valeurs, comme le 429 de l'endpoint. Le plugin claude-code doit être publié (2.2.7) ; sans lui, les comptes sans agent ne sont plus relus. Plus aucune adresse d'Anthropic, de claude.ai ni d'installation d'un harness dans src-tauri/src et crates/ hors tests ; les commandes de mise à jour (`update_command`) restent au cœur.
