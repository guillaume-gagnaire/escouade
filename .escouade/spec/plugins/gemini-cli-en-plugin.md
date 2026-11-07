---
title: "Gemini CLI en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.5, §4.6 (INT-10.7)","docs/TODO-2.2.md, INT-10.7","Ticket ESC-282","plugins/gemini/README.en.md"]
tickets: ["ESC-282","ESC-346","ESC-369"]
decisions: [63]
compared: null
---

# Gemini CLI en plugin

Gemini CLI est le troisième harness sorti du cœur (INT-10.7) : sa conversation passe par le plugin de première main `dev.escouade.gemini-cli`, un process (`escouade-plugins gemini`, sa crate `escouade-plugin-gemini` dans `plugins/gemini/`), livré et signé avec l'app, au registre des plugins livrés et sur le store (sa page pointe vers ce dossier). Pour l'utilisateur, rien ne change : mêmes agents, mêmes comptes, mêmes textes, même vue terminal.

## Ce que fait le plugin

- Il sert le harness que son manifeste déclare (`contributes.harnesses[0]`, id `gemini`, objet `acp` avec l'extension `gemini`) par le client ACP de `crates/escouade-acp`, comme Cursor CLI et Mistral Vibe.
- Le cœur trouve `gemini`, écrit dans le `GEMINI_CLI_HOME` de l'agent ses réglages (ni mise à jour, ni statistiques, sa façon de se connecter) et, pour un projet privé ou une discussion, ses règles ; il retire la connexion que Gemini y a rafraîchie, demande au plugin la description du lancement (`--acp`, dossier de confiance par `GEMINI_CLI_TRUST_WORKSPACE`), y ajoute `--policy` s'il y a des règles, la vérifie et lance Gemini.
- Ce que Gemini écrit va au plugin et revient en éléments de la conversation, comme le faisait le cœur : session (nouvelle, ou reprise par `session/load`, dont le rejeu de l'historique est laissé de côté), mode, modèle (laissé à son défaut si sa session ne l'offre pas), textes, raisonnements, outils sous les noms de Claude Code (l'outil lu dans l'id de l'appel, un sous-agent en `Task`, les outils MCP), permissions, fin de tour avec ses jetons, sans coût.
- **Son changement de mode** est un texte (« [MODE_UPDATE] plan ») : lu comme tel, jamais dans le fil.
- **Ses questions** : le premier message lui dit de finir son tour sur un bloc `<escouade_question>`, sorti du texte et montré en carte ; le tour attend les réponses (un message envoyé entre-temps attend lui aussi, l'arrêt finit le tour) et continue avec elles, ses jetons additionnés.
- **Son plan** : la permission de son `exit_plan_mode` est la carte « Plan à approuver », le plan lu dans le fichier qu'elle nomme ; approuvé, le mode choisi est posé une fois Gemini sorti de son mode plan.
- **Un refus coupe le tour** (`session/cancel`) ; le mot de l'utilisateur part ensuite. « Pour cet agent » est celui de Gemini, pour la session.
- **Sa limite d'usage** finit le tour « limité », la reprise prévue quand il la dit, sinon une minute (une heure pour son quota du jour).
- **Un projet privé** et **une discussion** : ses règles (`--policy`) refusent les chemins clôturés à ses outils de fichiers et à ses commandes dans tous les modes, et les écritures et commandes d'une discussion ; le plugin refuse d'office une permission qui nomme un chemin clôturé (`fenceOnPermissions`).

## Ce qui reste au cœur

Le moteur `Gemini` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes (une connexion Google par le fichier du dossier du compte, `GOOGLE_APPLICATION_CREDENTIALS`, ou une clé, `GEMINI_API_KEY`, jamais donnés au plugin), les modèles lus pour le menu d'un compte (un `gemini --acp` jetable), les textes rédigés (`gemini -p`, tout outil refusé), la vue terminal par ses hooks seuls, les fichiers de son `GEMINI_CLI_HOME`, le serveur MCP d'Escouade.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, le plugin n'est plus branché derrière Gemini : un agent de Gemini dit au lancement « Le plugin Gemini CLI (dev.escouade.gemini-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness (INT-9).

## Une ligne de la table

Gemini CLI n'ajoute qu'une ligne à la table des harness livrés, qui a grandi pour tous : ce que le cœur prépare avant le lancement peut ajouter des arguments (`--policy`), et il connaît la lecture seule et les variables données à l'agent.

## Différences assumées

En « Accepter les modifications », un `git add` ou `git commit` du worktree que le cœur acceptait sans demander demande désormais, comme pour Cursor CLI et Mistral Vibe : la règle lit le dépôt, ce qu'un plugin ne fait pas. La carte d'une commande git ne dit plus qu'elle écrit dans le dépôt, pour la même raison.

## Sans compte

Aucun tour avec un vrai modèle n'a été joué : l'essai réel `real_gemini` reste écrit et ignoré, faute de compte sur le poste. Le plugin est prouvé par le banc, les flux de l'étude de Gemini CLI 0.66.0 rejoués par son client et les e2e sur le faux Gemini.

## Un module, plus un process (ESC-346)

Le plugin est un module (`run.wasm`), en version `2.2.0`, et n'a plus d'entrée `escouade-plugins gemini`. Il sert le harness par le service de `escouade_acp::plugin`. Le plan que nomme `exit_plan_mode` se lit par l'hôte, dans le dossier du compte de l'agent (racine `agentHome`). Ce qui précède reste vrai pour la conversation. Voir *Tous les plugins en WASM*.
