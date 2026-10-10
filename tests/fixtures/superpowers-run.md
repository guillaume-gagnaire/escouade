# Le déroulé du faux `claude` pour « plan superpowers »

Ce que `tests/fixtures/fake-claude.mjs` joue quand le message contient « plan superpowers », et ce que l’application doit en tirer. Les tests qui s’appuient dessus : `src-tauri/src/core_tests.rs` (`a_superpowers_run_…`, `a_turn_cut_in_the_middle_of_a_subagent_…`) et `tests/e2e/plan.spec.ts` (+ `english.spec.ts`).

Trois variantes, choisies par le texte du message :

| Message contient | Effet |
|---|---|
| `plan superpowers` | le déroulé complet, avec une liste de tâches (`TaskCreate` / `TaskUpdate`) |
| `plan superpowers sans liste` | le même déroulé sans aucun outil de liste : l’avancée ne vient que du plan, du registre et des appels `Agent` |
| `interrompu` (avec l’un des deux) | le tour est coupé au milieu du premier implémenteur : un `result` en erreur, sans `tool_result` |

Le faux marque une pause de 25 ms entre deux frames (l’application les voit une à une). Une interruption (`interrupt`) arrête le déroulé. Le journal de lancement du faux garde `todoTools` (la valeur de `CLAUDE_CODE_ENABLE_TODO_TOOLS`, `null` sinon).

## Ce qui est écrit pour de vrai dans le dossier de l’agent

- `docs/superpowers/plans/2026-10-10-demo.md` : le H1 `# Démo Implementation Plan`, trois `### Task N: <titre>` (« Écrire la fonction », « Écrire les tests », « Brancher dans l’interface »), chacun avec deux cases `- [ ]`.
- `.superpowers/sdd/2026-10-10-demo/plan-path` : `docs/superpowers/plans/2026-10-10-demo.md`.
- `.superpowers/sdd/2026-10-10-demo/progress.md` : la ligne 1 `# SDD ledger — plan: docs/superpowers/plans/2026-10-10-demo.md`, puis, à la fin de chaque tâche, `Task N: complete (commits a1b2c3N..d4e5f6N, tests: npm test → 12 passed, review clean)`.
- `.superpowers/sdd/2026-10-10-demo/task-N-brief.md` : écrit au début de la tâche N.

## Les frames, dans l’ordre

1. Un texte (« J’écris le plan… »), puis `Write` du plan, `Bash sdd-workspace …` (le plan et le registre existent alors).
2. Avec liste : `TaskCreate` ×3, avec leurs `tool_result` « Task #N created successfully: <titre> ».
3. Pour chaque tâche N :
   - avec liste : `TaskUpdate { taskId, status: "in_progress" }` ;
   - `task-N-brief.md` écrit, puis `Bash task-start …` ;
   - **tâche 3 seulement** : une `AskUserQuestion` (« Quelle base de données ? ») : le tour attend la réponse, la tâche est « bloquée » ;
   - un `Agent` « Implémenter la tâche N » au premier plan (`prompt` : `Lis .superpowers/sdd/2026-10-10-demo/task-N-brief.md…`), puis ses trois appels internes (`Read`, `Edit`, `Bash`, avec `parent_tool_use_id` = l’id de l’appel `Agent`) et leurs `tool_result`, puis le `tool_result` de l’`Agent` avec `tool_use_result { status: "completed", agentId, content, totalToolUseCount: 3, totalDurationMs, totalTokens }` ;
   - un `Agent` « Relire la tâche N » de la même forme, avec deux appels internes ;
   - **tâche 2 seulement** : un `Agent` « Surveiller la suite de tests » lancé comme Claude Code 2.1.296 lance tous les siens (mesuré : pas de `run_in_background` dans l'appel, `system/task_started` avec `run_id`, `task_type: "local_agent"`, `is_backgrounded: true`, puis `tool_result` « Async agent launched successfully. agentId: … » et `tool_use_result { isAsync, status: "async_launched", agentId, resolvedModel, outputFile, … }`, un `task_progress` pendant qu'il travaille), puis un `Workflow` : `system/task_started` `task_type: "local_workflow"` avec `workflow_name`, deux `task_progress` (« Relecture : sécurité », « Relecture : style »), `task_notification`, `tool_result` ;
   - le registre reçoit `Task N: complete (…)`, `Bash task-done …` ;
   - avec liste : `TaskUpdate { taskId, status: "completed" }`.
4. Comme les skills terminent un plan : le dossier de travail supprimé pour de vrai (`Bash rm -rf .superpowers/sdd/2026-10-10-demo`), puis un texte « Plan terminé » et le `result`. 400 ms plus tard, après la fin du tour, `system/task_updated` puis `system/task_notification` du sous-agent en arrière-plan.

## Ce que l’application doit en dire

| Moment | `view().plan` |
|---|---|
| après les `TaskCreate` | trois tâches `pending`, `source: "tools"` |
| tâche 1 commencée | la première `inProgress` |
| implémenteur de la tâche 1 au travail | une rangée `running`, `planTask: "1"`, `doing` = ce que fait son dernier outil (« Lit src/slugify.ts »…) |
| tâche 1 finie | `done` |
| la question de la tâche 3 | deux `done`, la troisième `inProgress`, l’agent `waiting` (la fenêtre dit « Bloqué » / « En attente de ta réponse ») ; 5 sous-agents lancés, un seul tourne (celui d’arrière-plan) |
| dossier de travail supprimé | un plan de fichiers est fait d'un coup (la dernière ligne du registre n'a pas toujours pu être lue) ; une liste de l'agent reste la sienne |
| fin du tour | trois `done` ; `launched == 7` (3 implémenteurs, 3 relecteurs, 1 arrière-plan) ; l’arrière-plan tourne encore ; un workflow `done` (`revue-finale`, 5 outils, 640 jetons) |
| après la `task_notification` | plus rien ne tourne |
| sans liste | `source: "plan"`, titre « Démo », les mêmes trois tâches (avec leurs étapes `0/2`) : les deux premières `done` d’après le registre, la troisième `inProgress` d’après son brief ; le même `launched` |
| interrompu | une seule rangée, « Implémenter la tâche 1 », `interrupted` ; la première tâche reste `inProgress` |

## Les enregistrements de vrais flux

`tests/fixtures/real-run-subagents.jsonl` (un plan de deux tâches exécuté avec `subagent-driven-development`, réglage de la liste de tâches actif, puis un message qui demande `TaskCreate` / `TaskUpdate`) et `real-run-inline.jsonl` (le même plan avec `executing-plans`, réglage coupé) sont les frames que Claude Code 2.1.296 a écrites sur sa sortie standard lors des essais réels du 2026-10-11, sans les événements de flux, les chaînes longues coupées à 300 caractères et le nom de l'utilisateur remplacé. Les tests de `agent.rs` (`the_frames_of_a_real_…`) les rejouent frame par frame. Ce que ces essais ont montré est dans `docs/PROTOCOL.md` (« Ce que les flux réels ont montré »).
