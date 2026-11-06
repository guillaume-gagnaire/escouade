---
title: "Codex en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.4)","docs/TODO-2.2.md, INT-10.1 à 10.5","Ticket ESC-225","plugins/codex/README.md"]
tickets: ["ESC-225","ESC-346"]
decisions: []
compared: null
---

# Codex en plugin

La conversation des agents de Codex passe par le plugin de première main `dev.escouade.codex` (INT-10.4) : un process (`escouade-plugins codex`, sa crate `escouade-plugin-codex` dans `plugins/codex/`), livré et signé avec l'app, au registre des plugins livrés et sur le store (sa page pointe vers ce dossier). Pour l'utilisateur, rien ne change : mêmes agents, mêmes modes, même coût estimé, même vue terminal.

Codex n'est pas un harness ACP : il garde son client propre, `codex app-server` en JSON-RPC, une ligne par message.

## Ce que fait le plugin

- Il décrit le lancement (`app-server`, l'outil de plan, les profils de permissions d'un projet privé, le serveur d'Escouade par `-c`, son jeton dans une variable) ; le cœur trouve `codex`, vérifie la description et le lance, chaque compte dans son `CODEX_HOME`.
- Il ouvre le fil par le protocole (nouveau, repris ou copié), lit les modèles du compte, toutes pages, puis dit que l'agent est prêt.
- Ce que Codex écrit lui arrive et repart en éléments de la conversation : raisonnements et textes au fil de l'écriture, outils sous les noms de Claude Code avec leur diff, cartes de permission, questions, plan du mode Plan et son approbation, tâches, sous-agents, jauge de contexte, quota, fin de tour.
- Il glisse un message dans le tour en cours et prend le modèle, l'effort et le mode au tour suivant, comme Codex le demande.
- Il accepte sans carte `git add` et `git commit` du worktree de l'agent, en « Accepter les modifications » et « Auto », par les mêmes règles qu'avant.
- Il estime le coût d'un tour : la grille publique d'OpenAI est dans sa crate.

## Ce qui reste au cœur

Le moteur `Codex` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes et leur connexion, le quota et les modèles d'un compte sans agent (un app-server à lui, lancé puis arrêté), les textes rédigés (`codex exec`), la vue terminal et la confiance de ses hooks, le serveur MCP d'Escouade.

**La vue Codex lit son rollout par la traduction du plugin, prise comme une bibliothèque**, pour une seule traduction dans les deux vues.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, un agent de Codex dit au lancement « Le plugin Codex (dev.escouade.codex) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness : les demandes en attente restent répondables par leur id, et ce qui se traduisait peut être perdu (INT-9).

## Un module, plus un process (ESC-346)

Le plugin est un module (`run.wasm`), en version `2.2.0`, et n'a plus d'entrée `escouade-plugins codex`. Ce qui précède reste vrai pour la conversation, à deux points près : `git add` et `git commit` sont jugés par Escouade (`fs/gitProtected`, racine `agentRepo`), et `config.toml` se lit par l'hôte (racine `agentHome`). Voir *Tous les plugins en WASM*.
