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
- Le jeton : nouveau à chaque lancement (celui d'avant est refusé dès lors) ; refusé et le fichier supprimé quand ce process se termine (arrêt pour inactivité, archivage, plantage, `kill`), sauf si un process plus récent du même agent a déjà le sien ; tout de suite à la suppression de l'agent ou à la fermeture de son projet ; le dossier `mcp/` est vidé quand l'app s'arrête et quand elle démarre (avant tout agent : un plantage l'a peut-être laissé).
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
