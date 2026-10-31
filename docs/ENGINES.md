# Moteurs (harness) — l'interface entre Escouade et les CLI qui font tourner ses agents

Un **moteur** fait tourner les agents d'Escouade : Claude Code, Codex, OpenCode, GitHub Copilot CLI, Cursor CLI (ESC-42 : un client ACP qui lui est propre, pas de moteur ACP générique) Mistral Vibe (ESC-266 : un client ACP à lui, dont la traduction met les messages dans la forme de ceux de Cursor et lui passe la main) Antigravity CLI (ESC-270 : `agy -p` en stream-json, ses événements mis dans la forme d’ACP pour la même traduction) et Gemini CLI (ESC-274 : `gemini --acp`, un client ACP à lui, dont la traduction passe aussi la main à celle de Cursor). Ce que chacun fait pour l'utilisateur, sur le même plan (installation, compte, modèles, modes et ce qui protège, coût et quota, vue terminal, serveur MCP, ce qui n'est pas pris en charge), est dans docs/SPEC.md, « Harness » ; leur protocole dans docs/PROTOCOL.md. L'interface de l'app l'appelle **« Harness »**, le mot anglais gardé tel quel en français comme en anglais (décidé le 2026-10-16) ; le code et ce document disent `engine` et « moteur ».

Ce document décrit la forme de l'interface et ses choix. Il a été écrit avec le premier lot de la refonte multi-moteurs (ESC-26) : l'extraction de Claude Code derrière l'interface, sans changement de comportement. « Le banc de conformité » et la liste à cocher « Ajouter un moteur » sont venus avec le lot 10 (ESC-35).

## Où est quoi

| Fichier | Rôle |
|---|---|
| `src-tauri/src/engine/mod.rs` | L'interface : les traits `Engine` et `EngineProcess`, leurs types (`Spawn`, `Start`, `Escouade`, `Message`, `PermissionAnswer`, `Ready`, `Remote`, `OneShot`, `Weight`, `StartLost`), et `engine::of(meta)`, le moteur d'un agent. |
| `src-tauri/src/engine/registry.rs` | Le registre des moteurs : ce que chacun déclare (`EngineCaps`, `Tested`), sa version et son état sur la machine (`EngineInfo`), le refus d'une version trop ancienne (voir « Le registre »). |
| `src-tauri/src/engine/mcp.rs` | Le serveur MCP d'Escouade d'un process : `Escouade` (donné ou refusé), `Launch` (ses arguments et variables), `SERVER` et `tool_name` (les noms `mcp__escouade__<outil>`), voir « Le serveur MCP d'Escouade ». |
| `src-tauri/src/engine/program.rs` | `Program` : le programme d'un moteur, du système ou de WSL (lancé par `wsl.exe`), et `Located`, où il est, comme les réglages le montrent. Commun à tous les moteurs. |
| `src-tauri/src/engine/oneshot.rs` | Les textes rédigés (one-shots) : `writer`, le harness de « Modèle des textes rédigés » parmi ceux trouvés ; `ask`, la question posée à un moteur dans son délai (90 s par défaut, `LIMIT`) et sa réponse bornée (`ANSWER_MAX`, 1 Mo) quel que soit le moteur ; `run`, le lancement d'un CLI qui lit la question sur son entrée, sa sortie lue jusqu'à cette borne. |
| `src-tauri/src/engine/claude/` | Claude Code, première implémentation : `mod.rs` (le moteur `Claude`, ses arguments, ses messages, ses one-shots, et `EngineProcess` pour son process), `process.rs` (`ClaudeProcess` : le process stdio et ses requêtes de contrôle, l'ancien `claude.rs`), `locate.rs` (où est `claude`). |
| `src-tauri/src/agent/claude.rs` | La traduction des frames de Claude Code (stream-json) en éléments de conversation : l'ancien corps de `AgentRt::handle_frame`. |
| `src-tauri/src/engine/codex/` | Codex, second moteur (ESC-37) : `mod.rs` (le moteur `Codex`, ses capacités, la correspondance des modes, les modèles de `model/list`, ses messages), `process.rs` (`CodexProcess` : le client JSON-RPC de `codex app-server`, le fil et le tour en cours), `locate.rs` (le binaire natif derrière le lanceur npm), `exec.rs` (les textes rédigés par `codex exec`, ESC-38), `git.rs` (les `git add` et `git commit` acceptés d'office), `terminal.rs` (la vue Codex, ESC-47), `bench.rs` (le faux `codex` sur le banc, ses flux réels rejoués), `schema_tests.rs` (le schéma de la 0.163.0). Le protocole : docs/PROTOCOL.md, « Codex app-server ». |
| `src-tauri/src/agent/codex.rs` | La traduction des messages de Codex en éléments de conversation (`AgentRt::codex_frame`), et ses tests (`codex_tests.rs`). |
| `src-tauri/src/engine/opencode/` | OpenCode, troisième moteur (ESC-44) : `mod.rs` (le moteur `Opencode`, ses capacités, les règles de permission de chaque mode, les modèles des fournisseurs connectés, ses messages, OpenCode 2 refusé), `server.rs` (le `opencode serve` d'un compte, partagé par ses agents : son mot de passe, son flux `/global/event` rendu à la session de chaque agent), `process.rs` (`OpencodeProcess` : la session d'un agent, ses messages gardés pendant un tour, le serveur MCP d'Escouade donné à son dossier, ESC-45), `run.rs` (les textes rédigés par `opencode run`, ESC-45), `locate.rs` (le binaire natif derrière le lanceur npm), `bench.rs` (le faux `opencode` sur le banc, ses flux réels rejoués). Le protocole : docs/PROTOCOL.md, « OpenCode serve ». |
| `src-tauri/src/engine/copilot/` | GitHub Copilot CLI, quatrième moteur (ESC-39) : `mod.rs` (le moteur `Copilot`, ses capacités, ses modes, les modèles et leur multiplicateur, les crédits du mois et le coût d'un appel, le serveur MCP d'Escouade donné ou refusé (ESC-40), « Pour cette session » par type de permission, les valeurs d'un formulaire, ses messages), `process.rs` (`CopilotProcess` : le client du protocole du SDK, en-têtes `Content-Length`, la session d'un agent, les requêtes de Copilot au client), `locate.rs` (le binaire natif derrière le lanceur npm), `prompt.rs` (les textes rédigés par `copilot -p`, ESC-40), `terminal.rs` (la vue Copilot, ESC-48), `bench.rs` (le faux `copilot` sur le banc, ses flux réels rejoués), `schema_tests.rs` (les méthodes et les champs lus, contre les schémas gardés). Le protocole : docs/PROTOCOL.md, « Copilot SDK ». |
| `src-tauri/src/agent/copilot.rs` | La traduction des événements de Copilot en éléments de conversation (`AgentRt::copilot_frame`) et les règles qui répondent sans carte aux demandes sûres, et ses tests (`copilot_tests.rs`) ; l'essai réel `real_copilot` est dans `src-tauri/src/copilot_tests.rs`. |
| `src-tauri/src/agent/opencode.rs` | La traduction des événements d'OpenCode en éléments de conversation (`AgentRt::opencode_frame`), et ses tests (`opencode_tests.rs`) ; les tests par le cœur sont dans `src-tauri/src/opencode_tests.rs` (l'essai réel `real_opencode` aussi). |
| `src-tauri/src/engine/bench/` | Le banc de conformité (tests seulement) : les scénarios et leur pilote (`mod.rs`), le rejeu des flux enregistrés (`replay.rs`). Voir « Le banc de conformité ». |
| `src-tauri/src/engine/claude/bench.rs` | Claude Code sur le banc, avec le faux `claude`, et le rejeu de ses flux (`tests/fixtures/replays/claude/`). |
| `src-tauri/src/engine/minimal/`, `src-tauri/src/agent/minimal.rs` | Le faux moteur minimal (tests seulement) : le moins qu'un moteur écrive pour passer le banc. |
| `src-tauri/src/agent.rs` | La conversation normalisée d'un agent, son statut, ses demandes en attente, son usage : ce que chaque moteur remplit. |

Le reste du backend (cœur, tickets, pilote auto, serveur MCP, stats, comptes) ne parle plus à Claude Code que par ces deux traits : il ne construit plus aucune frame ni requête de contrôle (`core.rs` n'a plus un seul `json!`), ne lit plus les arguments de `claude`, ni son stderr.

## Les deux traits

### `Engine` : le moteur

Un objet par moteur, sans état (`static CLAUDE: Claude`). `engine::of(&meta)` donne celui d'un agent, `engine::default()` celui de ce qui n'a pas encore d'agent (une question sur un projet, les options d'un ticket). `engine::of` lit `AgentMeta.engine` (lot 2, ESC-27) par `engine::by_id` : `codex` rend Codex (ESC-37), `opencode` OpenCode (ESC-44), `copilot` Copilot CLI (ESC-39), `cursor` Cursor CLI (ESC-42), `vibe` Mistral Vibe (ESC-266), `agy` Antigravity CLI (ESC-270), `gemini` Gemini CLI (ESC-274). `engine::default()` rend Claude Code.

| Méthode | Ce qu'elle fait | Claude Code |
|---|---|---|
| `id()`, `name()` | Son id dans les données (`claude`) et son nom (`Claude Code`). | |
| `speaker()` | Le nom de ses agents dans les textes (« Claude attend ta réponse », voir « Les textes et les consignes »). | `Claude` |
| `instructions_room()` | La place des consignes ajoutées (`Spawn.append_prompt`) : un poids maximal et la façon de peser un texte, ou aucune borne. | 6 000, pesés comme cmd.exe échappe un argument de `.cmd` |
| `subagent_relay(langue, titre, tâche, texte)` | Ce que l'agent reçoit pour transmettre un message à un de ses sous-agents ; aucun s'il ne sait pas. | « … avec l'outil SendMessage … » |
| `locate(réglage, distribution)` | Son programme, du chemin d'un réglage (vide : là où il est d'habitude) ; un `Program` du système ou de WSL. | `resolve_binary` |
| `supports_effort(modèle)`, `supports_auto(modèle)` | Ce que prend un modèle (ce que prend le moteur, lui, est dans ses capacités : « Le registre ») ; pour le mode `auto` de Claude Code, la règle d'avant la première réponse du compte : ensuite, ce que son `claude` dit de chaque modèle (`supportsAutoMode`, `Core::takes_auto`). | effort : tous sauf Haiku ; `auto` : tous sauf un Haiku plus ancien que 5 |
| `spawn(Spawn, on_frame, on_exit)` | **Lancer, reprendre, copier** : le process d'un agent, d'une conversation neuve, reprise ou copiée selon `Spawn.start`. | `claude --output-format stream-json …` |
| `message(texte, pièces jointes)` | **Le message avec ses pièces jointes**, dans la forme de son moteur (`Message`, opaque) ; refusé pour un fichier d'un type ou d'une taille qu'il ne prend pas. | blocs `image`, `document`, `text` |
| `translate(rt, frame, fx)` | **La traduction** d'une frame du process dans la conversation de l'agent. | `AgentRt::claude_frame` |
| `start_lost(stderr)` | Pourquoi un process arrêté avant d'être prêt n'a pas pu démarrer : session perdue, ou point de copie absent. | « No conversation found », « No message found with message.uuid » |
| `one_shot(OneShot)` | Une question sans session, sans réglages de l'utilisateur ni serveur MCP : nom d'un agent, message de commit, titres, suggestion des étapes d'un worktree. | `claude -p --output-format json` |
| `writer(Weight)` | Le modèle qui répond aux one-shots, tel que « Modèle des textes rédigés » le nomme. | « Claude Haiku » (« Claude Sonnet » pour `Reading`) |

### `EngineProcess` : le process vivant d'un agent

Tenu par `AgentRt.proc` (`Option<Process>`, `Process = Arc<dyn EngineProcess>`).

| Méthode | Opération du ticket | Claude Code |
|---|---|---|
| `is_alive`, `usage`, `still_running`, `pids` | (process, CPU et RAM par agent) | le Job de l'agent |
| `close_input` | **Arrêter** en douceur : il finit ce qu'il fait, la session reste reprenable. | stdin fermé |
| `kill` | **Arrêter** net, tout l'arbre. | `TerminateJobObject` |
| `wait_exit`, `wait_gone` | Attendre la fin du process, puis de tout ce qu'il a lancé. | |
| `initialize` | Le prêt du process : ses commandes et ses modèles (`Ready`). | `initialize` |
| `send_message(uuid, Message)` | **Envoyer un message** (pièces jointes comprises). | frame `user` |
| `interrupt(cancel_queued, délai)` | **Interrompre** le tour (et les messages en file). | `interrupt` |
| `set_model`, `set_effort`, `set_mode` | **Changer de modèle, d'effort ou de mode** en cours de process. | `set_model`, `apply_flag_settings`, `set_permission_mode` |
| `answer_permission(id, PermissionAnswer)` | **Répondre à une permission** : une fois, toujours (règles proposées gardées), ou refus avec un message. | `control_response` `allow` / `deny` |
| `answer_question(id, appel, input)` | **Répondre à une question**, ses réponses dans `input`. | `control_response` `allow` avec `updatedInput` |
| `read_usage` | **Lire l'usage** : les fenêtres du quota du compte, que lit `usage::parse_windows`. | `get_usage` |
| `rename(titre)` | Le titre de la session. | `rename_session` |
| `stop_task`, `task_output`, `remote_control` | Propres à Claude Code pour l'instant : tâches de fond et sous-agents, Remote Control. | `stop_task`, `get_task_output`, `remote_control` |
| `control(requête, délai)`, `reply(id, réponse)` | Le protocole du moteur, brut : une requête à lui, une réponse à une des siennes. Pour les tests et la traduction (refuser une `elicitation`). | `control_request` / `control_response` |

## Le flux : frames et éléments normalisés

Le process lit son flux et le passe en `Value` à `on_frame`, sauf les réponses aux requêtes qu'on lui a faites, qu'il garde pour lui (`control`). Le cœur appelle `AgentRt::handle_frame`, qui le confie au moteur de l'agent (`Engine::translate`). Chaque moteur remplit la même conversation :

- les éléments `user`, `text`, `thinking`, `tool`, `question`, `permission`, `turn`, `notice`, `event` (`AgentRt::append`, `patch`, `push_user`, `notice`) ;
- le statut (`set_status`), l'activité (`set_activity`), les demandes en attente (`add_pending`) ;
- les changements du bandeau « Plan » (`plan_apply`) ;
- l'usage du tour, la fin de tour (`Effects.turn_end`), la limite (`Effects.limited`), le quota (`Effects.rate`).

Le passage à lire à voix haute d'une conversation vocale (`<spoken>…</spoken>`, ESC-97) est mis à part par la partie commune, pour tout moteur : `AgentRt::push` le sort d'un texte de l'agent lui-même (pas d'un sous-agent) dès qu'il est écrit en entier (`streaming: false`), à l'ajout comme au patch. Un message envoyé pendant un tour n'est mis en file que chez un moteur qui le fait attendre : Claude Code (`caps.mid_turn` `queued`) le lit à l'étape suivante du tour ; Cursor et OpenCode (`turnEnd`) le gardent dans le process d'Escouade jusqu'à la fin du tour, puis il en lance un ; chez Codex (`immediate`, `turn/steer`) et Copilot CLI (`immediate`, `session.send` en mode `immediate`), il entre dans le tour en cours. La fenêtre garde elle-même un message tapé dans le composer pendant un tour, le temps d'un délai ou jusqu'à la fin du tour selon cette valeur (`outbox.svelte.ts`).

Le nom des outils reste celui de Claude Code pour tous les moteurs (`Bash`, `Edit`, `Write`, `Task`, `mcp__<serveur>__<outil>`…, table dans docs/TODO-1.8.md, « Un moteur par agent ») : `tools.ts`, la ligne d'activité, `convsearch.rs` et les cartes ne changent pas.

## Les choix

1. **Deux traits objets**, plutôt qu'une énumération des moteurs : ajouter un moteur, c'est écrire son adaptateur et une ligne dans `engine::of`, sans toucher aux appels du cœur. Les méthodes asynchrones rendent un `BoxFuture` (`Pin<Box<dyn Future + Send>>`) : les traits restent utilisables en objets, sans dépendance nouvelle (`async-trait`).
2. **Le moteur (sans état) et le process (vivant) sont séparés.** Ce qui se décide avant le process (son programme, ses arguments, la forme d'un message, un one-shot, la lecture de son stderr) est au moteur ; ce qui parle au process est au process. Un message est préparé par `Engine::message` **avant** que le process démarre : un fichier refusé laisse le message dans le composer sans rien lancer, comme avant.
3. **Lancer, reprendre et copier sont une seule méthode**, `spawn`, avec `Start::New`, `Start::Resume { session }` ou `Start::Fork { of, at }` (le point de la copie : le dernier message de l'original quand il a été copié). C'est la forme des essais de Codex (`thread/start`, `thread/resume`, `thread/fork` avec `lastTurnId`) et ce que docs/TODO-1.8.md dit de Copilot (dernier événement) et d'OpenCode (dernier message). Un moteur qui ne sait pas copier refusera `Start::Fork` (et « Dupliquer la conversation » est masqué par ses capacités, `fork: none`, voir « Le registre »).
4. **`spawn` reçoit ce qu'il faut savoir, pas des arguments de `claude`** : `Spawn` porte le programme, le dossier, l'environnement, le modèle, l'effort, le mode, les consignes ajoutées, `Start`, les dossiers lisibles sans question (`read_dirs` : fichiers joints, fichiers du ticket) et l'accès au serveur MCP d'Escouade (`Escouade::Granted { url, token, file }` ou `Denied { file }`, voir « Le serveur MCP d'Escouade »). Chaque moteur en fait ses options : `--add-dir`, `--mcp-config` ou `--settings` + `--disallowedTools` pour Claude Code ; `-c mcp_servers.escouade.*` pour Codex (essayé, docs/TODO-1.8.md, point 0) ; `POST /mcp?directory=…` pour OpenCode ; `--additional-mcp-config` pour Copilot CLI.
5. **Le flux reste du JSON** (`serde_json::Value`) entre le process et la traduction : tous les protocoles connus en sont (stream-json de Claude Code, JSON-RPC de Codex, de Copilot et d'ACP, événements SSE d'OpenCode). Un moteur dont le transport n'est pas une ligne par message (en-têtes `Content-Length`, SSE) le lit dans son process et passe la même chose à `on_frame`.
6. **La traduction vit dans `agent/<moteur>.rs`**, un module enfant de `agent` : elle atteint l'état de l'agent (blocs en cours, demandes en attente, compteurs, plan) sans qu'il soit ouvert au reste du crate, comme `agent/transcript.rs` et `agent/sidechain.rs` le faisaient déjà. Le moteur l'appelle depuis `Engine::translate`.
7. **Les réponses aux demandes sont typées** (`PermissionAnswer::Allow`, `Always`, `Deny`) et portent à part l'id de la demande et l'appel d'outil qu'elle concerne (`call`) : chez Claude Code, `request_id` et `tool_use_id` ; chez Codex, l'id JSON-RPC de `requestApproval` et l'`itemId` ; chez ACP, l'id de `session/request_permission` et le `toolCallId`. Qui a répondu (« Refusé par <client> via Escouade ») reste écrit par le cœur, dans le message, pour tous les moteurs.
8. **Modèle, effort et mode gardent les valeurs d'Escouade** (celles de Claude Code : `auto`, `plan`, `acceptEdits`…) ; un autre moteur fera sa correspondance dans son adaptateur (pour Codex : `approvalPolicy`, bac à sable et `collaborationMode` pour le mode Plan). Les modes qu'il prend et leur sens sont dans ses capacités (« Le registre ») ; ses modèles viennent avec le composer (lot 4).
9. **Ce que Claude Code est seul à faire** (Remote Control, tâches de fond, arrêt d'un sous-agent) a ses méthodes à part dans `EngineProcess`, plutôt que des requêtes brutes dans le cœur. Un autre moteur les refusera (« non pris en charge ») et l'interface les masquera par ses capacités (docs/TODO-1.8.md, « Ce qui reste propre à Claude Code »).
10. **Une échappatoire, `control` et `reply`** : une requête et une réponse dans le protocole du moteur. Les tests s'en servent (`background_tasks`), et la traduction pour refuser ce qu'Escouade ne sait pas traiter. Le cœur n'en a pas besoin.
11. **Les one-shots disent leur poids, pas leur modèle** : `Weight::Light` (le modèle le plus rapide, sans outil : Haiku) ou `Weight::Reading` (un modèle plus fort, avec des outils qui ne font que lire : Sonnet avec `Read,Glob,Grep`). Le moteur choisit. Le harness qui répond est celui de « Modèle des textes rédigés » (`engine::oneshot::writer`), pas celui de l'agent ; `engine::oneshot::ask` tient pour tous le délai et la lecture bornée, et `engine::oneshot::run` lance un CLI qui lit la question sur son entrée et répond sur sa sortie. `Engine::writer(weight)` nomme son modèle dans le réglage (« Claude Haiku »).
12. **Les lectures propres à un moteur passent par lui** : le stderr d'un démarrage raté (`start_lost`), le quota (`read_usage`), les modèles (`Ready`).

## Ce qui reste propre à Claude Code hors de `engine/`

L'extraction s'est arrêtée là où d'autres lots reprennent :

- **La vue Claude Code** : passée par l'interface `TerminalView` (lot 9, ESC-34, « La vue terminal » ci-dessous). Restent propres à Claude Code, hors de cette interface : Remote Control (`--remote-control`, ses liens) et les sous-agents lus dans les transcripts (`agent/sidechain.rs`).
- **La connexion et la lecture du quota de Claude Code** : `accounts.rs` (création d'un compte, partage du dossier de Principal, connexion par `/login`, session copiée d'un compte à l'autre) et `usage.rs` (point d'accès OAuth, trousseau de macOS). Le modèle des comptes et du quota, lui, est commun (lot 5, voir « Les comptes et le quota »). `Program::launch` traduit encore `CLAUDE_CONFIG_DIR` et les options de chemin de Claude Code pour WSL (`PATH_OPTIONS`).
- **Les modèles** : `ModelInfo::list` lit la liste de `initialize` de Claude Code. Avec les menus du composer (lot 4).

## Les données (ESC-27)

Le moteur est gardé partout où il compte, par son id (`EngineId` de `model.rs` : `claude`, `codex`, `copilot`, `cursor`, `opencode`). Ce qui a été enregistré avant la 1.8 vaut Claude Code.

| Où | Champ | Avant la 1.8 | Ce qu'il dit |
|---|---|---|---|
| Agent | `AgentMeta.engine` | `claude` | Le moteur de l'agent, un à la fois. Il n'en change que par une passation (ESC-88). Sa copie (« Dupliquer la conversation ») prend celui qu'il a au moment de la copie. |
| Ticket | `Ticket.engine` | `null` | Le moteur de ses agents, que le pilote auto suit ; `null` : celui du projet. |
| Projet | `Project.engine` | `null` | « Harness par défaut » de ses nouveaux agents et de ses tickets ; `null` : Claude Code (`Project::engine()`). L'assistant « Nouveau projet » le copie avec les réglages des agents. |
| Tour | `TurnRow.engine`, élément `turn` (`engine`), colonne `engine` de la table `turns` de `stats.db` | `claude` (défaut de la colonne, ajoutée par `stats::migrate`) | Le moteur qui a fait le tour : celui de l'agent à ce tour, gardé même s'il en change ensuite. `StatsView.by_engine` les additionne. |

Le moteur d'un nouvel agent : celui demandé (`AgentOptions.engine`), sinon celui de son ticket, sinon celui de son projet. Les outils MCP `list_agents` et `get_agent_summary` le donnent (`engine`). Le choix et la marque du moteur sont à l'écran depuis le lot 4 (« L'interface », ci-dessous).

## Le registre (ESC-28)

`engine/registry.rs` tient ce que chaque moteur déclare et ce qu'on trouve de lui sur la machine. `registry::all()` liste les moteurs écrits (Claude Code, Codex, Copilot CLI et OpenCode aujourd'hui), le moteur par défaut en premier. Une ligne d'un moteur qu'Escouade ne pilote pas (OpenCode 2) est refusée quelle que soit la plage testée (`Engine::unsupported_line`, état `unsupported`).

### Ce que chaque moteur déclare (`Engine`)

| Méthode | Ce qu'elle dit | Claude Code |
|---|---|---|
| `locate(réglage, distribution)` | Son exécutable, comme `resolve_binary` le trouve pour `claude` (binaire natif sous Windows, ou programme de WSL). | `resolve_binary` |
| `version_args()` | Ce qui lui fait écrire sa version ; `registry::version` en lit le premier numéro (`2.1.301 (Claude Code)`), gardé tant que le fichier ne change pas. | `--version` |
| `tested()` | Sa plage de versions testées (`Tested { oldest, newest }`). | 2.1.283 (protocole de base, docs/PROTOCOL.md) à 2.1.301 (dernier essai réel) |
| `oldest_reason(langue)`, `update_command()` | Pourquoi une version plus ancienne est refusée, et la commande qui le met à jour. | « son flux JSON n'a été vérifié qu'à partir de celle-ci », `claude update` |
| `caps()` | Ses capacités (`EngineCaps`, ci-dessous). | voir `tests/fixtures/engines/claude.json` |

### Les capacités (`EngineCaps`)

| Champ | Ce qu'il dit | Ce que l'interface en fait | Claude Code |
|---|---|---|---|
| `modes` | Les modes d'Escouade qu'il prend, chacun avec son sens chez lui (`meaning`, `null` : celui du menu, donc de Claude Code). | Le menu « Mode » ne liste qu'eux, avec leur sens. | les cinq |
| `effort` | Ses modèles prennent un effort (chaque modèle peut encore n'en avoir aucun : `supports_effort`). | Le menu « Effort » est masqué sinon. | oui |
| `images`, `pdf` | Images et PDF joints à un message. | `Engine::message` refuse ce qu'il ne lit pas. | oui |
| `fork` | La copie d'une conversation et jusqu'où : `none`, `entry`, `turn`, `event`, `message`. | « Dupliquer la conversation » masqué pour `none`. | `entry` |
| `resumeOtherAccount` | Une conversation reprise sur un autre compte du même moteur. | « Reprendre sur <compte> » ; la bascule ne quitte jamais le moteur. | oui |
| `midTurn` | Un message envoyé pendant un tour : `queued` (lu à l'étape suivante), `immediate`, `turnEnd` (gardé par Escouade jusqu'à la fin du tour) ou `refused`. | Le composer le garde le temps du « Délai avant l'envoi pendant un tour » (`queued`, `immediate`) ou jusqu'à la fin du tour (`turnEnd`). | `queued` |
| `slashCommands` | Ses commandes `/` pour un client. | Le composer n'en propose pas sinon. | oui |
| `quota` | Le quota de ses comptes : `windows`, `credits` ou `none`. | Les barres de la barre d'état et du panneau des quotas ; sans quota, aucune barre (« Pas de quota ») et jamais de pause du pilote auto. | `windows` |
| `quotaWindows` | Les durées de ses fenêtres, en minutes. | Les barres montrées avant la première lecture (une lecture peut en donner moins : l'offre gratuite de Codex n'en a qu'une). | `[300, 10080]` |
| `cost` | Le coût d'un tour : `exact`, `estimated` ou `none`. | Le coût est masqué pour `none` (carte, en-tête, vue d'ensemble). | `exact` |
| `terminalView` | Sa propre interface dans un terminal. | « Vue Claude Code » et son réglage masqués sinon. | oui |
| `remoteControl` | Remote Control. | Le menu, l'icône de la carte et le réglage masqués sinon. | oui |
| `backgroundTasks` | Tâches de fond, workflows, arrêt d'un sous-agent. | La pastille « N en fond » et la liste sous la carte masquées sinon. | oui |
| `todoTools` | Les outils de tâches qu'on peut lui donner (`CLAUDE_CODE_ENABLE_TODO_TOOLS`). | Le réglage masqué si aucun moteur ne l'a. | oui |
| `defaultRefusal` | Il dit ce qu'il refuserait par défaut. | (l'avertissement vient de son flux) | oui |

La fenêtre reçoit les déclarations au démarrage (`InitialState.engines`, `EngineDecl`) et ne décide d'afficher une fonction que par `src/lib/engines.ts` (`can`, `canFork`, `showsCost`, `modesOf`, `anyCan`) : jamais d'après le nom du moteur, ce qu'un test vérifie en cherchant dans le code de la fenêtre (`src/lib/engines.test.ts`). Une fonction qu'un moteur n'a pas est masquée, pas grisée. Le fichier `tests/fixtures/engines/claude.json`, la déclaration de Claude Code que prennent les tests de la fenêtre, reste égal à celle du backend (test Rust).

### Version et état

`registry::state_of` donne l'état d'un moteur : `absent` (programme introuvable), `tooOld` (version plus ancienne que la plage : refusée), `notConnected` (aucun compte connecté), `untested` (plus récente : utilisée, avec un avertissement), `ready`. Une version qu'il ne dit pas est prise telle quelle. La commande `engines` le rend pour chaque moteur (`EngineInfo` : programme, version, plage, état, texte, capacités) ; Réglages › Harness l’affiche avec son état (voir « L’interface »).

Le refus est fait au démarrage du process d'un agent, en vue Escouade comme en vue Claude Code (`registry::check`, après `accounts::program`) : « Claude Code 2.0.14 est trop ancien pour Escouade, qui demande au moins la version 2.1.283 : … Mets-le à jour (claude update). » L'avertissement : « Claude Code 2.1.400 n'a pas été testé avec Escouade (testé jusqu'à la 2.1.301) : il est utilisé quand même, et ce qu'Escouade ne reconnaît pas de lui est ignoré. »

### La lecture tolère l'inconnu

Un champ inconnu est ignoré, une frame, un événement, un bloc ou un delta inconnus sont écartés sans casser le tour (`agent.rs`, `an_unknown_field_or_frame_of_a_newer_claude_code_leaves_the_turn_going`). Chaque nouvel adaptateur fait de même.

### Les schémas des versions testées

Le schéma de la plus récente version testée est gardé dans le dépôt : `tests/fixtures/engines/claude-code-<version>.schema.json`, les frames de Claude Code et leurs champs, relevés dans ses flux réels enregistrés (`tests/fixtures/real-run-*.jsonl`) et, pour ce qu'aucun enregistrement n'a, dans docs/PROTOCOL.md. Deux tests le tiennent : chaque champ qu'Escouade lit y est (`claude_code_frames_read_by_escouade_are_in_the_schema`), et chaque champ des flux enregistrés aussi. Passer `tested().newest` à une version nouvelle demande son schéma. Pour Codex, c'est la sortie de `codex app-server generate-json-schema` (`codex-0.163.0.schema.json`) ; pour OpenCode, son OpenAPI (`opencode-1.19.3.openapi.json`) ; pour Copilot CLI, les schémas du protocole de son SDK (`copilot-1.0.98.*.schema.json`) ; pour Cursor CLI, qui n'en publie pas, le relevé d'ACP et des extensions `cursor/*` dans le code de la version testée (`cursor-2026.10.22-27dcf5f.acp.json`).

## L'interface (ESC-29)

Ce que l'utilisateur voit des moteurs, toujours appelés « Harness » à l'écran. Avec Claude Code seul, rien ne change à l'écran, hors l'onglet renommé et l'écran de premier lancement.

### Ce que chaque moteur déclare de plus

`EngineDecl` (ce que la fenêtre reçoit au démarrage) porte, en plus de ses capacités :

| Champ | Méthode de `Engine` | Ce que l'interface en fait | Claude Code |
|---|---|---|---|
| `install` (`windows`, `unix`) | `install()` | La commande qui l'installe, sur l'écran « Aucun harness trouvé » et dans Réglages › Harness quand il est introuvable. | `irm https://claude.ai/install.ps1 \| iex`, `curl -fsSL https://claude.ai/install.sh \| bash` |
| `models` (`id`, `label`, `effort`, `auto`) | `models()` (Codex : son modèle par défaut, jusqu'à la première liste de `model/list`) | Le menu « Modèle » du composer, dans son ordre ; chaque modèle dit s'il prend un effort et le mode `auto` (`supports_effort`, `supports_auto`, un test vérifie l'accord). Le nom affiché devient la version que le moteur dit faire tourner (`ModelInfo`, `initialize`) quand il l'a dite. | les alias `fable`, `opus`, `sonnet`, `haiku` (Haiku sans effort ; avec `auto`, l'alias faisant tourner Haiku 5.5 depuis Claude Code 2.1.293) |
| `efforts` | `efforts()` | Le menu « Effort », vide (et masqué) sans `caps.effort`. | `low` à `max` |

Un harness qui liste les modèles du compte les donne au démarrage de son process (`Ready.catalog`, des `ToldModel` : id, nom, efforts, effort par défaut, images, modèle par défaut ; Codex : `model/list`). Le cœur les garde par harness (`Core::engine_models`, sauvés avec l'état, `UiEvent::EngineModels`, `InitialState.engineModels`), et ils prennent la place des modèles déclarés : le menu « Modèle », les efforts de chaque modèle (`effortsOf`, `modelTakes`), et son nom partout où un modèle est montré (`modelLabel`, `toldLabel`). Un nouvel agent d'un tel harness part sur son modèle par défaut quand le modèle demandé n'est pas des siens (`Core::model_for`).

`InitialState.enginesFound` (et la commande `claude_found`, relue quand le chemin est enregistré) liste les moteurs dont le programme est trouvé (`Core::engines_found` ; pour Claude Code, sur le compte où partent les nouveaux agents). C'est la seule liste que l'interface propose : un moteur absent n'apparaît que dans Réglages › Harness.

### Où chaque chose se montre

Tout passe par `src/lib/engines.ts`, jamais par le nom du moteur :

| Fonction | Ce qu'elle dit | Où |
|---|---|---|
| `offered()`, `choosesEngine()`, `engineOptions(actuel)` | Les moteurs trouvés ; s'il y a un choix (deux ou plus) ; les options d'un menu « Harness » (celui d'un ticket ou d'un projet reste listé même s'il n'est plus trouvé). | Réglages › Projets « Harness par défaut », menu « ▾ » de « Nouvel agent » (« Nouvel agent sur … », celui du projet en premier), champ « Harness » de la fenêtre d'un ticket (« Celui du projet (…) » par défaut). Tous masqués avec un seul moteur trouvé. |
| `showsMark()`, `EngineMark.svelte` | La marque du moteur d'un agent (son premier mot : « Claude », « Codex »…), dès que les agents non archivés tournent sur deux moteurs. | Carte de l'agent (barre latérale), vue d'ensemble, en-tête de sa conversation, carte de son ticket. |
| `modelsOf`, `effortsOf`, `modelTakes`, `modesOf` | Les menus du composer : modèles, efforts, et ce qu'un modèle prend ; les modes avec leur sens (`ModeCap.meaning`). Les commandes `/` viennent du process de l'agent, et ne sont proposées que si son moteur en a (`slashCommands`). | `Composer.svelte` |

Réglages › Harness (`AppTab.svelte`, onglet `claude`) : un groupe par moteur déclaré, avec son exécutable (le champ du chemin pour le moteur par défaut, le seul qui ait un réglage de chemin tant que chaque moteur n'a pas le sien ; « Trouvé : … » pour les autres), sa version et son état (`EngineInfo`, commande `engines`), et « Se connecter… », qui ouvre la connexion du compte où partent les nouveaux agents (la fenêtre « Se connecter au compte », qui revient ensuite à l'onglet). Un moteur introuvable y donne sa commande d'installation.

La fenêtre d'un ticket envoie `engine` (null : celui du projet) ; `TicketDraft.engine` est absent pour les outils MCP et les imports, qui laissent au ticket le sien. `create_agent` et `create_agent_on_branch` prennent `engine` (null : celui du ticket, puis du projet, voir « Les données »).

## Les comptes et le quota (ESC-30)

### Un compte, un moteur

Chaque compte (`model::Account`) a son moteur (`engine`, Claude Code pour un compte d'avant la 1.8). Ce qui tient ses comptes à part, le moteur le déclare (`Engine::accounts`, `registry::AccountStore`) : un dossier de configuration par compte, passé à ses process par une variable (`ConfigDir { var }` : `CLAUDE_CONFIG_DIR` pour Claude Code, `CODEX_HOME` pour Codex, `COPILOT_HOME` pour Copilot CLI, `OPENCODE_CONFIG_DIR` pour OpenCode, qui n'a que son Principal en 1.8). `accounts::launch_env` la pose ; Cursor CLI (ESC-43) garde sa connexion hors de tout dossier (`%APPDATA%\Cursor\auth.json`) : ses comptes autres que Principal ont un dossier comme les autres, mais leur identité est une clé d'API, gardée dans le trousseau du système (entrée `cursor:<compte>`, `Core::set_cursor_key`, relue au démarrage par `load_cursor_keys`) et passée par `CURSOR_API_KEY` (`launch_env`) ; Principal reçoit vides la clé et le jeton de l'environnement d'Escouade, un compte à clé le jeton, et un compte sans sa clé ne démarre rien (`cursor_ready`), sans quoi il tournerait sous la connexion de la machine. Un moteur peut aussi donner à chaque agent un dossier de config à lui, quel que soit son compte (`Engine::agent_home_var`, `DataDir::agent_home`, retiré avec l'agent) : Cursor CLI, dont le « Toujours » écrit ses règles dans ce dossier (« Pour cet agent », décidé le 2026-10-25).

Chaque moteur a son Principal, son installation habituelle, lancée sans dossier d'Escouade : `principal` pour Claude Code (l'id d'avant la 1.8), `principal-<moteur>` pour un autre (`accounts::principal_id`, `is_principal`). Celui d'un moteur autre que Claude Code naît quand ce moteur sert pour la première fois, à son premier agent ou ticket (`Core::accounts_of`, qui le dit à la fenêtre par `UiEvent::Accounts`) : un moteur qui n'a jamais tourné n'apparaît dans aucun compte, ni dans l'« Ordre des harness », et n'est le moteur de secours d'aucun projet (ESC-37). `accounts::normalize` fait le Principal de Claude Code et de chaque moteur du registre qui a des comptes, garde au moins un compte actif par moteur (aussi pour un moteur que cette version n'écrit pas, des comptes laissés par une version plus récente) et liste chaque moteur une fois dans `Settings.engine_order`.

### Les choix restent dans le moteur

`accounts::for_project(réglages, moteur, projet)` éteint les comptes des autres moteurs et ceux réservés à d'autres projets ; `pick`, `preferred` et `candidates` choisissent ensuite parmi les comptes de ce moteur seul. Donc :

- un nouvel agent part sur un compte de son moteur (`Core::account_for_new(projet, moteur)`), le compte préféré du projet ne comptant que s'il est de ce moteur ;
- la bascule automatique (`account_to_switch_to`) et « Reprendre sur <compte> » restent dans le moteur de l'agent ; passer à un autre moteur est une passation (ESC-88) ;
- un projet qu'aucun compte actif d'un moteur ne sert ne démarre rien sur ce moteur, et le refus le dit (`accounts::no_account(nom, moteur)` : « Aucun compte Codex actif ne sert le projet… ») ;
- `accounts::fallback_engines(réglages, moteur par défaut, projet)` donne les moteurs de secours d'un projet pour ESC-88 : son moteur par défaut, puis les autres dans « Ordre des harness » (`engine_order_set`, gardé par le backend comme les comptes), seulement ceux qui ont un compte actif qui le sert.

### Le quota

Un modèle commun : chaque compte a ses compteurs (`AccountUsage.meters`, des `QuotaMeter`), une fenêtre d'une durée (`minutes` : 300, 10 080, 43 200) en pourcentage avec sa remise à zéro, ou les crédits du mois (`minutes` : null). L'interface les libelle par leur durée (`meterLabel` : « 5h », « 7j » / « W », « 30j » ; « crédits »). Une lecture remplace les compteurs qu'elle donne et garde les autres (`model::merge_meters`). Claude Code donne ses deux fenêtres (`claude_meters`), par `get_usage`, `rate_limit_event` ou le point d'accès OAuth, que seuls ses comptes interrogent.

Ce qui est gardé d'un lancement à l'autre (`SavedPause`, `SavedWindows`, `Hold.lifted`) garde les deux fenêtres de Claude Code là où les versions d'avant les lisent, les autres compteurs à côté (`other`, `lifted_other`).

`UsageSnapshot.current` donne le compte en cours de chaque moteur. La barre d'état montre celui du moteur de l'agent affiché, sinon du moteur par défaut du projet (`statusEngine`), avec ses compteurs ; un moteur sans quota n'y a que le coût du jour. Le panneau des quotas et l'onglet « Comptes » groupent les comptes par moteur, dans « Ordre des harness ».

Le pilote auto se met en pause par moteur : `board::to_start` écarte un ticket dont tous les comptes de son moteur sont retenus (`Core::project_hold(projet, moteur)`), pendant que ceux d'un autre moteur démarrent. Une pause dit sa raison (`PauseReason::Window` avec ses `minutes`, `Credits` ou `Limit`). Un moteur sans quota n'a aucun compteur : seule une limite d'usage le retient.

### Le coût dans les stats

Chaque tour et chaque prompt gardent leur moteur (colonnes `engine` de `turns` et de `prompts`). `Stats::query_by` prend un filtre (`Filter { account, engine }`) ; `by_engine` met les moteurs côte à côte (tokens, coût, tours, agents, ceux du compte demandé), `by_account` les comptes du moteur demandé, `engines` liste les moteurs qui ont tourné. L'interface écrit un coût comme le moteur le donne (`caps.cost`) : exact, « ≈ … estimé » (`fCost`, `costHint`), ou « — ».

## Le serveur MCP d'Escouade (ESC-33)

Donner le serveur MCP d'Escouade au process d'un agent, le lui refuser, et le déclarer dans la config de l'utilisateur (« Les agents peuvent piloter Escouade ») passent par le moteur. Le cœur décide (`Core::agent_access`, avec l'interrupteur et l'état du serveur, comme avant) ; le moteur dit comment. Claude Code en est la première implémentation (`engine/claude/mcp.rs`), sans changement : mêmes arguments, mêmes fichiers, mêmes commandes.

| Méthode de `Engine` | Ce qu'elle fait | Claude Code |
|---|---|---|
| `grant_file(url, jeton)` | Ce que contient le fichier privé du process (écrit avant son lancement en `<données>/mcp/<id de l'agent>.json`, 0600, oublié à la fin du process) ; `None` : pas de fichier, le moteur passe le serveur autrement. | `{"mcpServers":{"escouade":{"type":"http","url","headers":{"Authorization":"Bearer …"}}}}` |
| `refusal()` | Ce que lit un process privé du serveur, dans un fichier commun à tous les agents (`mcp-refused.json`) ; `None` : ses arguments suffisent. | `{"deniedMcpServers":[{"serverName":"escouade"}]}` |
| `escouade_launch(Escouade)` | Les arguments (donnés en dernier) et les variables du process, donné ou refusé. **Jamais le jeton dans les arguments** : un fichier ou une variable le porte. | `--mcp-config <fichier>` ; refusé, `--settings <fichier>` (s'il y en a un) puis `--disallowedTools mcp__escouade` |
| `user_config()` | La config de l'utilisateur qu'Escouade écrit lui-même (`engine/userconfig.rs`, `UserConfig` : le fichier dans le dossier du compte, son format, la table des serveurs, les noms de l'adresse et des en-têtes) ; avec elle, les quatre méthodes suivantes n'ont pas à être écrites (leurs versions par défaut la lisent). `None` : le moteur déclare par ses commandes, ou pas du tout. | `None` (Codex : `config.toml`, TOML, `mcp_servers`, `url`, `http_headers`) |
| `declared(config)` | Ce que dit la config de l'utilisateur du serveur (`Standing` : pas de fichier, illisible, absent, déclaré). | l'entrée `mcpServers.escouade` du `.claude.json` du compte |
| `add_declaration`, `remove_declaration` | Ajouter ou retirer l'entrée du compte (`Target` : son programme, son environnement, sa config). | `claude mcp add --scope user --transport http …`, `claude mcp remove escouade --scope user` |
| `manual_declaration` | La commande à lancer à la main (« Copier la commande »). | PowerShell ou shell POSIX, avec `CLAUDE_CONFIG_DIR` le temps de la commande |

`Escouade` (`engine/mcp.rs`) est ce que le cœur donne au moteur : `Granted { url, token, file }` ou `Denied { file }`. Son `{:?}` masque le jeton, et celui de `mcp::Launch` ne montre que le nom des variables. La vue Claude Code (`tui.rs`) prend les mêmes arguments, et met le refus du moteur dans son propre fichier de réglages (Claude Code ne lit que le dernier `--settings`).

`mcp/install.rs` garde ce qui est commun : la comparaison (absente : ajout ; identique : rien ; différente : retrait puis ajout), le masquage des jetons dans les erreurs, le lancement d'une commande d'un compte (`run`), et la boucle sur les harness et leurs comptes : chaque compte passe par le moteur de son harness (`engine::by_id(account.engine)`).

**« Les agents peuvent piloter Escouade » (ESC-50, ESC-119).** Un seul interrupteur, `Settings.mcp_enabled` : allumé, le serveur est déclaré dans chaque harness installé (`Core::installed` : son programme trouvé pour un de ses comptes actifs ou son Principal) qui sait le déclarer (Claude Code par ses commandes, les autres par leur `user_config`), pour chacun de ses comptes actifs, ou son Principal tant qu'aucun n'est enregistré (créé au premier agent, il ne l'est pas pour une déclaration). Un harness installé depuis est vu au tour suivant, que la fenêtre amène en reprenant le focus (`Core::declare_on_focus`, une fois par minute au plus), en montrant l'onglet (`mcp_declare_status`) ou la liste des harness (`engines`) ; un compte ajouté, à la sauvegarde. Le `mcpHarnesses` d'ESC-50 (un interrupteur par harness) est ignoré au chargement. `UserConfig` dit le fichier (`files` : le premier présent dans le dossier du compte, sinon le premier), son format, la table des serveurs, les champs de l'entrée et ceux qu'elle porte d'office (`fixed` : `"type": "remote"`) ; Codex (TOML, `config.toml`), OpenCode (JSONC, `opencode.json` ou `opencode.jsonc` sous `$XDG_CONFIG_HOME/opencode`, `accounts::config_dir`) et Copilot CLI (JSONC, `mcp-config.json` de son `COPILOT_HOME`). TOML : `toml_edit` ; l'entrée remplace une entrée du même nom, à sa place ou après les autres serveurs ; retirée, la table implicite qu'elle avait créée part avec elle et les fins de ligne CRLF reviennent (un fichier sans saut de ligne final en gagne un). JSON avec commentaires et virgules finales (les deux harness les lisent, essayé sur OpenCode 1.19.3 et Copilot CLI 1.0.98) : `engine::jsonc` lit le texte en gardant où est chaque valeur, et l'entrée s'insère après le dernier membre de l'objet des serveurs (créé à la fin de l'objet du fichier s'il n'y en a pas), avec l'indentation et les fins de ligne du fichier ; retirée, seuls ses octets partent, l'objet des serveurs avec elle s'il est celui qu'`add` a écrit, et le fichier s'il est celui qu'`add` a fait de rien. Dans un objet des serveurs vide de l'utilisateur (`copilot mcp remove` du dernier laisse `"mcpServers": {}`), l'entrée prend une virgule finale : c'est ce qui la distingue d'un objet créé, et l'objet vide revient tel quel. Dans les deux formats, le fichier redevient celui d'avant à l'octet près. Un fichier qui est un lien (un compte qui partage les réglages de Principal sous macOS et Linux, `accounts::link_file`) le reste : c'est le fichier où il mène qui est écrit (`userconfig::written_at`), et deux comptes qui mènent au même fichier ne l'écrivent qu'une fois par tour (`same_file_key`), chacun recevant le même résultat ; supprimer un compte lié laisse l'entrée que Principal veut encore. Un fichier qui n'est pas un TOML ou un JSON valide, ou dont la table des serveurs n'en est pas une, n'est pas touché ; l'erreur va à la fenêtre, avec l'entrée à coller (`Declaration.paste`, `Declaration.file` pour tous : le fichier où est le jeton en clair). Éteindre change d'abord le jeton externe (`McpServer::turned_off`, dans la sauvegarde), puis le tour suivant retire l'entrée de tous les comptes de tous les harness. Les agents d'Escouade ne s'en servent pas : Codex et Copilot reçoivent leur entrée par leur lancement, qui l'emporte, OpenCode la sienne par dossier, son serveur gardant `mcp.escouade.enabled: false`, et un agent refusé coupe celle de l'utilisateur (`session.mcp.disable` pour Copilot).

**Les noms des outils.** Chaque moteur nomme le serveur `escouade` (`mcp::SERVER`), et chaque adaptateur nomme un appel d'outil MCP par `mcp::tool_name(serveur, outil)`, `mcp__<serveur>__<outil>` : les appels d'Escouade sont `mcp__escouade__<outil>` quel que soit le moteur, et la carte de permission, les libellés (`tools.ts`), la ligne d'activité et la recherche marchent tels quels. Codex (ESC-38) y vient par `-c mcp_servers.escouade.url=…` et `bearer_token_env_var`, le jeton dans une variable du process (`grant_file` à `None`), et `-c mcp_servers.escouade.enabled=false` pour le refus ; sa déclaration (ESC-50) par son `config.toml`. Essayé sur 0.163.0 : l'entrée de l'utilisateur (`http_headers`) atteint le serveur, et pour un agent, `bearer_token_env_var` l'emporte sur elle (son propre jeton est envoyé, pas celui de l'utilisateur).

Tests : `mcp/agents_tests.rs` (`no_engine_puts_the_token_on_the_command_line` vérifie la règle du jeton pour chaque moteur de `registry::all()`, et `keeps_the_token_off_the_command_line` resservira au banc de conformité ; le lancement réel, flux et vue Claude Code, n'a jamais le jeton sur sa ligne de commande), `mcp/install_tests.rs` (déclaration par le moteur), `mcp/declare_tests.rs` et `engine/userconfig_tests.rs` (Codex : interrupteur, jeton changé d'abord, fichier rendu à l'octet près ; `real_codex_drives_escouade`, à la main, sur le vrai `~/.codex`), `PermissionCard.test.ts` (même carte pour un agent d'un autre harness).

## Les textes et les consignes (ESC-31)

### Les textes

Un texte qui parle d'un agent le nomme comme son moteur, par `Engine::speaker` (« Claude » pour Claude Code) :

- dans la fenêtre, le paramètre `{engine}` des catalogues, que `speakerOf(agent.engine)` de `src/lib/engines.ts` remplit depuis la déclaration (`EngineDecl.speaker`) : « {engine} attend ta réponse », « {engine} travaille… », « {engine} demande une autorisation », « {engine} propose un plan », le placeholder et la file d'attente du composer, le sens des modes « Demander » et « Plan » (`modeTitle` de `src/lib/models.ts` ; le menu des réglages nomme le moteur par défaut), la vue d'ensemble. `{harness}` (`engineName`) nomme le moteur lui-même : « la session {harness} reste sur le disque », « hors du mode de permission de {harness} » ;
- dans le backend : la notification d'une question sans texte (`notify::question_body`), « {speaker} ne tourne plus pour cet agent », la session ou le point de copie perdus, et le nom du moteur (`name()`) dans « … s'est arrêté (code N) », « … n'a pas pu démarrer », « … introuvable ».

Un texte qui vaut pour tous les agents dit « les agents » : « Langue des textes rédigés par les agents », « Arrêter les processus des agents inactifs après », « 2 agents · 1,5 Go · 12 % CPU » dans la barre d'état. Restent « Claude » ou « Claude Code » : les comptes Claude et leurs quotas (lot 5), le Remote Control, la vue Claude Code (lot 9), les tâches de fond et les sous-agents (propres à Claude Code, masqués par les capacités), le serveur MCP (lot 8) et les textes rédigés par un one-shot (« Claude lit le projet… », lot 7).

### Les consignes

Le protocole du ticket (`board::protocol_prompt_for`) et les messages du pilote auto (premier message, boucles, reprise, retour de test, tests en échec, conflits, préparation du lancement) ne nomment aucun outil propre à Claude Code : « pose la question avec l'outil de question », pas `AskUserQuestion`. Un test le vérifie dans les deux langues (`board.rs`, `the_protocol_and_the_autopilots_messages_name_no_tool_of_claude_code`).

Le protocole est une consigne ajoutée : le pilote auto le met dans `AgentOptions.append_prompt`, que `Spawn.append_prompt` porte au moteur, qui le passe à sa façon (Claude Code : `--append-system-prompt` ; Codex : `developerInstructions` de `thread/start`). Sa longueur suit le moteur du ticket (`Engine::instructions_room`) : pour Claude Code, la ligne de commande d'un `.cmd` (6 000, pesés par `escaped_len`), les critères qui n'y tiennent pas remplacés par « … » ; sans borne, tous les critères.

Ce qui nomme un outil d'un moteur passe par lui : le message transmis à un sous-agent (`Engine::subagent_relay`, l'outil `SendMessage` de Claude Code).

## La vue terminal (ESC-34)

« Vue <harness> » : un agent tourne dans l'interface de son harness elle-même, dans un terminal intégré, à la place de la conversation d'Escouade. Une session n'a qu'un process à la fois : changer de vue arrête l'un et reprend la session dans l'autre. Le cœur (`tui.rs`) fait le même travail pour tous les harness (terminal et sortie gardée, serveur de hooks et jetons, lecture du journal au fil de l'eau, sortie aussitôt et session perdue, file des messages, arrêt) et ne parle à un harness que par son `TerminalView` (`engine/terminal.rs`), que `Engine::terminal()` donne, `None` pour un harness sans vue terminal ou dont l'interface s'ajoute au process au lieu de le remplacer (OpenCode, `Engine::attaches()`, « La vue OpenCode » ci-dessous) ; `caps().terminal_view` dit qu'il a l'une ou l'autre, ce qu'un test vérifie pour chaque harness.

| Méthode | Ce qu'elle dit | Claude Code (`engine/claude/terminal.rs`) | Codex (`engine/codex/terminal.rs`) |
|---|---|---|---|
| `shell()` | Le nom de son terminal. | `claude` | `codex` |
| `launch(TerminalLaunch)` | Ses arguments, son fichier privé et ses variables, depuis l'agent, la session nommée d'avance, Remote Control, le serveur de hooks (`HookServer` : adresse, variable du jeton, variable de l'adresse, `escouade-hook`) et le serveur MCP donné ou refusé. | `tui_args`, et les hooks HTTP dans le fichier de `--settings` | `resume <id>` ou `fork <id>`, `--no-daemon`, `-m`, l'effort, le mode (`--approve-for-me`, `-a on-request -s <bac à sable>`, ou sans bac à sable), les consignes d'une nouvelle session (`-c developer_instructions`), et `escouade-hook` pour chaque événement (`-c hooks.<Événement>=…`) ; l'adresse du serveur de hooks dans `ESCOUADE_HOOK_URL` |
| `escouade(harness, accès, dossier)` | Le serveur MCP d'Escouade pour l'interface, le dossier du compte à portée. | `Engine::escouade_launch` | le même, mais le refus n'est passé que si le `config.toml` du compte déclare une entrée `escouade` (sinon Codex ne démarre pas) |
| `prepare(programme, dossier, arguments, environnement)` | Ce qu'il faut ajouter aux arguments juste avant le lancement. | rien | la confiance de ses hooks pour ce seul process : `-c hooks.state={…}`, avec les clés et hachages que donne `hooks/list` d'un `codex app-server` lancé avec les mêmes hooks (et ceux de l'utilisateur déjà approuvés) |
| `read_dir_args(dossier)` | Ce qui lui fait lire un dossier sans demander (celui des fichiers joints). | `--add-dir=<dossier>` | rien : Codex lit tout sans demander |
| `names_its_session()` | Escouade nomme une session neuve avant le lancement ; sinon, son premier hook la dit. | oui (`--session-id`) | non : `SessionStart`, avec le premier message |
| `on_hook(rt, hook)` | L'état en direct : début et fin de tour, permission ou question en attente. | `AgentRt::on_hook` | `AgentRt::codex_hook` : `UserPromptSubmit`, `PermissionRequest`, `PostToolUse`, `Stop`, `Interrupt` |
| `running_dir`, `find_running` | Où il liste ses interfaces lancées, et la sienne : elle est passée ce qu'elle demande avant de lire un message. | `<dossier du compte>/sessions`, `tui::find_registered` | aucun |
| `find_log`, `on_log_entry` | La conversation : le journal de la session, et chaque entrée traduite. | `projects/*/<session>.jsonl`, `AgentRt::on_transcript` | `sessions/<a>/<m>/<j>/rollout-<date>-<session>.jsonl`, `AgentRt::codex_rollout` : ses `event_msg` changés en notifications de l'app-server, traduites par `codex_frame` |
| `says_session_lost`, `copy_session` | La reprise : ce qu'il écrit quand il ne trouve pas la session, et la copie d'une session d'un compte à l'autre. | « No conversation found », `accounts::copy_session` | « No saved session found », le rollout copié au même endroit |
| `paste_refusal(dossier du compte, dossier)` | Pourquoi un message ne peut pas être tapé maintenant (une question de l'interface à l'écran). | jamais | tant que le `config.toml` n'a pas de niveau de confiance pour le dépôt (sa racine, celle du projet pour un worktree) : Codex demande s'il faut faire confiance au dossier |
| `paste`, `submit`, `image_types` | Le collage d'un message (entre crochets, puis la touche d'envoi), et les images qu'il prend par un chemin collé. | collage entre crochets, Entrée, PNG/JPEG/GIF/WebP | collage entre crochets, Entrée, PNG/JPEG |
| `interrupt_key`, `quit_key` | La touche qui interrompt un tour, et celle qui le ferme pour de bon, envoyée deux fois. | Échap, Ctrl+C | Échap, Ctrl+C |

Les textes de la vue nomment le harness de l'agent (« Vue {engine} », « {engine} s'est arrêté. ») ; avec Claude Code, ils sont ceux d'avant. Le réglage des nouveaux agents (« Vue des nouveaux agents » : « Escouade » ou « Terminal du harness ») est montré tant qu'un harness a une vue terminal ; un agent dont le harness n'en a pas reste dans la vue d'Escouade (`Core::create_agent_with`), et le passage est refusé (`set_agent_view`).

Au retour dans la vue d'Escouade, la lecture du journal est oubliée (`AgentMeta.transcript_read`) : ce que le flux y ajoute ensuite est déjà dans la conversation, et l'interface suivante lit le journal à partir de sa fin. Sans cela, l'interface de Codex rejouerait les tours de l'app-server, écrits dans le même rollout.

### La vue Codex (ESC-47)

Faite d'après l'essai de Codex 0.163.0 (docs/TODO-1.8.md, point 0, « Vue terminal »). Ses choix :
- **Rien chez l'utilisateur.** Les hooks sont des options du process de l'interface (`-c hooks.<Événement>=…`), leur confiance aussi (`-c hooks.state=…`). Codex ignore sans rien dire un hook qu'on ne lui a pas dit d'approuver, et le hachage qu'il attend n'est pas documenté : `prepare` le lui demande (`hooks/list` d'un `codex app-server` lancé avec les mêmes hooks, dans le même `CODEX_HOME`, environ 0,1 s), plutôt que de le recalculer. Les hooks de l'utilisateur déjà approuvés sont redonnés avec, pour ne pas les perdre dans ce process.
- **`--no-daemon`** : l'interface a son propre process, jamais celui d'un démon partagé, qui n'aurait ni ses options ni ses hooks.
- **Le rollout** est le journal : ses `event_msg` (`task_started`, `item_completed`, `token_count`, `task_complete`, `turn_aborted`) ont la forme interne de Codex (snake_case, éléments en PascalCase) ; `agent/codex_tui.rs` en fait les notifications de l'app-server qu'ils racontent, traduites par `codex_frame` comme en vue Escouade. Un message tapé dans l'interface (`UserMessage`) devient le message de l'utilisateur. Ce qu'un tour fini raconte en retard (la commande coupée par Échap) est laissé de côté.
- **La fin d'un tour** vient du rollout ; si `Stop` ou `Interrupt` n'est pas suivi de sa fin, le cœur la fait au bout de 1,8 s (`end_tui_turn`, qui passe par `codex_tui_end` pour un agent Codex).
- **Le compte** : le dossier de Codex d'un Principal est `CODEX_HOME`, sinon `~/.codex` (`accounts::config_dir`).
- **Le faux `codex`** joue aussi l'interface (`tests/fixtures/fake-codex-tui.mjs`) : il exécute les hooks de ses options seulement s'ils sont approuvés avec le hachage que donne le `hooks/list` de son app-server, écrit son rollout, pose la question de la confiance du dossier, et refuse un `resume` sans rollout comme Codex. Sous Windows, il est lancé par un petit programme (`src-tauri/examples/fake_codex.rs`, que `cargo test` construit) : un `.cmd` ne passe pas `<session-flags>` (cmd.exe y lit une redirection), là où le vrai `codex.exe` le prend tel quel. Tests : `codex_tui_tests.rs` (dont `real_codex_view`, l'essai réel, à la main), `agent/codex_tui.rs` (le rollout réel de `tests/fixtures/codex/tui-rollout.jsonl` et les charges de `hooks-payloads.jsonl`), `engine/codex/terminal.rs`.

### La vue Copilot (ESC-48)

Faite d'après les essais de Copilot CLI 1.0.98 (docs/TODO-1.8.md, point 0, « Vue terminal », ESC-23, puis ceux d'ESC-48). Ses choix :
- **Rien chez l'utilisateur.** Les hooks sont ceux d'un plugin propre à l'agent, passé par `--plugin-dir` (`<données>/tui/<agent>.copilot-plugin/` : `plugin.json` et `hooks/hooks.json`, `{version: 1, hooks}`), donc de ce seul process. Essayé : `hooks.discover` d'un serveur lancé avec l'option les liste (`origin: plugin`), sans elle il n'y en a aucun, et l'interface dit « Loading: 1 plugin, 9 hooks » ; rien n'est écrit dans la `COPILOT_HOME` (ni `hooks/`, ni `settings.json`, ni `installed-plugins`).
- **Le nom de l'événement** n'est pas dans les charges de Copilot (`sessionId`, `cwd`, `prompt`, `toolName`… : tests/fixtures/copilot/tui-hooks.jsonl). Chaque entrée lance `escouade-hook <événement>` (ligne `powershell` sous Windows, `bash` ailleurs), qui le poste en `?event=<événement>` ; le serveur de hooks le met dans la charge (`escouade_hook_event`).
- **Le journal** est `session-state/<id>/events.jsonl` : les événements de la session tels que le serveur les envoie, sans ceux qu'il ne garde pas (`ephemeral` : les deltas, `assistant.usage`, `session.idle`). `agent/copilot_tui.rs` les passe au traducteur de la vue Escouade (`cp_event`) et y ajoute ce qui manque : le message tapé (`user.message`), l'usage et le coût de chaque appel (`session.usage_record`, dont `copilotUsage.totalNanoAiu`), la fin d'un tour (le hook `agentStop`, puis la fin faite par le cœur, `copilot_tui_end`) ou son interruption (`abort` ; aucun `agentStop` ne vient alors). Interrompre demande Échap deux fois : la première affiche « esc again to interrupt » (essayé), la seconde part 0,4 s après (`TerminalView::interrupt_again`).
- **L'attente** : une permission attend une fois que `notification` (`notification_type: permission_prompt`) le dit, avec ce que `permissionRequest` a dit de la commande juste avant (`permissionRequest` seul ne suffit pas : une règle peut encore l'accorder) ; une question, un formulaire et la fin d'un plan attendent dès leur événement du journal (`user_input.requested`…). Escouade n'y répond jamais : c'est l'interface qui demande.
- **La session** : Escouade nomme une nouvelle session (`--session-id <uuid>`) et en reprend une par `--resume <id>` ; une session sans événement n'est pas reprise (Copilot refuse : « Cannot create session from empty events array ») et une session inconnue fait sortir l'interface sur « No session, task, or name matched ». L'interface ne sait pas copier une session : une copie d'agent est faite d'abord par un serveur à part (`TerminalView::copy_first`, `sessions.fork` au dernier événement copié), puis reprise.
- **La confiance du dossier** : à chaque lancement sur un dossier que la `COPILOT_HOME` ne connaît pas, l'interface demande « Confirm folder trust » (« 1. Yes » pour ce process, « 2. Yes, and remember this folder », écrit dans `trustedFolders` de `config.json`, que suivent des commentaires `//`). Un dossier au-dessus suffit (essayé). Escouade ne colle rien tant que le dossier n'y est pas et que l'interface n'a rien dit depuis son lancement (hook ou ligne du journal) : « 1. Yes » n'écrit rien, et le premier message que l'utilisateur tape lui-même débloque ensuite ceux d'Escouade.
- **Les options** : celles du serveur d'Escouade (`--no-auto-update`, `--disable-builtin-mcps`, `--disallow-temp-dir`, `--secret-env-vars`), `--model` (jamais `auto` pour un agent sur « Auto »), `--reasoning-effort` (jamais avec `auto`), et le mode : `--allow-all` en « Bypass », `--mode plan`, `--allow-tool=write` en « Édits auto », `--mode autopilot` avec `write`, `shell(git add)` et `shell(git commit)` en « Auto », comme les règles d'Escouade en vue Escouade. Les consignes ajoutées (`systemMessage`) n'ont pas d'option dans l'interface : elles ne valent qu'en vue Escouade, qui les redonne à chaque reprise.
- **Le modèle d'un compte Free (ESC-122)** : l'interface n'accepte un `--model` nommé que si le compte le liste (`models.list`) ; sinon elle dit en rouge « ✗ Model "gpt-6-luna" from --model flag is not available. Using "auto" instead. » et tourne sur `auto`. Un compte Free n'en liste aucun (par un jeton) ou `auto` seul (connecté par `copilot login`), essayé sur 1.0.98 ; le serveur, lui, accepte le modèle nommé mais tourne aussi sur `auto` (`session.auto_mode_resolved`). Avant de lancer l'interface d'un agent sur un modèle nommé, `TerminalView::for_account` demande donc ses modèles à un serveur du compte (démarré puis arrêté, une fois par compte et par lancement de l'app : un forfait changé se voit au suivant) et passe `--model auto`, sans `--reasoning-effort`, pour un modèle qu'il ne liste pas. L'agent garde son modèle.
- **Ce qui reste dans la `COPILOT_HOME`** : chaque `--plugin-dir` y a un dossier de données, `plugin-data/_direct/<SHA-256 de {"source":"local","path":"<dossier du plugin, résolu>"}>`, que Copilot crée vide et ne retire jamais. Le plugin d'un agent ne change pas d'un lancement à l'autre : un dossier par agent, retiré avec le plugin quand l'agent est supprimé (`TerminalView::forget`), et seulement s'il est vide. Le `config.json.trusted-folders.lock` vide que Copilot laisse après avoir retenu un dossier (« Yes, and remember this folder ») est son verrou, partagé avec les sessions de l'utilisateur dans `~/.copilot` : Escouade n'y touche pas.
- **Le serveur MCP d'Escouade et les comptes (ESC-40)** : l'interface les a comme le serveur, par le même `Engine::escouade_launch` (`TerminalView::escouade` par défaut) : donné, `--additional-mcp-config @<données>/mcp/<agent>.json` et le jeton dans son environnement seul ; refusé, `--disable-mcp-server escouade`, qui vaut dans l'interface (en mode serveur, sans effet, le serveur coupe l'entrée de la session ; l'interface n'a pas de session à qui le demander). Pour un compte à lui, les jetons GitHub de l'environnement d'Escouade sont vidés (`TerminalView::masked_env`), comme pour son serveur et `copilot -p`. Ce que l'interface ne permet pas : refuser elle-même un compte resté sur celui du GitHub CLI (`gh-cli`, qu'elle ne dit qu'à l'écran) ; la vue Escouade le refuse au démarrage de son serveur.
- **Le faux `copilot`** joue aussi l'interface (`tests/fixtures/fake-copilot-tui.mjs`, quand il n'a pas `--server`) : il lance les hooks de son plugin (et ceux de `<COPILOT_HOME>/hooks/`) avec les charges de Copilot, écrit `events.jsonl` et la session telle que le faux serveur la garde, pose la question de la confiance du dossier, et refuse un `--resume` sans événement comme Copilot ; le faux serveur écrit lui aussi `events.jsonl`. Tests : `copilot_tui_tests.rs` (dont `real_copilot_view`, l'essai réel, à la main : une réponse, une commande autorisée dans l'interface, une commande interrompue, puis un tour en vue Escouade sur la même session ; vu sur la carte : en cours, attend, en cours, terminé), `agent/copilot_tui.rs` (le journal réel de `tests/fixtures/copilot/tui-events.jsonl` et les charges de `tui-hooks.jsonl`), `engine/copilot/terminal.rs`.

### La vue OpenCode (ESC-46)

L'interface d'OpenCode ne prend pas la place du process : elle s'y ajoute. `Engine::attaches()` le dit (OpenCode seul ; `terminal()` reste `None`, `caps().terminal_view` vaut `true`), et le cœur passe alors par `attach.rs` au lieu de `tui.rs` :
- **L'état** est `AgentMeta.attached` (jamais avec `terminal`, jamais pour un agent de ticket ; un nouvel agent le prend quand « Vue des nouveaux agents » dit « Terminal du harness ») et le terminal `AgentRt.attach`. Tout ce qui lit `terminal` (messages collés, interruption, sous-agents, remote control) voit donc un agent en vue Escouade : les messages de la fenêtre et du serveur MCP passent par le process, comme avant. La fenêtre montre le terminal pour `terminal || attached` (`inHarnessView`).
- **Le lancement** : le process d'abord (`ensure_process`, qui ouvre la session), puis `EngineProcess::attach()` donne les arguments (`attach <adresse> --session <id> --dir <worktree>`) et, à part, les variables secrètes (`OPENCODE_SERVER_PASSWORD`) : le mot de passe n'est jamais sur la ligne de commande ni dans le journal (`engine::terminal::Attach`). Le terminal a l'environnement du compte, comme le serveur.
- **Ce qui reste d'Escouade** : son client SSE. La conversation, l'état de la carte, les permissions et questions, la fin des tours et le coût viennent du flux, sans hook ni journal ; un message tapé dans l'interface est un tour vu par Escouade (« tour commencé ailleurs »), et le message lui-même s'affiche comme celui de l'utilisateur, une fois : le process marque chaque message `user` de la session d'après les prompts qu'il a envoyés et pas encore vus (`typedElsewhere`), et le traducteur n'affiche que les autres, ceux d'Escouade l'étant déjà, une permission répondue d'un côté se ferme de l'autre (`permission.replied`).
- **Changer de vue** se fait aussi en plein tour : rien n'est arrêté que l'interface. Quitter la vue, fermer le terminal ou « Relancer » n'arrête jamais le serveur, qui sert les autres agents du compte.
- **Une interface sans process** : une tâche suit l'interface (`watch_attached`, toutes les 300 ms) et l'arrête quand l'agent est supprimé, quand son process n'est plus là depuis 3 s, ou quand il est sur un autre serveur (relancé : une autre adresse ou un autre mot de passe). Ce qu'elle ferait sans process, Escouade ne l'entendrait pas. L'archivage l'arrête tout de suite. La vue reste ; « Relancer » en lance une autre.
- **Le faux `opencode attach`** (`tests/fixtures/fake-opencode.mjs`) se branche sur le faux serveur avec le mot de passe de son environnement, dit le titre de la session, puis ce qu'elle fait (texte, permission demandée ou répondue, fin du tour) ; Entrée répond « Allow once » à la permission en attente, sinon le texte tapé part comme message. Il note ses lancements (`<OPENCODE_CONFIG_DIR>/attaches.jsonl`, et `{"attach"}` / `{"shown"}` dans `FAKE_OPENCODE_LOG`). Tests : `opencode_tests.rs` (`the_opencode_view_joins_the_agents_session_beside_escouade`), `attach.rs`, `engine/opencode/mod.rs`, e2e `opencode.spec.ts` (« Vue OpenCode »).

### `escouade-hook`

Codex et Copilot CLI lancent leurs hooks comme des commandes, pas en HTTP. `escouade-hook` (`src-tauri/src/bin/escouade-hook.rs`, la bibliothèque standard seule) lit l'événement sur son entrée et le poste au serveur de hooks d'Escouade comme le ferait un hook HTTP de Claude Code : à l'adresse de `ESCOUADE_HOOK_URL` (la boucle locale seulement), avec le jeton de `ESCOUADE_HOOK_TOKEN` en `Authorization: Bearer`. Sans jeton, il sort aussitôt sans rien lire ni envoyer : un hook déclaré dans un dossier que l'utilisateur partage avec ses propres sessions ne les touche pas. Il sort toujours avec le code 0 et n'écrit rien : le harness continue comme sans lui. Son argument, s'il en a un, est le nom de l'événement (`escouade-hook agentStop`), posté en `?event=<nom>` (lettres et chiffres seulement) : les charges de Copilot ne le disent pas.

C'est un second binaire du crate : `tauri build` le construit et l'installe à côté de l'app (Windows : le dossier d'installation, l'installeur NSIS l'y copie et le désinstalleur l'enlève ; macOS : `Escouade.app/Contents/MacOS` ; Linux : `/usr/bin` du .deb et de l'AppImage), ce que la CI vérifie sur chaque paquet. `engine::terminal::hook_command()` le trouve à côté du programme de l'app (ou un dossier au-dessus, pour les tests). Ses tests : `tests/escouade_hook.rs` (le programme, avec et sans jeton) et `terminal_tests.rs` (posté au vrai serveur de hooks, l'agent suit).

## Le banc de conformité (ESC-35)

Une suite de scénarios commune que chaque moteur passe avec son faux moteur (`src-tauri/src/engine/bench/mod.rs`). Elle juge ce que la traduction du moteur fait de chaque scénario **dans la conversation de l'agent** (éléments normalisés, statut, demandes en attente, plan, `Effects`), jamais ses frames : la même suite vaut pour tous les protocoles.

Le banc pilote le moteur comme le cœur le fait, sans le cœur : `Engine::spawn` (avec `Spawn::of`, comme `Core::ensure_process`), `EngineProcess::initialize`, `Engine::message` puis `send_message` et `AgentRt::push_user` (comme `Core::deliver`), `AgentRt::answer_permission_by` et `answer_question_by` (comme la fenêtre), `interrupt` avec `AgentRt.interrupted` (comme `Core::interrupt`), `AgentRt::detach` puis `close_input` (l'arrêt d'un agent inactif). Chaque frame passe par `Engine::translate` du moteur essayé, pas par `engine::of` : un moteur se met sur le banc avant d'avoir sa ligne dans `engine::by_id`.

### Les scénarios

| Scénario | Ce que fait le banc | Ce qu'il vérifie | Demandé |
|---|---|---|---|
| `Text` | Un message ordinaire. | Un élément `text`, un `turn` sans erreur, `TurnEnd::Finished` avec le texte, statut `done`, une session. | toujours |
| `Thinking` | Un message qui fait raisonner. | Un élément `thinking` non vide, puis un `text`. | toujours |
| `Approval` | Une commande qui demande son approbation, autorisée. | Une demande en attente de type permission, une carte `permission` d'outil `Bash`, statut `waiting` ; après l'accord, la décision sur la carte, l'outil `Bash` en `ok`, le tour fini. | toujours |
| `Edit` | Une modification de fichier. | Un outil `Edit`, `Write` ou `MultiEdit` en `ok`, le fichier parmi `touchedFiles`, `Effects.files_changed`. | toujours |
| `Question` | Une question à l'utilisateur, répondue par son premier choix. | Une carte `question` (forme d'`AskUserQuestion` : `questions[].question`, `options[].label`) en attente ; ses réponses sur la carte, le tour fini. | toujours |
| `Interrupt` | Un tour long, interrompu. | `TurnEnd::Interrupted`, l'élément `turn` marqué `interrupted`. | toujours |
| `PlanMode` | La fin du mode plan, un plan à approuver. | Une permission d'outil `ExitPlanMode`, accordée, le tour fini. | si `caps.modes` a `plan` |
| `Subagents` | Un sous-agent lancé. | Un outil `Task` (ou `Agent`), une ligne dans `meta.plan.agents`, finie (`done`) au bout du tour. | toujours |
| `Limit` | Un tour arrêté par la limite d'usage. | `TurnEnd::Limited`, `Effects.limited`, l'élément `turn` marqué `limited`. | si `caps.quota` n'est pas `none` |
| `Resume` | Un tour, l'arrêt du process, un nouveau process, un tour. | `Start::of` donne `Resume` sur la session, et le second tour la garde. | toujours |
| `Fork` | Un tour, puis une copie (`fork_of` et `fork_at` de l'original, comme « Dupliquer la conversation »). | La copie a sa propre session, et `fork_of` est oublié dès son premier tour ; pour `fork: entry`, l'original a une `last_entry`. | si `caps.fork` n'est pas `none` |
| `Quota` | `EngineProcess::read_usage`. | Des fenêtres que `usage::parse_windows` lit. | si `caps.quota` n'est pas `none` |

Un scénario qui échoue dit le moteur, le scénario et la conversation entière ; un process qui ne dit plus rien le fait échouer au bout de 60 s.

### Brancher un moteur

Un moteur se branche par une `Target`, écrite à côté de lui (`engine/<moteur>/bench.rs`), et un test qui appelle `bench::run` : rien du banc ni du cœur ne change.

```rust
impl Target for Codex {
    fn engine(&self) -> &'static dyn Engine { &CODEX }
    fn program(&self) -> Program { /* tests/fixtures/fake-codex.cmd */ }
    fn env(&self, dir: &Path) -> Vec<(String, String)> { /* ex. CODEX_HOME dans dir */ }
    fn prompt(&self, s: Scenario) -> Option<String> { /* le message qui fait jouer s au faux */ }
}

#[tokio::test]
async fn the_fake_codex_passes_the_bench() {
    let played = run(&Codex, &bench_dir("bench-codex")).await;
    assert_eq!(played, Scenario::ALL);
}
```

`run` joue chaque scénario que ses capacités demandent (`Scenario::applies`), chacun dans son dossier, et rend ceux qu'il a joués : le test dit lesquels il attend. Un faux qui ne joue pas un scénario demandé (`prompt` rend `None`) fait échouer le banc.

- **Claude Code** : `engine/claude/bench.rs`, avec le faux `claude` (`tests/fixtures/fake-claude.mjs`, dont le message choisit le scénario ; « raisonnement » a été ajouté pour `Thinking`). Il joue les douze scénarios (`the_fake_claude_passes_the_bench`).
- **Codex** : `engine/codex/bench.rs`, avec le faux `codex app-server` (`tests/fixtures/fake-codex.mjs`, son protocole JSON-RPC rejoué d'après les flux réels de la 0.163.0 ; « commande », « fichier », « question », « propose un plan », « sous-agent », « lent », « limite »…). Il joue les douze scénarios (`the_fake_codex_passes_the_bench`) : la fin du mode Plan est la carte `ExitPlanMode` que Codex n'a pas, le tour restant ouvert jusqu'à l'approbation, et le quota passe par `{"meters": […]}`, que `usage::parse_windows` lit pour tout moteur.
- **Le faux moteur minimal** : `engine/minimal/` et sa traduction `agent/minimal.rs`, pour les tests seulement. C'est le moins qu'un moteur écrive pour passer le banc : un `Engine` (ses méthodes que le banc n'appelle pas sont `unreachable!`), un process joué sur un fil de l'app dans un protocole à lui (`start`, `thinking`, `say`, `call`, `done`, `ask`, `question`, `end`), des outils à lui (`shell`, `patch`, `agent`) nommés comme ceux de Claude Code (`Bash`, `Edit`, `Task`). Ses capacités n'ont ni copie, ni quota, ni mode `plan` : il joue les huit autres scénarios (`the_minimal_fake_engine_plugs_into_the_bench_and_passes_what_its_capabilities_call_for`). Il s'est branché par ses deux déclarations de module (`engine/mod.rs`, `agent.rs`), comme tout moteur, sans une ligne du cœur ni du banc.

### Les flux enregistrés

Un flux réel enregistré se rejoue dans la traduction d'un moteur et se compare aux éléments attendus (`engine/bench/replay.rs`). Un cas est un fichier JSON de `tests/fixtures/replays/<moteur>/` :

```json
{
  "about": "Ce qui a été enregistré, avec quelle version, et ce que ça montre.",
  "flux": "../../real-run-send-message.jsonl",
  "agent": { "cwd": "C:\\Users\\dev\\repo" },
  "items": [{ "kind": "text", "parent": null, "streaming": false, "text": "LAUNCHED" }],
  "agentAfter": { "sessionId": "e172c854-…", "status": "done", "touchedFiles": [] }
}
```

- `flux` : le flux tel qu'il est venu, une frame (une valeur JSON) par ligne, chemin relatif au cas. Les enregistrements restent où ils sont (`tests/fixtures/real-run-*.jsonl`, `tests/fixtures/codex/`).
- `agent` (facultatif) : l'agent avant la première frame, par-dessus les valeurs d'un nouvel agent, comme son état est sauvé (`AgentMeta`, en camelCase).
- `items` : la conversation que font les frames, dans l'ordre, sans son `id` ni ses heures (`ts`, `startedAt`, `endedAt`, inventés à chaque rejeu). Un élément attendu n'est comparé que par ce qu'il dit : une clé qu'il omet n'est pas comparée (un objet en lui non plus ; une liste a sa longueur). Les nombres sont comparés à leur dernier bit près.
- `agentAfter` (facultatif) : l'agent après la dernière frame, comparé de même.

Un écart dit le cas, l'élément, le chemin de la clé, l'attendu et l'obtenu. `ESCOUADE_BLESS=1 cargo test --lib <test>` réécrit les cas d'après ce que font leurs frames (leurs `items`, et les clés de leur `agentAfter`, ou six d'entre elles s'il n'en a pas) au lieu de les comparer : le diff se relit avant d'être commité. Un test par moteur rejoue tous ses cas (`replay::check_all(&CLAUDE, "claude")`). Aujourd'hui : trois flux réels de Claude Code (sous-agent à qui l'on écrit, sous-agent arrêté, commande passée en arrière-plan), quatre de Codex 0.163.0 (un tour complet avec commandes approuvées, sous-agent et plan ; le mode Plan avec sa question et son plan ; une reprise ; un tour en échec), lus dans leur forme d'enregistrement (`{"dir", "msg"}`, seul ce que Codex a dit est traduit), et un flux du faux moteur minimal, écrit à la main dans son protocole. Les heures du plan de l'agent (`startedAt`, `endedAt` de ses sous-agents) ne sont pas comparées non plus.

## Ajouter un moteur

À cocher dans l'ordre, dans le ticket du moteur. `<moteur>` est son id dans les données (`codex`), déjà dans `EngineId`.

### L'adaptateur

- [ ] `src-tauri/src/engine/<moteur>/mod.rs` : un `static` qui implémente `Engine` (sans état), déclaré dans `engine/mod.rs` (`mod <moteur>;`).
- [ ] `spawn` : lancer, reprendre et copier selon `Spawn.start` (`New`, `Resume`, `Fork`), avec le modèle, l'effort, le mode, les consignes ajoutées (`append_prompt`), les dossiers lisibles sans question (`read_dirs`) et le serveur MCP d'Escouade (`spawn.escouade`) ; un `Fork` refusé si le moteur ne copie pas.
- [ ] Un `EngineProcess` pour son process, dans le Job de l'agent (`job::Job::for_child`) : `initialize` (`Ready` : commandes `/`, modèles), `send_message`, `interrupt`, `set_model`, `set_effort`, `set_mode`, `answer_permission`, `answer_question`, `read_usage`, `rename`, `close_input`, `kill`, `wait_exit`, `wait_gone`. Ce qu'il ne sait pas faire (`stop_task`, `task_output`, `remote_control`) répond « non pris en charge ».
- [ ] Un transport qui n'est pas une ligne par message (en-têtes `Content-Length`, SSE) est lu dans le process, qui passe une `Value` par frame à `on_frame` ; les réponses à ses propres requêtes restent dans le process.
- [ ] `message` : la forme de son message, pièces jointes comprises, et le refus d'un fichier qu'il ne lit pas (`caps.images`, `caps.pdf`).
- [ ] `start_lost` : la session ou le point de copie perdus, lus dans son stderr.
- [ ] `one_shot` : `Weight::Light` (son modèle le plus rapide, sans outil) et `Weight::Reading` (outils de lecture seule), sans session, sans réglages de l'utilisateur ni serveur MCP.
- [ ] Sa ligne dans `engine::by_id`, et dans `registry::all()`.

### Les capacités et le registre

- [ ] `caps()` : ce qu'il sait faire, ni plus ni moins (`EngineCaps`, voir « Le registre ») ; chaque mode avec son sens chez lui (`ModeCap.meaning`). L'interface masque le reste.
- [ ] `models()`, `efforts()`, `supports_effort`, `supports_auto` : les menus du composer.
- [ ] `locate`, `version_args`, `tested()`, `oldest_reason`, `update_command`, `install()` : le trouver, le refuser trop ancien, l'installer.
- [ ] Le schéma de sa plus récente version testée dans `tests/fixtures/engines/<moteur>-<version>.schema.json`, et les deux tests qui le tiennent (champs lus par Escouade, champs des flux enregistrés).

- [ ] Ses comptes (`Engine::accounts`, la création et la connexion d'un compte) et la lecture de son quota en `QuotaMeter` (voir « Les comptes et le quota »).

### Le traducteur

- [ ] `src-tauri/src/agent/<moteur>.rs` (`mod <moteur>;` dans `agent.rs`) : `AgentRt::<moteur>_frame`, que `Engine::translate` appelle.
- [ ] Les éléments normalisés : `text`, `thinking`, `tool` (puis son résultat : `status` `ok` ou `error`, `result`), `permission` et `question` avec leur demande en attente (`add_pending`, statut `waiting`), `notice`, `event`, et le `turn` de fin de tour avec `Effects.turn_end` (`Finished`, `Interrupted`, `Limited`, `Error`), `Effects.limited` et `Effects.rate`.
- [ ] Les noms d'outils de Claude Code (`Bash`, `Edit`, `Write`, `Task`…, table de docs/TODO-1.8.md, « Un moteur par agent »), un outil MCP nommé par `mcp::tool_name`, un outil inconnu sous son propre nom ; un fichier modifié passé à `touch`.
- [ ] La session : `meta.session_id` dès qu'il la dit, `fork_of` et `fork_at` oubliés au premier tour d'une copie, `last_entry` (ou son équivalent) pour copier jusqu'au bon point.
- [ ] Le plan : sous-agents (`plan_tool_use`, `on_tool_result`), liste de tâches, fin de tour (`PlanState::end_turn`).
- [ ] L'inconnu toléré : un champ, une frame ou un événement inconnus sont ignorés sans casser le tour (un test, comme `an_unknown_field_or_frame_of_a_newer_claude_code_leaves_the_turn_going`).

### Le serveur MCP d'Escouade et les textes

- [ ] `grant_file`, `refusal`, `escouade_launch` : le serveur donné ou refusé, **jamais le jeton dans les arguments** (`no_engine_puts_the_token_on_the_command_line` le vérifie pour chaque moteur de `registry::all()`).
- [ ] `declared`, `add_declaration`, `remove_declaration`, `manual_declaration` : « Les agents peuvent piloter Escouade » (ou seulement `user_config`).
- [ ] `speaker`, `instructions_room`, `subagent_relay` : comment les textes nomment ses agents, la place des consignes ajoutées, le message transmis à un sous-agent.

### Le faux moteur et le banc

- [ ] `tests/fixtures/fake-<moteur>.mjs`, avec son `.cmd` (Windows) et un script sans extension (macOS, Linux) : son vrai protocole, rejoué d'après ses flux enregistrés ; le message choisit le scénario, comme pour le faux `claude`.
- [ ] `src-tauri/src/engine/<moteur>/bench.rs` : sa `Target` et le test qui appelle `bench::run`, avec la liste des scénarios attendus ; le banc passe.
- [ ] Ses flux réels enregistrés (`tests/fixtures/<moteur>/`), un cas par flux dans `tests/fixtures/replays/<moteur>/` (béni, relu), et le test `replay::check_all(&<MOTEUR>, "<moteur>")`.
- [ ] S'il a une vue terminal (`caps.terminal_view`) : son interface factice, lancée dans un pseudo-terminal, qui écrit son journal et appelle ses hooks (lot 9).
- [ ] Les e2e : un agent sur ce moteur (message, approbation, diff, fin de tour), un ticket lancé dessus par le pilote auto.

### Les essais réels

- [ ] Un essai réel avant la release : un vrai compte, un ticket en worktree jusqu'au commit et au merge, un plan avec sous-agents, une limite d'usage si possible ; ses flux enregistrés (voir ci-dessus).
- [ ] La plage de versions testées mise à jour d'après l'essai (`tested()`).
- [ ] docs/PROTOCOL.md : sa section (versions testées, méthodes et événements utilisés, correspondance des modes et des outils) ; ce document : sa colonne dans les tables ; README (installation), CHANGELOG et site.

## Vérification de l'extraction (ESC-26)

Sans changement de comportement : `core_tests.rs`, `tickets_tests.rs` et les tests de `agent.rs` sont inchangés ; les autres fichiers de tests n'ont changé que par leurs imports (`crate::claude::…` devenu `crate::engine::…`). Les tests unitaires des arguments et des messages de Claude Code ont suivi leur code dans `engine/claude/mod.rs`, ceux du programme et de sa localisation dans `engine/program.rs` et `engine/claude/locate.rs`.
