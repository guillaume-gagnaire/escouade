# Protocole Claude Code (stream-json) — notes d'implémentation

Validé empiriquement sur Claude Code 2.1.283 (types de référence : `@anthropic-ai/claude-agent-sdk` 0.3.283, `sdk.d.ts` / `sdk-tools.d.ts`).

## Lancement d'un agent

```
claude --output-format stream-json --verbose --input-format stream-json
       --permission-prompt-tool stdio --include-partial-messages
       --permission-mode <auto|plan|acceptEdits|bypassPermissions>
       --model <fable|opus|sonnet|haiku> --effort <low|medium|high|xhigh|max>
       [--resume=<session_id> [--fork-session [--resume-session-at=<uuid>]]]
       [--allow-dangerously-skip-permissions]
```

Une ligne JSON par message, dans les deux sens (stdin / stdout).

`--resume=<id> --fork-session` (copie d'un agent, vérifié sur 2.1.289 avec `--input-format stream-json`) : la conversation reprise continue sous un nouveau `session_id` (les événements d'un hook `SessionStart`, s'il y en a, le portent dès le démarrage, avec la source `fork`), et le fichier de la session d'origine n'est pas touché, alors qu'un `--resume` simple y ajoute des lignes dès le démarrage. La nouvelle session n'est écrite qu'au premier message : tant qu'aucun `system/init` n'est venu, il faut forker de nouveau. Une session d'origine introuvable échoue comme un `--resume` (« No conversation found with session ID »).

`--resume-session-at=<uuid>` (option cachée, absente de `--help`, vérifiée sur 2.1.289) ne garde de la session reprise que les messages jusqu'à l'entrée `<uuid>` incluse (« any chain-entry UUID, typically the kept turn's last entry »). Le binaire la dit « ignorée hors du mode print », mais nos lancements stream-json sans `-p` y sont : forkée à l'entrée du premier de deux tours, la session ne contient plus que ce tour (`export_conversation`, une requête de contrôle sans paramètre, rend son texte sans appel au modèle). L'`uuid` d'une frame `assistant` du fil principal est celle de son entrée dans le transcript (le CLI la recopie de son message), sauf pour les messages qu'il fabrique (`is_meta`) ; de même pour une frame `user` qui porte un `tool_result` et pour `system/compact_boundary`. Escouade garde comme point de copie la dernière entrée d'assistant ou de résultat d'outil du fil principal (le résultat d'un outil termine un tour interrompu ou en erreur avant toute réponse). Une entrée introuvable fait échouer le démarrage (« No message found with message.uuid of: <uuid> », code 1) : la copie forke alors la session entière.

Compaction : la chaîne repart de l'entrée `compact_boundary` (`parentUuid: null`). Forkée à une entrée d'avant la frontière, la session est refusée comme ci-dessus ; à la frontière ou au résumé qui la suit, elle est acceptée mais ne garde que ces entrées-là (`export_conversation` ne rend aucun message), et à une entrée d'après, elle garde le résumé et ce qui suit jusqu'à elle (vérifié sur une session faite à la main). Escouade oublie donc son point de copie à chaque `compact_boundary`, jusqu'à la prochaine entrée. Limite connue : si l'original compacte après la copie et que la copie redémarre avant son premier tour, son point de copie n'est plus dans la session, et elle reprend celle-ci entière, telle qu'elle est alors (avec l'avis « Point de copie introuvable… »).

## Hôte → CLI

- Message utilisateur :
  `{"type":"user","message":{"role":"user","content":<string|blocks>},"parent_tool_use_id":null,"uuid"?,"priority"?:"now"|"next"|"later"}`
  - `content` peut contenir des blocs `image` (base64) et `document` (`"title":<nom>` ; PDF : `"source":{"type":"base64","media_type":"application/pdf","data":…}`, texte : `"source":{"type":"text","media_type":"text/plain","data":<texte>}`) ; `priority` gère la file d'attente pendant un tour en cours.
- Requête de contrôle : `{"type":"control_request","request_id":"req_N","request":{"subtype":...}}`
  - `initialize` → réponse : `commands` (slash commands + skills), `models`, `account`.
    - `models` : `[{value, resolvedModel, displayName, description, supportsEffort…}]`, l'alias ou l'id et le modèle qu'il lance (2.1.284 : `opus` → `claude-opus-5-5`, `sonnet` → `claude-sonnet-5-5`, `haiku` → `claude-haiku-4-5-20251001`). Fable n'y figure que par ses ids complets (`claude-fable-5-1`, `claude-fable-5`…), mais `--model fable` lance bien le plus récent.
  - `interrupt` (`cancel_queued?: bool`).
  - `set_model` `{model}` · `set_permission_mode` `{mode}` · `apply_flag_settings` `{settings:{effortLevel}}`.
  - `get_usage` `{skip_behaviors:true}` → `rate_limits.five_hour/seven_day.{utilization (0-100), resets_at}`, `subscription_type`, coût de session.
  - `file_suggestions` `{query}` → `{suggestions, cwd}` (autocomplétion `@`).
  - `rename_session`, `get_context_usage`, `cancel_async_message`, `stop_task`…
- Réponse à une requête du CLI :
  `{"type":"control_response","response":{"subtype":"success","request_id":<id du CLI>,"response":<payload>}}`

## CLI → hôte

| Frame | Usage |
|---|---|
| `system/init` | session_id, model, permissionMode, tools, slash_commands, cwd (émis à chaque tour) |
| `system/status` | `status: requesting \| compacting \| null`, `permissionMode` |
| `stream_event` | deltas Anthropic : `message_start`, `content_block_start` (text / thinking / tool_use), `content_block_delta` (`text_delta`, `thinking_delta`, `input_json_delta`), `content_block_stop`, `message_delta` (usage final de l'appel), `message_stop` |
| `assistant` | message complet (un par bloc de contenu), `parent_tool_use_id` non nul = sous-agent |
| `user` | `tool_result` + `tool_use_result` structuré (Edit/Write : `structuredPatch`, `gitDiff{additions,deletions}` ; Bash : `stdout/stderr` ; AskUserQuestion : `answers`) |
| `control_request` `can_use_tool` | permission / question : `tool_name`, `input`, `tool_use_id`, `decision_reason`, `permission_suggestions`, `requires_user_interaction` |
| `rate_limit_event` | `rate_limit_info.unifiedWindows.{five_hour,seven_day}.{utilization (0-1), resetsAt (epoch s)}` |
| `result` | fin de tour : `subtype` (success / error_*), `duration_ms`, `total_cost_usd` (cumulé process), `usage`, `modelUsage{model:{inputTokens,outputTokens,cacheReadInputTokens,cacheCreationInputTokens,costUSD}}` (cumulé process) |

## Répondre à `can_use_tool`

- Autoriser : `{"behavior":"allow","updatedInput":<input>,"toolUseID":<tool_use_id>}` ; « toujours » : ajouter `updatedPermissions` (reprendre `permission_suggestions`).
- Refuser : `{"behavior":"deny","message":"…","toolUseID":…}`.
- **AskUserQuestion** : `allow` avec `updatedInput = {...input, answers: {"<question>": "<label>"}}` (multi-select : labels séparés par des virgules ; réponse libre = texte saisi).

## Coûts et stats

- Coût par tour = delta de `modelUsage[*].costUSD` entre deux `result` du même process (les compteurs repartent de zéro à chaque (re)lancement).
- Tokens par tour = deltas de `inputTokens`, `cacheReadInputTokens + cacheCreationInputTokens`, `outputTokens`.

## Transcripts

`~/.claude/projects/<cwd encodé>/<session_id>.jsonl` — pas de titre généré en mode `-p` ; l'app tient son propre journal de conversation normalisé.

## Avancée : ce que le flux dit

Escouade tient pour chaque agent une avancée (`AgentMeta.plan`, `src-tauri/src/plan.rs`) : sa liste de tâches, ses sous-agents, ses workflows. Elle est dérivée du flux, par un seul réducteur, que la fenêtre ne recalcule pas. Chaque forme ci-dessous est datée : « mesuré » (un vrai `claude`, sans appel à l'API : dossier de config jetable) ou « lu dans le binaire 2.1.296 » (schémas et code de Claude Code, pas de sa documentation : ils peuvent changer). **Rien de ce qui touche aux workflows n'a été vu sur un flux réel** : le support repose sur ces schémas et sur les scénarios du faux `claude` des tests.

### Les outils de liste de tâches ne sont pas toujours là

Deux familles, mesuré sur 2.1.296 avec les options de lancement ci-dessus (la liste `tools` de `system/init`) :

- `TodoWrite` `{ todos: [{ content, status: pending|in_progress|completed, activeForm }] }` : la liste entière à chaque appel, elle remplace la précédente.
- « Task v2 » : `TaskCreate { subject, description, activeForm?, metadata? }`, `TaskUpdate { taskId, status?: pending|in_progress|completed|deleted, subject?, activeForm?, addBlocks?, addBlockedBy?, owner?, metadata? }`, `TaskGet`, `TaskList` ; par défaut à la place de `TodoWrite`.

Avec les modèles actuels (`opus`, `sonnet`, `fable`, `haiku` → ids 5.x ou `claude-haiku-4-5-20251001`), **aucune des deux familles n'est dans `tools`** : un agent d'aujourd'hui n'émet aucune liste de tâches, même quand un skill lui dit d'en tenir une. Elles y sont avec `--model claude-opus-4-6` ou `claude-haiku-4-5` (mesuré), et avec la variable d'environnement `CLAUDE_CODE_ENABLE_TODO_TOOLS=1` (mesuré : `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate` ; avec en plus `CLAUDE_CODE_ENABLE_TASKS=false`, `TodoWrite` à leur place ; variable lue dans le binaire, pas dans la documentation). Escouade la met au lancement de chaque agent (`Settings::agent_env`) tant que le réglage « Les agents tiennent une liste de tâches » est actif (`todoTools`, activé par défaut) ; désactivé, elle n'ajoute rien et ne force jamais la valeur contraire (l'environnement de l'utilisateur reste maître). Les autres lancements de `claude` (nom de l'agent, message de commit, connexion d'un compte) ne l'ont pas. Contrepartie : les outils entrent dans le prompt de chaque agent (quelques centaines de jetons).

### La liste de tâches, comme Claude Code la reconstruit lui-même (lu dans le binaire 2.1.296)

Son observateur interne ne lit que les frames `assistant` du fil principal (`parent_tool_use_id` nul). Escouade reprend ses règles :

- `TodoWrite` remplace la liste (id de chaque tâche = son rang, à partir de 1).
- `TaskCreate` ajoute une tâche en attente tout de suite ; son id n'est connu qu'au `tool_result`, dont le texte est `Task #<id> created successfully: <subject>` (regex `^Task #(\S+) created successfully`). Un `tool_result` en erreur retire la tâche.
- `TaskUpdate` agit par `taskId` : `status`, `subject`, `activeForm`, `addBlockedBy` (les tâches qu'elle attend) ; `addBlocks` dit la même dépendance de l'autre côté ; `deleted` retire la tâche ; un id inconnu crée l'entrée.
- `TaskGet` et `TaskList` ne changent rien.
- Un même appel reçu deux fois (même `tool_use_id`) compte une fois ; un `TaskCreate` quand toutes les tâches sont faites ouvre une nouvelle liste.

### Les sous-agents

L'outil s'appelle `Task` dans la liste `tools` de 2.1.296 et `Agent` ailleurs (dans d'autres versions et dans le faux `claude`) : Escouade prend les deux. Entrée (lu dans le binaire) : `{ description (3-5 mots), prompt, subagent_type?, name?, run_in_background?, model?, isolation? }` ; le `prompt` n'est jamais conservé.

- **Au premier plan** : les frames `assistant` et `stream_event` du sous-agent portent `parent_tool_use_id` = l'id du `tool_use` qui l'a lancé (même pour un sous-agent de sous-agent) ; le `tool_result` du parent porte `tool_use_result` `{ status: "completed", agentId, content, totalToolUseCount, totalDurationMs, totalTokens, usage }` (lu dans le binaire).
- **En arrière-plan** (`run_in_background`) : le `tool_result` est tout de suite « Async agent launched successfully. agentId: … » (`tool_use_result.status` `async_launched`) ; la fin n'arrive que par `system/task_notification`, que Claude Code rejoue ensuite à Claude (voir `on_replay` dans `agent.rs`).

### Frames `system` sur les tâches (lu dans le binaire 2.1.296)

| Frame | Champs utiles | Usage |
|---|---|---|
| `task_started` | `task_id`, `tool_use_id`, `description`, `task_type`, `subagent_type?`, `is_backgrounded?`, `workflow_name?`, `prompt?`, `skip_transcript?`, `ambient?`, `spawn_depth?`, `parent_task_id?` | Une tâche commence, aussi pour un `Bash` en arrière-plan. `task_type` : `local_agent`, `local_bash`, `local_workflow`, `remote_agent`, `in_process_teammate`, `monitor_mcp`, `dream`. Escouade ne compte que `local_agent` (une rangée de sous-agent, reliée à l'appel par `tool_use_id`) et `local_workflow` (une exécution de workflow), jamais une tâche `ambient` ou `skip_transcript`. |
| `task_updated` | `task_id`, `patch { status: pending\|running\|completed\|failed\|killed\|paused, description?, end_time?, error?, is_backgrounded? }` | `completed`, `failed`, `killed` closent la rangée ; `is_backgrounded` la passe en arrière-plan. |
| `task_notification` | `task_id`, `tool_use_id`, `status: completed\|failed\|stopped`, `summary`, `usage? { total_tokens, tool_uses, duration_ms }` | La fin d'une tâche, dans n'importe quel ordre avec le `tool_result` du premier plan : une rangée déjà close ne rouvre pas. |
| `task_progress` | `task_id`, `tool_use_id`, `description`, `usage`, `last_tool_name?`, `summary?` | Un workflow en envoie toujours (`description` = « <titre de phase> : <libellé de l'agent en cours> ») ; un sous-agent seulement avec l'option SDK `agentProgressSummaries`, qui coûte un appel de résumé : Escouade ne la demande pas. |
| `background_tasks_changed` | `tasks: [...]` | Un niveau, pas des bords : ignoré. |

Un `task_progress`, `task_updated` ou `task_notification` d'une tâche qu'aucune rangée ne porte est ignoré (pas de rangée fantôme).

### Les workflows (lu dans le binaire, jamais vu sur un flux réel)

Outil `Workflow` (dans `tools`) ; sa tâche a `task_type: "local_workflow"` et un `workflow_name`. Pendant l'exécution, des `task_progress` ; la fin par `task_notification`. Escouade en fait une exécution (`WorkflowRun`) : son titre, sa phase courante (`now`), ses totaux (`usage`), son état.

### Le plan et le registre superpowers du dossier de l'agent (superpowers 6.4.1)

Pour un agent qui n'a pas de liste de tâches (réglage coupé, ancien flux) mais qui suit un plan superpowers, Escouade lit **en lecture seule** le plan et le registre d'exécution que les skills `writing-plans`, `executing-plans` et `subagent-driven-development` laissent dans son dépôt (`src-tauri/src/planfiles.rs`). Les formes viennent des scripts et des `SKILL.md` de superpowers **6.4.1** (`sdd-workspace`, `task-brief`, `task-start`, `task-done`), pas d'une documentation : une autre version peut les changer.

**Ce qui est lu, et seulement cela.** Sous la racine du dépôt de l'agent (`git rev-parse --show-toplevel` de son dossier, sinon son dossier) :

- `.superpowers/sdd/<nom du plan>/progress.md`, le registre, le plus récent (date de modification) dont la ligne 1 est `# SDD ledger — plan: <chemin du plan>` et dont le plan existe ; les noms des fichiers `task-<id>-brief.md` du même dossier (leurs noms, jamais leur contenu) ; à défaut de chemin utilisable dans la ligne 1 (écrit depuis un autre dossier : `../…`), le fichier `plan-path` du dossier (`sdd-workspace` y écrit le chemin du plan, relatif au dépôt) ;
- le plan, que cette ligne désigne : un fichier `.md` du dépôt (chemin relatif à la racine ou absolu, avec l'une ou l'autre barre) ;
- sans registre, un plan de `docs/superpowers/plans/*.md` que l'agent a écrit ou modifié dans cette conversation (chemins des `tool_use` `Write`, `Edit` et `MultiEdit` du fil principal, cinq au plus).

Jamais un autre fichier, jamais `~/.claude`. Tout chemin passe par `paths::contained` (pas de `..`, pas de lien qui sort du dépôt) ; un dossier de travail, un registre ou un fichier de brief qui est un lien n'est pas suivi. Fichiers lus au plus 256 Kio (au-delà : coupés, jamais un échec ; un registre plus gros donne ses deux bouts, la ligne 1 et les dernières lignes), hors du verrou de l'agent. Un registre ou un plan qu'aucune activité n'a touché depuis le début de la conversation de l'agent (le plus récent de `progress.md` et des briefs) n'est pas le sien : il vient d'une autre conversation ou d'un autre agent du même dépôt.

**Le plan** (`writing-plans`) : le titre est le premier `# ` hors bloc de code, sans le suffixe « Implementation Plan » (ou « plan d'implémentation ») s'il y en a un, coupé à 160 caractères ; une tâche est un en-tête `## Task <id>: <titre>` à `#### …` (`<id>` : `3`, `L1`, `3b` ; séparateur `:`, `.`, `—` ou `-`) hors des blocs de code, la première pour un même id, cent au plus ; ses étapes sont les cases `- [ ]` / `- [x]` de sa section (jusqu'à l'en-tête suivant de son niveau ou au-dessus) : `steps` = (cochées, toutes). Un plan sans en-tête de tâche donne une liste vide. BOM et fins de ligne `\r\n` sont acceptés.

**Le registre** : des lignes `Task <id>: <texte>` ; `complete` en tête du texte (y compris `complete (commits a..b, tests: … → …)`) = faite ; `dispatched`, `implementer`, `review`, `re-review` ou `fix round …` = au travail ; le reste (`Ruling`, `minor (deferred)`, `parked`, lignes libres, tableaux) n'a pas d'effet. De deux lignes sur une même tâche, la dernière qui dit quelque chose l'emporte (un `fix round` après un `complete` la rouvre). Le script `task-brief` (et `task-start`) écrit `task-<id>-brief.md` au début de chaque tâche : une tâche qui a son brief et aucune ligne `complete` est **en cours**. Les ids sont comparés tels quels.

**Précédence.** La liste de tâches de l'agent (`TodoWrite`, `TaskCreate`) l'emporte dès qu'il en a une non vide ; les fichiers donnent alors seulement le titre et le chemin du plan (`title`, `planFile`) et la fenêtre ne reçoit qu'une liste. Sans liste, c'est celle des fichiers (`source: "plan"`). Une tâche donnée faite par le registre ne repasse jamais à « à faire ». Quand l'agent se met à tenir sa propre liste, celle des fichiers lui laisse la place. Une lecture qui échoue, ou qui ne trouve rien, ne change rien et ne dit rien (jamais un message dans la conversation).

**Quand relire.** Au `tool_result` d'un `Bash`, d'un sous-agent (`Task` / `Agent`) ou de l'écriture d'un plan par le fil principal, et à la fin de chaque tour ; au plus une fois toutes les 2 s par agent (une demande reçue pendant ce délai donne une relecture au bout), et les fichiers ne sont relus que si la date ou la taille de `progress.md` ou du plan, ou la liste des briefs, a changé.

### Ce qu'Escouade en garde

`AgentMeta.plan` (`PlanState`, voir `src/lib/types.ts`) : `tasks` (100 au plus), `agents` (30 : toutes celles qui tournent, puis les dernières terminées), `launched` (tous les sous-agents lancés depuis la remise à zéro), `workflows` (10). Le sous-agent d'une tâche (`planTask`) est celle que son `prompt` ou sa `description` nomme (`task-<id>-brief.md` ou `task-<id>-report.md`, puis `Task <id>`) si la liste l'a, sinon la seule tâche en cours. Ce que fait un sous-agent (`doing`) vient de ses propres appels d'outils, dans les mêmes mots que `activity`. Remise à zéro : une nouvelle session Claude, ou le premier message d'un tour quand le plan est fini (toutes les tâches faites, ou aucune, et rien qui tourne) ; elle oublie aussi le titre et le plan des fichiers, et les fichiers du dossier ne sont relus que s'ils ont été touchés depuis. Quand le processus meurt, les rangées qui tournaient passent à `interrupted` ; une rangée au premier plan l'est aussi à la fin du tour.

## Serveur MCP d'Escouade

Escouade sert ses outils à Claude Code (un terminal, un autre outil, ses propres agents) en MCP « Streamable HTTP », par le SDK officiel `rmcp` (3.5) derrière `hyper` (`src-tauri/src/mcp/`). Il tourne quand « Claude peut piloter Escouade » est activé ou qu'un projet laisse ses agents l'utiliser, et s'arrête avec l'app.

### Transport et garde-fous

- `http://127.0.0.1:<port>/mcp`, sur la boucle locale seulement. Port choisi au premier démarrage entre 47000 et 47999, puis gardé (`Settings.mcpPort`) ; s'il est pris, un autre est choisi et enregistré.
- Sans état (pas de `Mcp-Session-Id`), réponses en JSON (pas de flux SSE). Les deux poignées de main passent : `initialize` en `2025-11-25`, et le chemin `2026-07-28` (sonde `server/discover`, puis chaque requête porte sa version dans `_meta` et l'en-tête `MCP-Protocol-Version`, sans `initialize`), celui que prend Claude Code 2.1.296.
- Chaque requête, avant `rmcp` :
  - `Host` (ou l'autorité d'une URI absolue) autre que `127.0.0.1:<port>` ou `localhost:<port>`, ou absent → 403 ;
  - un en-tête `Origin` (une page web) → 403 ;
  - `Authorization: Bearer <jeton>` absent ou inconnu → 401 avec `WWW-Authenticate: Bearer` (comparaison à temps constant) ;
  - un autre chemin que `/mcp` → 404.
- Jetons : celui de Claude hors Escouade (32 octets aléatoires en base64url, dans le trousseau, entrée `mcp`, ou dans `mcp-token.json`, lisible par son seul propriétaire, quand le trousseau refuse) ; celui de chaque process d'un agent d'Escouade, en mémoire et dans le fichier de config que ce process reçoit (voir « Les agents d'Escouade »). Le jeton dit qui appelle : un outil le sait (`Caller`).
- Une connexion a 10 s pour envoyer les en-têtes d'une requête (et reste 10 s au plus sans requête) ; 64 connexions à la fois au plus, les suivantes attendent.
- Journal d'activité : chaque appel d'outil (refus et erreurs compris, y compris un outil inconnu ou des arguments qui ne collent pas au schéma) et chaque requête refusée, sans jamais les jetons ; les 200 derniers, en mémoire.

### Les agents d'Escouade

Ce que reçoit chaque lancement du process `claude` d'un agent (`Core::agent_access`, à la fin des arguments : les deux options prennent plusieurs valeurs) :

| Serveur | Projet (« Les agents peuvent utiliser Escouade ») | Arguments |
|---|---|---|
| arrêté (ou pas encore démarré) | activé | rien (il n'y a rien à joindre) |
| en marche | activé | `--mcp-config <dossier de données>/mcp/<id de l'agent>.json` |
| arrêté ou en marche | désactivé (défaut) | `--disallowedTools mcp__escouade` |

Le projet qui ne laisse pas ses agents utiliser Escouade refuse donc leurs outils même quand le serveur est arrêté : une entrée `escouade` peut rester déclarée dans la config de Claude Code de l'utilisateur (elle l'était pendant que Claude pilotait Escouade), que l'agent hérite.

- Le fichier : `{ "mcpServers": { "escouade": { "type": "http", "url": "http://127.0.0.1:<port>/mcp", "headers": { "Authorization": "Bearer <jeton du process>" } } } }`, écrit par `paths::write_private` (0600 sous Unix). Un chemin et non du JSON en ligne : un lanceur `.cmd` prend 8 191 caractères au plus, et le journal de l'app écrit les arguments (le jeton n'y est jamais).
- Le jeton : nouveau à chaque lancement (celui d'avant est refusé dès lors) ; refusé et le fichier supprimé quand ce process se termine (arrêt pour inactivité, archivage, plantage, `kill`), sauf si un process plus récent du même agent a déjà le sien ; tout de suite à la suppression de l'agent ou à la fermeture de son projet (l'une comme l'autre attendent la fin d'un démarrage de son process en cours, un échauffement par exemple, pour ne pas laisser un process avec un jeton valable derrière elles) ; le dossier `mcp/` est vidé quand l'app s'arrête et quand elle démarre (avant tout agent : un plantage l'a peut-être laissé).
- Un serveur `escouade` passé par `--mcp-config` (source « dynamic ») remplace celui du même nom de la portée `user` : seul le premier est contacté, avec son en-tête.
- Essai du 2026-10-10, Claude Code 2.1.296, `CLAUDE_CONFIG_DIR` et dossier de travail jetables, le serveur d'Escouade en marche et `escouade` déclaré en portée `user` (`claude mcp add --scope user --transport http escouade http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <jeton externe>"`), puis `claude -p ok --output-format stream-json --verbose …` (sans connexion : le `system/init` vient avant « Not logged in ») :
  - sans option : `mcp_servers` `escouade=connected`, 31 outils dont les 7 `mcp__escouade__*` (les six de lecture et `whoami`, l'outil des tests : l'essai tournait sur un serveur de test) ;
  - `--disallowedTools mcp__escouade` : `escouade=connected` toujours (Claude Code se connecte et liste les outils), mais 24 outils, aucun `mcp__escouade__*` ;
  - `--mcp-config <fichier d'un agent>` : `escouade=connected`, les 7 outils ; avec `--disallowedTools mcp__escouade` en plus : aucun ;
  - `--mcp-config` d'un fichier au jeton inconnu : `escouade=failed`, aucun outil, et le journal d'activité montre deux « Requête refusée : jeton inconnu » : l'entrée `user` (au jeton valable) n'a pas été essayée.

### Réponses et erreurs

- Un outil répond un seul bloc `text` : du JSON, clés en camelCase, dates en millisecondes depuis l'epoch.
- Un refus ou un échec est un résultat d'erreur que le modèle lit (`isError: true`, le texte dit pourquoi, dans la langue de l'interface). Des arguments qui ne collent pas au schéma donnent aussi un résultat d'erreur (texte de `rmcp`, « failed to deserialize parameters: … ») ; un outil inconnu, une erreur JSON-RPC.
- Résolution des noms : un projet par son id ou son nom (sans casse) ; un agent par son id (archivés compris) ou son nom (sans casse, parmi les agents non archivés, sinon parmi les archivés), dans le projet donné s'il y en a un ; un ticket par son id ou sa clé (sans casse). Un nom inconnu ou ambigu est une erreur qui le dit et liste 5 choix au plus (ceux dont le nom contient le texte demandé d'abord, « … » s'il y en a d'autres ; quand tous les agents en jeu sont archivés, l'erreur le dit et les propose), par exemple « Plusieurs projets s’appellent « demo » : demo (id 1a2b), demo (id 3c4d). Donne son id. ».

### Outils de lecture

Tous annotés `readOnlyHint: true`, `openWorldHint: false`.

| Outil | Arguments | Réponse |
|---|---|---|
| `list_projects` | — | `[{ id, name, path, branch, agents, tickets: { todo, doing, review, done } }]`, dans l'ordre de l'app. `branch` : la branche courante, `null` hors dépôt ou sur une HEAD détachée ; `agents` : le nombre d'agents non archivés. |
| `list_agents` | `project?` | `[{ id, name, project, projectId, status, waiting, ticket, model, account, cost }]` : les agents non archivés, du plus ancien au plus récent, de tous les projets ou d'un seul. `project` : le nom du projet, `projectId` son id (deux projets peuvent porter le même nom) ; `status` : `idle`, `running`, `waiting`, `done` ou `error` ; `waiting` : une question ou une permission attend la réponse de l'utilisateur (une question posée en texte, elle, est dans le dernier message : `get_agent_summary`) ; `ticket` : la clé, `null` sans ticket ; `account` : le nom du compte Claude ; `cost` : en dollars, tour en cours compris. |
| `list_tickets` | `project`, `column?` (`todo`, `doing`, `review`, `done`) | `[{ id, key, title, column, after, agent, blocked }]` dans l'ordre du Kanban : les colonnes de « À faire » à « Terminé », « À faire » par priorité, « En cours » et « À tester » par arrivée, « Terminé » du plus récent au plus ancien. `after` : les clés des tickets dont il dépend ; `agent` : le nom de son agent ; `blocked` : pourquoi il n'avance plus seul, sinon `null`. |
| `get_ticket` | `ticket?` | `{ id, key, title, description, criteria: [{ text, ok, note }], loops: { iteration, max }, column, after, progress, external: { service, key, url, error } \| null }`. Sans `ticket`, un agent d'Escouade lit le sien ; un agent sans ticket reçoit une erreur, Claude hors Escouade un refus. `progress` : les fonctionnalités en place, telles que l'agent les a listées ; `external` : le ticket Jira, Trello ou GitHub d'origine (`error` : l'échec de sa dernière synchro). |
| `get_usage` | — | `{ accounts: [{ name, current, fiveHour: { pct, resetsAt } \| null, sevenDay: … }], autopilotPause: { reason, until } \| null }`. `current` : le compte où partiraient les nouveaux agents, celui de la barre d'état (le premier actif dont les fenêtres sont sous le seuil du pilote auto, sinon le premier actif) ; `fiveHour`, `sevenDay` : les fenêtres de ce compte, telles que lues la dernière fois, `null` si elles ne l'ont pas été ; `pct` de 0 à 100 ; `reason` : `fiveHour`, `week` ou `limit`. |
| `get_agent_summary` | `agent`, `project?` | `{ id, name, status, ticket, lastMessage, touchedFiles }`, jamais la conversation. `ticket` : la clé, `null` sans ticket. `lastMessage` : la dernière réponse de l'agent (fil principal, pas un sous-agent), `null` avant la première ; entière jusqu'à 2 000 caractères, au-delà son début (500 caractères au plus) et sa fin (le reste, là où l'agent dit ce qu'il a conclu ou ce qu'il demande), chacun coupé sur un mot entier (dans un mot seulement s'il est très long), joints par « … » ; 2 000 caractères au plus en tout. `touchedFiles` : les 100 derniers fichiers qu'il a commencé à modifier, relatifs à son dossier. |


### Outils qui agissent

Annotés `readOnlyHint: false`, `openWorldHint: false` (`stop_agent` : `destructiveHint: true`, les autres `false`). Chacun passe par la même fonction du cœur que la fenêtre : la fenêtre en est avertie par les mêmes événements, la synchro Jira / Trello / GitHub suit, le Kanban est replanifié. Un ticket se nomme par sa clé (sans casse) ou son id ; ceux d'`after` et de `before` sont cherchés dans le projet du ticket. Textes limités : titre 200 caractères sur une ligne, description 10 000, 30 critères de 500 au plus (un critère par ligne, les lignes vides sautent ; sans critère, les deux par défaut).

| Outil | Arguments | Réponse |
|---|---|---|
| `create_ticket` | `project`, `title`, `description?`, `criteria?: string[]`, `after?: string[]` (clés) | le ticket comme `get_ticket` : à la fin de « À faire » ; avec le pilote auto, il part de lui-même dès qu'une place est libre et que ceux dont il dépend sont terminés. |
| `update_ticket` | `ticket`, `title?`, `description?`, `criteria?`, `after?` | le ticket comme `get_ticket`. Seuls les champs donnés changent (`criteria` et `after` remplacent la liste entière : `after: []` retire les dépendances). Rien de donné : erreur. |
| `move_ticket` | `ticket`, `position: "top" \| "bottom" \| "before"`, `before?` (clé, avec `before` seulement) | `{ ticket, todo: [clés de « À faire » dans le nouvel ordre] }`. Les rangs ne changent que pour ce ticket, sauf quand deux voisins n'ont plus de rang entre eux (la colonne est renumérotée). Déjà à sa place : rien ne change. |
| `start_ticket` | `ticket` | `{ key, starting: true }` : le ticket est lancé à la main comme par « Lancer », le Kanban le démarre dès qu'il peut (il passe « En cours » en quelques secondes : `get_ticket`). |
| `create_agent` | `project`, `message`, `worktree?: bool`, `model?` | `{ id, name }`. L'agent est créé (sans changer l'agent sélectionné de la barre latérale) puis reçoit `message`, **sous le nom de son auteur** (voir plus bas). L'appel ne répond **qu'une fois ce premier message remis** : il attend donc la préparation du worktree de l'agent (les commandes de préparation du projet) et le démarrage de son process, et tient entre-temps un verrou (un seul `create_agent` à la fois, pour que deux appels simultanés ne trouvent pas la même place libre). `name` est celui du moment (Haiku le renomme d'après le texte du message, sans l'en-tête). `worktree` : `true` un worktree à lui, `false` le dossier du projet, absent comme le réglage du projet. `model` : un nom que Claude Code propose (alias ou modèle résolu, tels que la fenêtre les liste quand ils sont connus ; sinon seulement une forme sûre : lettres, chiffres et `._-:[]/`). Le compte est celui par défaut. |
| `send_message` | `agent`, `project?`, `text` | `{ id, name, queued }` : `queued` est vrai quand l'agent travaille (le message est lu après son tour). Le message est remis **sous le nom de son auteur** (voir plus bas). |
| `stop_agent` | `agent`, `project?` | `{ id, name, interrupted }` : `interrupted` est vrai quand un tour tournait (l'agent reste, on peut lui écrire après). |

Garde-fous, ceux de la fenêtre et du pilote auto ; un refus est un résultat d'erreur qui dit pourquoi, noté « refusé » dans le journal d'activité :

- **Pause du pilote auto, compte par compte** (un compte est retenu par une fenêtre de quota au-delà de « Pause au-delà du quota », par la pause après une limite d'usage, ou par un agent à lui qui attend son quota) : `start_ticket` et `create_agent` ne sont refusés que si aucun compte n'est utilisable pour le projet (son « Compte préféré », sinon tous les comptes actifs : un seul libre suffit, comme pour le pilote auto) ; `send_message` est refusé quand le compte de l'agent destinataire est retenu, pas quand un autre l'est. Le message dit jusqu'à quand (« Le pilote auto est en pause jusqu'à 14:30 : … », « 2026-10-12 14:30 » un autre jour), celui du premier compte qui sera libre, et, avec plusieurs comptes, de quel compte il s'agit pour `send_message`. Rien n'est modifié (le ticket n'est pas marqué lancé).
- **Maximum en parallèle du projet** (« En parallèle », 1 à 6, 2 par défaut) : `start_ticket` est refusé quand autant de tickets sont « En cours » sans être bloqués, `create_agent` quand autant d'agents non archivés du projet travaillent (en cours ou en attente d'une réponse). Deux `create_agent` simultanés ne trouvent pas la même place libre.
- **Un ticket qui attend d'autres tickets** (non « Terminé ») n'est pas lancé : la fenêtre demande d'abord à l'utilisateur de confirmer, ici le refus nomme ces tickets ; `update_ticket` avec `after` leur retire la dépendance. Un Kanban qui ne démarre rien (branche cible introuvable…) dit pourquoi.
- **Un ticket qui a démarré** : ni modifié (`update_ticket`), ni déplacé ni placé avant (`move_ticket`), ni relancé (`start_ticket`, « Ce ticket est déjà parti. »). Un `after` qui ferait attendre deux tickets l'un l'autre est refusé avec le message du Kanban (« DEM-2 attend déjà DEM-1 (directement ou non). »).
- **Un agent ne s'écrit pas et ne s'arrête pas lui-même** (`send_message`, `stop_agent`) ; un agent archivé ne reçoit pas de message.
- **Un agent d'Escouade n'agit que dans son propre projet** : `create_ticket`, `update_ticket`, `move_ticket`, `start_ticket`, `create_agent`, `send_message`, `stop_agent` et `split_ticket` visant un autre projet (ou un ticket, un agent d'un autre projet) sont refusés (« L'agent X n'agit que dans son propre projet (demo) : le projet autre n'est pas le sien. »). Les outils de lecture restent ouverts à tous les projets. Claude hors Escouade, lui, agit sur n'importe quel projet.

**L'auteur d'un message est toujours dit.** Un texte qui vient du serveur n'est pas pris pour les mots de l'utilisateur : `send_message`, et le premier message de `create_agent`, sont remis à l'agent, et montrés dans sa conversation, précédés de « Message de <auteur> : » (« Message from <author>: » en anglais, dans la langue de l'interface), l'auteur étant le nom de l'agent qui appelle ou « Claude (hors Escouade) » (« Claude (outside Escouade) ») pour Claude hors Escouade, comme le journal d'activité le nomme. L'en-tête ne compte pas pour le nom que Haiku donne à un agent d'après son premier message, et un texte qui commence par `/` n'est plus une commande (il suit l'en-tête).

Ce qui ne colle pas à ce qui est demandé (titre vide, nom inconnu ou ambigu, modèle inconnu, `before` sans `position: "before"`) est une erreur, pas un refus. Un `create_agent` dont le message n'a pas pu partir dit que l'agent existe (son id) mais pas son message.

### Outils réservés aux agents d'Escouade

`report_progress` et `split_ticket` sont listés pour tous les clients, mais un client qui n'est pas un agent d'Escouade (Claude hors Escouade) reçoit un refus qui le dit : « Cet outil est réservé aux agents d'Escouade : Claude hors Escouade ne peut pas l'appeler. » Il est noté « refusé » dans le journal.

| Outil | Arguments | Réponse |
|---|---|---|
| `report_progress` | `line` | `{ line }` telle que gardée. Une ligne d'état de 120 caractères au plus (plus long : erreur qui donne le compte, rien n'est gardé), sur une seule ligne (sauts de ligne et espaces multiples deviennent une espace). Rien de caché n'est gardé : les caractères de commande, de format (marques de largeur nulle, marques et forçages de direction comme U+202E…), les séparateurs de ligne et de paragraphe et ceux qui ne se dessinent pas (sélecteurs de variante, caractères d'étiquette…) sont retirés, comme la fenêtre les épelle dans une demande d'autorisation ; la ligne répondue est celle gardée (`null` si rien ne reste). Gardée dans `AgentMeta.progress_line` (enregistrée avec l'agent) et envoyée à la fenêtre : la carte de l'agent (barre latérale, vue d'ensemble) et celle de son ticket « En cours » la montrent jusqu'au prochain appel. Une ligne vide l'enlève (`{ line: null }`). |
| `split_ticket` | `tickets: [{ title, description?, criteria? }]` (1 à 10), `chain?: bool` (vrai par défaut) | les clés des tickets créés, dans l'ordre : `["DEM-5", "DEM-6"]`. Ils sont créés en fin de « À faire », **après le ticket de l'agent** : le premier lui, chacun après le précédent avec `chain`, tous après lui seul sans (ils peuvent alors tourner en parallèle). Tout est vérifié avant de créer le premier ; refusé à un agent sans ticket. Le ticket de l'agent n'est pas touché. |
