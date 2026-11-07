---
title: "Cursor CLI en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.1)","docs/TODO-2.2.md, INT-10.1 à 10.5","Ticket ESC-222"]
tickets: ["ESC-222","ESC-281","ESC-282","ESC-346"]
decisions: []
compared: null
---

# Cursor CLI en plugin

Cursor CLI est le premier harness sorti du cœur (INT-10.1) : sa conversation passe par le plugin de première main `dev.escouade.cursor-cli`, un process (`escouade-plugins cursor`, sa crate `escouade-plugin-cursor` dans `plugins/cursor/`), livré et signé avec l'app, au registre des plugins livrés et sur le store (sa page pointe vers ce dossier). Pour l'utilisateur, rien ne change : mêmes agents, mêmes textes, même vue terminal.

## Ce que fait le plugin

- Il sert le harness que son manifeste déclare (`contributes.harnesses[0]`, id `cursor`, objet `acp` avec l'extension `cursor`) par le client ACP de `crates/escouade-acp`, comme `dev.escouade.acp` sert les harness décrits ; la description est la sienne, pas celle d'un autre plugin.
- Le cœur trouve `agent` (le lanceur de son installeur lancé en son `node` et son `index.js`), demande au plugin la description du lancement (`agent acp`, `MSYSTEM` et `EXEPATH` retirés), la vérifie et lance Cursor, chaque agent dans son dossier (`CURSOR_CONFIG_DIR`).
- Ce que Cursor écrit va au plugin (`agent/frames`) et revient en éléments de la conversation : session, mode, modèle et **effort** (l'option de chaque modèle, lue de `cursor/list_available_models`, demandée à côté de la session), textes, raisonnements, outils sous les noms de Claude Code avec leur diff, permissions, questions, plan, tâches, sous-agents, fins de tour et refus de Cursor (limite d'usage, modèle hors offre, clé ou connexion refusées), dits comme avant.
- Les images et fichiers texte d'un message partent tels que le cœur les a faits (`content` de `agent/sendMessage`) ; ce qui a été dit dans sa vue terminal va avant le message suivant (`relay` d'`engine/launch`).
- **La clôture d'un projet privé** tient sur ses permissions (`fenceOnPermissions`) : une demande qui nomme un chemin clôturé est refusée d'office, comme avant ; ses lectures et écritures, faites par Cursor lui-même, ne passent pas par Escouade, ce que disent les sens de ses modes.
- **Un compte par sa clé** : le manifeste nomme la variable de la clé (`keyVar` : `CURSOR_API_KEY`) ; le cœur la donne à Cursor, jamais au plugin, et dit seulement au plugin que le compte passe par elle (`byKey`), pour le texte d'une clé refusée.

## Ce qui reste au cœur

Le moteur `Cursor` du registre garde ce qui n'est pas la conversation : ses capacités et ses modes, les comptes (connexion de la machine, comptes par clé), les modèles lus pour le menu d'un compte, les textes rédigés, la vue terminal (son PTY et le suivi de son transcript), le serveur MCP d'Escouade. La traduction de `agent/cursor.rs` reste pour Mistral Vibe et Gemini CLI jusqu'à leur sortie (INT-10.6, 10.7).

## Un arrêt se dit

Coupé pour l'app ou pas chargé, le plugin n'est plus branché derrière Cursor (`harness::sync`) : un agent de Cursor dit au lancement « Le plugin Cursor CLI (dev.escouade.cursor-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness (INT-9).

## Trouvé avec le vrai Cursor CLI

- `agent acp` ne sort pas quand son entrée se ferme : le manifeste le dit (`closeGraceMs` : 2000) et le cœur l'arrête ce délai après, comme son client le faisait (un agent au repos arrêté, puis repris par `session/load`).
- Chaque process d'un agent numérote ses éléments depuis 1 : leurs ids portent la fin de la clé du process, pour qu'une session reprise n'écrase pas la conversation d'avant.

## Depuis INT-10.6

Le branchement de Cursor CLI n'est plus propre à lui : il est une ligne de la table des harness livrés, avec Mistral Vibe (*Mistral Vibe en plugin*). La traduction de `agent/cursor.rs` reste pour Gemini CLI et Antigravity CLI.

## Depuis INT-10.7

Gemini CLI a sa ligne dans la même table (*Gemini CLI en plugin*). La traduction de `agent/cursor.rs` ne reste que pour Antigravity CLI.

## Un module, plus un process (ESC-346)

Le plugin est un module (`run.wasm`), en version `2.2.0`, et n'a plus d'entrée `escouade-plugins cursor`. Il sert le harness par le service de `escouade_acp::plugin`, et ne dépend plus de `plugins/acp`. Il ne déclare aucun fichier. Ce qui précède reste vrai pour la conversation. Voir *Tous les plugins en WASM*.
