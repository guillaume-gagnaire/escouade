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
