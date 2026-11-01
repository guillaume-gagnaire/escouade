---
title: "Les harness ACP décrits"
subject: "plugins"
sources: ["docs/PLUGINS.md §1.1, §4.4, §4.12","docs/TODO-2.2.md, PLG-6","Essai réel de Goose 1.57.0 sur Ollama, 2026-11-01 (ESC-178)"]
tickets: ["ESC-178"]
decisions: []
compared: null
---

# Les harness ACP décrits

Un agent qui parle ACP v1 (Agent Client Protocol) devient un harness d'Escouade par un plugin **déclaratif** : une entrée de `contributes.harnesses` avec un objet `acp`. Aucun programme du plugin ne tourne : le plugin livré avec Escouade `dev.escouade.acp` (`escouade-plugins acp`, présent dans toutes les versions, release comprise) sert ces harness sur le contrat des harness en process.

## Le client ACP générique

- Le client est la crate `crates/escouade-acp`, tirée de celui de Cursor CLI ; ce qui est propre à Cursor (`cursor/ask_question`, `cursor/create_plan`, `cursor/update_todos`, ses sous-agents, ses refus « Upgrade your plan », Auto en `default[]`) est son module `cursor`, pris quand le manifeste nomme l'extension `cursor`.
- Les aides partagées (`mcp_servers`, `preamble`, `option_of`, `fenced`, `line_hunks`, les questions en texte) sont celles de la crate ; Cursor CLI, Mistral Vibe et Gemini CLI dans le cœur les appellent jusqu'à leur sortie en process (INT-10.1, 10.6, 10.7).
- Le contrat gagne des éléments : `delta` (le texte en continu), `toolInput` (l'entrée ou le nom d'un appel connus plus tard), `todos`, `mode`, `subagent` (le modèle d'un sous-agent), `result` de `toolDone` (le diff d'une écriture), une permission avec son outil, sa description, sa raison, son diff, « Pour cet agent » et son sous-agent, un tour en erreur ; `resume` dans `agent/answerPermission` (le mode après un plan), `described` et `lang` dans `engine/launch` et `agent/attach`, `fenceByClient` dans la description d'un lancement.

## Ce que le manifeste décrit

Le programme (`locate.names`, cherché dans le PATH ; un plugin tiers ne donne jamais de chemin), `versionArgs`, les versions testées, la variable du dossier de config (`accountVar`), les modes et leur sens (`caps.modes`, par id ou `{ id, meaning }`), les modèles, ce qu'il sait faire (`EngineCaps` : images, PDF, lecture seule…), l'installation (`install`, `updateCommand`), les textes (`speaker`, `writer`). L'objet `acp` : `args`, `env`, `removeEnv`, `fs` (vrai par défaut), `modes` (le mode ACP de chaque mode d'Escouade), `readOnlyMode`, `modelOption`, `questionsInText`, `tools` (les noms de Claude Code de ses outils `other`), `subagentTools`, `extensions`.

## Ce qu'Escouade tient

- **L'id** d'un harness décrit est `plugin:<plugin>:<harness>` (le nu pour un plugin `dev.escouade.*`, qui garde l'id qu'avait son harness dans le cœur). Il entre au registre au démarrage, à l'installation, à l'activation (`Core::sync_harnesses`, la commande `engines_declared`), et en sort quand son plugin ou `dev.escouade.acp` est désinstallé ou désactivé : ses agents restent, « Harness absent », leurs données et leur conversation gardées, et reviennent avec lui.
- **L'installation** montre le programme que ses agents lanceront (la permission `harness:<programme> <arguments>`, forte, non révocable).
- **Le serveur MCP d'Escouade** va à la session (`mcpServers` de `session/new` et `session/load`), son jeton dans un en-tête sur l'entrée de l'agent, jamais dans ses arguments.
- **La clôture des projets privés** tient sur les lectures et écritures qui passent par le client (`fs/read_text_file`, `fs/write_text_file`), refusées pour un autre projet, et sur les permissions qui nomment un chemin clôturé (refusées d'office). Une lecture ou une écriture que l'agent fait lui-même pendant qu'une clôture tient est dite une fois dans la conversation. Un harness déclaré `"fs": false` ne démarre pas dans un projet privé, et le dit.
- **La lecture seule** (une discussion de l'onglet Docs) demande un `readOnlyMode` : l'agent y est mis, ses écritures et commandes sont refusées.
- **Les permissions** : chaque demande est une carte (avec son diff pour une écriture) ; « Édits auto » laisse passer les écritures dans le dossier de l'agent, « Bypass » tout.
- **Les questions** d'un agent sans outil pour en poser (`questionsInText`) : un bloc `<escouade_question>` en fin de tour devient une carte, les réponses partent comme message suivant.
- Les jetons d'un tour sont comptés quand l'agent les donne (`usage` de la réponse au prompt) ; ni quota, ni coût, ni vue terminal, ni textes rédigés pour un harness décrit.

## Goose

Le plugin `dev.escouade.goose` (`plugins/goose/`) décrit Goose : `goose acp`, modes approve, smart_approve et auto, `chat` en lecture seule, le modèle par l'option `model`, ses outils `shell`, `write`, `edit`, `read` sous les noms de Claude Code, `GOOSE_PATH_ROOT` par compte. Essai réel du 2026-11-01 avec Goose 1.57.0 sur Ollama (`real_goose`) : réponse, écriture et commande demandées puis accordées, jetons comptés. Goose écrit ses fichiers lui-même même quand le client annonce `fs` : `"fs": false`, aucun projet privé ne lui est ouvert.
