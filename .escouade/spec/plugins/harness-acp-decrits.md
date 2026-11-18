---
title: "Les harness ACP décrits"
subject: "plugins"
sources: ["docs/PLUGINS-DESIGN.md §1.1, §4.4, §4.12, §8.27, §8.31","docs/TODO-2.2.md, PLG-6","Essai réel de Goose 1.57.0 sur Ollama, 2026-11-01 (ESC-178)","Audit de sécurité 2026-11 (ESC-418), fiches SEC-05, SEC-06 et SEC-30 : docs/security/audit-2026-11/rapport.md","docs/PLUGINS.md §1.3, §2.2","Essai réel de Prime Agent 0.10.3 dans une distribution WSL jetable, 2026-11-16 (ESC-485) : tests/fixtures/prime-agent/","https://github.com/PrimeIntellect-ai/prime-agent (README, CHANGELOG 0.10.3, crates/pa-cli/src/interactive_mode/daemon.rs, crates/pa-core/src/platform/process.rs, crates/pa-daemon/src/platform/paths.rs)","Essai réel d'Unharnessed 0.9.8 sur Ollama dans une distribution WSL jetable, 2026-11-16 (ESC-484) : tests/fixtures/unharnessed/","https://github.com/unharnessed-agent/Unharnessed (README, crates/uh-daemon, vscode/scripts/package.mjs)"]
tickets: ["ESC-178","ESC-222","ESC-281","ESC-282","ESC-347","ESC-361","ESC-362","ESC-424","ESC-428","ESC-442","ESC-443","ESC-489","ESC-484","ESC-485","ESC-491"]
decisions: [152,213,161,162,198,205,207,206]
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

## Ajouts d'ESC-424 (audit de sécurité, SEC-05 et SEC-06)

Ceci remplace, dans « La confiance d'un harness décrit », les phrases sur les variables et sur `versionArgs`.

- **Les variables déclarées.** Un lancement (`engine/launch`, `terminal/launch`, `engine/oneShot`, `account/read`, la relance, l'attach que rend `agent/open` pour un terminal qui rejoint un process partagé) ne pose que les variables que le plugin qui décrit le harness déclare (`permissions.env`), celles de son `acp.env` (que l'installation montre déjà) et celles que le cœur pose lui-même (`accountVar`, `keyVar`, `ESCOUADE_HOOK_URL`). Toute autre est refusée : l'agent ne démarre pas, et la variable est nommée.
- **Les familles refusées à un tiers, même déclarées** : `ESCOUADE_*`, `LD_*`, `DYLD_*`, `GIT_*` sauf `GIT_AUTHOR_*` et `GIT_COMMITTER_*`, `PYTHON*`, `RUBY*`, `PERL*`, `NODE_*`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `JDK_JAVA_OPTIONS`, `npm_config_*`, `*_PROXY`, `SSL_*`, `CURL_CA_BUNDLE`, `REQUESTS_CA_BUNDLE`, `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `XDG_*`, `PATH`, `PATHEXT`, `COMSPEC`, `SHELL`, `BASH_ENV`, `ENV`, `ZDOTDIR`, `IFS`, `PROMPT_COMMAND`, `EDITOR`, `VISUAL`, `PAGER`, `BROWSER`, quelle que soit la casse. Cela vaut pour `acp.env`, `accountVar` et `keyVar`. Le lecteur du manifeste refuse déjà un manifeste tiers qui en nomme une (code `variable`), dans l'app, dans `escouade-plugin validate` et au store.
- **La première main** est tenue à ce qu'elle déclare, sans la liste des familles : Antigravity CLI pose `HOME`, `USERPROFILE` et ses dossiers temporaires sur le dossier de son agent. Une version de la première main d'avant `permissions.env` garde ses lancements comme avant.
- **Pas de shell ni d'interpréteur.** Un harness d'un tiers ne nomme comme programme ni `sh`, `bash`, `zsh`, `dash`, `ksh`, `fish`, `cmd`, `powershell`, `pwsh`, `python*`, `node`, `deno`, `bun`, `ruby`, `perl`, `php`, `lua`, `osascript`, `wscript`, `cscript`, `mshta`, `rundll32`, `env`, `wsl`… (sans casse, `.exe` compris). Le manifeste est refusé (code `interpreter`), et un nom de ce genre est retiré de `locate.names` : le programme n'est pas trouvé.
- **`versionArgs` attend un geste.** La version d'un harness décrit par un tiers n'est lue qu'après un geste de l'utilisateur sur ce harness dans la session de l'app : un agent ou un terminal de lui lancé. Ni l'installation, ni le démarrage, ni la liste des harness ne la lancent ; d'ici là, sa version est inconnue. La première main la lit comme avant.
- **L'installation** montre aussi une ligne `env:<variable>` par variable déclarée, non révocable, et une mise à jour qui en ajoute une redemande l'accord.

## Ajouts d'ESC-443 (audit de sécurité, SEC-30)

Ceci complète « L'installation montre tout ce qui tournera » et la liste des familles refusées d'ESC-424.

- **`removeEnv` est jugé comme `env`.** Un manifeste tiers dont l'`acp.removeEnv` nomme une variable des familles refusées est refusé, avec la même raison (code `variable`, à `acp.removeEnv[i]`), dans l'app, dans `escouade-plugin validate` et au store. `CODEX_CA_CERTIFICATE` rejoint ces familles, comme les autres fichiers d'autorités.
- **Le cœur ne retire jamais, pour un tiers, ce qu'il pose lui-même.** Sur chaque lancement d'un plugin tiers (`engine/launch`, la relance, `terminal/launch`, `engine/oneShot`, `account/read`), une variable à retirer qui est d'une famille refusée, ou que le cœur donne (le proxy et le fichier d'autorités des réglages, `accountVar`, `keyVar`, l'adresse des hooks), reste posée ; le journal le dit. La première main retire ce qu'elle dit (Copilot CLI, ses jetons).
- **L'installation montre ce que le harness dit sans le lancer.** Avec ce qu'il lance, la fenêtre d'installation et de mise à jour d'un plugin, et celle de l'accord d'un projet privé, montrent sa commande d'installation (`harnessInstall:`, une fois si elle est la même sur tous les systèmes), sa commande de mise à jour (`harnessUpdate:`), notées « écrites par le plugin : Escouade l'affiche sans la lancer », et chaque variable qu'il retire (`harnessUnset:`). Ces lignes ne se retirent pas ; une mise à jour qui en change redemande l'accord.
- **La commande d'un tiers est dite la sienne.** Là où Escouade affiche la commande d'installation ou de mise à jour d'un harness décrit par un plugin tiers (l'accueil « Aucun harness trouvé », Réglages › Comptes et harness, le refus d'une version trop ancienne), elle est dite « donnée par le plugin <id>, pas par Escouade », le plugin nommé par son identifiant, qu'un tiers ne peut pas prendre à la première main. Les plugins `dev.escouade.*` gardent l'affichage d'avant.

## Ajouts d'ESC-489 : les modes déclarés sont ceux qui sont jugés

Ceci complète « Ce que le manifeste décrit » (`caps.modes`). La règle est dans *Comptes*, « Le mode où tourne vraiment le harness ».

- **Seuls les modes d'Escouade comptent** : `plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions`. Un mode d'un autre nom est ignoré (aucun menu ne l'offre, rien ne le jugerait).
- **Aucun mode déclaré, ou aucun qu'Escouade connaisse** : le harness est tenu pour Bypass, son seul mode, dont le sens dit « X ne déclare aucun mode : rien ne dit qu’il demande avant d’agir, Escouade le tient pour Bypass. ». Ses agents ne démarrent qu'avec « Autoriser le mode Bypass ».
- **Un seul mode** : c'est le mode de ses agents, montré et jugé ; un harness à un seul mode Bypass ne démarre pas sans « Autoriser le mode Bypass », et le refus dit d'autoriser le mode ou de choisir un autre harness.
- **L'installation le montre** : avec ce que le harness lance, une ligne forte et non révocable `harnessBypass:<nom>` (« Ses agents ne tournent qu’en Bypass : ils ne demandent jamais avant d’agir ») ou `harnessNoMode:<nom>` (« Ne déclare aucun mode : Escouade tient ses agents pour Bypass »), notée « appliqué par Escouade : ils ne démarrent qu’avec « Autoriser le mode Bypass » (Réglages › Agents) ». Un harness de plusieurs modes n'a pas de ligne de plus.
- Un manifeste qui ment sur ses modes ne se détecte pas : ce qu'il déclare est ce qui est jugé.

## Prime Agent (ESC-485)

Le plugin `dev.escouade.prime-agent` (`plugins/prime-agent/`) décrit Prime Agent, l'agent de code de Prime Intellect : `prime-agent --mode acp --no-session`, essayé avec la 0.10.3 dans une distribution WSL jetable, sur un modèle local (Ollama), sans compte. Il demande Escouade 2.2.2 (`escouade.min`).

- **Un seul mode, `bypassPermissions`**, au sens écrit en clair : Prime Agent ne demande jamais rien (ni `session/request_permission`, ni `fs/*`), le code Python de son modèle tourne avec les droits de l'utilisateur, sans bac à sable. Il n'a ni `session/set_mode`, ni mode qui ne change rien : pas de `acp.modes`, pas de lecture seule ; `"fs": false`, aucun projet privé ne lui est ouvert.
- **Un seul outil**, une cellule d'un noyau IPython (`tool_call` « Python cell », `kind: execute`, son code dans `rawInput.code`), montré sous le nom « Python » avec son code (`acp.tools`). Ses sous-agents (`rlm.spawn`) ne paraissent que comme des cellules.
- **Le modèle** par l'option `model` (valeurs `["<fournisseur>","<modèle>"]`) ; aucun modèle n'est déclaré, celui de Prime Agent sert. Le niveau de réflexion (option `thought_level`) n'est pas posé par le client. Ni jetons ni coût par ACP.
- **`--no-session`** : sa session appartient à la connexion d'Escouade et part avec elle (aussitôt l'entrée fermée, 30 s après un client tué) ; sans lui, sessions, workers et noyaux restaient dans son démon. Le prix : pas de sous-agents. Prime Agent 0.10.3 ne charge pas de session (`loadSession: false`) : après un redémarrage, l'agent repart d'une session neuve.
- **Son démon** (`prime-agent --mode daemon`), un par utilisateur (socket `/tmp/prime-agent-rust-<uid>/daemon.sock`, tube `\\.\pipe\prime-agent-daemon` sous Windows), partagé par ses sessions de terminal et tous les agents d'Escouade : sous Unix il quitte le groupe de process de l'agent (`setsid`) et reste, vide, après lui ; sous Windows il reste dans le Job de l'agent qui l'a lancé et meurt avec lui, emportant les sessions des autres agents. Un manifeste ne peut pas donner à chaque agent un démon à lui (`acp.env` est fixe) : le correctif est à faire dans l'app (un socket d'Escouade par compte et l'arrêt du démon au dernier agent).
- **Un seul compte** : `PRIME_AGENT_CODING_AGENT_DIR` déplace son dossier, mais une session ouverte sur un démon déjà lancé prend le dossier du démon ; pas d'`accountVar`.
- **Le serveur MCP d'Escouade** passe par `session/new` (HTTP, jeton en en-tête), et son modèle l'appelle depuis Python ; Escouade n'écrit rien dans `~/.prime/agent/settings.json`.
- **Ce que le client fait aujourd'hui d'un agent qui ne charge pas de session** : à la reprise, il ouvre une session neuve sans le dire, et sans redonner les consignes de l'agent, puisque le départ n'est pas « nouveau ». Les tests du plugin le gardent tel quel, pour qu'un changement se voie.

## Unharnessed (ESC-484)

Le plugin `dev.escouade.unharnessed` (`plugins/unharnessed/`, MIT) décrit Unharnessed : `unharnessed --mode acp`, trouvé par son nom dans le PATH, essayé avec la 0.9.8 construite depuis sa source (commit `fe2cf28`) dans une distribution WSL jetable, sur un modèle local (Ollama), sans compte ni clé de l'utilisateur. Il demande Escouade 2.2.2 (`escouade.min`), qui juge ses agents dans son seul mode (ESC-489).

- **Un seul mode, `bypassPermissions`**, son sens écrit en clair : Unharnessed ne demande jamais (aucun `session/request_permission`), son code Python et ses commandes tournent avec les droits de l'utilisateur, sans bac à sable. Il n'a aucun mode ACP : `acp.modes` est absent. Pas de `readOnly` ni de `readOnlyMode` : une discussion ne démarre pas. `"fs": false` : aucun agent dès qu'un projet est privé. Sans « Autoriser le mode Bypass », ses agents ne démarrent pas et le disent (le trou constaté par ESC-484 est fermé par ESC-489).
- **Ni `install` ni `updateCommand`** : rien d'officiel ne l'installe seul. L'extension VS Code (`Unharnessed.unharnessed-agent` 0.0.4) n'est publiée que pour macOS Apple Silicon ; son programme est dans `~/.vscode/extensions/unharnessed.unharnessed-agent-<version>-darwin-arm64/bin/`, un dossier qui change à chaque version, que `locate` ne trouve pas (le chemin se donne dans les réglages). Ailleurs, il se construit depuis la source ; il faut `uv` pour son noyau.
- **Son outil** : une cellule Python (`tool_call` de genre `execute`, titre « Python cell », le code dans `rawInput.code`), montrée par `acp.tools` comme l'outil « Python » avec son code.
- **Le modèle** est une option `model` dont les valeurs sont des paires `["<fournisseur>","<modèle>"]` ; le manifeste ne déclare aucun modèle, l'agent tourne sur le défaut d'Unharnessed. Son effort n'a pas été essayé : pas de table `acp.efforts`.
- **Un compte** : `accountVar` `UNHARNESSED_CODING_AGENT_DIR` (identifiants, réglages, sessions) ; le venv du noyau reste partagé (`~/.unharnessed/kernel-venv`).
- **Le serveur MCP d'Escouade** passe par `mcpServers` de `session/new` (HTTP, jeton en en-tête), appelé depuis le noyau (`mcp.call_tool`) ; rien par `unharnessed mcp add`.
- **Sa session** : un processus ne charge que la session reçue par `--resume <id>` au lancement, sous l'id enregistré (`_meta.unharnessed.saved.sessionId`), pas celui de `session/new`. Son manifeste le dit (`acp.resume`, ESC-491) : la reprise le relance avec `--resume`, puis charge la session.
- **Ses tours à lui** (`session_info_update` `turnStart.origin: "wake"`, à la fin d'une commande lancée en arrière-plan) finissent par les phases de son flux (`responseBoundary` sans `terminalQuiescenceExpected`, `terminalQuiescence`) : `acp.turnEnd`. Ce que tient son contexte et sa fenêtre sont dans `_meta.unharnessed.usage` (`tokens`, `window`) : `acp.usage`. Son coût (0, `billed: false`, sur un modèle local) et ses jetons d'entrée et de sortie ne sont pas lus : il ne les dit pas comme tels.

## Ajouts d'ESC-491 : la reprise sans session, les tours du harness, l'effort, le contexte

Ceci remplace, dans « Prime Agent », la phrase sur le niveau de réflexion et la dernière puce (« Ce que le client fait aujourd'hui… »), et complète « Ce que le manifeste décrit » et « Ce qu'Escouade tient ».

- **La reprise d'un agent dont le harness ne recharge pas sa session.** Un harness qui annonce `loadSession: false` (et que son manifeste ne relance pas par `acp.resume`), ou dont la session est perdue (`session/load` en erreur), donne à l'agent une session neuve, dite dans la conversation : « <harness> ne reprend pas sa session : l'agent repart d'une session neuve. » (ou, la session perdue, « La session <harness> n'a pas pu être reprise… »), suivi de « Ses consignes lui sont redonnées avec son prochain message, et un résumé de ce qu'il a fait. ». Le prochain message de l'agent porte de nouveau ses consignes (le protocole de son ticket, celui des questions en texte), comme à un premier lancement, puis le résumé : le dossier de passation d'ESC-88 tourné en reprise (« # Reprise… », `handover::recap`), que le cœur donne à chaque lancement d'un harness décrit qui reprend une session (`LaunchParams::recap`). Une session rechargée ne reçoit rien de plus.
- **`acp.resume`** : `args`, les arguments ajoutés au lancement pour reprendre une session, `{session}` y valant son id (jamais mis s'il pourrait être une option : lettres, chiffres, `-`, `_`, `.`, `:`, pas de `-` en tête) ; `session`, un pointeur JSON dans la réponse à `session/new` vers l'id sous lequel le harness garde la session, quand ce n'est pas celui de la session : c'est celui qu'Escouade garde et reprend, le client parlant à la session vivante. Unharnessed s'en sert ; Prime Agent non (`--no-session` : rien à reprendre).
- **Les tours qu'un harness lance seul.** Un texte, une réflexion, un appel d'outil ou un plan venu hors de tout `session/prompt` ouvre un tour du harness : l'agent passe « au travail » (`start`), son texte reste à ce tour. Le tour finit quand une mise à jour correspond à une entrée de `acp.turnEnd` (des pointeurs JSON dans `update` et la valeur de chacun, toutes vraies), sinon au message suivant, qui le clôt avant de partir, ou à l'arrêt (interrompu). Sa fin est un tour comme un autre : l'agent revient au repos, la notification et le Kanban le suivent, et la réponse suivante ne commence plus par ses mots. Ne valent que pour un harness décrit sans extension (Cursor, Gemini, Vibe gardent leur lecture).
- **Le niveau de réflexion** : `acp.efforts` donne la valeur de l'option de réflexion pour chaque effort d'Escouade, comme `acp.modes` pour les modes ; `acp.effortOption` son id, sinon l'option de catégorie `thought_level`. Elle n'est posée que si la session l'offre avec cette valeur et ne l'a pas déjà, à l'ouverture, au changement de modèle (la réponse de `session/set_config_option` et `config_option_update` disent les options d'un modèle qui raisonne) et au changement d'effort ; une fois par valeur. Un harness décrit qui a une table prend un effort sur tout modèle qu'il ne déclare pas sans. Absente, rien ne change. Prime Agent : `low`, `medium`, `high`.
- **Le modèle à valeur composée** : une valeur d'option faite de parties (`["ollama","qwen3"]`, en texte JSON) est trouvée par sa dernière partie ou ses parties jointes par `/`. Rien à déclarer.
- **Le contexte, jamais deviné** : `acp.usage` (`context`, `window`, des pointeurs JSON dans `update`) dit où le harness donne ce que tient son contexte et sa fenêtre ; sans lui, `usage_update` d'ACP (`used`, `size`). Seul ce qui est dit est montré ; les jetons d'un tour restent ceux de la réponse au prompt, le coût n'est jamais estimé.
- **Le code entier d'un appel long** : la ligne d'un outil qu'Escouade ne connaît pas montre le début de son premier texte ; s'il fait plusieurs lignes ou plus de 160 caractères, son détail le montre en entier, replié (« code en entier (200 lignes) »), au-dessus de la sortie.
- **Le contrat reste compatible** : tous ces champs sont facultatifs, jugés par le lecteur du manifeste (codes `pointer` et `session` ; `escouade-plugin validate`, l'app, le store), et ignorés par une app qui ne les connaît pas. Les réponses d'`agent/sendMessage` et `agent/interrupt` peuvent porter des `items` à côté de leurs `writes` (la fin d'un tour du harness), ce que le cœur sait déjà.

- **Les tours du harness ne valent qu'avec `acp.turnEnd`.** Ce qui précède sur les tours qu'un harness lance seul ne vaut que pour un manifeste qui dit comment ils finissent (`acp.turnEnd`, Unharnessed et Prime Agent) : le message suivant et l'arrêt ne les finissent que quand sa marque n'est pas venue. Sans `acp.turnEnd`, ce qu'un harness dit après la réponse à son prompt est lu comme avant (Goose, et les flux enregistrés des autres harness, rejoués à l'identique par les tests de parité).
