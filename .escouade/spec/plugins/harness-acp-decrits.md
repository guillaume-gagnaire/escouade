---
title: "Les harness ACP décrits"
subject: "plugins"
sources: ["docs/PLUGINS.md §1.1, §4.4, §4.12, §8.27, §8.31","docs/TODO-2.2.md, PLG-6","Essai réel de Goose 1.57.0 sur Ollama, 2026-11-01 (ESC-178)"]
tickets: ["ESC-178","ESC-222","ESC-281","ESC-282","ESC-347","ESC-361","ESC-362"]
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

## Ajouts d'INT-10.1 (ESC-222)

- Le service de `dev.escouade.acp` sert aussi un plugin de première main qui a son propre harness (`serve_as`) : Cursor CLI (*Cursor CLI en plugin*), dont la description est celle de son manifeste.
- L'objet `acp` gagne `fenceOnPermissions` (avec `"fs": false`, la clôture tenue sur les seules permissions, crue d'un plugin livré seulement) ; le harness gagne `keyVar` (la variable de la clé d'un compte).
- Le contrat gagne `relay` et `byKey` dans `engine/launch`, `content` dans `agent/sendMessage` (le message tel que le cœur l'a fait : ses images, ses fichiers texte) ; avec l'extension `cursor`, l'effort passe par l'option de chaque modèle (`cursor/list_available_models`).

## Ajouts d'INT-10.6 (ESC-281)

- Mistral Vibe est sorti du cœur à son tour (*Mistral Vibe en plugin*) : l'extension `vibe` du client (`crates/escouade-acp`, `vibe.rs`) tient ses outils, ses formulaires (`elicitation/create`), son mode Plan joué par Escouade, son refus qui coupe le tour, sa copie par `session/fork`, sa réflexion et son coût. Seul Gemini CLI, dans le cœur, appelle encore les aides partagées.
- Le contrat gagne l'élément `session` : la session ouverte avant tout tour, sans passer l'agent « en cours ».
- Dans un flux enregistré rejoué, les règles d'Escouade n'accordent aucune permission à la place de l'essai.

## Ajouts d'INT-10.7 (ESC-282)

- Gemini CLI est sorti du cœur à son tour (*Gemini CLI en plugin*) : l'extension `gemini` du client (`crates/escouade-acp`, `gemini.rs`) tient ses appels sans `rawInput`, son mode dit par un texte, son plan lu dans un fichier, son tour tenu pour une question, ses jetons et sa limite d'usage.
- Pour tout harness `questionsInText` : un bloc qui n'est pas une question est montré tel qu'il a été écrit, et l'arrêt d'un tour tenu par une question le finit.

## Ajouts d'ESC-347 : `dev.escouade.acp` en module

- `dev.escouade.acp` est un module WASM (`run.wasm`), plus l'entrée `escouade-plugins acp` : la phrase du début le dit encore. Son service est `escouade_acp::plugin`.
- **Les fichiers qui passent par le client** vont à l'hôte, par `fs/*`, avec la clé de l'agent. Lecture dans le dossier de l'agent et dans son dépôt, écriture dans le dossier de l'agent, 4 Mio par fichier. Avant, le client atteignait tout chemin absolu hors clôture.
- Ce qui sort de ces dossiers est refusé en `-32000` : « Refused by Escouade: … is out of this agent's folders. » L'agent garde les mêmes codes et les mêmes textes pour le reste : `-32000` pour un chemin relatif, un projet privé, un agent en lecture seule, `-32002` pour un fichier qu'on ne lit pas.
- **La clôture** est appliquée par l'hôte sur ces fichiers, liens suivis, quel que soit le code du plugin.
- Une carte de permission juge un chemin par son texte, d'après la forme du chemin et non le système de la cible : `C:\Prive` et `c:\prive` sont le même chemin sous Windows, y compris dans un module.
- Le diff d'un `write` de Goose lit le fichier par le même chemin : hors de ces dossiers, il se montre comme un fichier nouveau.

## La confiance d'un harness décrit (ESC-361)

Un harness décrit est servi par `dev.escouade.acp`, mais c'est son auteur qui est jugé. Est de première main un plugin dont l'id commence par `dev.escouade.` : le manifeste refuse cet id à un tiers, et il ne s'installe que signé. Goose l'est, sans être livré avec l'app.

- **`fenceOnPermissions`** n'est cru que de la première main. Celui d'un manifeste tiers est effacé : en `"fs": false`, le harness ne démarre pas dans un projet privé, et le dit. Un tiers en `"fs": true` y entre : la clôture y est tenue par le client sur les fichiers qui passent par lui, et ce que l'agent lit ou écrit seul est dit une fois dans la conversation.
- **Les variables** d'un plugin qui n'est pas de première main sont jugées comme `proc/*` les juge : ni `ESCOUADE_*`, ni `LD_*`, ni `DYLD_*`, ni `PATH`, `PATHEXT`, `COMSPEC`, `NODE_OPTIONS`, `BASH_ENV`, `ENV`, `IFS`, quelle que soit la casse. Cela vaut pour `acp.env`, pour `accountVar` et `keyVar` que le cœur pose, et pour un harness tiers à module. L'agent ne démarre pas, et la variable refusée est nommée.
- **Le programme** d'un tiers se cherche par des noms nus : un nom de `locate.names` qui porte un chemin est écarté.
- **L'installation montre tout ce qui tournera** : une ligne `harness:` par nom du programme, ses variables devant (`VAR=valeur programme arguments`), et une ligne de plus pour `versionArgs` quand ce n'est pas `--version`.
- **`versionArgs`** ne tourne qu'une fois le plugin installé et allumé avec `dev.escouade.acp` : ni un aperçu, ni un plugin coupé ne lancent rien. Il tourne ensuite à chaque liste des harness, sans agent ni geste.

## Ajouts d'ESC-362

- **La clôture et la lecture seule** reviennent par `agent/attach`, sur un agent créé comme sur un agent déjà là : elles tiennent à travers un redémarrage du plugin, sur les permissions comme sur `fs/*`, et le cœur l'emporte, une clôture retirée se retire. Voir *Le contrat des harness en process*.
- **Une écriture dans `.git`** a sa carte, même en « Édits auto » et, pour Gemini et Vibe, en « Demander » : elle n'est plus prise pour une écriture dans le dossier de l'agent. Seul « Bypass » la laisse passer.
- **Un agent recréé garde sa session.** Seul un lancement ouvre une session, et le harness tourne encore : le plugin reprend du cœur la session, le tour en cours et les permissions en attente. Ses messages repartent sur cette session, ce que dit le harness est lu, et la clôture tient sur le reste du tour pour Cursor, Gemini et Mistral Vibe, qui n'ont qu'elle (`fenceOnPermissions`). Le tour en cours finit par sa réponse, que le plugin reconnaît à son `stopReason` ou à son erreur. Une permission en attente se répond par son id, avec les options par défaut d'ACP : ce qu'elle proposait est perdu. Un plan ou une question tenus par le client ne se reprennent pas.
