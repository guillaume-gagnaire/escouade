# Les plugins d'Escouade : analyse (ESC-170)

Demandé par l'utilisateur le 2026-10-25 : « pour la v2.1.0, planifie pour plus tard un système de plugins, permettant à tout le monde d'apporter des fonctionnalités à Escouade, fais une analyse poussée pour voir tout ce qu'on pourrait manipuler via ces plugins, et planifie une documentation étendue sur le sujet ».

Ce document est l'analyse. Il ne change pas l'app. Le découpage en tickets, le plan de la documentation et les questions qui décident sont dans `docs/TODO-2.2.md`, chapitre « Plugins ».

- §1 : l'inventaire de ce qu'un plugin pourrait manipuler, point d'extension par point d'extension, depuis le code ;
- §2 : l'architecture, comparée sur ce que font VS Code, Zed, Obsidian, Raycast, JetBrains, Claude Code, Tauri et MCP, et trois expériences jetables ;
- §3 : le modèle de confiance et de distribution ;
- §4 : tout ce qui est externe devient un plugin (ESC-203, 2026-10-26) : les harness, les sources du Kanban, les hébergeurs et leurs PR, en plugins de première main, tous en process ; un plugin qui tient plusieurs points (GitHub : tickets et PR) ; les options comparées et ce que coûte le « tout en process » décidé, l'ordre de migration, l'effet sur les tickets en cours.
- §5 : les programmes en JS/TS sans Node (ESC-319, 2026-11-01) : Javy, Extism et componentize-js comparés et mesurés, le runtime WASM retenu et son prototype, le contrat de l'hôte, le pipeline du store, le sort du `run` natif, la migration de Slack et de Règles de l'équipe.

**Depuis ESC-203, la règle est : tout ce qui est externe est un plugin.** Les intégrations d'aujourd'hui deviennent des plugins de première main, livrés et signés avec l'app, et, comme les plugins tiers, des process qui parlent le même protocole (§4, décidé le 2026-10-26).

## En bref

- **Un plugin est un dossier avec un manifeste** (`escouade-plugin.json`). Il apporte des **contributions déclarées** : un thème, une langue, un harness ACP, un webhook, des modèles de projet, des entrées de menu… Escouade les lit et les montre avec ses propres composants. Il peut aussi apporter un **programme**, qui parle à Escouade en **JSON-RPC 2.0 sur stdio**, comme un serveur MCP ou un agent ACP : il réagit aux événements, décide d'une autorisation, fournit une source du Kanban.
- **Pas de HTML libre dans la fenêtre en 2.1** : l'interface d'un plugin est déclarée (manifeste et blocs), rendue par Escouade. Une vue libre viendra plus tard, dans une iframe isolée (le modèle de MCP Apps).
- **Les programmes en JS/TS tournent en WASM, sans Node** (révisé le 2026-11-01, ESC-319, §5) : le store compile le bundle commité en WASM (Javy et le QuickJS d'Escouade, reproductible), l'app le fait tourner par wasmtime, dans son process. Les mêmes messages, un autre transport ; le réseau et les fichiers y sont appliqués, plus seulement déclarés. Les programmes natifs restent pour la première main et pour les binaires tiers attestés.
- **La 2.1.0** : le socle (manifeste, chargement, Réglages › Plugins, permissions, journal, faux Escouade pour les tests), puis trois points : les **événements** (webhooks, Slack, Discord, commandes), les **harness ACP déclarés**, les **thèmes**. Les **règles d'autorisation** suivent, si la 2.1.0 a la place. Les sources du Kanban, les hébergeurs, les outils MCP, l'interface et les langues viennent après.
- **La place de marché store.escouade.dev arrive avec la 2.1** (ESC-202, `docs/STORE.md`) : soumission depuis un dépôt public, comptes GitHub, revue en local par l'utilisateur et badge « Vérifié par Escouade » signé hors du serveur. L'installation depuis un dossier ou un dépôt git reste permise, « non vérifié ».

## 1. L'inventaire

Chaque point d'extension dit : **où** il vit aujourd'hui, **ce qu'un plugin y ferait**, **un exemple**, puis sa **valeur** pour un utilisateur, sa **difficulté** de construction et son **risque** (sécurité, stabilité, compatibilité). Valeur, difficulté et risque sont notés de 1 (faible) à 3 (fort). Le tableau du §1.13 les résume.

Ce qui rend un point facile, c'est un **trait ou un registre déjà là**, derrière lequel le cœur ne sait pas qui répond : les harness (`Engine`, `EngineProcess`, `registry`), les hébergeurs (`Forge`, `Provider`, `forge::Registry`) et le coffre (`SecretStore`) en ont un. Les sources du Kanban (`integrations::Client`, une énumération fermée), les services de voix (`voice::online::Provider`, une énumération fermée) et les langues (`i18n::Lang`, `Fr` et `En`) n'en ont pas : il faudra d'abord les ouvrir.

### 1.1 Les harness

**Où.** `src-tauri/src/engine/` : les traits `Engine` (sans état : `locate`, `tested`, `caps`, `models`, `efforts`, `spawn`, `message`, `translate`, `one_shot`, `grant_file`, `escouade_launch`, `fence_settings`, `user_config`…) et `EngineProcess` (le process vivant : `send_message`, `interrupt`, `set_model`, `set_mode`, `answer_permission`, `answer_question`, `read_usage`…), dans `engine/mod.rs` ; le registre `engine/registry.rs` (`EngineCaps`, `EngineDecl`, `Tested`, `Version`, `Range`, `EngineState`) ; un dossier par harness (`claude/`, `codex/`, `copilot/`, `cursor/`, `opencode/`, `vibe/`, `agy/`, `gemini/`, chacun avec `locate.rs`, `process.rs`, `bench.rs`, et `terminal.rs` pour une vue terminal) ; le code propre à un harness hors d'`engine/` (`signin.rs`, `usage.rs`, `pricing.rs`, `codeindex/hint.rs`, `memory.rs::claude_offer`) ; la traduction de leurs frames en conversation, `src-tauri/src/agent/<harness>.rs` ; le banc de conformité `engine/bench/` (scénarios, `replay`), et `engine/minimal/`, un faux moteur qui montre le moins qu'un moteur écrit. Le guide « Ajouter un moteur » de `docs/ENGINES.md` liste une quarantaine de cases.

**Ce qu'un plugin y ferait.** Deux niveaux :

1. **Un harness ACP décrit** : Cursor CLI parle déjà ACP v1 à un client d'Escouade (`engine/cursor/process.rs`, `agent/cursor.rs`). Généralisé en un **adaptateur ACP générique**, un plugin n'a plus qu'à _décrire_ son agent dans son manifeste : son programme et ses arguments (`agent acp`), comment le trouver (`locate` : noms, chemins, `--version`), ses versions testées, la variable de son dossier de config (`agent_home_var`), ses modes et leur sens (`ModeCap`), ses modèles, ce qu'il sait faire (`EngineCaps` : images, PDF, copie, message en cours de tour…), sa commande d'installation, ses textes (`speaker`, `writer`). Pas de code : Escouade fait le reste, banc compris. Ce que l'ACP ne dit pas (le quota, le coût, les comptes) reste « inconnu » (`QuotaKind`, `CostKind`), comme chez Cursor.
2. **Un harness en programme** : un plugin qui implémente `Engine` et `EngineProcess` lui-même, par le protocole du plugin (`engine/spawn`, `engine/translate`…). Pour un harness qui ne parle ni ACP ni rien de connu. La traduction doit produire les éléments normalisés (`user`, `text`, `thinking`, `tool`, `question`, `permission`, `turn`, `notice`, `event`) et les noms d'outils de Claude Code (`Bash`, `Edit`…), ce que le banc vérifie.

**Exemple.** « Gemini CLI » ou « Goose », qui parlent ACP : un manifeste de 40 lignes, sans code, et l'agent apparaît dans le menu « Harness » de la barre latérale, avec ses modèles, ses modes et sa vue de conversation.

**Valeur 3.** Chaque nouveau harness a coûté plusieurs tickets (Cursor : ESC-24, ESC-42, ESC-43, ESC-49). Les agents ACP se multiplient : la communauté suivrait leur rythme. **Difficulté 2** pour le niveau 1 (le client ACP existe ; il faut en retirer ce qui est propre à Cursor, `cursor/*`, et faire du registre une liste ouverte : `EngineId` est aujourd'hui une énumération de `model.rs`), **3** pour le niveau 2 (la traduction touche l'état de l'agent de près, `agent/<harness>.rs`). **Risque 3** : un harness lance un programme avec les droits de l'utilisateur, reçoit le jeton du serveur MCP d'Escouade (`grant_file`, jamais dans les arguments) et doit appliquer la clôture des projets privés (`fence_settings`) ; un harness qui l'ignore ouvre les projets privés. L'adaptateur ACP générique doit donc tenir la clôture lui-même (ce qu'ACP permet : les lectures et écritures de fichiers passent par le client, `fs/read_text_file`), et le dire quand l'agent ne passe pas par lui.

**La vue terminal** (`engine/terminal.rs`, `TerminalView`, `escouade-hook`) reste hors des plugins en 2.1 : chaque vue a demandé ses hooks, ses bandes de confiance et ses pièges (ESC-46 à 49).

### 1.2 Les sources du Kanban

**Où.** `src-tauri/src/integrations/` : `mod.rs` (`Client`, une énumération `Jira`, `Trello`, `Github`, `Linear` qui distribue chaque appel par `match` : `me`, `containers`, `states`, `issues`…, avec `Container`, `ExternalIssue`, `IssuePage`, `IssueFilter`, `Query`, `Bases`), un fichier par service, `sync.rs` (la synchro des colonnes et de l'état), `text.rs` (le texte d'un ticket importé), `secrets.rs` (le coffre), `fake.rs` (le faux service des tests, `tests/fixtures/fake-trackers.mjs`). Dans la fenêtre : `src/components/settings/IntegrationsTab.svelte`, `src/components/modals/ImportModal.svelte`, l'étape `wizard/IntegrationsStep.svelte`, `src/lib/integrations.ts`.

**Ce qu'un plugin y ferait.** Fournir un service de plus : se connecter (les champs du compte, déclarés), lister les conteneurs (projets, tableaux), leurs états, les tickets filtrés et paginés, et pousser l'état d'un ticket quand il change de colonne (`sync.rs`).

**Exemple.** YouTrack, Asana, ClickUp, Notion, Azure Boards, ou un CSV dans le dépôt.

**Valeur 3** : chaque équipe a son outil, et Escouade n'en couvrira jamais que quelques-uns (GitLab Issues est déjà demandé, ESC-165). **Difficulté 2** : remplacer l'énumération par un trait (comme `Forge`), puis un `Client` qui parle au plugin ; les types sont déjà sérialisables. **Risque 2** : le plugin a besoin du réseau et d'un secret ; il lit des tickets que l'utilisateur lui confie ; un plugin lent bloque l'import (il faut un délai par appel).

### 1.3 Les hébergeurs de code

**Où.** `src-tauri/src/forge/` : les traits `Forge` (`me`, `create_pr`, `pull_request`, les vérifications de CI `Check`, les relectures `Review`, `ReviewComment`, les déploiements `Deployment`, le merge `MergeStrategy`, la limite d'appels `RateLimit`, chaque méthode refusée par défaut `unsupported`) et `Provider` (`kind`, `name`, `request_word`, `capabilities`, `default_host`, `own_host`, `repo_of`, `probe`, `scopes`, `token_page`, `cli`, `client`), le registre `forge::Registry`, `remote.rs` (l'analyse des remotes), `accounts.rs`, `connect.rs` ; GitHub (`github.rs`, `github_api.rs`) et Azure DevOps (`azure.rs` : sa déclaration, son client est dans le plugin `dev.escouade.azure-devops`, ESC-241 et 280), GitLab, Bitbucket et Gitea dans leurs plugins livrés (`dev.escouade.gitlab`, `dev.escouade.bitbucket`, `dev.escouade.gitea`, ESC-56 à 58). Dans la fenêtre : `settings/ForgesSection.svelte`, `modals/PrModal.svelte`, `src/lib/prs.ts`, `src/lib/forges.ts`.

**Ce qu'un plugin y ferait.** Un hébergeur de plus (`Provider` + `Forge`) ; ou seulement une partie : les déploiements d'un service qui n'est pas l'hébergeur (Vercel, Netlify, Coolify, Fly.io), la CI d'un autre service (Jenkins, Buildkite), sur une PR déjà connue.

**Exemple.** « Déploiements Coolify » : sur la carte d'un ticket en revue, l'état du déploiement d'aperçu de sa branche, et son adresse.

**Valeur 2** (GitHub, GitLab, Bitbucket et Gitea couvrent la plupart ; les déploiements et la CI tierce valent plus). **Difficulté 1** pour un hébergeur entier (le trait et le registre existent, `unsupported` permet d'en faire une partie), **2** pour des déploiements découplés de l'hébergeur (le modèle les attache à `Forge`). **Risque 2** : un jeton d'hébergeur peut écrire dans les dépôts (`scopes`) ; il va au coffre et n'est donné qu'à son plugin.

### 1.4 Les services de voix

**Où.** `src-tauri/src/voice/` : la dictée locale (Whisper, `transcribe.rs`, `models.rs`, `vad.rs`, `audio.rs`), la transcription en ligne (`online.rs` : `Provider`, une énumération de cinq services, `PROVIDERS`, `Spec`, `Net`, et le catalogue `online_catalog.rs`), le direct (`live.rs`, par WebSocket), la voix en ligne (`online_speech.rs`), la conversation vocale (`conversation.rs`, `spoken.rs`). Dans la fenêtre : `settings/VoiceTab.svelte`, `SpeechSettings.svelte`, `OnlineKey.svelte`, `OnlineModel.svelte`, `src/lib/dictation.svelte.ts`, `src/lib/online-voice.svelte.ts`.

**Ce qu'un plugin y ferait.** Un service de transcription (un fichier audio, ou un flux en direct) ou de voix (un texte, un flux audio), avec son catalogue de modèles et de voix.

**Exemple.** Wispr Flow (déjà demandé, ESC-164), AssemblyAI, Azure Speech, ou un serveur Whisper de l'équipe.

**Le contrat (INT-12, ESC-284, fait le 2026-11-01).** `src-tauri/src/voice/service.rs` : le trait `VoiceService` porte les cinq usages d'un service en ligne, et le cœur n'appelle plus que lui :

- **le compte** : `account_entry` (l'entrée du trousseau, `dictation-<id>`, une clé comme INT-6), lue par `keys`, `save_key` et `delete_key` ;
- **la dictée d'un fichier** : `default_model`, `transcribe` (bloquant, annulable) ;
- **le direct** : `offers_live`, `live_model`, `open_live` (un `LiveStream` : l'audio poussé, le texte partiel par rappel, le texte final à `finish`) ;
- **la synthèse** : `max_chars`, `speak` (le son rendu par morceaux à un rappel, annulable) ;
- **le catalogue** : `catalog` (la documentation, ou ce que l'API a listé) et `list` (relit les modèles et les voix).

`CoreService(Provider)` répond pour OpenAI, Groq, Mistral, ElevenLabs et Deepgram avec le code d'avant, sans changement de comportement ; `services()`, `find(id)` et `of(provider)` sont le registre. Whisper et Piper n'en font pas partie : hors ligne, sans compte, ils restent au cœur. Un plugin répond au même trait (INT-13, fait : §4.13 ; Wispr Flow, ESC-164, s'écrira dessus plus tard). **Mesure d'INT-0 pour la voix** (`voice::service::tests`, faux serveurs WebSocket, 20 phrases par service, Windows 11, build de debug) : le délai du texte final après la fin de la phrase, en médiane, **avant** 2,4 ms (OpenAI), 2,0 ms (Mistral), 2,0 ms (ElevenLabs), 1,5 ms (Deepgram) ; **après**, par le contrat, les mêmes (2,4 ; 2,0 ; 2,0 ; 1,5 ms). La mesure refuse plus de 5 ms de plus en médiane ; elle se lance à la main (`cargo test the_contract_adds -- --ignored --nocapture`), car deux médianes de 2 ms comparées sur une machine de CI chargée échoueraient au hasard. La CI garde le passage du direct par le contrat, avec une borne large. Le passage par un plugin, mesuré par INT-13 (§4.13) : moins de 1 ms de plus en médiane comme au 99e centile quand le son est poussé au rythme du micro (cœur 0,5 ms, plugin 1,0 à 1,2 ms), sous le seuil de 5 ms.

**Valeur 2.** **Difficulté 2** pour le fichier entier (une requête, une réponse), **3** pour le direct (de l'audio en continu, des délais de quelques centaines de millisecondes mesurés en ESC-136 et ESC-137 : un process par stdio tient le débit, mais le passage par un plugin s'ajoute au délai). **Risque 2** : le plugin entend ce que l'utilisateur dicte ; il a besoin du réseau et d'une clé.

### 1.5 Les outils du serveur MCP d'Escouade

**Où.** `src-tauri/src/mcp/` : `tools.rs` (la liste et les descriptions des outils : `list_projects`, `get_board`, `create_ticket`, `start_ticket`, `answer_permission`, `create_agent`, `send_message`, `get_usage`… 52 en tout, par familles : le Kanban et les agents, la mémoire, la spec, les décisions et les documents ; les `index_*` de l'index du code sont retirés avec lui par ESC-310), `read.rs` et `act.rs` (ce qu'ils lisent et font), `resolve.rs` (`Sight`, `may_act_on` : ce qu'un agent voit et sur quoi il agit, les projets privés), `http.rs` (le serveur et son jeton), `install.rs` (la déclaration dans chaque harness), `activity.rs` (le journal des appels, `McpActivity`). Dans la fenêtre : `settings/McpSection.svelte`, `src/lib/mcp.svelte.ts`, les noms en mots des outils dans `src/lib/tools.ts` (`ESCOUADE_TOOLS`).

**Ce qu'un plugin y ferait.** Deux sens :

- **ajouter des outils** au serveur d'Escouade, que les agents appellent comme les autres (`mcp__escouade__<plugin>_<outil>`) : l'outil passe par le serveur d'Escouade, qui garde le jeton, la vue (`Sight`) et le journal, puis par le plugin ;
- **utiliser les outils** d'Escouade, comme un client : un plugin qui lit le Kanban, crée un ticket, lance un agent. Le plugin reçoit un jeton à lui, et ses droits sont ceux qu'il a demandés (lecture seule, ou actions), jamais plus que ceux d'un agent (`may_act_on`).

**Exemple.** « Sentry » : un outil `sentry_issue` que l'agent appelle pour lire l'erreur d'un ticket ; ou un plugin qui crée un ticket par alerte de production.

**Valeur 2** (un agent peut déjà avoir ses propres serveurs MCP ; l'intérêt est de les grouper avec le reste d'un plugin et de les donner à tous les harness d'un coup, par l'interrupteur unique d'ESC-119). **Difficulté 1** pour ajouter des outils (une table, un appel relayé), **2** pour le client (un jeton par plugin, sa portée). **Risque 3** : un outil d'agent est un chemin pour l'injection de consignes ; un plugin client peut agir sur les projets ; les projets privés doivent rester hors de sa vue, comme pour un agent d'un autre projet.

### 1.6 Les événements

**Où.** Il n'y a pas de bus d'événements interne. Le cœur (`src-tauri/src/core.rs`, 9 300 lignes) change l'état et envoie à la fenêtre un `UiEvent` par un seul canal (`hub.rs`, `model.rs` : `Agent`, `AgentRemoved`, `Conv`, `Git`, `Usage`, `Ticket`, `TicketRemoved`, `Project`, `BoardIssue`, `AutopilotPause`, `Toast`, `McpActivity`, `Voice`…) ; ces événements portent l'état entier, pour la fenêtre, pas le fait. Les faits vivent là où ils arrivent :

- **un agent** : son statut (`AgentStatus`, `set_status` dans `agent.rs`), sa création, son arrêt, son passage sur un autre harness (`handover.rs`), son changement de compte au seuil (`switching.rs`) ;
- **un tour** : sa fin (`Effects.turn_end`, `board::TurnEnd` : `Finished`, `Interrupted`, `Limited`, `Error`), la limite atteinte (`Effects.limited`), le quota (`Effects.rate`) ;
- **une demande** : une autorisation ou une question en attente (`add_pending`), sa réponse (`PermissionAnswer::Allow`, `Always`, `Deny` ; « Refusé par <client> via Escouade » dit déjà qui a répondu) ;
- **un ticket** : sa création, son passage d'une colonne à l'autre (`model::Column` : `Todo`, `Doing`, `Review`, `Done`), son lancement par le pilote auto, son rapport de critères (le bloc `escouade`), sa validation et son refus, son merge et ses conflits (`board.rs`) ;
- **un projet** : son ajout, sa fermeture (`closed.rs`), ses réglages, un worktree créé ou retiré (`worktrees.rs`) ;
- **un lancement** : un lancement de test démarré, prêt, arrêté (`testlaunch.rs`, `RunView.svelte`), les tests d'une validation passés ou non ;
- **l'app** : son démarrage, une mise à jour disponible (`updates.rs`).

**Ce qu'un plugin y ferait.** S'abonner aux faits qu'il a déclarés, et réagir : notifier ailleurs, lancer une commande, appeler une URL, ou, pour quelques-uns, **décider** (voir §1.7).

**Exemple.** « Slack » : « Ticket ESC-170 à tester, 3 critères sur 3 » dans le salon de l'équipe, avec le lien de la PR ; « Commande à la fin du tour » : `say "Claude a fini"` ; un webhook générique vers n8n ou Zapier.

**Valeur 3** (demandé de toutes les formes ; c'est le point d'entrée de la plupart des plugins). **Difficulté 2** : il faut un bus de faits, typé et versionné, appelé aux endroits du cœur qui changent l'état, et non dérivé des `UiEvent` (qui portent trop et changent avec l'interface). **Risque 2** : un fait dit le nom d'un ticket, d'un projet, d'une branche, parfois un extrait de conversation ; ceux d'un projet privé ne vont qu'aux plugins activés pour lui. Un plugin lent ne retarde rien : les faits partent dans une file, sans attendre de réponse.

**Construit (ESC-177, PLG-5).** Le bus est `src-tauri/src/facts.rs` : un `Body` typé par fait (39 en version 1, de `agent.created` à `app.started`, avec ceux des docs, de la mémoire, des décisions, des PR, des comptes et du workflow, ESC-200), émis là où l’état change (`AgentRt::set_status`, `add_pending`, `Core::apply`, `edit_ticket_with`, la validation, `remove_worktree`, `move_session`, `Space::spec_write`…), les noms remplis par un fil du bus (`Resolve`). `plugins/events.rs` les distribue : `permissions.events`, une seule règle avec Réglages › Plugins (PLG-4) : les `Choices` (`Plugins::reaches` sur `active_in`, un projet privé éteint d’office, un fait sans projet connu gardé comme privé) et les permissions retirées (`granted` : `events:<e>` retiré coupe le fait, ses webhooks et ses commandes), une file de 256 par plugin, `events/fact` au programme, `contributes.webhooks` (vers un hôte de `permissions.network` seulement, écrit en clair, tiré d’un secret ou fait d’une valeur du fait ; `*.d` couvre les sous-domaines de `d`, pas `d` ; la lecture du manifeste refuse un webhook écrit en clair vers un autre hôte, `manifest::host_covered`) et `contributes.commands` (après l’accord, une empreinte rangée dans `Choices.commands_approved`, donné ou retiré dans l’onglet Permissions). La page « Les événements » du site les décrit fait par fait.

### 1.7 Les autorisations et les questions

**Où.** `EngineProcess::answer_permission` et `answer_question` (`engine/mod.rs`), la réponse dans le cœur (`core.rs`, `answer_permission`), dans l'agent (`agent.rs`, `answer_permission_by` : qui a répondu), dans la fenêtre (`conv/PermissionCard.svelte`, `QuestionCard.svelte`, `src/lib/answer-keys.svelte.ts`), par la voix (`voice/conversation.rs`), et par un autre agent (`mcp__escouade__answer_permission`). Le refus par défaut de Claude Code (`caps.default_refusal`) et les règles « Toujours » de chaque harness (`PermissionAnswer::Always { rules }`).

**Ce qu'un plugin y ferait.** **Décider** d'une demande avant qu'elle n'arrive à l'utilisateur, selon une règle : autoriser, refuser avec un message, ou laisser l'utilisateur répondre. Avec un délai court au-delà duquel la demande va à l'utilisateur comme aujourd'hui. La réponse dit « Autorisé par <plugin> via Escouade ».

**Exemple.** « Règles de l'équipe » : `npm test`, `cargo nextest` et `git status` autorisés dans tous les worktrees ; `git push --force` et `rm -rf` hors du worktree refusés ; le reste à l'utilisateur. Ou une politique lue dans le dépôt.

**Valeur 3** (les demandes d'autorisation sont ce qui interrompt le plus ; un mode Bypass est souvent la seule issue). **Difficulté 2** (le chemin de réponse existe ; il faut une étape avant `add_pending`, et un ordre quand plusieurs plugins répondent : un refus l'emporte). **Risque 3** : un plugin qui autorise tout est un mode Bypass caché. Il faut une permission à part, montrée en rouge, jamais accordée à un plugin proposé par un projet, et le journal de chaque décision.

### 1.8 Les textes rédigés, les consignes et la passation

**Où.** Les textes que le modèle rédige : les one-shots (`engine/oneshot.rs`, `Engine::one_shot`, `Weight::Light` et `Reading`, « Modèle des textes rédigés ») pour le nom d'un agent (`core.rs`, `rename_agent`, le nom donné au premier message d'ESC-156), le message de commit et la description d'une PR (`board.rs`, `pr_body`, la forme conventionnelle), les étapes de mise en place d'un worktree et les commandes de lancement (`core.rs`, `suggest_worktree_steps`, `suggest_run_commands`). Les consignes : le protocole d'un ticket (`board.rs`, `protocol`, `protocol_prompt_for`, borné par `PROTOCOL_BUDGET` et `Engine::instructions_room`), le message transmis à un sous-agent (`subagent_relay`). La passation à un autre harness (`handover.rs`, le fichier écrit depuis le journal `conv.rs`).

**Ce qu'un plugin y ferait.** **Transformer** un texte avant qu'il serve : le message de commit, la description d'une PR (un gabarit d'équipe, un lien vers le ticket Jira), le nom d'un agent ; **ajouter des consignes** au protocole d'un ticket (les règles de l'équipe, la définition de « fini ») dans la place qui reste ; enrichir le fichier de passation.

**Exemple.** « Gabarit de PR » : la description suit le modèle `.github/pull_request_template.md` du dépôt, avec la case « Tests » cochée d'après le rapport de critères.

**Valeur 2.** **Difficulté 1** (des fonctions pures, appelées à quelques endroits). **Risque 2** : des consignes ajoutées sont une injection possible dans chaque agent ; leur taille doit rester dans `instructions_room` ; un plugin qui renvoie n'importe quoi ne doit pas casser un commit (le texte d'Escouade sert de repli, comme pour un one-shot raté).

### 1.9 L'interface

**Où.** Svelte 5, dans `src/components/` et `src/lib/` :

| Emplacement | Fichiers | Ce qu'un plugin y mettrait |
| --- | --- | --- |
| Panneau latéral | `SidePanel.svelte`, `FilesPanel.svelte`, `branches/`, `editor/` | un panneau de plus à côté de Git et des fichiers (« Sentry », « Docs de l'équipe ») |
| Barre latérale | `Sidebar.svelte`, `src/lib/sidebar.ts` | une section (« Revues en attente ») |
| Réglages | `modals/SettingsModal.svelte`, `settings/*Tab.svelte`, `Group.svelte`, `Row.svelte` | un onglet, ou une section d'un onglet, avec les champs de ses réglages |
| Menus | `ContextMenu.svelte`, `src/lib/menu.svelte.ts` (`MenuItem`), `agent-actions.ts`, `project-actions.ts`, `launch-actions.ts`, `term-actions.ts` | une entrée sur un agent, un ticket, un projet, un lancement |
| Commandes et raccourcis | `src/lib/shortcuts.ts`, `QuickOpen.svelte` | une commande nommée, avec un raccourci proposé |
| Carte dans la conversation | `conv/ToolRow.svelte`, `src/lib/tools.ts` (le résumé d'un outil), `conv/EventMessage.svelte` | le rendu d'un outil (le sien ou un autre) : une table, un lien, une image |
| Badge | `board/` (cartes des tickets), `Sidebar.svelte` (cartes des agents) | « CI verte », « Déployé », « Sentry : 3 erreurs » |
| Barre d'état | `StatusBar.svelte` | un élément (« Coolify : en ligne ») |
| Barre des projets | `ProjectRail.svelte`, `ProjectTile.svelte` | un badge sur la tuile d'un projet |
| Page plein cadre | `Stats.svelte`, `Overview.svelte` | une page à elle (« Coûts par client ») |
| Fenêtre | `modals/Modal.svelte` | une fenêtre de saisie (connexion, choix) |

**Ce qu'un plugin y ferait.** Ajouter un élément à un emplacement, dont **Escouade** fait le rendu à partir d'une description : un texte, une icône du jeu d'Escouade (`Icon.svelte`, `IconSprite.svelte`), une couleur de statut, une action (une méthode du plugin) ; pour un panneau ou une page, des blocs (liste, table, champs, boutons, markdown par `Markdown.svelte`). Plus tard, une vue libre en HTML dans une iframe isolée (voir §2.3).

**Exemple.** « Coûts par client » : une page plein cadre qui regroupe les statistiques par préfixe de ticket, une entrée « Facturer ce ticket » dans le menu d'un ticket.

**Valeur 3** pour les menus, badges, barre d'état et réglages (là où un plugin devient visible), **2** pour les panneaux et pages. **Difficulté 2** pour l'interface déclarée (un composant par type de bloc, aux règles de `DESIGN.md`, dans les deux langues), **3** pour une vue libre (isolation, thème, taille, focus, accessibilité, et les trois moteurs de rendu). **Risque 1** pour l'interface déclarée (Escouade fait le rendu), **3** pour une vue libre dans la fenêtre principale (une iframe qui atteindrait l'IPC de Tauri aurait les droits de l'app : CVE-2024-35222, voir §2.3).

### 1.10 Les thèmes et les langues

**Les thèmes. Où.** `src/app.css` (177 propriétés `--…` : surfaces, texte, filets, états, `@property --p`), `src/lib/theme.ts` (la teinte de la fenêtre par `--p`, la couleur du projet en oklch, `PROJECT_COLORS`, `BRAND`), `src/lib/appearance.ts`, le thème de xterm tiré des jetons (ESC-80, `TerminalView.svelte`), celui de CodeMirror (`src/lib/editor/`), les règles de `DESIGN.md`. **Ce qu'un plugin y ferait** : un jeu de valeurs pour les jetons, clair ou sombre, et une palette de couleurs de projet. **Exemple** : « Solarized », « Contraste élevé ». **Valeur 2**, **difficulté 1** (des données ; les anciens jetons, `--ok`, `--accent`, `--line2`…, ont été renommés en jetons 2.0 par ESC-172 et leur liste est dans `DESIGN.md` « Les jetons » : un thème ne dépend que de ces noms), **risque 1** (des valeurs CSS filtrées : couleurs, longueurs ; jamais `url()` ni du CSS libre, qui pourrait charger une ressource ou masquer une demande d'autorisation).

**Les langues. Où.** `src/lib/i18n/` : un catalogue par langue et par domaine (`fr/*.ts`, `en/*.ts`, 26 fichiers et près de 3 900 lignes en français), typé (`types.ts`, `type-checks.ts`), avec `GLOSSARY.md`, et un test qui refuse le texte écrit en dur (`hardcoded.test.ts`) ; côté Rust, `src-tauri/src/i18n.rs` (`Lang` : `Fr`, `En`) et des textes écrits dans le code par `match lang` (le protocole d'un ticket, les notifications, les refus). Le site, `website/content/docs/{fr,en}/`. **Ce qu'un plugin y ferait** : une langue de plus, ses catalogues complets. **Exemple** : « Español ». **Valeur 2** (Escouade est en deux langues), **difficulté 3** (`Lang` est fermé, les textes du cœur ne sont pas en catalogue ; une langue partielle doit retomber sur l'anglais clé par clé ; les consignes données aux agents dans une langue doivent rester dans leur budget), **risque 1** (des textes ; mais un texte de consigne traduit est une consigne, à borner comme au §1.8).

### 1.11 Les recettes, les worktrees, l'assistant et `.escouade.yml`

**Où.** Les lancements de test d'un worktree (`src-tauri/src/testlaunch.rs` : les ports réservés, les fichiers copiés, les tests d'une validation, la disponibilité d'un serveur ; `src/lib/recipe.ts`, `RunView.svelte`, `RunsSection.svelte`, `modals/TestLaunchModal.svelte`), isola, un outil externe déjà branché (`isola.rs` : `isola up`, `ls --json`, `down`, `destroy`), la mise en place et le retrait d'un worktree (`worktrees.rs`, `worktrees.setup`, `worktrees.teardown`), l'assistant « Nouveau projet » (`src/lib/wizard.ts`, `wizard-form.svelte.ts`, `components/wizard/*Step.svelte`), le fichier partagé du projet (`projectfile.rs`, `KEYS` : `agents.engine`, `board.*`, `git.*`, `worktrees.*`, et `schemas/`).

**Ce qu'un plugin y ferait.**

- **un environnement de lancement**, à la manière d'isola : prendre en charge `up`, l'adresse, `down` et `destroy` d'un worktree (Docker Compose, devcontainers, Tilt, un cluster de préproduction) ;
- **des étapes de mise en place** proposées selon ce qu'il reconnaît dans le dossier (un `pnpm-lock.yaml`, un `Cargo.toml`), en plus de celles que rédige le modèle ;
- **des modèles de projet** pour l'assistant (« Nuxt + Supabase » : la recette, les étapes, les réglages du Kanban) ;
- **des clés à lui** dans `.escouade.yml`, sous `plugins.<id>.*`, validées par le schéma qu'il déclare.

**Exemple.** « Docker Compose » : chaque worktree a ses conteneurs, sur ses ports réservés, retirés avec lui.

**Valeur 3** pour l'environnement de lancement (c'est là que les projets diffèrent le plus), **2** pour le reste. **Difficulté 2** (isola est déjà un cas particulier à généraliser ; `KEYS` est la seule liste de `.escouade.yml`, il faut un espace à part pour les plugins). **Risque 3** : une recette lance des commandes. Les commandes de `.escouade.yml` passent déjà par une approbation à empreinte stricte (ESC-146) ; celles d'un plugin aussi.

### 1.12 Les statistiques, les notifications et la recherche

**Les statistiques. Où.** `src-tauri/src/stats.rs` (SQLite : tokens et coût par agent, par compte, par jour), `src/lib/stats.ts` (les indicateurs), `Stats.svelte` et `Overview.svelte`, la grille des prix `pricing.rs`. Rien ne s'exporte aujourd'hui. **Ce qu'un plugin y ferait** : lire les statistiques (en lecture seule), les exporter (CSV, Grafana, un tableur), ajouter une page ou un indicateur. **Exemple** : « Export mensuel » en CSV par projet et par harness. **Valeur 2**, **difficulté 1**, **risque 1** (des chiffres, mais pour un projet privé seulement si le plugin y est activé). La **télémétrie** (`telemetry.rs`, Aptabase, envoyée seulement avec l'accord de l'utilisateur) n'est pas un point d'extension : un plugin ne lit ni n'envoie rien par elle.

**Les notifications. Où.** `src-tauri/src/notify.rs` (le son, la notification du système, la barre des tâches, le badge du plateau), `Toasts.svelte`, `settings/AppTab.svelte`. **Ce qu'un plugin y ferait** : un canal de plus (ntfy, Pushover, Telegram, la montre), à côté de ceux du système, avec les mêmes réglages de ce qui notifie ; et des notifications à lui. **Exemple** : « ntfy » pour être prévenu sur son téléphone qu'un agent attend. **Valeur 3** (avec les événements, c'est ce que demandent ceux qui laissent tourner l'autopilote), **difficulté 1**, **risque 1** (le texte d'une notification est borné, `MAX_BODY`).

**La recherche. Où.** `convsearch.rs` (Ctrl+K, dans les conversations), `search.rs` (`git grep` de l'éditeur), `QuickOpen.svelte` et `src/lib/editor/quick-open.ts`. **Ce qu'un plugin y ferait** : une source de résultats de plus dans Ctrl+K (les tickets Linear, la doc interne, les erreurs Sentry), chaque résultat avec son action. **Valeur 2**, **difficulté 2** (un délai, un nombre de résultats borné, comme `MAX_HITS` et `TIME_LIMIT`), **risque 1**.

### 1.13 Ce que j'ai trouvé en plus

- **Les comptes et le quota** (`accounts.rs`, `usage.rs`, `switching.rs`, `QuotaMeter.svelte`) : un plugin qui lit le quota d'un harness qui ne le dit pas (un tableau de bord de fournisseur), ou qui décide du changement de compte. Valeur 1, difficulté 2, risque 2. Après.
- **Le coffre** (`integrations/secrets.rs`, `SecretStore` : `get`, `set`, `delete`, le trousseau du système) : pas un point d'extension, mais le service que tous les plugins utilisent (§3.5).
- **Les fichiers joints et les images d'un ticket** (`attachfiles.rs`, `ticketfiles.rs`, `ticketimages.rs`) : un plugin qui joint une capture d'écran prise par un navigateur sans tête au rapport d'un ticket. Valeur 2, difficulté 2, risque 2. Après.
- **Le pilote auto** (`board.rs`, l'autopilote, `maxParallel`, `AutopilotPause`) : un plugin qui ordonne la file des tickets (par priorité Jira, par coût). Valeur 2, difficulté 2, risque 2. Après, sur les événements et les sources.
- **Les commandes locales et `/` des harness** (`agent/localcmd.rs`, `src/lib/localcmd.ts`, `caps.slash_commands`) : des commandes `/` d'Escouade apportées par un plugin, dans le composer. Valeur 2, difficulté 2, risque 2. Après.
- **Les mises à jour d'Escouade** (`updates.rs`, `selfupdate.rs`) : jamais par un plugin.

### 1.14 Le tableau

| Point d'extension | Valeur | Difficulté | Risque | Version proposée |
| --- | :-: | :-: | :-: | --- |
| Événements (notifier, commandes, webhooks) | 3 | 2 | 2 | **2.1.0** |
| Harness ACP décrit (adaptateur générique) | 3 | 2 | 3 | **2.1.0** |
| Thèmes | 2 | 1 | 1 | **2.1.0** |
| Autorisations décidées par une règle | 3 | 2 | 3 | 2.1.0 si la place, sinon 2.1.x |
| Notifications : un canal de plus | 3 | 1 | 1 | 2.1.x (sur les événements) |
| Interface déclarée : menus, badges, barre d'état, réglages | 3 | 2 | 1 | 2.1.x |
| Sources du Kanban | 3 | 2 | 2 | 2.2 |
| Environnements de lancement, étapes, modèles de projet | 3 | 2 | 3 | 2.2 |
| Textes rédigés et consignes | 2 | 1 | 2 | 2.2 |
| Outils MCP ajoutés, plugin client du serveur | 2 | 1–2 | 3 | 2.2 |
| Hébergeurs, déploiements, CI tierce | 2 | 1–2 | 2 | 2.2, après ESC-56 à 69 |
| Statistiques et export | 2 | 1 | 1 | 2.2 |
| Recherche (Ctrl+K) | 2 | 2 | 1 | 2.2 |
| Services de voix | 2 | 2–3 | 2 | 2.3 |
| Interface libre : panneau, page en iframe | 2 | 3 | 3 | 2.3 |
| Langues | 2 | 3 | 1 | 2.3 |
| Harness en programme, vue terminal | 2 | 3 | 3 | plus tard |

**Révisé par ESC-203** (§4) : les sources du Kanban et les hébergeurs ne sont plus seulement des points ouverts aux tiers en 2.2. Leur contrat est posé dès la 2.1.0 (les sources) et la 2.1.x (les hébergeurs), et les intégrations d'Escouade deviennent des plugins de première main en process, après le transport (PLG-2) et le banc (PLG-3). Les versions sont dans `docs/TODO-2.2.md`, « Tout l'externe en plugins ».

## 2. L'architecture

### 2.1 Ce que font les autres

| App | Où tourne le code | Isolation | Permissions | Compatibilité |
| --- | --- | --- | --- | --- |
| **VS Code** | un _extension host_, un process Node à part (ou un Web Worker dans le navigateur) ; les vues libres dans des _webviews_, des iframes servies d'une autre origine | un plantage ne fait pas tomber l'éditeur ; aucun bac à sable pour le code : une extension a les droits de l'utilisateur | aucune au niveau du code ; la _Workspace Trust_ (mode restreint) coupe ce qui exécute le code d'un dossier, et chaque extension déclare son support (`capabilities.untrustedWorkspaces`) ; « Workspace Trust can't prevent a malicious extension from executing code » | `engines.vscode` dans `package.json`, API stable et API proposée à part ; activation paresseuse (`activationEvents`) |
| **Zed** | des composants WebAssembly (WIT, wasmtime), écrits en Rust | un vrai bac à sable : l'extension n'a que ce que l'API lui donne (client HTTP, lancement d'un process, réglages) | par les fonctions exposées ; pas d'interface libre : les extensions ne peuvent pas modifier l'interface | une version du WIT par version de Zed : une extension garde l'API avec laquelle elle a été faite |
| **Obsidian** | du JS dans la fenêtre Electron, sans séparation | aucune : un plugin a accès aux fichiers, au réseau, et peut installer des programmes | aucune ; un mode restreint, actif par défaut, empêche tout code tiers ; une revue à la soumission, pas à chaque version | `minAppVersion` dans le manifeste |
| **Raycast** | TypeScript, React et Node ; un réconciliateur maison transforme l'arbre React en une interface native (AppKit) | l'interface est déclarée par des composants de Raycast, jamais en HTML | une revue de chaque extension par PR dans un dépôt public, une CI qui valide le manifeste ; licence MIT exigée | le manifeste est un `package.json` augmenté |
| **JetBrains** | du Java ou du Kotlin dans la JVM de l'IDE | aucune ; un plugin a l'IDE entier | aucune ; les plugins du Marketplace sont signés et vérifiés | `since-build` et `until-build`, et le _Plugin Verifier_ qui vérifie la compatibilité binaire sur une plage de versions |
| **Claude Code** | un dossier avec `.claude-plugin/plugin.json` : skills, commandes, agents, hooks (des commandes, du HTTP), serveurs MCP et LSP en process, thèmes, moniteurs | les hooks et serveurs sont des process avec les droits de l'utilisateur | `userConfig` : des valeurs demandées à l'activation, celles qui sont sensibles au trousseau ; un chemin hors du dossier du plugin est refusé | `claude plugin validate`, des places de marché qui sont des dépôts git (`marketplace.json`) ; `version` fixe la version installée |
| **Tauri** | des _capabilities_ (ACL) par fenêtre ou webview ; un modèle « isolation » | une webview sans capability n'a pas d'accès à l'IPC ; une iframe ajoutée à la page atteignait l'IPC (CVE-2024-35222, corrigée en 2.0.0-beta.20) | par commande et par fenêtre | — |
| **MCP** | des serveurs en process (stdio) ou en HTTP, JSON-RPC 2.0 ; MCP Apps (SEP-1865) : des vues HTML dans une iframe isolée, qui parlent JSON-RPC par `postMessage` | l'iframe n'atteint ni le DOM, ni le stockage, ni les scripts de la page hôte ; une CSP déclarée | l'hôte peut exiger un accord avant un appel d'outil venu d'une vue ; tout le trafic est journalisé | négociation des capacités à l'initialisation |

Ce qu'on en retient :

1. **Personne ne bac-à-sable le code et l'interface libre à la fois, sauf par la séparation.** VS Code sépare le code (un process) de l'interface (des iframes d'une autre origine) ; MCP Apps fait de même. Zed bac-à-sable le code et n'offre pas d'interface ; Raycast offre une interface riche mais _déclarée_.
2. **Sans bac à sable, la sécurité vient de la confiance et de la revue** (Obsidian, JetBrains, Raycast, Claude Code) : un mode restreint par défaut, une revue humaine, des signatures, et le droit de tout retirer.
3. **La compatibilité vient d'une version d'API déclarée et vérifiée** (`engines.vscode`, `minAppVersion`, `since-build`, le WIT par version de Zed), plus un outil qui le vérifie avant la publication (`claude plugin validate`, _Plugin Verifier_).
4. **Escouade parle déjà le langage des process** : ses harness sont des process (stream-json, JSON-RPC de Codex et de Copilot, ACP de Cursor, HTTP et SSE d'OpenCode), son serveur MCP aussi, et tout son outillage de tests repose sur des faux process (`tests/fixtures/fake-*.mjs`) et des Job objects qui tuent un arbre de process entier (`job.rs`).

### 2.2 Trois expériences jetables

Faites hors du dépôt, dans `%TEMP%\esc170`, le 2026-10-25, sur la machine de développement (Windows 11).

**Un module WASM chargé depuis Rust** (`wasm/`, wasmtime 38, Cranelift, un module en WAT avec une fonction de l'hôte) :

- compilation du module : 3,4 ms ; instanciation : 27 µs ; un million d'appels de l'hôte vers le module : 11,5 ms, **11 ns par appel** ;
- une boucle infinie arrêtée par le **carburant** (`consume_fuel`, 5·10⁷ unités) en 11 ms, ou par l'**époque** (`epoch_interruption`, incrémentée par un autre fil) en 200 ms, à la milliseconde : `Trap::OutOfFuel` et `Trap::Interrupt` ;
- une mémoire qui grossit sans fin arrêtée à la limite (`StoreLimits`, 16 Mio : 241 pages de 64 Kio accordées, puis `memory.grow` refusé) ;
- un plantage (`unreachable`) rendu en erreur, l'hôte toujours vivant, le module appelable à nouveau ;
- le coût : **11 Mo** de plus dans l'exécutable avec Cranelift, et 518 Mo de `target` à la compilation.

Le bac à sable de WASM tient ce qu'il promet, à un coût d'appel négligeable. Ce qu'il ne dit pas : le confort d'un auteur en TypeScript ou en Python (il faut embarquer un interpréteur compilé en WASM : QuickJS, StarlingMonkey, CPython, plusieurs Mo par plugin et un démarrage plus lent), le débogage (pas de débogueur pas à pas ordinaire dans l'hôte), et les appels asynchrones de l'hôte (le réseau) qui demandent les fonctions asynchrones de wasmtime.

**Une iframe isolée dans le moteur de WebView2** (`iframe/`, Edge sans tête par Playwright, une iframe `sandbox="allow-scripts"` en `srcdoc`, sans `allow-same-origin`, avec une CSP `default-src 'none'`, dans une page qui imite l'IPC de Tauri par `window.__TAURI_INTERNALS__`) :

- l'iframe a l'origine `null` ; lire le DOM de la page, appeler `parent.__TAURI_INTERNALS__.invoke()` et le `localStorage` : **refusés** (`SecurityError`) ; `fetch` vers l'extérieur : **refusé** par la CSP (`TypeError`) ;
- `postMessage` dans les deux sens fonctionne : un JSON-RPC entre la vue et la fenêtre est possible ;
- une boucle de 1,5 s dans l'iframe n'a pas gelé la page : son minuteur de 50 ms a eu au plus 65 ms d'écart. Chromium met l'iframe d'origine opaque dans un autre process de rendu.
- **Non essayé** : WKWebView (macOS) et WebKitGTK (Linux). Le navigateur WebKit de Playwright n'est pas installé sur cette machine. WebKit met les iframes dans le process de la page, sauf là où son isolation des sites est active : une boucle infinie gèlerait sans doute la fenêtre sous macOS et Linux. À vérifier sur un Mac et sous Linux avant d'offrir une vue libre.

**Un plugin en process à côté** (`proc/`, un hôte Node et un plugin Node en JSON-RPC sur stdio, une ligne par message, la forme du protocole de Codex et d'ACP) : premier échange 67 ms après le lancement (le démarrage de Node), puis **43 µs par aller-retour** sur 10 000 demandes d'autorisation. Assez pour un événement, une décision ou un texte ; trop pour un appel par image d'une animation, ce qu'aucun point d'extension ne demande.

### 2.3 Les quatre façons, comparées

**A. Un process à côté, en JSON-RPC sur stdio** (comme un serveur MCP, un agent ACP, l'_extension host_ de VS Code, un serveur LSP).

- **Isolation** : un plantage ne touche qu'un process ; Escouade le redémarre avec un budget (trois fois, puis le plugin est arrêté et le journal le dit). Une fuite de mémoire est mesurable (`resources.rs` lit déjà la mémoire et le CPU des process des agents) et bornable par le Job object (Windows : limite de mémoire du Job ; ailleurs, `setrlimit`). Une boucle infinie ne gèle rien : chaque appel a un délai, au-delà duquel la réponse par défaut s'applique. Le Job tue tout l'arbre du plugin à sa fin ou à celle de l'app (`job.rs`).
- **Permissions** : Escouade ne peut **appliquer** que ce qui passe par lui : les faits et les données qu'il envoie (rien d'un projet privé à un plugin non activé pour lui : la vue `Sight` du serveur MCP), les secrets qu'il donne (ceux du plugin seulement), les outils qu'il ouvre. Le réseau et le disque, il ne peut que les **déclarer** et les montrer : le process a les droits de l'utilisateur, comme chez VS Code, Obsidian, JetBrains et Claude Code. La clôture des projets privés (`fence.rs`) n'est pas tenue contre un plugin qui va lire le disque lui-même. Un bac à sable du système (AppContainer, Seatbelt, bubblewrap, ce que Codex fait pour ses commandes) le permettrait plus tard, à un coût élevé sur trois systèmes.
- **Performances** : 43 µs par aller-retour mesurés ; le démarrage d'un runtime (67 ms pour Node) ; la mémoire d'un process par plugin (quelques dizaines de Mo pour Node), d'où un démarrage paresseux, au premier fait auquel il est abonné, comme les `activationEvents` de VS Code.
- **Langages et débogage** : n'importe lesquels (TypeScript, Python, Go, Rust, un script shell pour un hook) ; un débogueur ordinaire s'attache au process (`--inspect` pour Node) ; le journal de stderr va dans Réglages › Plugins. Mais il faut un runtime : un plugin en TypeScript exige Node, que tous les utilisateurs n'ont pas (Claude Code a son installeur natif). Le SDK doit permettre de livrer un exécutable unique (`bun build --compile`, `deno compile`, Go, Rust).
- **Compatibilité** : la version du protocole est négociée à `initialize`, comme ACP et MCP (`protocolVersion`, les capacités de chaque côté) ; un champ inconnu est ignoré des deux côtés, comme la lecture des harness le fait déjà (« La lecture tolère l'inconnu », `docs/ENGINES.md`).
- **Coût dans Escouade** : faible. Le client JSON-RPC sur stdio, les Job objects, les faux process et le banc existent ; le cœur appelle un trait, comme pour les harness.

**B. Un module WASM chargé par le cœur** (wasmtime, ou Extism par-dessus ; ce que fait Zed).

- **Isolation** : la meilleure. Plantage, mémoire et boucle infinie bornés et rattrapés (essayé au §2.2), dans le process d'Escouade sans le mettre en danger.
- **Permissions** : **appliquées** : le module n'a que les fonctions que l'hôte lui donne. Le réseau passe par une fonction de l'hôte qui ne laisse sortir que vers les hôtes déclarés (le `allowed_hosts` d'Extism) ; le disque par des dossiers préouverts (WASI) que la clôture des projets privés peut filtrer ; les secrets par une fonction qui ne rend que ceux du plugin. C'est la seule façon de tenir `fence` contre un plugin.
- **Performances** : 11 ns par appel ; un démarrage en millisecondes ; mais un interpréteur embarqué pour JS ou Python, de plusieurs Mo, et un code JS bien plus lent qu'avec Node.
- **Langages et débogage** : Rust, C, Go (TinyGo), AssemblyScript sont naturels ; JS et Python passent par un interpréteur compilé en WASM, avec des limites (pas de modules natifs, pas de `npm` qui touche au système) ; le débogage se fait surtout au journal.
- **Compatibilité** : une interface WIT par version majeure, gardée côté hôte comme Zed le fait ; l'hôte peut charger des modules de plusieurs versions de l'API à la fois.
- **Coût dans Escouade** : moyen à fort. 11 Mo de plus, une interface WIT à tenir, les fonctions asynchrones de l'hôte, un SDK par langage. Et ce que les points d'extension de la 2.1.0 demandent d'abord (le réseau pour un webhook, un process pour un harness, une commande pour une notification) devrait de toute façon sortir du bac à sable, par des fonctions de l'hôte qui le réouvrent.

**C. Du JS dans la WebView, en iframe isolée ou en Web Worker** (les _webviews_ de VS Code, MCP Apps).

- **Isolation** : sous WebView2, une iframe d'origine opaque tourne dans un autre process (essayé) ; sous WKWebView et WebKitGTK, sans doute dans celui de la fenêtre, où une boucle la gèlerait (non essayé). Un Web Worker a son propre fil partout, mais pas de DOM. Une fuite de mémoire pèse sur la fenêtre.
- **Permissions** : sans `allow-same-origin`, avec une CSP `default-src 'none'`, l'iframe n'atteint ni l'IPC de Tauri, ni le DOM, ni le stockage, ni le réseau (essayé). Les capabilities de Tauri ne s'appliquent pas à une iframe : c'est l'origine opaque qui protège, et ce n'est sûr que si **aucune** iframe de plugin n'a `allow-same-origin` (CVE-2024-35222). La CSP d'Escouade (`tauri.conf.json`) devra ajouter `frame-src` pour les vues des plugins, servies par un protocole à part.
- **Performances** : bonnes pour une vue ; nulles pour la logique hors de la fenêtre (un plugin dans la WebView ne vit que tant que la fenêtre est ouverte, et Escouade tourne aussi fenêtre cachée, dans le plateau).
- **Langages et débogage** : JS et TypeScript, les outils du navigateur (les DevTools de WebView2 s'ouvrent déjà pour les e2e).
- **Compatibilité** : le thème (les jetons de `app.css` passés à la vue), la taille, le focus et les raccourcis sont à reprendre dans chaque vue ; trois moteurs de rendu à essayer.
- **Coût dans Escouade** : moyen ; le protocole `postMessage`, un composant hôte, le protocole qui sert les fichiers du plugin.

**D. Les mélanges.**

- **D1. Le code en process, l'interface déclarée** : le plugin décrit ses contributions (manifeste, puis blocs rendus par Escouade : texte, liste, table, champs, boutons, markdown), et le code tourne en process. C'est le modèle de Raycast pour l'interface et de VS Code pour le code (ses menus, commandes, réglages et barre d'état sont déclarés dans `package.json`). Pas de HTML d'un plugin dans la fenêtre.
- **D2. D1, plus des vues libres en iframe isolée plus tard**, pour ce que les blocs ne savent pas montrer (un graphe, un tableau de bord) : le modèle de MCP Apps, qui parle aussi JSON-RPC.
- **D3. Un seul protocole, deux runtimes** : les mêmes messages JSON-RPC, portés soit par stdio vers un process, soit par une fonction de l'hôte vers un module WASM. Un plugin qui n'a besoin d'aucun droit du système (une règle d'autorisation, un gabarit de PR, un ordre des tickets) se compile en WASM et devient installable sans crainte ; les autres restent des process.
- **D4. Le déclaratif seul**, sans code : thèmes, langues, harness ACP décrits, webhooks (une URL et un gabarit par fait), modèles de projet, recettes. Une bonne part de la valeur, sans aucun code tiers à faire tourner.

| | A. Process | B. WASM | C. JS en iframe | D1 + D3 + D4 |
| --- | --- | --- | --- | --- |
| Plantage, boucle, mémoire | redémarré, délai, Job | rattrapés, bornés | dépend du moteur de rendu | comme A, et B pour les modules |
| Réseau, disque, `fence` | déclarés, non appliqués | appliqués | appliqués (CSP, origine opaque) | appliqués pour le déclaratif et le WASM, déclarés pour le process |
| Secrets, projets privés | appliqués (ce qui passe par Escouade) | appliqués | appliqués | appliqués |
| Coût d'un appel | 43 µs | 11 ns | `postMessage` | — |
| Langages | tous | Rust, Go, C ; JS et Python avec un interpréteur | JS | tous |
| Débogage | ordinaire | au journal | DevTools | ordinaire |
| Vit sans fenêtre | oui | oui | non | oui |
| Coût pour Escouade | faible | moyen à fort | moyen | faible en 2.1, moyen ensuite |

### 2.4 La recommandation

**D : un protocole des plugins en JSON-RPC 2.0, servi en 2.1 par un process à côté (A), avec une interface déclarée (D1) et un niveau déclaratif sans code (D4) ; un runtime WASM du même protocole plus tard (D3), puis des vues libres en iframe isolée (D2).**

> **Révisé le 2026-11-01 (ESC-319, §5).** Le WASM n'attend plus : il devient **le runtime des programmes en JS et en TS**, pour que Node ne soit jamais un prérequis. Le store compile le bundle commité (`run.js`) en WASM par Javy et le plugin Javy d'Escouade, de façon reproductible ; l'app le fait tourner par wasmtime, dans son process, et **applique** ce que le tableau du §2.3 disait seulement déclaré : le réseau (`permissions.network`, par `net/fetch` servi par l'hôte), les fichiers (ceux du dossier du plugin seulement), la mémoire et le temps. Le protocole, le SDK et l'interface déclarée ne changent pas (D3 : un protocole, deux transports). Le process (A) reste pour la première main en Rust et pour les binaires tiers attestés ; `node` n'est plus une commande d'un plugin tiers du store. La webview est écartée (§5.1). Les raisons 2 et 3 ci-dessous valent toujours pour les plugins qui lancent des programmes ; pour les autres, le §5.3 montre qu'un interpréteur embarqué coûte moins qu'on ne le craignait (1,3 Mo par plugin, 71 µs par décision, 2 ms pour démarrer).

Pourquoi :

1. **C'est ce qu'Escouade sait déjà faire.** Ses huit harness et son serveur MCP sont des process en JSON-RPC ou proches ; ses tests reposent sur des faux process et un banc de conformité ; ses Job objects tuent un arbre entier. Le socle coûte le moins, et le banc des plugins se calque sur celui des harness.
2. **Les points qui valent le plus ont besoin du système.** Un harness est un process ; un webhook, une source du Kanban, un hébergeur ont besoin du réseau ; une notification ou un environnement de lancement lancent des commandes. En WASM, il faudrait les rouvrir par des fonctions de l'hôte, et le bac à sable n'aurait plus grand-chose à garder.
3. **Les auteurs écrivent en TypeScript et en Python**, ceux des agents et des serveurs MCP. Un process les accueille sans interpréteur embarqué, avec leur débogueur.
4. **L'honnêteté plutôt qu'un faux bac à sable** : un plugin en process a les droits de l'utilisateur, comme chez VS Code, Obsidian, JetBrains et Claude Code. Escouade le dit avant l'installation, applique ce qu'il peut (les faits, les secrets, les projets privés, les outils MCP), et réserve le niveau « sans crainte » au déclaratif, puis au WASM.
5. **L'interface déclarée garde le design** : chaque contribution est rendue par les composants d'Escouade, dans les deux langues, aux règles de `DESIGN.md`, sans HTML d'un tiers dans la fenêtre qui a l'IPC. Les vues libres attendent d'avoir été essayées sous WebKit.
6. **Le WASM reste ouvert** : comme le protocole ne dépend pas du transport, un runtime WASM s'ajoutera sans changer l'API des plugins, pour ceux qu'un projet peut proposer sans risque.

## 3. Le modèle de confiance et de distribution

### 3.1 Le manifeste

Un fichier `escouade-plugin.json` à la racine du dossier du plugin (du JSON, comme `package.json`, `plugin.json` de Claude Code et les schémas de `schemas/` ; un schéma publié pour l'autocomplétion) :

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.slack",
  "name": "Slack",
  "version": "1.2.0",
  "description": { "fr": "Les tickets à tester et les agents qui attendent, dans un salon Slack.", "en": "…" },
  "author": { "name": "Exemple", "url": "https://exemple.dev" },
  "license": "MIT",
  "repository": "https://github.com/exemple/escouade-slack",
  "escouade": { "api": "1", "min": "2.1.0" },
  "icon": "assets/icon.png",
  "run": { "command": "bin/escouade-slack", "windows": "bin/escouade-slack.exe" },
  "activation": ["event:ticket.column", "event:agent.status"],
  "permissions": {
    "events": ["ticket.*", "agent.status"],
    "network": ["hooks.slack.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "settings": [{ "key": "channel", "type": "string", "title": { "fr": "Salon", "en": "Channel" } }],
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook", "en": "Webhook URL" } }]
  }
}
```

- `id` : unique, en nom de domaine inversé (pas de collision entre deux auteurs) ; `name` et `description` dans les deux langues, l'anglais par défaut ;
- `escouade.api` : la version **majeure** du protocole qu'il parle ; `min` (et `max` facultatif) : les versions d'Escouade où il a été essayé. Un plugin d'une API que l'app ne sert plus est désactivé, et la raison montrée, comme une version trop ancienne d'un harness (`Tested`, `oldest_reason`) ;
- `icon` : facultatif (PLG-13, ESC-292). Son logo, un chemin relatif dans le dossier du plugin vers un **PNG carré de 256 px de côté au moins** (512 conseillé), **1 Mo au plus**. Ni SVG (il peut porter du script), ni image distante, ni chemin hors du dossier (`..`, absolu, lecteur, adresse). Les deux lecteurs (`src-tauri/src/plugins/manifest.rs` et `schemas/plugin-manifest.mjs`) en lisent l'en-tête sans le décoder, sur le même corpus (`tests/fixtures/plugins/manifests.json` : absent, introuvable, pas carré, trop petit, trop lourd, pas un PNG, SVG, `..`, absolu, distant), et disent chaque refus en français et en anglais (`file`, `png`, `heavy`, `square`, `small`, `path`). Le store le décode (4096 px de côté au plus) et le réencode sans métadonnées en 256 et 64 px (docs/STORE.md §3.3) ; l'app le lit dans le dossier du plugin installé par la commande `plugin_icon`, qui ne prend que l'id du plugin et ne sort jamais de son dossier (un lien qui en sort est refusé). Sans icône, la tuile garde ses deux lettres ;
- `run` : facultatif. Sans lui, le plugin est **déclaratif** : rien ne s'exécute ;
- `activation` : ce qui le démarre (un fait, une commande, l'ouverture des réglages) ; sans activation, il ne tourne pas ;
- `permissions` : ce qu'il demande, montré avant l'installation (§3.4) ;
- `contributes` : ses points d'extension (`themes`, `harnesses`, `webhooks`, `menus`, `statusBar`, `badges`, `settings`, `secrets`, `commands`, `projectTemplates`, `escouadeYml`…), chacun avec son schéma.

### 3.2 L'installation, la signature, la mise à jour

- **2.1.0 : un dossier local, et un dépôt git** (une URL et une étiquette, `git clone --depth 1` dans le dossier des plugins de l'app, `DataDir`) ; le dossier d'un plugin en développement est chargé en place et rechargé à chaque changement (comme `--plugin-dir` de Claude Code).
- **Avec la 2.1, la place de marché store.escouade.dev** (ESC-202, `docs/STORE.md`, qui remplace l'idée d'un registre en dépôt public pour la 2.2) : un auteur soumet un plugin depuis son dépôt public et une étiquette ; le store fige le commit et l'archive, contrôle seul le manifeste, la licence et la doc, et le liste ; l'app le parcourt dans Réglages › Plugins.
- **L'intégrité** : chaque version installée est fixée par son empreinte (le commit git, ou le SHA-256 de l'archive) ; une mise à jour change l'empreinte et redemande l'accord si les permissions ont grandi.
- **La signature** : avec le store, en 2.1. Une version revue en local par l'utilisateur (un agent de ce dépôt, `/review-plugin`) est signée par lui avec une clé minisign gardée sur son poste, jamais sur le serveur ; l'app la vérifie avec la clé publique embarquée : « Vérifié par Escouade », version par version, retirable par une liste de révocation signée (`docs/STORE.md`, §4). Un plugin installé depuis un dossier ou un dépôt git reste installable, marqué « non vérifié ».
- **La mise à jour** : jamais en silence. Escouade cherche une nouvelle étiquette à son démarrage, la montre dans Réglages › Plugins, et l'installe sur un clic ; une version refusée laisse la précédente en place.

### 3.3 L'activation, les projets privés et `.escouade.yml`

- **Installé** n'est pas **activé**. Un plugin s'active pour toute l'app, ou projet par projet ; par défaut, pour tous les projets **sauf les projets privés**.
- **Un projet privé** (ESC-140) : un plugin ne l'atteint que s'il y est activé un par un. Ses faits, ses tickets, ses statistiques, ses textes ne vont pas aux autres plugins ; un plugin client du serveur MCP le voit comme un agent d'un autre projet (`Sight`), c'est-à-dire pas du tout. Ce qu'Escouade ne peut pas garantir (un process qui lit le disque), Réglages › Plugins le dit en face du projet privé.
- **Un projet qui propose un plugin** : `.escouade.yml` gagne une clé `plugins.recommended` (des identifiants et une source), comme les extensions recommandées de VS Code. Escouade le **propose**, n'installe ni n'active rien seul : un dépôt cloné ne doit pas pouvoir lancer un programme. Un plugin déclaratif (ou WASM, plus tard) peut s'activer après un clic ; un plugin avec un programme demande l'accord complet, comme une installation. Ses réglages propres vont sous `plugins.<id>.*` (§1.11), validés par son schéma, jamais un secret.
- **Fait le 2026-10-30** (PLG-14, ESC-288) : `plugins.recommended` (des identifiants du store, la source est store.escouade.dev) et `plugins.<id>` (des valeurs simples) rejoignent `KEYS` et le schéma comme clés du fichier seul, jamais appliquées au projet ni exportées ; l’ouverture du projet dit combien il en propose, une bande en tête de Réglages › Projets les nomme avec « Voir dans le store » (la page du plugin dans la vue Plugins), « Pas pour ce projet » la fait taire sur ce poste. `settings/get` prend un `project` : les réglages du fichier passent devant ceux de l’utilisateur, seulement les clés de `contributes.settings` d’une valeur de leur type, jamais un secret, et seulement si le plugin est actif dans ce projet (un projet privé ne donne rien à un plugin qui n’y est pas activé).

### 3.4 Ce que l'utilisateur voit, et ce qu'il retire

Avant l'installation, une fenêtre dit, en mots : qui l'a fait et d'où il vient (dépôt, empreinte, « non vérifié »), ce qu'il ajoute (ses contributions), et ce qu'il pourra faire, permission par permission, la plus forte d'abord :

- **« Lance un programme sur votre ordinateur, avec vos droits »** (tout plugin avec `run`) ;
- « Décide des autorisations de vos agents » (en rouge) ;
- « Ajoute des consignes à vos agents », « Ajoute des outils à vos agents » ;
- « Agit sur vos projets » (crée des tickets, lance des agents) ou « Lit vos projets » ;
- « Reçoit ces événements : … » ;
- « Se connecte à : hooks.slack.com » (déclaré) ;
- « Garde un secret : Adresse du webhook ».

Après : désactiver (pour l'app ou un projet), retirer une permission (le plugin continue sans elle, ou s'arrête s'il ne peut pas), désinstaller (son dossier, ses réglages et ses secrets effacés, avec une case pour garder ses données) ; et le **journal** de chaque plugin : ses démarrages, ses plantages, ses appels, les autorisations qu'il a décidées.

### 3.5 Les secrets

Les secrets d'un plugin vont au coffre d'Escouade (`integrations/secrets.rs`, `SecretStore` : le trousseau du système, un fichier dans le bac à sable des tests), sous un nom à lui (`plugin:<id>:<clé>`), saisis dans les champs qu'il déclare (`contributes.secrets`), jamais dans `.escouade.yml` ni dans ses réglages en clair. Il les obtient par le protocole, au moment où il en a besoin (`secrets/get`), seulement les siens ; jamais ceux d'un autre plugin, d'un compte, d'un harness ou d'une intégration. Le texte d'une erreur les masque, comme les déclarations du serveur MCP le font déjà (`add_declaration`, `secrets`).

### 3.6 La télémétrie

Rien de neuf sans accord. Si l'utilisateur a accepté les statistiques d'usage (`telemetry.rs`), Escouade compte les fonctions des plugins comme les autres (« un plugin installé », « un plugin activé », « installé depuis le store », sans son nom) ; le store compte les installations de ses plugins, sans identifiant, avec le même accord (`docs/STORE.md`, §5.4). Un plugin n'a pas accès à la télémétrie d'Escouade ; s'il envoie ses propres statistiques, c'est une permission réseau qu'il déclare et que l'utilisateur voit.

## 4. Tout ce qui est externe est un plugin, en process (ESC-203)

Demandé par l'utilisateur le 2026-10-26 : « Faudra prévoir de faire passer toutes les intégrations externes sous forme de plugins (Claude code, copilot CLI, etc, y compris la gestion des tickets externes et la gestion des PR). Ex : un plugin github pourra gérer les tickets ET les PR. »

Les §1 à §3 font des plugins des **ajouts** : ils viennent à côté des intégrations écrites dans le cœur. Ce chapitre va plus loin : **chaque intégration externe devient un plugin**, celles d'aujourd'hui comprises, et **chaque plugin est un process**, ceux d'Escouade comme ceux des tiers (décidé le 2026-10-26, §4.11). Il ne change pas l'app ; le plan, version par version, est dans `docs/TODO-2.2.md`, chapitre « Tout l'externe en plugins ».

### 4.1 La décision

- **Est externe** ce qui parle à un programme ou à un service qu'Escouade n'a pas écrit : un harness (`claude`, `codex`, `copilot`, `agent` de Cursor, `opencode`), l'API d'un service de tickets (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues), celle d'un hébergeur de code et de sa CI et de ses déploiements (GitHub, GitLab, Bitbucket, Gitea et Forgejo, Azure DevOps), la CLI d'un service (`gh`, `glab`). **Chacun devient un plugin** : un manifeste, une fiche dans Réglages › Plugins, un interrupteur, un journal.
- **Les intégrations d'aujourd'hui deviennent des plugins de première main** : écrites et tenues par Escouade, livrées et signées avec l'app (§4.3).
- **Tout en process** : un plugin de première main est un programme à part qui parle le protocole des plugins (JSON-RPC 2.0 sur stdio, PLG-2), exactement comme un plugin tiers. **Un seul chemin, un seul contrat** : ce qui marche pour GitHub par Escouade marche pour un GitHub d'un tiers, et le banc le prouve sur les deux.
- **Un plugin peut tenir plusieurs points d'extension** : le plugin « GitHub » donne les tickets (Issues), les PR, la CI et les déploiements, sur une seule connexion (§4.4).
- **Plus tard, par la même règle** : les services de voix (§1.4) et les environnements de lancement comme isola (§1.11) sont aussi externes. Ce ticket ne les planifie pas ; ils suivront le même chemin une fois les trois familles faites.

### 4.2 Ce qui reste dans le cœur, et pourquoi

| Reste dans le cœur | Où | Pourquoi |
| --- | --- | --- |
| Les agents et leur état | `agent.rs` (`AgentRt`, `Effects`), `conv.rs`, `handover.rs`, `switching.rs` | Tous les harness écrivent dans le même état. Le plugin d'un harness ne le touche plus : il envoie des **éléments normalisés** et des **effets** (§4.5), que le cœur applique. |
| Le lancement des process des harness | `proc.rs`, `job.rs`, `shellenv.rs`, `resources.rs` | Le cœur lance le harness d'après la description que le plugin lui rend (§4.6) : le Job object, la mesure des ressources, le lancement sous WSL et la vérification de la clôture restent les siens, et le harness survit à un plantage de son plugin. |
| Les terminaux et les vues TUI | `pty.rs`, `engine/terminal.rs`, `escouade-hook`, `TerminalView.svelte` | Le PTY reste au cœur : les octets du terminal et les frappes ne passent jamais par un plugin (§4.6). |
| Le Kanban, le pilote auto, le protocole des tickets | `board.rs`, `sharing.rs` | C'est le produit. |
| La synchro d'une source avec le Kanban | `integrations/sync.rs` | La logique (les colonnes et les états, les conflits, quand pousser) est la même pour toutes les sources ; le plugin ne fait que lire et écrire chez son service. |
| Le flux des PR d'un ticket | ESC-59 à 62 : la colonne « En revue », la CI corrigée par l'agent, les retours de relecture, le merge | Même raison : le plugin d'hébergeur donne les appels (`Forge`), le cœur décide quoi en faire. |
| Git local, les worktrees, plusieurs dépôts | `git.rs`, `worktrees.rs`, `repos.rs` | `git` est un outil local, au centre de chaque fonction ; il n'y a pas de service derrière. |
| Le serveur MCP d'Escouade | `mcp/` (`resolve.rs` : `Sight`, `may_act_on`) | Il dit ce qu'un agent voit ; les agents l'appellent directement, jamais à travers le plugin de leur harness. |
| La clôture des projets privés | `fence.rs` | Le cœur calcule la règle ; le plugin la traduit pour son harness (`fence_settings`) ; le cœur vérifie la description de lancement avant de lancer (§4.6). Un harness qui ne peut pas l'appliquer est refusé pour un projet privé. |
| Le coffre | `integrations/secrets.rs` (`SecretStore`) | Le service que tous les plugins utilisent, avec sa règle : **chaque plugin n'a que ses secrets** (§3.5). Les plugins de première main n'y échappent pas : le plugin « Jira » n'obtient pas le jeton du plugin « GitHub ». |
| La jauge des comptes et le changement au seuil | `accounts.rs`, `usage.rs`, `QuotaMeter.svelte` | Le plugin lit le quota (`engine/readUsage`), le cœur décide. |
| Les mises à jour, la télémétrie, les statistiques | `updates.rs`, `selfupdate.rs`, `telemetry.rs`, `stats.rs` | Jamais par un plugin (§1.12, §1.13). |

### 4.3 Les plugins de première main

- **Un binaire, plusieurs plugins.** Le code d'aujourd'hui (`engine/<harness>/`, `agent/<harness>.rs`, `integrations/<service>.rs`, `forge/<hébergeur>.rs`) sort du cœur dans des crates Rust, une par plugin (`plugins/<id>/`), avec les types du protocole dans une crate partagée avec le cœur (`escouade-plugin-protocol`, la crate Rust de PLG-9, avancée). Elles sont réunies dans **un seul exécutable à plusieurs entrées**, `escouade-plugins`, livré à côté de l'app comme `escouade-hook` (`src-tauri/src/bin/`) : `escouade-plugins github`, `escouade-plugins claude`… Un exécutable par plugin coûterait 5 à 10 Mo chacun (tokio, reqwest, serde à chaque fois), une quinzaine en tout ; un seul ne coûte qu'une fois. Chaque plugin reste **un process à lui** : le même exécutable, lancé avec son nom.
- **Son manifeste** est un `escouade-plugin.json` du même schéma, avec `"run": { "command": "{escouade}/escouade-plugins", "args": ["github"] }`. L'origine (`builtin`) est donnée par l'app, jamais par le manifeste. Les ids `dev.escouade.*` sont réservés (ESC-314) : le store ne les accepte que d'une soumission du dépôt de l'app (`STORE_FIRST_PARTY_REPO`, comparé par son id GitHub) et les refuse à tout autre ; l'app ne les prend que d'une version du store **signée** (`Hand::First`, l'origine `store`) ou d'un dossier en développement, jamais d'un dossier copié ni d'un dépôt cloné ; le banc et `escouade-plugin` les prennent avec `--first-party`. Un plugin de première main venu du store reste un plugin du store (origine `store`, rangé dans « Ajoutés ») ; un plugin livré avec l'app a l'origine `builtin` (INT-4), qui seule le range dans « Livrés avec Escouade » et seule prend aussi les programmes `{escouade}/…` (`Hand::Builtin`) : la vue les trie par leur origine, jamais par leur id.
- **Fait (INT-4, ESC-210, 2026-10-31).** Le registre est `src-tauri/src/plugins/builtin.rs` : la liste des dossiers `plugins/builtin/<id>/` (manifeste et README, embarqués), l'origine `Origin::Builtin` (`{ "kind": "builtin" }`), `Hand::Builtin`. Leur version est celle de l'app (le `version` du manifeste est remplacé à la lecture), leur empreinte `builtin:<version>`, ni mise à jour ni liste de révocation. Ils se coupent pour l'app ou un projet (les choix sont gardés dans `plugins.json`, comme ceux des autres), jamais désinstallés. **Un plugin `builtin` et le même id venu du store ou d'un dossier** : le `builtin` l'emporte toujours ; l'aperçu dit le refus (`Preview.refusal`), `install` le refuse, et une copie du même id déjà installée avant cède la place au démarrage (ses choix gardés). Les quatre exemples de `plugins/<id>/` (ESC-314) sont des plugins du store, pas des `builtin`. Le programme est `escouade-plugins` (`src-tauri/src/bin/escouade-plugins.rs`, une entrée par nom : `escouade-plugins minimal`), sur la crate `crates/escouade-plugin-protocol` (`PROTOCOL`, `Hello`, `serve`) que le cœur partage ; avant de lancer un programme `{escouade}/…` le runtime vérifie qu'il est bien celui du dossier de l'app (`builtin::check_program`) et, en release, la signature là où le système la vérifie (`codesign` sous macOS ; sous Windows, Authenticode valide dès que celle de l'app l'est). Le premier est `dev.escouade.minimal`, la démonstration, inscrit au registre des builds de développement et des tests seulement : le registre d'une release ne tient que les vrais, que INT-7/8 livrent un à un (Trello le premier, INT-7.1), tandis que `escouade-plugins` est empaqueté et signé dès maintenant. Le statut Authenticode se lit par PowerShell avec le chemin en variable d'environnement (`ESCOUADE_SIGNED_PATH`), gardé pour la session par fichier (chemin, taille, date) ; la règle (celui de l'app valide ⇒ celui du programme valide) est celle de la release. Le paquet : `escouade-plugins` est signé comme `escouade-plugin` (Authenticode via `scripts/windows-signing.mjs`, fusion universelle macOS via `scripts/universal-hook.sh`, les paquets Linux), vérifié par la CI (présence sur les trois systèmes, `codesign --verify` sous macOS) et par `release.yml` après la construction ; la notarisation et l'Authenticode ne se voient qu'au prochain Release.
- **Dans le dépôt de l'app, chacun lisible seul** (décidé par l'utilisateur le 2026-10-29) : pas de dépôt séparé, le protocole et la livraison signée restant ceux de l'app. Mais chaque dossier `plugins/<id>/` se lit comme le dépôt d'un plugin tiers : son `escouade-plugin.json`, un `README.md` (ce qu'il fait, ses permissions en mots, ses comptes, comment le lancer seul contre le faux Escouade de PLG-9) et ses tests. Il sert d'exemple aux auteurs, en plus de ceux de PLG-10. Sa page sur le store pointe vers ce dossier à l'étiquette de la dernière version d'Escouade (`github.com/guillaume-gagnaire/escouade/tree/v<version>/plugins/<id>`), comme le dépôt et le commit d'un plugin tiers. Un test de la CI vérifie que chaque dossier a son manifeste et son README, et que la liste du store (`store/server/utils/builtin.ts`) est celle des dossiers.
- **Livrés et mis à jour avec l'app.** Leur version est celle d'Escouade ; ils n'ont pas de mise à jour à part. `escouade-plugins` est signé comme l'app (Authenticode sous Windows, la signature et la notarisation de chaque binaire sous macOS, la clé de l'updater de Tauri pour `latest.json`), et l'app vérifie avant de le lancer que c'est bien le sien (son chemin dans le dossier de l'app, et sa signature là où le système la vérifie). Ils ont le badge « Livré avec Escouade », qui vaut « Vérifié par Escouade » (ESC-202). Un correctif vient avec une release ; pas de liste de révocation pour eux.
- **Leur logo** (décidé par l'utilisateur le 2026-10-29 sur l'aperçu `design/plugin-logos/`, ESC-292) : le logo de la marque quand ses règles publiées permettent de l'afficher pour dire qu'on s'intègre à elle, sans rien demander ; sinon le glyphe de sa catégorie sur la tuile du logo d'Escouade (l'invite pour un harness, la carte pour une source de tickets, la branche pour un hébergeur). Dans le doute, jamais la marque. Les tracés des marques viennent de Simple Icons 16.36 (CC0 pour le tracé) ; le store les dessine dans la page (`store/shared/marks.ts`). Pages relues le 2026-10-29 :

  | Plugin | Page relue | Ce qu'elle dit | Retenu |
  |---|---|---|---|
  | GitHub | brand.github.com/foundations/logo | « Use a permitted GitHub logo to inform others that your project integrates with GitHub » ; l'Invertocat en blanc ou en noir, jamais recoloré, déformé ni accolé à autre chose ; pas de partenariat suggéré | Marque, blanche sur sombre, noire sur clair |
  | OpenCode | github.com/anomalyco/opencode | Dépôt sous MIT, aucune règle de marque | Marque |
  | Gitea et Forgejo | github.com/go-gitea/gitea (LICENSE MIT) ; codeberg.org/forgejo/meta, branding/README.md | Logo de Gitea dans un dépôt MIT ; celui de Forgejo en CC BY-SA 4.0 (à créditer) | Marque de Gitea, telle quelle |
  | Copilot CLI | brand.github.com/foundations/logo | Rien sur l'icône de Copilot pour un tiers (« no longer has a standalone logo ») | Catégorie |
  | GitLab | about.gitlab.com/press/press-kit → règles de marque du handbook | La page des règles ne se lit pas sans compte GitLab | Catégorie |
  | Jira, Trello, Bitbucket | atlassian.design/foundations/logos | Rien sur l'usage par un tiers ; les logos d'intégration sont réservés aux employés d'Atlassian | Catégorie |
  | ClickUp | clickup.com/brand | Un logo partenaire « may be approved » : un accord | Catégorie |
  | Cursor | cursor.com/brand | Des logos à télécharger, aucune règle d'usage | Catégorie |
  | Claude Code, Mistral Vibe, Linear | claude.ai, mistral.ai, linear.app | Pas de page de règles notée par Simple Icons | Catégorie |
  | Codex, Azure DevOps | — | Retirés de Simple Icons à la demande d'OpenAI et de Microsoft | Catégorie |
  | Antigravity CLI, Gemini CLI | about.google/brand-resource-center | Une marque Google demande en général une autorisation | Catégorie |

  Une marque passe de la catégorie à son logo quand sa page le permet : on la relit, on note ici la date, et `shared/marks.ts` gagne son tracé.
- **Activés partout par défaut**, projets privés compris : ils l'étaient avant d'être des plugins, et leur code est celui d'Escouade. C'est la seule exception au §3.3.
- **Ce que l'utilisateur voit.** Réglages › Plugins les liste en tête, groupés sous « Livrés avec Escouade », avec la même fiche que les autres (contributions, permissions en mots, comptes, état du process, journal). Les écrans d'aujourd'hui gardent leur place : Réglages › Harness, l'onglet Intégrations et la section des hébergeurs deviennent des **vues par point d'extension** des plugins actifs, de première main ou tiers. On cherche « GitHub » dans les intégrations, pas dans une liste de plugins.
- **Les désactiver.** Un interrupteur pour l'app, ou par projet, comme les autres. Ils ne se désinstallent pas. Un harness désactivé : ses agents restent, marqués « harness absent », leurs données gardées (comme un harness de plugin désinstallé, PLG-6) ; on ne peut plus en créer. Une source désactivée : les tickets importés restent, la synchro s'arrête et la carte dit pourquoi. Un hébergeur désactivé : les PR ne sont plus suivies, la carte le dit ; rien n'est perdu.
- **Les remplacer.** Un plugin tiers peut servir le même point pour le même service (« Jira Data Center » maison, un client GitHub d'entreprise derrière un proxy). Les deux peuvent être installés ; c'est **le compte** qui dit quel plugin le sert (on ajoute un compte « Jira » par l'un ou par l'autre), et un projet utilise ses comptes. Jamais de remplacement silencieux : un plugin tiers ne prend pas un id ni un compte d'un plugin de première main.

### 4.4 Un plugin, plusieurs points

Le manifeste (PLG-1, §3.1) déclare déjà des contributions ; il gagne quatre points d'extension et une contribution qu'ils partagent :

- `accounts` : **un genre de compte** que ses points partagent. Ses hôtes (l'instance publique, les instances auto-hébergées et comment les reconnaître : `default_host`, `own_host`, `probe` de `forge::Provider`), ses façons de se connecter (un jeton, la CLI du service, OAuth plus tard), les champs à saisir (`site`, `email`, `key` : aujourd'hui des champs de `integrations::Account` propres à Jira et Trello, demain déclarés), les droits demandés (`scopes`), la page qui crée un jeton (`token_page`). Le secret va au coffre sous `plugin:<id>:account:<compte>`. **Une connexion, plusieurs usages** : c'est déjà le cas pour github.com, dont le compte d'hébergeur est celui de GitHub Issues (`forge/accounts.rs`, `Accounts::github`) ; la règle devient générale (GitLab : un compte pour ses Issues et ses MR).

  _Fait le 2026-11-01 (INT-6, ESC-212)_ : `src-tauri/src/plugins/accounts.rs`. **`AccountKind`** (id, plugin et son nom, hôte public, instances à soi, sonde, façons de se connecter `Connect::Token` et `Connect::Cli`, champs `Field { key, secret }`, droits, page du jeton, sources et hébergeur servis) : ceux du cœur tirés de `forge::Registry::standard()` et des sources à compte propre (`standard()`, sous l'id du plugin de première main qui les servira : `dev.escouade.github`, `dev.escouade.azure-devops`…), ceux d'un plugin actif lus de `contributes.accounts` (`of_manifest`, avec ses `ticketSources` et `forges` qui le nomment ; les clés `defaultHost`, `selfHosted`, `probe`, `connect`, `fields`, `scopes`, `tokenPage`, toutes facultatives). **Un compte sert tous les points de son plugin** : `points` et `shared` (github.com : GitHub Issues et ses PR ; une instance GitLab : GitLab Issues et ses MR ; une organisation Azure DevOps : Azure Boards et ses PR ; un GitHub Enterprise : ses PR seules). **Le plugin qui sert un compte**, choisi à son ajout quand plusieurs servent le même genre (« Servi par » dans les deux formulaires, `servers`, `chosen`) : gardé dans `Account::plugin` et `ForgeAccount::plugin` (vide pour la première main), et le cœur passe alors par `ProcessTicketSource` et `ProcessForge` (`client_for`, `client_of` ; GitLab Issues et Azure Boards suivent le plugin du compte de leur hébergeur, une instance par plugin jusqu'à INT-8). **La limite d'appels comptée par compte** : `Runtime::budget` et `spend`, sous `budget_key` (le nom du compte au coffre) ; toute réponse d'un plugin peut la porter (`{ value, rateLimit }`, celles de `tickets/*` aussi) et `rate_limit()` d'une source ou d'un hébergeur du même compte la lit. **Le coffre** : `plugin:<id>:account:<compte>`, le compte étant l'hôte de l'instance (plusieurs par genre) ou l'id du genre (Jira, Trello, Linear, ClickUp, un seul compte) ; même contenu que l'ancienne entrée. **Migration réversible** (décidé le 2026-11-01) : les anciennes entrées (`jira`, `github`, `forge:<hôte>`) restent tenues à côté pour la première main, écrites et effacées avec les nouvelles, et lues d'abord tant qu'elles sont tenues (`secrets::read_entry` : un jeton changé par une version d'avant après un retour est repris et recopié) ; `integrations.json` garde sa forme et dit `vault: 2` à sa prochaine écriture ; `revert` remet les anciennes seules. Tests : `plugins_accounts_tests.rs` (genres, manifeste, choix, partage, GitLab qui sert Issues et MR sur le faux plugin et dans le cœur, budget partagé, aller et retour, coffre qui refuse le compte d'un autre plugin) ; `fake-plugin.mjs` gagne `--ticket-budget`.
- `ticketSources` : une source du Kanban, sur un compte déclaré (§4.5).
- `forges` : un hébergeur de code, ses PR, sa CI et ses déploiements, selon ses capacités (`Capabilities`), sur un compte déclaré.
- `harnesses` : un harness ; par le protocole `engine/*` (§4.5), ou décrit par ACP (PLG-6), servi alors par le plugin ACP générique d'Escouade.
- Plus tard, `ci` et `deployments` à part, pour un service qui n'est pas l'hébergeur (Jenkins, Coolify, Vercel, §1.3) : ils s'attachent à une PR déjà connue d'un `forge`.

Ce que partagent les points d'un même plugin : **le process** (un seul pour ses sources et ses hébergeurs), **le compte** (l'identité, `me`, une seule fois), **le jeton** au coffre, **la connexion** (la CLI `gh`, une session OAuth), **la limite d'appels** (le budget de 5 000 appels par heure de GitHub est celui du jeton : les Issues et les PR le consomment ensemble ; `forge::RateLimit` ne compte aujourd'hui que ceux de l'hébergeur), **le journal** et **l'activation** (désactiver « GitHub » pour un projet coupe ses tickets et ses PR ; une source ou un hébergeur peut aussi être coupé seul, dans sa vue).

**L'exemple complet : le plugin « GitHub »** (de première main) :

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.github",
  "name": "GitHub",
  "version": "2.1.0",
  "description": { "fr": "Les issues dans le Kanban, les PR, la CI et les déploiements de vos dépôts GitHub.", "en": "…" },
  "escouade": { "api": "1", "min": "2.1.0" },
  "run": { "command": "{escouade}/escouade-plugins", "args": ["github"] },
  "activation": ["account:github", "project:remote-github"],
  "permissions": {
    "network": ["api.github.com", "*.ghe.com", "<instances ajoutées par l'utilisateur>"],
    "secrets": ["account:*"],
    "commands": ["gh auth token --hostname <hôte>"]
  },
  "contributes": {
    "accounts": [
      {
        "id": "github",
        "name": "GitHub",
        "hosts": { "default": "github.com", "own": ["github.com", "www.github.com", "ssh.github.com", "*.ghe.com"], "selfHosted": true },
        "auth": [{ "kind": "token", "scopes": ["repo"] }, { "kind": "cli", "program": "gh", "args": ["auth", "token", "--hostname", "{host}"] }],
        "tokenPage": { "github.com": "https://{host}/settings/personal-access-tokens/new", "*.ghe.com": "https://{host}/settings/personal-access-tokens/new", "*": "{url}/settings/tokens/new" }
      }
    ],
    "ticketSources": [{ "id": "github-issues", "name": "GitHub Issues", "account": "github", "containers": "repositories", "states": ["open", "closed", "label:*"], "push": true, "comment": true }],
    "forges": [
      {
        "id": "github",
        "account": "github",
        "requestWord": "PR",
        "capabilities": { "pullRequests": true, "draft": true, "autoMerge": true, "checks": true, "jobLogs": true, "rerun": true, "reviews": true, "reviewThreads": true, "deployments": true, "mergeStrategies": ["merge", "squash", "rebase"] }
      }
    ]
  }
}
```

- **Le compte** : github.com, GHE.com et GitHub Enterprise Server (une instance ajoutée par son adresse), reconnus par leur hôte (`GithubProvider::own_host`) ; un jeton (`repo`) ou celui de `gh` (`forge/github.rs`, `CliToken`). Ces règles sont **déclarées** : le cœur reconnaît le remote d'un dépôt sans lancer le plugin.
- **Les Issues** : les dépôts comme conteneurs, ouvert, fermé ou un label comme état (`integrations/github.rs`), la synchro poussée et le commentaire de fin (`sync.rs`, `Client::set_state`, `comment`).
- **Les PR** : ouvrir, suivre, merger (`Forge::create_pr`, `pull_request`, `pull_requests_of_branch`, `merge`, `auto_merge`).
- **La CI** : les vérifications d'un commit, le journal d'un job, la relance (`checks`, `job_log`, `rerun_failed`).
- **Les relectures** : `reviews`, `comments`, `review`, `reply`.
- **Les déploiements** : `deployments`, par environnement (ESC-67).

Dans la fenêtre, l'utilisateur ajoute **un** compte GitHub ; l'import d'issues et le suivi des PR le proposent tous deux. Pour un projet, il peut couper les Issues et garder les PR.

_Fait le 2026-11-01 (INT-8.1, ESC-217)_ : **le premier plugin de première main réel**, `dev.escouade.github`, inscrit au registre des releases (`plugins/builtin.rs`, `GITHUB`). Son dossier `plugins/builtin/dev.escouade.github/` se lit seul (règle d'INT-4) : le manifeste (`contributes.accounts` du genre `github`, avec `sourcesPublicOnly` : GitHub Issues sur le compte de github.com seulement ; `ticketSources` et `forges` `github` ; `secrets: ["account:*"]`), le README (ce qu'il fait, ses permissions en mots, son compte, comment le lancer contre le faux Escouade) et **sa crate** `escouade-plugin-github` (`src/pulls.rs`, l'ancien `forge/github_api.rs` ; `src/issues.rs`, l'ancien `integrations/github.rs` ; `lib.rs`, ses deux genres). Son programme : `escouade-plugins github`. Sa page du store pointe vers ce dossier (`source` dans `store/server/utils/builtin.ts`, testé contre les dossiers).
- **Le modèle des hébergeurs en plugin** (pour GitLab, Bitbucket, Gitea et Azure DevOps après lui) : le commun est dans la crate `crates/escouade-plugin-kit`, partagée par le cœur et les plugins Rust de première main. `serve` : la boucle en process, asynchrone (les appels répondus à mesure qu'ils finissent, plusieurs à la fois, un lot en lot ; le plugin demande `secrets/get` et écrit `log` pendant qu'il répond). `hosts` : le service de `account/me`, `forge/*` et `tickets/*` (le compte lu de l'appel, son jeton demandé par `secrets/get` de `account:<hôte>`, un client gardé par compte et par jeton pour que les lectures conditionnelles et les limites servent l'appel suivant, les erreurs typées en codes, `{ value, rateLimit }` dans chaque réponse d'un hébergeur). `forge` : le trait `Forge` (le contrat, que le cœur réexporte : `ProcessForge` et les hébergeurs encore au cœur l'implémentent), `Unsupported`, `Pace`. `errors` (`HttpError`, `Refused`, les mêmes types dans le cœur, `integrations` les réexporte), `http` (le client des réglages réseau, `send_named`, `call`, `call_page`), `text` (`criteria_of`, `criteria_in`), `i18n` (`tr!`, la langue de `ESCOUADE_LANG`). Les types de données des hébergeurs (`PullRequest`, `Check`, `RepoRef`…) sont dans `escouade-plugin-protocol::forge` (le cœur les réexporte), ceux des sources dans `::tickets` (la source y est une chaîne ; le cœur garde ses types à `Service` interné), et les codes `code::REFUSED`, `code::http(statut)`. **Ajouter un hébergeur** : une crate `plugins/builtin/<id>/` qui dépend du kit, un `ForgeKind` (son id, son nom, `client(http, Credentials) -> Box<dyn Forge>`) et, s'il a des tickets, un `SourceKind` (`TicketSource` du kit) ; `run()` donne `Hosts::new(forges, sources)` à `serve::run` ; une ligne dans `ENTRIES` de `bin/escouade-plugins.rs`, une dans `builtin::REGISTRY`, le membre dans le `[workspace]` de `src-tauri/Cargo.toml` ; côté cœur, `Provider::plugin()` (et `TicketSourceKind::plugin()`) rend l'id du plugin, et ses tests gardent leurs scénarios, leur client construit par `builtin::served_forge`.
- **Le jeton, prêté par le cœur** (décidé ici) : le cœur tient déjà les jetons des comptes de première main (le coffre lu avec ses anciens noms, INT-6) et celui de la CLI (`gh auth token`, qu'il sait trouver : `gh_cli`). Il les prête au plugin avant chaque appel (`Runtime::lend`, par plugin et par clé `account:<hôte>`) ; `secrets/get` donne le prêté d'abord, puis le coffre ; jamais celui d'un autre plugin, et masqué dans son journal. Le plugin ne lance donc pas `gh` (pas de `permissions.commands`) et le jeton n'est toujours dans aucun appel.
- **L'environnement d'un plugin livré** : `ESCOUADE_LANG` (la langue du cœur), `ESCOUADE_PROXY`, `ESCOUADE_NO_PROXY`, `ESCOUADE_INSECURE_TLS` (Réglages › Réseau), donnés à son lancement (`Runtime::set_builtin_env`) ; un changement arrête ses process, le suivant les prend.
- **Ce que voit l'utilisateur** : rien de changé à l'usage. Les plugins livrés sont dans Réglages › Plugins sous « Livrés avec Escouade » ; la vue des hébergeurs lit le plugin qui sert chaque genre depuis le registre (`ProviderView::served_by` : coupé, ou arrêté et pourquoi), et la carte d'un agent dit « Hébergeur arrêté : PR plus suivies » (`AgentMeta::pr_down`, posé quand la lecture des PR tombe sur `PluginDown`, levé à la suivante qui marche ; les PR lues en dernier restent).
- **Les tests, inchangés, contre le plugin** : `forge/github_tests.rs` (37 tests ; les scénarios n'ont pas changé, `GithubProvider.client` y est le client servi par le process ; ceux qui lisent le client lui-même, son allure et ses réponses gardées, le prennent dans la crate), `integrations/github_tests.rs` (les 7 de GitHub Issues, sortis de l'ancien module) et l'essai réel `real_github_reads` (github.com avec le jeton de `gh`, en lecture), plus ceux du cœur qui passent par un compte GitHub (`agentprs_tests`, `integrations_tests`) : les cœurs des tests ont le registre (`Plugins::load_for_core`). Les crates du protocole, du kit et de GitHub sont membres d'un `[workspace]` dont `src-tauri` est la racine : nextest et clippy les prennent.

_Fait le 2026-11-01 (ESC-56)_ : **le client GitLab écrit directement en plugin**, `dev.escouade.gitlab` (registre des releases, `GITLAB` ; programme `escouade-plugins gitlab`), sur le modèle de GitHub. Sa crate `escouade-plugin-gitlab` (`src/merges.rs` : MR, merge et auto-merge, jobs du pipeline de tête, journal par `/jobs/:id/trace`, relance du pipeline, approbations et relecteurs, discussions, déploiements par branche ; `lib.rs` : son genre, l'API à `<instance>/api/v4`). Le cœur garde de GitLab ce qu'il en déclare (`forge/gitlab.rs` : hôtes, droits, `glab`, `Provider::plugin`) et GitLab Issues (`integrations/gitlab.rs`) jusqu'à INT-8.2 (ESC-218) : le manifeste ne déclare que `forges`, et `accounts::kinds` garde au genre `gitlab` du plugin la source du cœur que ses comptes servent encore (le compte d'une instance reste celui de GitLab Issues). Tests : `forge/gitlab_tests.rs` contre le process, nourri de réponses enregistrées sur gitlab.com (`tests/fixtures/forges/gitlab/`, `record.mjs` ; discussions et déploiements d'après la doc, gitlab.com les refusant sans jeton). Pas d'essai réel (décision du 2026-10-16) : ce qui reste à essayer est dans `docs/TODO-2.2.md`.

_Fait le 2026-11-01 (INT-8.3, ESC-57)_ : **Bitbucket Cloud**, `dev.escouade.bitbucket`, sur le modèle ci-dessus (`plugins/builtin/dev.escouade.bitbucket/`, `escouade-plugins bitbucket`, `builtin::BITBUCKET`). Le compte : e-mail et jeton d'API Atlassian (`needsEmail`), sans instance à soi. Le client (`src/pulls.rs`) n'a pas d'auto-merge (la capacité est déclarée absente par `BitbucketProvider::capabilities`), compte lui-même la limite horaire de 1000 appels (Bitbucket ne dit pas ce qui reste ; il croit `X-RateLimit-Limit`, prend `-NearLimit` pour « un cinquième restant », attend un 429) et ne suit une page suivante que sur son adresse. Ses tests : `forge/bitbucket_tests.rs` sur un faux serveur aux réponses écrites d'après la documentation (`tests/fixtures/forges/bitbucket/`) ; rien n'a été essayé sur bitbucket.org (liste des essais à faire dans `docs/TODO-2.2.md`). Le kit n'a changé que d'une ligne : `errors::said` lit aussi le `error.message` de Bitbucket.

_Fait le 2026-11-01 (INT-8.5, ESC-280)_ : **Azure DevOps et Azure Boards**, ensemble, `dev.escouade.azure-devops` (`plugins/builtin/dev.escouade.azure-devops/`, `escouade-plugins azure-devops`, `builtin::AZURE_DEVOPS`), sur le modèle ci-dessus et sur le compte d'une organisation (celui d'une collection pour un Server) qui sert les deux points. `pulls.rs` est l'ancien `forge/azure_api.rs` et `boards.rs` l'ancien `integrations/azure.rs`, passés tels quels ; `html.rs` porte `html_to_markdown`, qui n'était qu'à Azure Boards. **Le jeton de `az`** (Entra ID, en Bearer) est lu par le cœur (`Provider::cli`, `credentials`) et prêté au plugin comme un jeton ordinaire, sous `account:<hôte>` ; le plugin ne lance aucun programme. Le cœur dit l'adresse du faux serveur (`ESCOUADE_AZURE_DEVOPS_API`) dans `Credentials::api`. **Plusieurs organisations pour Azure Boards** : un compte du cœur est un hôte, et un conteneur est `<organisation>/<projet>` ; `plugins::adapters::ByOrganization` tient un `ProcessTicketSource` par compte et envoie chaque appel à celui que nomme son conteneur (`Core::azure_client`). **Le kit** gagne `TicketSource::states_view` (par défaut rien : le cœur devine les colonnes par les noms) et `hosts::StatesView`, pour qu'Azure Boards donne les états avec les défauts de chaque colonne tirés de leurs catégories ; `tickets/states` répond alors `{ states, defaults }`, ce que `ProcessTicketSource` lisait déjà. Ses tests : `forge/azure_tests.rs`, `integrations/azure_tests.rs` et `azure-devops.spec.ts`, leurs scénarios inchangés ; les essais réels `real_azure_devops` et `real_azure_boards` restent compilés et ignorés, à jouer avec un jeton (liste dans `docs/TODO-2.2.md`).

### 4.5 Le contrat de chaque point d'extension

Les traits Rust d'aujourd'hui deviennent, dans le cœur, des **adaptateurs** qui parlent au process : `ProcessTicketSource: TicketSource`, `ProcessForge: Forge`, `ProcessEngine: Engine` et `ProcessEngineProcess: EngineProcess`. Le reste du cœur ne voit pas la différence. Les types sont ceux de la crate du protocole, partagée par le cœur et les plugins de première main, et générés en TypeScript pour le SDK (PLG-9). Ceux des sources et des hébergeurs sont déjà sérialisables (`Container`, `ExternalIssue`, `IssuePage`, `PullRequest`, `Check`…).

| Point | Côté cœur | Méthodes JSON-RPC | Aujourd'hui |
| --- | --- | --- | --- |
| Compte | `AccountKind` (nouveau, tiré de `forge::Provider` et de `integrations::Account`) | déclaratif (hôtes, sonde, auth, champs, droits, page du jeton) ; `account/me` | `Provider::{default_host, own_host, probe, scopes, token_page, cli, needs_email}`, `Client::me` |
| Source du Kanban | `TicketSource` et `TicketSourceKind` (nouveaux, à la place de l'énumération `integrations::Client`) | `tickets/containers`, `tickets/states`, `tickets/issues` (filtre, page), `tickets/setState`, `tickets/comment` ; capacités déclarées | `Client::{containers, states, issues, set_state, comment}` (`integrations/mod.rs`) |
| Hébergeur | `Provider` et `Forge` (`forge/mod.rs`) | `forge/createPr`, `forge/pullRequest`, `forge/pullRequestsOfBranch`, `forge/pullRequests`, `forge/merge`, `forge/autoMerge`, `forge/checks`, `forge/jobLog`, `forge/rerunFailed`, `forge/reviews`, `forge/comments`, `forge/comment`, `forge/review`, `forge/reply`, `forge/deployments` ; `unsupported` par défaut ; la limite d'appels dans chaque réponse | tel quel |
| Harness | `Engine` et `EngineProcess` (`engine/mod.rs`) | déclaratif : `locate` (noms, chemins, `--version`), versions testées, `EngineCaps`, modes ; sans état : `engine/models`, `engine/efforts`, `engine/launch` (la description de lancement), `engine/oneShot`, `engine/userConfig`, `engine/fenceSettings` ; par agent : `agent/frames` (reçu), `agent/items` (envoyé), `agent/write`, `agent/sendMessage`, `agent/interrupt`, `agent/setModel`, `agent/setMode`, `agent/answerPermission`, `agent/answerQuestion`, `agent/readUsage`, `agent/attach` (après un redémarrage) ; vue terminal : `terminal/launch`, `terminal/hook` | `Engine::{locate, tested, caps, models, efforts, spawn, message, translate, one_shot, grant_file, escouade_launch, fence_settings, user_config…}`, `EngineProcess::{send_message, interrupt, set_model, set_mode, answer_permission, answer_question, read_usage…}` |
| Lecteur de workflow (ESC-200, choix 158) | `plugins/reader.rs` : `ProcessWorkflowReader`, `with_read` | `workflow/read` (les fichiers du projet que le manifeste déclare, `contributes.workflowReaders[].files`) → `{ workflows: [{ id, name, steps, transitions }] }` | rien : Escouade lit seule (`workflow.rs`) |
| Environnement de lancement | `launchenv::Launch` et `Env` (`launchenv.rs`, INT-14) | déclaratif (`detect`, `program`, `up`) ; `launch/ls`, `launch/down`, `launch/destroy` | `isola::{manages, configured, read_config, services, run}` |

**L’environnement de lancement (INT-14, ESC-286, 2026-11-01)** : le point `contributes.launchEnvironments` (`id`, `name`, `detect` : le fichier qui dit qu’un worktree est le sien ; `program` : la CLI, cherchée sur le PATH par le cœur ; `up` : la commande qu’un lancement de test exécute dans un terminal du projet, après l’approbation de `detect` lu en entier) et `src-tauri/src/launchenv.rs`. Le cœur fait ce qui n’a pas besoin du process : décider qu’un worktree est celui d’un environnement (`Launch::for_dir` : plugin livré avec l’app, actif pour l’app et le projet, fichier présent, programme installé), lire `detect`, lancer `up`. Le plugin, un process, fait le reste par `launch/ls` (les services et leurs adresses), `launch/down` et `launch/destroy` ; le cœur lui donne le programme trouvé, le dossier, la branche et la langue de ses messages. isola est le premier (`plugins/builtin/dev.escouade.isola/`, entrée `isola` d’`escouade-plugins`, `src-tauri/src/bin/escouade-plugins/isola.rs`) ; `isola.rs` ne garde que la recherche de la CLI et le type `Service`. Sans le plugin (coupé pour l’app ou le projet), sans `isola` ou sans `.isola.toml`, un ticket part comme avant, sans isola. **Seuls les plugins livrés avec l’app servent ce point** : `up` est une commande du terminal du cœur, qu’on ne confie pas à un plugin d’ailleurs (Docker Compose, devcontainers, Tilt viendront avec leur propre contrat de confiance). `fake-isola` et les tests du cœur qui l’emploient passent tels quels contre le process du plugin.

**Les adaptateurs des sources et des hébergeurs (INT-5, ESC-211, 2026-10-31)** : `src-tauri/src/plugins/adapters.rs`. `ProcessTicketSource` (`account/me`, `tickets/containers`, `states`, `issues`, `setState`, `comment`) et `ProcessForge` (`account/me`, `forge/*` ; chaque réponse est `{ value, rateLimit? }` et le dernier budget se lit par `rate_limit()`) ; `ProcessTicketSourceKind` en fait une sorte du registre (le registre lira les manifestes avec INT-6). Le jeton n'est jamais dans un appel : le plugin le prend lui-même par `secrets/get` (`account:<id>`). **Erreurs typées** : le plugin dit une erreur HTTP par le code `-(32000 + statut)` (`-32401` : « reconnecte le compte », `forge::refused`), un refus qui ne changera pas par `-32010` (`Refused`), un appel que son hébergeur ne sait pas faire par `-32601` (`Unsupported`). **Plantage, arrêt, délai** : `PluginDown`, qui n'est pas un refus durable (`lasting` est faux) : la synchro garde l'opération et la retente avec ses attentes, et les PR lues en dernier restent (`agentprs` garde ce qu'il avait lu quand l'appel échoue). Délai : 60 s pour `tickets/*`, `forge/*` et `account/me`. Le faux plugin (`tests/fixtures/fake-plugin.mjs`) sert une source et un hébergeur ; `plugins_adapters_tests.rs` joue contre lui les scénarios des tests des intégrations d'aujourd'hui (liste, états, tickets, état poussé, commentaire, PR, CI, 401, 404, refus, plantage).

**Trello en plugin, en process (INT-7.1, ESC-213, 2026-11-01)** : la crate `plugins/builtin/dev.escouade.trello/` (`escouade-plugin-trello`, son manifeste, son README), appelée par `escouade-plugins trello` et inscrite au registre (`builtin::TRELLO`). Elle pose sa source sur le kit `escouade-plugin-kit` comme GitHub y pose ses Issues : un `SourceKind` (`TrelloSource`) qui construit un `TicketSource` par compte, le kit répondant au protocole, demandant le secret (`secrets/get` de `account:trello`, ici `{"key":…,"token":…}`) et gardant un client par compte et par secret. **Pour ajouter une source** (Linear, Jira, ClickUp) : une crate `plugins/builtin/<id>/` (membre du workspace de `src-tauri/Cargo.toml`, dépendance de l'app, une ligne dans `ENTRIES`) avec un `SourceKind` et son client ; son manifeste (`accounts`, `ticketSources`, `permissions.network` et `secrets`, `fields[].missing` pour les textes du formulaire) et son README ; une ligne dans `builtin::REGISTRY` ; dans le cœur, `plugin()` à sa `TicketSourceKind` (qui ne garde que son nom : `Unserved`) et, dans `Core::client_for`, le secret à prêter et l'adresse de l'API (`Bases`) ; ses tests de `integrations/<service>.rs` deviennent `plugins_<service>_tests.rs`, joués contre le plugin lancé comme l'app le lance. **Le secret est prêté** (`Runtime::lend`) par le cœur à chaque appel : la clé et le jeton que le cœur tient, même quand le trousseau ne les rend pas. **Les textes de `sync.rs`** de la source (« Indique le jeton. », « Indique la clé d'API. ») sont dans le manifeste (`fields[].missing`, `builtin::missing`). **Un arrêt se dit** : un plugin arrêté ou coupé pour l'app rend `PluginDown` (« Le plugin dev.escouade.trello ne répond pas : plugin stopped / … is switched off »), la synchronisation attend et recommence. Le kit apprend le compte (`Credentials::user`, l'identifiant de « Mes cartes »). L'onglet Intégrations lit toujours le registre (`InitialState::sources`) : Trello y a son nom, la vue Plugins le montre sous « Livrés avec Escouade », et sa page du store pointe vers son dossier. Tests : les quatre de l'ancien client, mot pour mot, dans `plugins_trello_tests.rs` (le plugin lancé par le runtime sur le faux Trello de `integrations/fake.rs`) plus la langue, le secret jamais envoyé, l'arrêt et la coupure, les textes du manifeste ; les tests du cœur (`integrations_tests.rs`, `fake-trackers.mjs`, l'e2e `integrations.spec.ts`) passent inchangés à travers le plugin.

**Le lecteur de workflow (ESC-200, LOOP-8, 2026-11-01)** : `src-tauri/src/plugins/reader.rs`. Un plugin déclare `contributes.workflowReaders[]` (un `id`, un `name`, et `files`, des globs depuis le dossier du projet : sans `..`, sans chemin absolu) ; Escouade lui donne par `workflow/read` ceux de ces fichiers qui sont du texte (au plus 200, 256 Ko chacun, ceux du dossier de chaque dépôt pour un projet de plusieurs) et il rend `{ workflows: [{ id, name, steps: [{ name, command, description?, path?, agents?, reads?, writes? }], transitions: [{ from, to, back?, condition?, max? }], exit?, harness?, repo? }] }`. Escouade en fait un `Workflow` d'id `plugin:<plugin>:<id>` (il ne prend jamais la place d'un workflow lu par l'app), que la suite suit comme les autres : le suivi des agents, l'onglet, les faits `workflow.*`. Un graphe qui ne tient pas (sans nom, sans étape, deux étapes du même nom, une transition vers une étape absente, plus de 50 workflows) est refusé en entier, avec la raison ; un plugin qui plante est un `PluginDown` (ce qui était lu reste). Délai : 60 s. Comme les adaptateurs ci-dessus, rien dans l'app n'appelle encore un lecteur : le registre lira les manifestes avec INT-6. **Les faits `workflow.*`** (même ticket) : `workflow.step_entered`, `workflow.step_left` (durée, coût, issue), `workflow.looped_back` (sa raison), `workflow.left`, `workflow.changed` (les ids des workflows relus qui diffèrent), émis par `AgentTrack::facts_since` d'après les passages du suivi et par `Core::workflow_read` ; ils portent l'agent, le ticket et le projet comme les autres faits.

**Le contrat des harness (INT-9, ESC-221, 2026-11-01)** : les types sont dans `crates/escouade-plugin-protocol/src/engine.rs` (`escouade_plugin_protocol::engine`), l'adaptateur dans `src-tauri/src/plugins/harness.rs`, l'application des éléments à l'agent dans `src-tauri/src/agent/items.rs`.
- **Le lancement reste au cœur.** `engine/launch` reçoit l'agent (dossier, modèle, effort, mode, consignes, d'où il repart, dossiers lus, serveur d'Escouade avec son jeton, chemins clôturés tels que son programme les écrit, lecture seule, dossier de l'agent) et rend une description sans programme : arguments, variables, variables à retirer, fichiers à écrire, premières lignes, `fenceApplied`, `readOnly`. Le cœur la vérifie (`check_launch`) : le jeton jamais dans les arguments ; pour un projet privé, `fenceApplied` et chaque chemin refusé présent dans les arguments, les variables ou les fichiers ; `readOnly` tenu quand il est demandé (une discussion de l'onglet Docs) ; les fichiers seulement dans le dossier de l'agent (la variable de son compte), sans `..`. Puis il lance le programme qu'il a trouvé lui-même (`locate` déclaré ; `{escouade}/…` vérifié par `builtin::check_program`) dans son Job object, par `Program::launch` (WSL compris). Un refus empêche le lancement et dit pourquoi.
- **Les frames, par lots et en chaînes brutes.** Le cœur lit les lignes du harness et les passe au plugin dans `agent/frames` (`{ agent, frames: ["<ligne>", …] }`), sans les relire. Un lot est ce qui a été lu pendant que le précédent était chez le plugin, jusqu'à 64 Kio, **sans attente** : une ligne seule part tout de suite (l'attente de 8 ms prévue au §4.6 ajoutait jusqu'à 8 ms par ligne au rythme réel, au-delà du seuil de 5 ms). Le plugin rend des **éléments** (`start`, `text`, `thinking`, `tool`, `toolDone`, `touched`, `permission`, `question`, `turn` avec ses jetons et la limite atteinte, `notice`, `write`) ; un `op` inconnu est laissé de côté. Le cœur écrit les `write` sur l'entrée du harness, dans leur ordre, et applique les autres à `AgentRt` (`apply_items`, les mêmes règles que les traductions du cœur : la session, le texte du tour, les lignes du plan, la demande en attente, le tour fermé avec ce qui restait ouvert). `agent/sendMessage`, `agent/interrupt`, `agent/setModel`, `agent/setEffort`, `agent/setMode`, `agent/answerPermission` et `agent/answerQuestion` rendent des `writes`, joués dans l'ordre des appels ; `agent/readUsage` rend le quota lu.
- **`agent/attach` et la file pendant une relance.** Le cœur donne l'agent avant son lancement, et de nouveau après chaque relance (`Runtime::attach`), avec ce qu'il sait : la session, le modèle, l'effort, le mode, les demandes en attente. Un lot resté sans réponse parce que le plugin a planté est redonné une fois le plugin relancé ; les lignes lues entre-temps attendent dans une file bornée (20 000 lignes). La conversation dit « Le plugin de Minimal a redémarré. ». Après le dernier plantage permis, le harness de l'agent est arrêté, la conversation et la fin de son process le disent (« … s'est arrêté après 3 plantages : l'agent s'arrête, sa session est gardée… »), et la session reste pour le message suivant une fois le plugin relancé.
- **`engine/oneShot`** décrit la question (arguments, variables, entrée) ; le cœur la lance avec sa limite, et `engine/oneShotRead` lit la réponse dans ce que le process a écrit.
- **La vue terminal** garde son PTY au cœur : `terminal/launch` décrit le lancement de l'interface (vérifié comme celui d'un agent, plus les bandes de confiance à montrer), et `terminal/hook` va dans les deux sens : le cœur passe le hook et ce qu'il veut y répondre (`reply` : un contexte pour le modèle, ou un refus ; l'index du code, qui devait y répondre, est retiré depuis ESC-310), le plugin rend les éléments de l'agent et la réponse dans la forme de son harness. Le branchement à `TerminalView` vient avec le premier harness en process qui a une vue (INT-10.1) ; d'ici là, `caps.terminalView` reste faux.
- **Ce qui est déclaré** (`contributes.harnesses[]`, `HarnessDecl`) se lit sans lancer le plugin : le nom, le programme (`locate`), les versions testées, les capacités (modes, effort, images, `readOnly`, `accountsByKey`), les modèles, la variable du dossier des comptes, et **la reprise de la mémoire** (`memory.dir`, avec `{configDir}` et `{projectKey}` : ce que `Engine::memory_dir` rend pour `memory::claude_offer`). `plugins::harness::register` met au registre (`registry::register`) les harness d'un manifeste ; rien ne l'appelle encore au démarrage : chaque harness y viendra avec sa sortie (INT-10).
- **La preuve, sur le faux moteur minimal devenu un plugin.** `dev.escouade.minimal` déclare le harness « Minimal » ; son harness est `escouade-plugins minimal-harness` (le script du faux en process d'`engine/minimal/`, devenu un programme), et la traduction d'`agent/minimal.rs` est passée dans le plugin (`src-tauri/src/bin/escouade-plugins/minimal.rs`), qui garde seul l'état de traduction (les appels en cours). `plugins_harness_tests.rs` : le banc des harness joué par `ProcessEngine` (texte, raisonnement, approbation, fichier, question, interruption, sous-agent, reprise : les scénarios du faux en process), un plantage en plein tour (le harness continue, rien n'est donné deux fois, l'outil en cours est fermé à la fin du tour), le plugin arrêté après ses plantages, un lancement clôturé et en lecture seule avec le serveur d'Escouade, les quatre refus de `check_launch`, le one-shot, la vue terminal et ses hooks, ce qui est déclaré, l'enregistrement. Le faux en process (`engine/minimal/`) reste celui des tests de la passation et du banc écrit dans le cœur.

**Les harness ACP décrits (PLG-6, ESC-178, 2026-11-01)** : le client ACP est la crate `crates/escouade-acp` (tirée de celui de Cursor CLI ; ce qui est propre à Cursor, `cursor/*`, dans son module `cursor`), servie par le plugin livré `dev.escouade.acp` (`escouade-plugins acp`) pour chaque entrée `contributes.harnesses[]` qui a un objet `acp` (`args`, `env`, `removeEnv`, `fs`, `modes`, `readOnlyMode`, `modelOption`, `questionsInText`, `tools`, `subagentTools`, `extensions`), la description donnée avec chaque agent (`described`). Le harness entre au registre avec son plugin et en sort avec lui (`harness::sync`, « harness absent »), sous l'id `plugin:<plugin>:<harness>` (nu pour `dev.escouade.*`). La clôture tient sur les `fs/*` du client (`fenceByClient`) et est dite manquante sinon ; `"fs": false` ferme les projets privés. Le contrat gagne `delta`, `toolInput`, `todos`, `mode`, `subagent`, le `result` de `toolDone`, une permission complète (outil, description, raison, diff, « Pour cet agent », sous-agent) et le tour en erreur. Premier plugin : Goose (`plugins/goose/`), essayé pour de vrai avec Goose 1.57.0 sur Ollama (`real_goose`). Tests : `plugins_acp_tests.rs`, e2e `acp-harness.spec.ts`.

Ce que le contrat exige de chaque point, vérifié par le banc des harness (`engine/bench/`) et celui des plugins (PLG-3) : les erreurs typées (`HttpError` : 401 dit « reconnecte le compte », 404, limite atteinte), un délai par appel, la tolérance de l'inconnu ; pour un harness, la clôture (`fenceSettings`, vérifiée par le cœur), le jeton du serveur MCP jamais dans les arguments (`grant_file`, `no_engine_puts_the_token_on_the_command_line`, contrôlé sur la description de lancement), les noms d'outils normalisés.

### 4.6 Tout en process : ce que ça coûte, et comment on le tient

Ce qui existe : environ 41 000 lignes de Rust dans `engine/`, `agent/`, `integrations/` et `forge/`, tests compris ; huit faux harness (`tests/fixtures/fake-*.mjs`), un faux service de tickets (`fake-trackers.mjs`), un faux `gh` ; le banc des harness ; des essais réels par harness (`real-harnesses.spec.ts`, `real_fence`, `real_*_view`, `real_claude_models`). La décision les garde tous, et les rejoue à chaque migration.

**Ce qui a été comparé** (avant la décision) :

| | I. Intégré (Rust dans l'app) | P. Process (retenu) | W. WASM |
| --- | --- | --- | --- |
| **Le flux d'un agent** | inchangé | un passage de plus par frame (43 µs par aller-retour mesurés au §2.2), la sérialisation, et la traduction à séparer de l'état de l'agent (plus bas) | 11 ns par appel ; mais un harness lance un process, ce que le bac à sable ne permet pas sans le rouvrir |
| **Les terminaux** | inchangés | inchangés : le PTY reste au cœur | inchangés : le PTY reste au cœur |
| **Sources et hébergeurs** | inchangés | négligeable devant un appel HTTP (des centaines de ms) | négligeable |
| **La clôture** | tenue par le code du cœur | traduite par le plugin, vérifiée par le cœur sur la description de lancement | appliquée au module, pas au harness qu'il lance |
| **Les jetons** | restent dans le process d'Escouade | chaque plugin reçoit les siens (`secrets/get`) et le jeton MCP de ses agents | appliqués : le module n'a que les siens |
| **Effort de migration** | faible | fort : sortir le code en crates, séparer la traduction de l'état, un adaptateur par point, l'empaquetage signé | très fort, impossible pour un harness |
| **Risque de régression** | faible | moyen à fort, tenu par les mesures, le banc et les essais réels rejoués (plus bas) | fort |
| **Ce que ça apporte** | rien de plus que le contrat | **un seul chemin** : le banc, le protocole et le journal servent les deux ; ce qui manque au contrat se voit chez Escouade avant un tiers ; un plantage d'intégration ne fait plus tomber l'app ; une intégration pourra un jour se corriger sans release | un bac à sable réel |

L'utilisateur a retenu **P pour tout** (§4.11). Ce qu'elle coûte, point par point, et comment on le tient :

**1. Le flux d'un agent.**

- **Qui lance le harness : le cœur.** Le plugin rend une **description de lancement** (`engine/launch` : programme, arguments, variables, dossier, fichiers de config à écrire, `grant_file`), le cœur la vérifie (clôture, jeton absent des arguments, chemin du programme trouvé par `locate`) puis lance le harness dans son Job object, sous WSL au besoin, comme aujourd'hui (`proc.rs`, `job.rs`). Le harness survit à un plantage de son plugin, et `resources.rs` continue de mesurer son arbre. L'autre façon, le plugin qui lance son harness lui-même, a été écartée : le cœur perdrait le Job, la vérification de la clôture et la survie au plantage.
- **Le chemin d'une frame** : le harness écrit une ligne (stream-json de Claude Code, JSON-RPC de Codex et de Copilot, ACP de Cursor) ; le cœur la lit et l'envoie au plugin dans `agent/frames` ; le plugin la traduit et rend `agent/items`, des éléments normalisés (`user`, `text`, `thinking`, `tool`, `question`, `permission`, `turn`, `notice`, `event`, les mêmes que la conversation d'aujourd'hui, `ConvOp`) et des **effets** (`Effects` d'`agent.rs` : fin de tour, quota, limite atteinte, compte déconnecté, fichiers changés, plan écrit) ; le cœur les applique à `AgentRt` et à la fenêtre. Ce que le plugin écrit au harness (une réponse JSON-RPC de Codex, un message ACP) passe par `agent/write`, que le cœur écrit sur le stdin du harness. OpenCode parle HTTP et SSE à son serveur : là, le cœur lance `opencode serve` et le plugin lui parle lui-même, sur `127.0.0.1`.
- **Par lots.** Le cœur ne fait pas un aller-retour par ligne : il envoie au plugin tout ce qu'une lecture a rendu (jusqu'à 64 Kio, ou 8 ms d'attente au plus), et le plugin rend ses éléments par lots aussi, en fusionnant les morceaux de texte d'un même bloc (les deltas d'un message en cours). Le flux reste continu : aucune frame n'attend la fin d'un tour, seulement la fin de son lot.
- **La sérialisation.** Une frame est déjà du JSON : elle est passée au plugin comme une chaîne, sans être relue par le cœur (`RawValue`), puis relue une fois par le plugin, comme le cœur la relit aujourd'hui. Le coût en plus est l'échappement d'une chaîne dans une chaîne et une copie par passage. Les grosses frames (un fichier lu en entier, une image) sont les cas à mesurer.
- **La traduction à séparer de l'état.** C'est le gros du travail : aujourd'hui, `claude_frame` et ses pareils sont des méthodes d'`AgentRt` (`agent/claude.rs` et les autres) qui lisent et écrivent directement l'état de l'agent : 61 membres d'`AgentRt` pour Claude Code, 41 à 50 pour les autres. Chaque plugin garde l'état qui ne sert qu'à traduire (les outils en cours, les blocs ouverts, le plan, les identifiants des demandes) et rend ce que le cœur doit appliquer. Ce qu'un plugin ne peut pas savoir (le mode courant, le modèle choisi, l'agent en pause), le cœur le lui donne à `agent/attach` et à chaque changement.
- **La mesure, avant et après.** Un banc de flux (INT-0, dans `docs/TODO-2.2.md`) rejoue des flux enregistrés de chaque harness (`tests/fixtures/<harness>/`, et un long tour réel de Claude Code avec des milliers de frames), au rythme réel et au plus vite, et mesure : le délai de la ligne lue à l'événement envoyé à la fenêtre (médiane et 99e centile), le débit maximal, le CPU et la mémoire du cœur et du plugin, le temps d'un rejeu entier. Mesuré **avant** toute migration, puis après celle de chaque harness. Les seuils proposés, à confirmer par la première mesure : au plus 5 ms de plus au 99e centile, un débit au moins dix fois le plus fort flux réel observé, au plus 30 Mo par process de plugin. Un harness qui les dépasse ne passe pas en process tant que ce n'est pas corrigé.
- **Le transport seul, mesuré avec PLG-2 (ESC-174, 2026-10-29).** `round_trips_and_throughput_are_measured` (`src-tauri/src/plugins_runtime_tests.rs`) mesure le runtime des plugins contre le faux plugin en Node, sous Windows 11 : un aller-retour en médiane de 72 µs, 146 µs au 99e centile (2 000 appels ; 43 µs au §2.2, entre un hôte Node et un plugin Node, sans Job ni file) ; 30 Mo/s de frames d'environ 1 Kio passées en chaînes brutes, par lots de 64 (30 000 frames par seconde). Bien sous les seuils proposés ; INT-0 n'existe pas encore : la comparaison au flux réel des harness se fera avec lui.
- **Le banc des plugins, avec PLG-3 (ESC-175, 2026-10-29).** `src-tauri/src/plugins/bench/` joue neuf scénarios (onze avec INT-5 : une source de tickets et un hébergeur, joués quand le manifeste les déclare : `account/me` et une lecture, une réponse ou une erreur typée) contre le dossier d'un plugin quelconque, par un faux Escouade (`bench/fake.rs`, le pilote JSON-RPC sans l'app) : manifeste, initialisation, version refusée (la 999 offerte), délais (chaque appel mesuré, une méthode inconnue refusée par -32601), un fait reçu (`events/fact`, une notification), une décision (`permissions/decide`), un secret (seulement les siens, jamais écrits), plantage au milieu d'un appel puis relance, fermeture ; un scénario hors des permissions du manifeste n'est pas joué. La CLI `escouade-plugin` (`src/bin/`, livrée à côté de l'app et signée comme elle) en fait `test`, `validate` et `dev` ; `--measure` mesure les allers-retours (58 µs en médiane, 147 µs au 99e centile sur l'exemple Slack). Les exemples de PLG-10 (ESC-182, `plugin-examples/`, `plugins/` depuis ESC-314 : Contraste élevé, Webhook Discord, Slack, Règles de l'équipe) passent le banc dans les tests et en CI (`plugin-examples/ci.mjs`) ; les scénarios par point (source, hébergeur, harness) et le rejeu d'INT-0 viendront avec ces points et INT-0.
- **Les mesures de référence, avec INT-0 (ESC-206, 2026-10-30) : les chiffres « avant ».** `npm run bench:int0 -- --label <nom>` les refait en une commande (`scripts/bench/int0.mjs`, résultats dans `bench-results/`, tableaux en Markdown dans `bench-results/int0-<nom>.md`). Le **banc de flux** (`src-tauri/src/fluxbench.rs`, en release) rejoue par le vrai chemin du cœur (la ligne lue en JSON, `Core::on_frame`, ses effets appliqués, les événements de la fenêtre sérialisés par le canal de Tauri, les deltas par `conv_flusher`) les flux des cas de rejeu de chaque harness (`tests/fixtures/replays/<harness>/`) et un long tour réel de Claude Code 2.1.312 (`tests/fixtures/claude/long-turn.jsonl`, 3 371 frames en 138 s, enregistré par `scripts/bench/record-claude-turn.mjs`) : 20 fois au plus vite, puis une fois à leur rythme enregistré (les flux qui le disent ; un silence de plus de 2 s raccourci à 2 s). Une ligne qui ne fait aucun événement de la fenêtre (gardée par le cœur, réponse à une demande du cœur, message du client enregistré avec le flux) est comptée à part. Le CPU est celui du process, de la première ligne de chaque rejeu à son dernier événement (ce que la ligne déclenche compris : sauvegarde, stats ; le process au calme avant) ; la mémoire, celle du process de test entier (39 Mo au départ, 95 Mo de pic après les 5 700 rejeux, leurs agents gardés). Windows 11, i9-14900KF :

  | Flux, au plus vite | Lignes (×20) | Sans événement | Médiane | 99e centile | Lignes/s | Mo/s | CPU |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | Claude Code (3 cas) | 2 120 | 560 | 12 µs | 463 µs | 28 032 | 14,8 | 188 ms |
  | Codex (4) | 11 660 | 6 380 | 30 µs | 410 µs | 103 768 | 39,7 | 234 ms |
  | Copilot CLI (4) | 59 080 | 37 937 | 204 µs | 579 µs | 166 461 | 111,8 | 469 ms |
  | Cursor CLI (2) | 9 420 | 1 980 | 47 µs | 173 µs | 151 092 | 52,8 | 94 ms |
  | OpenCode (4) | 102 940 | 24 255 | 145 µs | 658 µs | 301 315 | 104,0 | 531 ms |
  | Mistral Vibe (6) | 18 040 | 3 325 | 84 µs | 734 µs | 90 079 | 32,2 | 172 ms |
  | Antigravity CLI (6) | 1 380 | 278 | 21 µs | 499 µs | 14 870 | 5,5 | 141 ms |
  | Gemini CLI (10) | 2 800 | 1 653 | 33 µs | 617 µs | 17 232 | 6,0 | 1 250 ms |
  | Le long tour de Claude Code | 67 420 | 1 960 | 4,2 ms | 7,8 ms | 342 948 | 119,8 | 672 ms |

  | Flux, à son rythme | Lignes | Sans événement | Médiane | 99e centile | CPU |
  | --- | --- | --- | --- | --- | --- |
  | Codex | 583 | 319 | 8,2 ms | 31,1 ms | 94 ms |
  | Cursor CLI | 471 | 99 | 21,4 ms | 32,0 ms | 31 ms |
  | OpenCode | 5 147 | 1 213 | 16,2 ms | 31,5 ms | 203 ms |
  | Mistral Vibe | 902 | 167 | 22,9 ms | 32,3 ms | 109 ms |
  | Le long tour de Claude Code | 3 371 | 98 | 24,0 ms | 32,3 ms | 469 ms |

  Ce qu'ils disent : la traduction et l'envoi d'une ligne prennent moins de 0,8 ms au 99e centile pour chaque harness ; le reste est le **regroupement des deltas** (`conv_flusher` les envoie 16 ms après le premier, des milliers de lignes fondues en une cinquantaine d'événements), d'où les 4,2 ms du long tour au plus vite et, au rythme réel, ~32 ms au 99e centile partout : 16 ms que le minuteur de Windows (~15,6 ms) arrondit vers le haut. Le plus fort flux réel enregistré passe 217 lignes en une seconde (le long tour ; 134 pour OpenCode) : le cœur en tient 68 (Antigravity) à 1 580 fois plus. Les seuils proposés plus haut deviennent : au plus 5 ms de plus au 99e centile, à mesurer au plus vite (moins de 0,8 ms aujourd'hui) comme à son rythme (~32 ms) ; un débit d'au moins 2 200 lignes/s par harness (dix fois 217) ; 30 Mo par process de plugin, à mesurer avec INT-9 (rien ne tourne encore en process).

  Le **démarrage** (`tests/e2e/startup-bench.spec.ts`, l'app réelle en build de debug, le faux Claude Code, 10 lancements sur le même dossier ; médiane, min – max) : jusqu'à la fenêtre (première peinture de la page) **621 ms** (459 – 1 251), 553 ms au premier lancement sur un dossier neuf ; **premier agent prêt** (le process du harness lancé → sa réponse à `initialize`, mesuré par le cœur, `ready in` d'app.log) **261 ms** (72 – 1 513, surtout le démarrage de Node du faux CLI) ; du message envoyé à la réponse affichée 236 ms (24 – 2 344 : le clic sur la carte lance parfois le process avant le message). **Une frappe dans une vue terminal** (`terminal-latency.spec.ts` au repos, la « Vue Claude Code » du faux CLI, 40 frappes) : de la touche à l'écho peint **13,4 ms** en médiane, 18,2 ms au 95e centile, 25,2 ms au plus.

  **Avec INT-9 (ESC-221, 2026-11-01), le premier harness en process.** `int9_measure_the_stream_through_the_plugin` (`plugins_harness_tests.rs`, ignoré, lancé à la main : `cargo nextest run --run-ignored only int9_measure --no-capture`) mesure le harness « Minimal » à travers son plugin, du moment où le harness écrit la ligne à celui où le cœur en reçoit les éléments, en build de debug sous Windows 11 : **au rythme réel** (une ligne toutes les 5 ms, 2 000 lignes) **0,36 ms en médiane, 0,66 ms au 99e centile** (seuil : 5 ms de plus) ; **au plus vite** (50 000 lignes d'un coup) **41 119 lignes/s** (seuil : 2 200 ; le délai est alors celui de la file, 216 ms en médiane) ; **le process du plugin : 14 Mo** (seuil : 30 Mo, lu par `Runtime::usage`). C'est dans les seuils. Chaque harness qui passe en process (INT-10) refait ces mesures et celles d'INT-0.

**2. Les terminaux et les vues TUI.** Le PTY reste au cœur (§4.2). Le plugin **décide du lancement**, rien de plus.

- **Passe par le process** : la description du lancement de la vue (`terminal/launch` : la commande, `--resume <id>`, les hooks à écrire, le dossier de config, les bandes de confiance à montrer, les variables à retirer comme `MSYSTEM` pour Cursor) ; la traduction des événements des hooks (`escouade-hook` les envoie au cœur, qui les passe au plugin par `terminal/hook` : quelques-uns par tour) ; le suivi d'un transcript quand le harness n'a pas de hook (Cursor, ESC-49), lu par le plugin qui rend des éléments.
- **Ne passe jamais par le process** : les octets du PTY vers xterm, les frappes, le redimensionnement, le copier-coller (le délai d'une frappe, mesuré de 15 à 40 ms en ESC-100 et ESC-104, ne bouge pas) ; les appels des agents au serveur MCP d'Escouade (le harness parle au cœur directement) ; le coffre entier (le plugin n'obtient que ses secrets) ; l'état d'un autre agent.

**3. Les essais réels**, à refaire à la migration de chaque harness, et notés dans son ticket : `real-harnesses.spec.ts` (ESC-52), `real_fence` (ESC-140), la vue terminal (`real_claude_view`, `real_codex_view`, `real_cursor_view`…), les modèles du compte (`real_claude_models`), la passation d'un harness à l'autre (ESC-88), le changement de compte au seuil (ESC-148). Ils demandent de vrais comptes : ceux que le mode auto refuse (ESC-12, ESC-110) se lancent à la main, et le ticket ne se ferme pas sans eux. Pour les sources et les hébergeurs : l'essai réel GitHub (`github_tests.rs`, par `gh`) à chaque migration, et le faux service pour les autres, inchangé.

**4. Le démarrage de l'app.**

- **Rien de lancé avant la fenêtre.** Ce qu'il faut à l'écran d'accueil est **déclaré** dans les manifestes et lu sans process : les harness trouvés (`locate` déclaratif), leurs capacités, les hôtes des hébergeurs (pour reconnaître un remote), les champs des comptes.
- **Démarrage paresseux** ensuite : une source à sa première synchro, un hébergeur à la première PR à suivre, un harness au premier agent qui reprend ou à la première ouverture du menu des modèles. Les agents qui reprennent au démarrage lancent leur plugin en parallèle, pas l'un après l'autre.
- **Le coût** : un process `escouade-plugins` démarre en quelques millisecondes (un exécutable Rust, pas un runtime) ; INT-0 mesure le temps jusqu'à la fenêtre et jusqu'au premier agent prêt, avant et après. Seuil proposé : la fenêtre au même temps qu'aujourd'hui, le premier agent prêt à moins de 100 ms de plus. Avant (INT-0, 2026-10-30, build de debug) : la fenêtre en 621 ms, le premier agent prêt en 261 ms (médianes ; détail au point 1).

**5. Un plugin de première main qui plante.**

- **La relance** : celle de PLG-2, trois fois en cinq minutes, puis le plugin est arrêté.
- **L'agent, pendant la relance** : son harness vit toujours (le cœur l'a lancé). Le cœur garde les frames reçues pendant la relance (une file bornée), relance le plugin, lui redonne l'agent (`agent/attach` : sa session, son mode, son modèle, les demandes en attente que le cœur connaît) et lui rejoue les frames gardées. Ce qui peut se perdre : l'état de traduction en cours (un outil commencé avant le plantage reste « en cours » jusqu'à la fin du tour, où le cœur le ferme). La conversation dit « Le plugin Claude Code a redémarré », sans autre gêne.
- **Après le troisième plantage** : les agents de ce harness s'arrêtent proprement (interruption, puis fin du Job), passent en erreur avec « Le plugin Claude Code s'est arrêté après trois plantages », et un bouton « Relancer » ; leur session est gardée, et le message suivant les reprend (`--resume`) une fois le plugin relancé. Une notification le dit une fois, le journal du plugin garde le détail (stderr, dernier message).
- **Pour une source** : la synchro se met en pause, la carte et l'onglet Intégrations disent « GitHub Issues : arrêté », la synchro reprend au prochain passage. **Pour un hébergeur** : l'état des PR passe à « inconnu », sans être perdu, jusqu'à la relance.
- **Les tests** : le faux plugin de PLG-2 sait planter ; chaque adaptateur a son test de plantage (en plein tour pour un harness, en pleine synchro pour une source).

**6. La sécurité.** Un plugin de première main en process a les droits de l'utilisateur, comme aujourd'hui le cœur. Ce qui change : il reçoit, dans son process, les secrets de ses comptes et le jeton du serveur MCP de ses agents (que le harness reçoit déjà) ; rien d'autre. Le cœur ne lance que l'`escouade-plugins` de son propre dossier, signé avec l'app.

### 4.7 Ce qui doit sortir du cœur pour ça

Les énumérations fermées à ouvrir :

- **`EngineId`** (`model.rs`, énumération `Claude`, `Codex`, `Copilot`, `Cursor`, `Opencode`, `Vibe`, `Agy`, `Gemini`, et `Bench` en test), gardé dans les agents (`AgentMeta::engine`), les tickets, les projets et la colonne `engine` des statistiques. Il devient un id ouvert (une chaîne), **sérialisé à l'identique** (`claude`, `codex`…) : aucune donnée à migrer. Les `match` sur `EngineId` (une trentaine de fichiers hors tests) passent par le registre et les capacités (`EngineCaps`), comme l'interface le fait déjà.
- **`engine::registry::all()`**, une liste écrite en dur des huit harness : construite depuis les plugins actifs. _Depuis INT-3 : `registry::BUILT_
- **`Service`** (`model.rs` : `Jira`, `Trello`, `Github`, `Linear`, `Clickup`, `Gitlab`, `Azure`), gardé dans `ExternalRef` de chaque ticket importé : un id ouvert, sérialisé à l'identique. Côté fenêtre : le type `Service`, `SERVICE_IDS` et les noms courts (`SHORT`) de `src/lib/integrations.ts`, lus depuis le registre. _Depuis INT-1 : `Service` est un id interné comme `EngineId` (`Service::JIRA`…, `Service::new`), le registre `integrations::source::registry()` dit s’il est connu, et la fenêtre lit `InitialState::sources` (`setSources`)._
- **`integrations::Client`** (`integrations/mod.rs`, une énumération qui distribue chaque appel par `match`) : les traits `TicketSource` et `TicketSourceKind`, et un registre, comme `forge::Registry`. Les champs de `integrations::Account` propres à un service (`site`, `email`, `key`) deviennent des champs déclarés par le genre de compte. _Depuis INT-1 : `Client` n'existe plus ; `TicketSource` (les cinq appels, `me` et `states_view`), `TicketSourceKind` (id, nom, nom court, hébergeur, compte propre, construction) et `Registry` sont dans `integrations/source.rs` ; les champs du compte restent à INT-6._ _Depuis INT-6 : les champs de chaque genre de compte sont déclarés (`AccountKind::fields`) ; `integrations::Account` les garde encore pour les sources du cœur, jusqu'à leur sortie (INT-7, INT-8)._ _Depuis INT-1 : `Client` n’existe plus ; `TicketSource` (`me`, `containers`, `states`, `states_view`, `issues`, `set_state`, `comment`), `TicketSourceKind` (id, nom, nom court, code host, compte propre, construction) et `Registry` sont dans `integrations/source.rs` ; les champs du compte restent à INT-6._
- **`forge::Registry::standard()`** (`forge/mod.rs`), écrit en dur : construit depuis les plugins. `Provider::kind()` et `scopes()` rendent des `&'static str` : à passer en chaînes possédées ; `CliToken::parse` et `Probe::matches` sont des pointeurs de fonctions : à rendre déclaratifs (une commande et le champ lu ; un chemin et la clé attendue). _Depuis INT-2 : `Registry::from_providers(liste)` (`standard()` en part), `kind()` rend une `String` et `scopes()` un `Vec<String>`, `CliToken.read` (`CliRead` : `Stdout`, `StdoutWord`, `After(étiquette)`) et `Probe` (chemin, statuts, statut et clé attendus) sont des données._

Ce qui se sépare :

- **la traduction des harness et l'état des agents** : les méthodes `claude_frame`, `codex_*`, `copilot_*`, `cursor_*`, `opencode_*` d'`AgentRt` deviennent des traducteurs à état propre qui rendent des éléments et des effets (§4.6, 1) ;
- **`Engine::spawn`** devient `engine/launch` (une description) plus le lancement par le cœur ;
- **les textes de chaque intégration** écrits dans le cœur (par exemple ceux de `sync.rs` : « GitHub : donne un jeton, ou connecte la CLI gh ») vont dans le catalogue de son plugin.

Les traits qui deviennent le contrat : `Engine` et `EngineProcess` (`engine/mod.rs`), `Forge` et `Provider` (`forge/mod.rs`), `TicketSource` et `AccountKind` (nouveaux), implémentés dans le cœur par les adaptateurs en process. `SecretStore` reste un service du cœur, que les plugins utilisent par `secrets/get`. Les faux (`integrations/fake.rs`, `fake-trackers.mjs`, `fake-gh`, les `fake-<harness>.mjs`) restent où ils sont : ils simulent le service ou le harness, pas le plugin, et servent tels quels aux tests de chaque plugin.

### 4.8 L'ordre, une étape livrable à la fois

Chaque étape est livrable seule, garde ses tests et ses essais réels inchangés, et ne migre qu'une intégration à la fois. L'ordre va du plus simple au plus risqué ; **le transport en process (PLG-2) et le banc (PLG-3) passent avant toute migration de première main**.

0. **Les mesures de référence** (INT-0), avant tout : le flux de chaque harness, le démarrage, la mémoire. Ce sont les chiffres « avant ».
1. **Les contrats dans le cœur**, sans rien sortir : `TicketSource` et `Service` ouvert (les sept sources derrière, sans changer de comportement), `forge::Registry` ouvert, `EngineId` ouvert. Des refactors, livrables en 2.0.x ou avec la 2.1.0.
2. **Le socle** : le manifeste (PLG-1), **le transport en process (PLG-2)** et **le banc (PLG-3)** ; le registre des plugins de première main et l'exécutable `escouade-plugins` empaqueté et signé, d'abord vide ; les adaptateurs en process des sources et des hébergeurs, essayés sur le faux plugin.
3. **Les comptes partagés** : un genre de compte par plugin ; l'exception de github.com (`Accounts::github`) devient la règle. Migration des comptes enregistrés (`integrations.json`) et des noms au coffre, réversible. _Fait le 2026-11-01 (INT-6, ESC-212, §4.4)._
4. **Les sources du Kanban en process**, une par ticket : Trello, Linear, Jira, ClickUp. Le plus simple : quelques appels HTTP, des types sérialisables. GitHub Issues, GitLab Issues et Azure Boards sortent avec leur hébergeur (étape 5).
5. **Les hébergeurs en process**, un par ticket : **GitHub d'abord, avec ses Issues** (le plugin « GitHub » complet ; _fait le 2026-11-01, INT-8.1, §4.4_), puis GitLab avec ses Issues (ESC-56 et ESC-165), Bitbucket, Gitea, et Azure DevOps avec Azure Boards (INT-8.5, ESC-280, ESC-241 et 242).
6. **Le contrat des harness en process** : la traduction séparée de l'état, `engine/launch`, les lots, `agent/attach`, la vue terminal ; prouvé sur le faux moteur minimal (`engine/minimal/`) devenu un plugin, mesuré contre INT-0.
7. **Les harness en process**, un par ticket, du moins utilisé au plus utilisé : Cursor CLI (avec le plugin ACP générique de PLG-6), Mistral Vibe (INT-10.6, ESC-281), Gemini CLI (INT-10.7, ESC-282), OpenCode, Copilot CLI, Codex, Antigravity CLI (INT-10.8, ESC-283), **Claude Code en dernier**. Chacun mesuré avant et après, banc, e2e et essais réels rejoués.
8. **Les services de voix en ligne et les environnements de lancement** (revue du 2026-10-29, §4.12) : le contrat de la voix (INT-12, ESC-284), puis OpenAI, Groq, Mistral, ElevenLabs et Deepgram en plugins (INT-13, ESC-285) ; le point « environnement de lancement », isola en premier plugin (INT-14, ESC-286).
9. **Plus tard** : les plugins tiers en WASM (les sources d'abord, 2.2) et un harness tiers par `engine/*` documenté (2.3).

### 4.9 L'effet sur les tickets en cours ou à venir

| Tickets | Effet | Décidé |
| --- | --- | --- |
| ESC-201, ClickUp (en cours) | Il ajoute une variante à `Service` et à `integrations::Client`, comme Linear. | **Fini tel quel** : l'étape 1 le passe derrière le contrat avec les autres, l'étape 4 le sort en process. |
| ESC-165, GitLab Issues | Une source de plus, sur les comptes de l'hébergeur GitLab (`forge::accounts`) : une variante de `Service` et de `integrations::Client`, comme ClickUp. | **Fait dans le cœur pour la 2.0.3** (décidé le 2026-10-26, à la demande d'un testeur) : l'étape 1 le passe derrière le contrat avec les autres, l'étape 5 le sort avec le plugin « GitLab » (INT-8.2, ESC-218), son compte déjà partagé avec l'hébergeur. |
| ESC-56, 57, 58, clients GitLab, Bitbucket, Gitea | Le trait `Forge` existe ; les `Provider` sont écrits. | **Écrits directement dans la crate de leur plugin** si l'étape 2 est faite ; sinon dans le cœur, et sortis à l'étape 5. Pas d'attente. |
| ESC-241, Azure DevOps | Un cinquième `Provider` et son client complet (`forge/azure.rs`, `azure_api.rs`) ; Azure Boards suit sur le même compte. | **Fait dans le cœur** (demandé le 2026-10-27) ; sorti en plugin de première main avec Azure Boards (INT-8.5, ESC-280, fait le 2026-11-01), ses tests inchangés. |
| ESC-242, Azure Boards | Une source de plus, sur les comptes de l'hébergeur Azure DevOps (`forge::accounts`) et par son client (`forge::azure_api::Azure::call`) : une variante de `Service` et de `integrations::Client`, comme GitLab Issues. | **Fait dans le cœur** (demandé le 2026-10-27) ; l'étape 1 le passe derrière le contrat avec les autres, l'étape 5 le sort avec le plugin « Azure DevOps » (INT-8.5, ESC-280, fait le 2026-11-01), comme GitLab Issues avec GitLab, ses tests et son essai réel `real_azure_boards` inchangés. |
| ESC-59 à 63, 65 à 68, PR, CI, relectures, déploiements, vues | Ce sont des fonctions du cœur qui appellent `Forge`. | **Inchangés** : ils restent au cœur et marchent pour tout plugin d'hébergeur. |
| ESC-69, 70, docs et essais des hébergeurs | Les pages parlent des plugins. | La page « Hébergeurs » dit qu'ils sont des plugins de première main ; l'essai réel passe aussi par « plugin arrêté puis relancé ». |
| ESC-53, parcours enregistrés | — | Inchangé. |
| ESC-172 à 184, PLG | Le socle porte maintenant les intégrations d'Escouade. | Modifiés, ticket par ticket : `docs/TODO-2.2.md`, « Les tickets PLG à modifier ». |
| ESC-36, moteur ACP générique | Doublon de PLG-6. | **À fermer**, renvoyé à ESC-178. |
| ESC-202, place de marché (en cours) | — | Le store refuse les ids `dev.escouade.*` à tout autre dépôt que celui de l'app (ESC-314) ; l'app montre « Livré avec Escouade » à côté de « Vérifié par Escouade ». |

**Les quatre exemples deviennent de première main (ESC-314, 2026-10-31).** `dev.escouade.high-contrast`, `dev.escouade.discord`, `dev.escouade.slack` et `dev.escouade.team-rules`, en 1.1.0, vivent dans `plugins/<id-court>/` du dépôt de l'app (décision 8 de `docs/TODO-2.2.md`) : chacun son manifeste (`repository` = `…/tree/main/plugins/<id-court>`), son README, sa LICENSE MIT (ligne `SPDX-License-Identifier`, la racine étant sous GPL-3.0), son CHANGELOG et ses tests (`node --test` pour les deux déclaratifs : les contrastes WCAG du thème, les webhooks de Discord). Ils ne sont pas livrés avec l'app : ils passent par le store comme un tiers, soumis depuis leur dossier à leurs étiquettes `plugins/<id-court>/v1.2.3` (décision 21, point 7), revus et signés par le chef. `plugin-examples/` disparaît ; `plugins/ci.mjs` les vérifie en CI Linux.

### 4.10 La place de marché : ce qu'un plugin tiers peut de plus, et de moins

Avec « tout en process », un plugin tiers et un plugin de première main ont **le même transport et le même contrat**. Ce qui les sépare est la confiance, pas la technique.

**De plus qu'un plugin de première main :**

- sortir à son rythme : une API de service qui change se corrige sans attendre une release d'Escouade ;
- servir ce qu'Escouade ne tiendra pas : YouTrack, Asana, un Jira Data Center derrière un proxy, une forge d'entreprise, un harness ACP de plus ;
- s'écrire dans n'importe quel langage (TypeScript avec le SDK, PLG-9).

**De moins :**

- **activé d'office nulle part dans un projet privé** (§3.3), là où un plugin de première main l'est ;
- **un harness tiers** passe d'abord par l'ACP décrit (PLG-6) ; `engine/*` lui sera ouvert quand il aura été tenu par les huit harness d'Escouade et documenté (2.3), pas avant ;
- **ses secrets seulement**, jamais ceux d'un plugin de première main, même pour le même service ;
- **ni les ids ni les comptes d'un plugin de première main** ;
- **pas de lancement sans accord** : sa fenêtre d'installation dit « Lance un programme sur votre ordinateur, avec vos droits » (§3.4) ; celle d'un plugin de première main n'existe pas, il vient avec l'app ;
- **son badge** est « Vérifié par Escouade » s'il est passé par la revue (ESC-202), « non vérifié » sinon ; jamais « Livré avec Escouade ».

### 4.11 Les décisions (2026-10-26)

Posées par le chef d'escouade, répondues par l'utilisateur le 2026-10-26 :

1. **L'architecture des intégrations : tout en process.** Les plugins de première main (Claude Code, Codex, Copilot CLI, Cursor CLI, OpenCode, GitHub, Jira, Trello, Linear, ClickUp…) deviennent des process JSON-RPC à part, comme ceux des tiers : un seul chemin, un seul contrat. Ils restent livrés et signés avec l'app. _La recommandation était autre_ (le Rust intégré pour la première main, le process pour les tiers) ; ce qu'il en coûte, et comment on le tient : §4.6.
2. **L'ordre** : sources, comptes partagés, hébergeurs, transport, harness en dernier ; revu là où le process l'impose : les mesures de référence d'abord, puis le transport (PLG-2) et le banc (PLG-3) avant toute migration (§4.8). _Retenu._
3. **Ce qui reste dans le cœur** : le tableau du §4.2, la synchro, le flux des PR et le PTY compris. _Retenu_ ; avec le process, le lancement des harness y reste aussi.
4. **Un compte par plugin, partagé par ses points.** _Retenu._
5. **Les plugins de première main désactivables, non désinstallables, activés partout par défaut, projets privés compris.** _Retenu._
6. **Remplacer un plugin de première main par un tiers** : permis, compte par compte, jamais en silence. _Retenu._
7. **Les tickets en cours** : ESC-201 fini tel quel ; ESC-165 réécrit en plugin « GitLab » ; ESC-56 à 58 écrits dans la crate de leur plugin dès le socle, sinon sortis plus tard ; ESC-36 fermé (§4.9). _Retenu._
8. **Les plugins de première main restent dans le dépôt de l'app, chacun lisible seul** (2026-10-29, sur la question de l'utilisateur « tu as bien prévu de les passer sur des dépôts séparés ? ») : un dossier `plugins/<id>/` avec son manifeste, son README et ses tests, vers lequel pointe sa page du store ; pas de dépôts séparés, qui demanderaient une PR par dépôt à chaque changement du protocole et une livraison signée à part. Posé dans INT-4 (ESC-210), suivi par chaque sortie (INT-7 à INT-14). _Retenu._

À confirmer avec l'utilisateur, conséquence de la décision 1 : **PLG-6 (ESC-178) sort de la 2.1.0**. L'adaptateur ACP générique s'écrit directement en plugin de première main, sur le contrat des harness (étape 6), plutôt que dans le cœur puis à nouveau dehors.

### 4.12 La revue du 2026-10-29

Demandée par l'utilisateur après la sortie de la 2.1.0 : « revérifie qu'il te manque rien et que tu as bien planifié l'implémentation du site de la marketplace, le lien avec Escouade, et le passage en plugins de toutes les features aujourd'hui intégrées ». Le plan du 2026-10-26 a été confronté au code de la 2.1.0 et aux 46 tickets des plugins, des intégrations et du store. Les décisions du §4.11 tiennent ; voici ce qui manquait, et ce qui est tranché.

**Ce qui manquait au passage en plugins**, construit depuis le plan, ou oublié, a maintenant son ticket :

| Étape | Ce qu'elle sort | Ticket | Après |
|---|---|---|---|
| INT-8.5 | Azure DevOps et Azure Boards (ESC-241, 242), ensemble, comme GitLab et GitLab Issues | ESC-280 | INT-5, INT-6, INT-8.4 |
| INT-10.6 | Mistral Vibe, sur le client ACP générique de PLG-6 | ESC-281 | INT-9, PLG-6, INT-10.1 |
| INT-10.7 | Gemini CLI, sur le même client ACP | ESC-282 | INT-10.6 |
| INT-10.8 | Antigravity CLI, sur le contrat des harness (stream-json, pas d'ACP), avant Claude Code | ESC-283 | INT-9, INT-10.4 |
| INT-12 | Le contrat des services de voix en ligne : dictée, direct, synthèse, catalogue, compte | ESC-284 | INT-0, INT-4, INT-5 |
| INT-13 | OpenAI, Groq, Mistral, ElevenLabs et Deepgram en plugins | ESC-285 | INT-12, INT-6 |
| INT-14 | Le point « environnement de lancement » (`up`, `ls`, `down`, `destroy`), isola en premier plugin | ESC-286 | INT-4, INT-5 |

L'ordre des harness devient : Cursor CLI, Mistral Vibe, Gemini CLI, OpenCode, Copilot CLI, Codex, Antigravity CLI, Claude Code.

**Ce qui manquait au store et au lien avec Escouade** :
- le lien profond `escouade://plugin/<id>` (STORE-11, ESC-287), qui ouvre une page et n'installe jamais rien ;
- les plugins proposés par un projet, `plugins.recommended` de `.escouade.yml` (PLG-14, ESC-288) ;
- le reste rejoint les tickets du store : la compatibilité, les limites de débit et la copie de la liste de révocation dans STORE-6 (ESC-233) ; la dernière version compatible, Réglages › Réseau, la télémétrie avec l'accord et le passage d'une installation par git au store dans STORE-9 (ESC-236) ; les pages des plugins de première main, les catégories et les signalements ouverts à tous dans STORE-4 (ESC-231).

**Les décisions de la revue** (le chef d'escouade, à confirmer par l'utilisateur s'il le veut autrement) :
1. **Wispr Flow (ESC-164) s'écrit en plugin**, sur le contrat de la voix (INT-12), pas dans le cœur : la règle « tout l'externe en plugins » s'applique aux demandes d'avant elle.
2. **Whisper, Piper et le modèle d'embeddings de l'index restent au cœur** (§4.2) : ils tournent hors ligne, sans compte, et ne parlent à aucun service une fois téléchargés.
3. **Le client ACP de Cursor devient générique dans PLG-6**, avant toute migration de harness ACP : Vibe et Gemini l'appellent déjà (`engine/vibe/process.rs`, `engine/gemini/mod.rs`), sortir Cursor seul les casserait.
4. **Le contrat des harness (INT-9) porte aussi** `read_only` (une garantie de sécurité que le cœur vérifie sur la description de lancement, comme la clôture), la réponse aux hooks, la reprise de la mémoire et `accounts_by_key`.
5. **Les plugins de première main ont une page sur le store**, avec la marque « Escouade » ; ils s'installent avec l'app, et leurs ids `dev.escouade.*` restent réservés.
6. **« Signaler » est ouvert à tous** sur le store, par un formulaire avec une adresse e-mail (DSA) ; publier demande toujours un compte GitHub.
7. **L'exemple « harness ACP décrit » de PLG-10 passe à PLG-6**, sur Goose, puisque Gemini CLI est intégré : les exemples et le store n'attendent plus le contrat des harness.
8. **Le compte Mistral**, partagé par Vibe et la voix, est tranché dans INT-13, avec INT-6.

**Les nouveaux points d'extension que suggère la 2.1.0**, pour après les étapes ci-dessus, sans ticket encore :
- les **lecteurs de documents** (`docsextract.rs` lit texte, PDF, docx, odt ; un plugin ajouterait l'OCR, pptx, xlsx) : sans réseau, le premier candidat au WASM ;
- les **sources de documents** (Confluence, Drive, Notion), que `docs/FACTORY.md` renvoie déjà aux plugins ;
- les **faits** des Docs, de la mémoire, des PR et des comptes sur le bus (ajoutés à PLG-5, ESC-177).

Les docs elles-mêmes (§1.1, §1.5, §4.7, §4.8, §4.10, le tableau des versions de `docs/TODO-2.2.md`, `docs/STORE.md` §5.1) sont remises d'accord avec la 2.1.0 par ESC-279, avant INT-0.

### 4.13 Les services de voix en plugins (INT-13, ESC-285, fait le 2026-11-01)

**Ce qui est livré.** OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont cinq plugins de première main (`plugins/builtin/dev.escouade.<service>/` : manifeste, README), au registre des plugins livrés avec l'app (`plugins/builtin.rs`), chacun un process `escouade-plugins voice-<service>` (`src-tauri/src/bin/escouade-plugins/voice.rs`). Le cœur ne les appelle que par le contrat d'INT-12 : `ProcessVoiceService: VoiceService` (`voice/plugin_host.rs`, sur le modèle de `ProcessTicketSource`) répond à la dictée, au direct, à la synthèse et au catalogue par JSON-RPC ; le côté plugin (`voice/plugin_guest.rs`) sert `voice/*` avec le code des cinq services tel qu'il était dans le cœur (`online.rs`, `live.rs`, `online_speech.rs`, `online_catalog.rs`), sans le changer. **Écart assumé avec §4.3** : le programme `escouade-plugins` n'est donc plus sans `escouade_lib` pour ces cinq entrées. Sortir ce code dans une crate à part (la forme de §4.3) demanderait de défaire son lien avec les réglages, le trousseau et la conversation, pour un gain de taille du seul exécutable ; elle reste à faire quand INT-10 aura fixé la forme des crates.

**Le contrat s'est ouvert** (revue d'INT-12) : `Provider` n'est plus une énumération mais l'identifiant du service, interné (`Provider::of(id)`, `Copy`) ; les cinq du cœur en sont les constantes (`Provider::OpenAi`…) et ni `Spec` ni `OnlineError` n'en dépendent plus. Le registre (`voice/service.rs`) liste aussi les services des plugins : `register`, `unregister`, `hide`, `services`, `find`, `of`. Un plugin d'un id du cœur prend sa place ; un service dont le plugin est parti répond par une erreur (`Gone`), jamais par un plantage. `core.rs` met le registre d'accord avec les interrupteurs des plugins (`plugin_host::sync_builtin`) au démarrage et à chaque changement : un plugin coupé retire son service (la dictée repasse en local), un plugin qui ne démarre pas laisse le service du cœur répondre, le journal du plugin disant pourquoi.

**Le protocole `voice/*`** (`voice/wire.rs`), de l'app vers le plugin : `voice/describe` (id, nom, modèle par défaut, direct ou non, modèle du direct, longueur maximale d'un passage), `voice/transcribe` (le son en flottants 32 bits, `Spec` : modèle, clé, adresse, réseau), `voice/live/open`, `voice/live/audio` (notification), `voice/live/finish` (le texte définitif), `voice/live/close`, `voice/speak` (le son revient par la notification `voice/audio`, en 16 bits), `voice/catalog`, `voice/list`, `voice/cancel` (notification : un appel que `cancel` a coupé). Du plugin vers l'app : `voice/partial` (le texte compris jusqu'ici) et `voice/audio`. Les notifications d'un plugin que le journal ne connaît pas vont à un écouteur (`Runtime::on_notification`), dans l'ordre où le plugin les a écrites. Une erreur du service traverse en refus JSON-RPC (codes `-32101` à `-32106` : pas de clé, clé refusée, réseau, service, quota, voix) et revient la même `OnlineError`. La clé reste au trousseau de l'app (`dictation-<service>`, l'entrée d'avant) et part dans chaque appel, vers le seul plugin de ce service, qui ne la garde ni ne l'écrit.

**Le compte Mistral (tranché avec INT-6).** Un plugin « Mistral » porte la voix et, plus tard, Mistral Vibe (INT-10.6), comme « GitHub » porte les Issues et les PR : une seule clé, qui sert tous les points du plugin (la règle de §4.4, un compte par plugin). Tant que Vibe n'est pas en plugin, sa clé reste la sienne (`accounts.rs`, par compte) et la voix lit `dictation-mistral`. À INT-10.6, le plugin `dev.escouade.mistral` déclarera le compte (`contributes.accounts`) et `dictation-mistral` deviendra l'ancien nom lu d'abord (la migration réversible d'INT-6), de sorte que personne n'ait à ressaisir sa clé. Les quatre autres services ont chacun leur plugin et leur clé.

**Les mesures** (`voice/plugin_tests.rs`, `cargo nextest run --run-ignored only int13_measure --no-capture`, faux serveurs WebSocket, Windows 11, build de debug, 20 phrases de 0,3 s par service, le son poussé au rythme du micro : une trame de 20 ms toutes les 20 ms) : le délai du texte final après la fin de la phrase, en médiane et au 99e centile. Dans le cœur : 0,5 ms (p99 0,7) pour OpenAI, Mistral et ElevenLabs, 0,6 ms (p99 0,9) pour Deepgram. **Par le plugin** : 1,0 ms (p99 1,4), 1,0 (1,6), 1,1 (1,5) et 1,2 (1,7) : **moins de 1 ms de plus en médiane comme au 99e centile**, sous le seuil d'INT-0 (5 ms). Poussé d'un coup (une demi-seconde de son sans attendre, ce que le micro ne fait pas), le plugin ajoute 8 à 10 ms en médiane et 20 à 60 ms au 99e centile : le temps d'encoder, de passer le tuyau et de décoder les 25 trames en attente. Le seuil est la mesure à la main ; la CI ne garde que le passage du direct par le plugin, avec des bornes larges (`the_live_transcription_goes_through_the_plugin_and_says_what_the_core_says`).

**L'essai réel ElevenLabs, rejoué par le plugin** (`int13_real_elevenlabs_through_the_plugin`, la clé de la machine donnée à ce seul process) : la dictée d'une phrase française en 1,0 s et une mauvaise clé refusée ; le direct (textes partiels pendant que la phrase se dit, définitif 165 ms après la fin) ; une phrase lue (premier son en 2,5 s, 34 morceaux) ; 9 modèles et 31 voix listés.

**Wispr Flow (ESC-164) n'est pas à faire** pour l'instant (décision de l'utilisateur du 2026-11-01) : il s'écrira, le jour venu, comme un sixième plugin sur ce protocole.

## 5. Les programmes en JS/TS sans Node : un runtime WASM (ESC-319)

Demandé par l'utilisateur le 2026-11-01, en apprenant qu'un plugin en TypeScript (`dev.escouade.slack`, `dev.escouade.team-rules`) se lance par `node dist/plugin.mjs` : « je veux pas rendre node obligatoire, donc soit tu les exécutes dans la partie navigateur de tauri, soit tu prends en compte wasm directement et chaque plugin ts qui est soumis sera buildé en wasm ».

Ce chapitre est l'étude, avec un prototype mesuré (`prototypes/wasm-plugins/`, son README dit comment refaire chaque mesure). Il ne change pas encore l'app : les tickets du §5.9 le font, dans l'ordre. Il révise la recommandation D du §2.4 (voir la révision en tête du §2.4).

**En bref.** Un plugin en JS ou en TS se publie comme aujourd'hui, son bundle commité (`dist/plugin.mjs`). Le store le compile en WASM avec **Javy** et le **plugin Javy d'Escouade** (`escouade-js`, QuickJS), dans un conteneur sans réseau, avec des outils épinglés ; deux compilations, sous Windows comme sous Linux, donnent le même module à l'octet. L'app le fait tourner dans son propre process par **wasmtime**, une instance par plugin qui vit comme le process d'avant. Le protocole ne change pas : les mêmes lignes JSON-RPC, portées par l'entrée et la sortie standard du module. Escouade sert en plus le réseau (`net/fetch`, seulement vers `permissions.network`) et les fichiers du dossier du plugin. Les `dist/plugin.mjs` de team-rules et de slack tournent ainsi **sans aucune retouche** : team-rules décide en 71 µs (médiane), slack poste par le `fetch` de l'hôte et se voit refuser tout autre hôte.

### 5.1 Pourquoi pas la webview

La webview a été écartée avant l'étude, pour faire tourner la logique des plugins :

- elle vit et meurt avec la fenêtre, alors qu'Escouade tourne aussi fenêtre cachée, dans le plateau (§2.3, C) ;
- elle n'existe ni dans le mode serveur (`docs/REMOTE.md`) ni dans le banc en ligne de commande (`escouade-plugin test`), ni dans la revue du store ;
- elle partage son moteur et l'IPC de Tauri avec l'interface : un plugin y côtoie ce qui peut tout faire (CVE-2024-35222 pour une iframe) ;
- il y a trois moteurs selon le système (WebView2, WKWebView, WebKitGTK et ses plantages connus, ESC-290) : trois comportements à essayer, trois isolations différentes ;
- une liste d'hôtes **par plugin** y est difficile à tenir : la CSP est celle de la page, pas celle d'un plugin.

### 5.2 Les candidats, vérifiés le 2026-11-01

| | Javy | Extism | componentize-js |
|---|---|---|---|
| Ce que c'est | un compilateur JS → WASM de la Bytecode Alliance : QuickJS (rquickjs) précompilé par Wizer, un module WASI preview 1 | un cadre de plugins : un hôte Rust (crate `extism`, sur wasmtime) et des PDK ; celui de JS compile par QuickJS-ng, Wizer et binaryen | un module ESM → un composant du Component Model, SpiderMonkey par StarlingMonkey, interface en WIT |
| Version | **javy 9.1.0** (CLI, 2026-07-30) ; `javy-plugin-api` 7.1.0 ; `javy-codegen` 4.0.0 (2026-03-22, sur wasmtime 42, en retard sur la CLI) | **extism 1.41.0** (2026-10-09, sur wasmtime 49) ; **js-pdk 1.7.0** (2026-08-17 ; son binaire se dit « 1.6.1 ») | **componentize-js 0.23.0** (2026-09-21) ; StarlingMonkey v0.4.0 (2026-10-26) ; jco |
| Dépôt, licence | <https://github.com/bytecodealliance/javy>, Apache-2.0, 2 775 étoiles, poussé le 2026-10-30 | <https://github.com/extism/extism>, <https://github.com/extism/js-pdk>, BSD-3-Clause, 5 822 et 94 étoiles | <https://github.com/bytecodealliance/ComponentizeJS>, <https://github.com/bytecodealliance/StarlingMonkey>, Apache-2.0, 399 et 295 étoiles |
| Docs lues | `docs/docs-using-exports.md`, `-dynamic-linking.md`, `-extending.md`, `-invoking.md`, `-js-api-support.md` | le README du js-pdk (« Synchronous execution model… no `setTimeout`… `fetch()` … completes synchronously »), <https://extism.org/docs/concepts/runtime-apis> | le README (« This is an experimental project, no guarantees are provided for stability ») |
| Ce qu'il faut pour compiler | un exécutable (31,7 Mo Windows, 45,6 Mo Linux, glibc ≥ 2.35) ; **pas de Node** | `extism-js` (16,6 Mo) **et** `wasm-merge`, `wasm-opt` de binaryen (version_133) ; pas de Node | **Node** et le paquet npm (343 Mo de `node_modules` : jco, wizer, weval, le moteur de 10,5 Mo) |
| L'asynchrone | les promesses, avec la boucle d'événements de Javy ; pas de minuteur ; l'hôte n'a pas de fonction asynchrone : le réseau passe par le protocole | synchrone : une promesse ne se résout que sur une valeur déjà là ; `fetch` est fait par l'hôte, bloquant, avec ses `allowed_hosts` | SpiderMonkey complet, `fetch` par `wasi:http` (coupé ici) |

Ce que les essais ont appris, au-delà des docs :

- **Javy, tel quel, ne sait pas vivre longtemps.** Ses exports (`-C wit`) sont faits pour un appel : chacun copie le bytecode d'un segment de données puis le jette (`data.drop`), si bien qu'un deuxième appel lit un segment jeté et s'arrête (« out of bounds memory access »). Et chaque appel évalue à nouveau le module, que QuickJS ne libère jamais : en retirant les `data.drop`, l'instance a tenu environ 1 500 décisions avant de manquer de mémoire à 64 Mio. **D'où `escouade-js`** (`prototypes/wasm-plugins/escouade-js/`) : un plugin Javy écrit avec `javy-plugin-api` 7.1, le QuickJS de Javy avec la boucle d'événements, `TextEncoder` et `Javy.IO`, plus un export, `escouade-turn`, qui appelle `globalThis.__esc.pump()` puis fait tourner les tâches en attente, sans rien évaluer à nouveau. Le module est évalué une fois par `_start`. 24 000 appels plus tard, sa mémoire n'a pas bougé d'un octet.
- Le construire sous Windows demande deux contournements : bindgen de `rquickjs-sys` ne trouve pas les en-têtes du clang du WASI SDK qu'il télécharge (`BINDGEN_EXTRA_CLANG_ARGS=-I<wasi-sdk>/lib/clang/18/include`), et `javy init-plugin` lit la section `target_features` du module (garder `strip = false`, sinon binaryen refuse la mémoire en bloc et le SIMD). Le script `build-escouade-js.sh` les fait.
- **Extism garde ses instances** sans rien de tel : ses exports s'appellent autant de fois qu'on veut. Mais ses deux compilations du même bundle **diffèrent** (de 13 octets en taille, d'un million d'octets en contenu) : rien à comparer à la revue.
- **componentize-js** produit un composant de 12 Mo, différent à chaque compilation, et ne compile que sous Node : ce que l'utilisateur veut éviter, au moins chez l'auteur (`escouade-plugin dev`).

### 5.3 Les mesures

Sous Windows 11 (la machine de développement, chargée par d'autres agents d'Escouade), hôtes construits au profil release de l'app (`lto`, `codegen-units = 1`, `strip`), wasmtime 50.0.1 pour Javy et componentize-js, 49 pour Extism. Les mêmes `dist/plugin.mjs`, le même prélude, les mêmes lignes ; 2 000 appels par mesure. Node 24.14 pour la référence, le process d'aujourd'hui.

| | Javy + `escouade-js` | Extism | componentize-js | Node (aujourd'hui) |
|---|---|---|---|---|
| **Poids ajouté à l'app** (exécutable minimal contre un témoin de 2,10 Mo avec les mêmes crates) | **+13,3 Mo** (15,36 Mo) | +18,6 Mo (20,74 Mo) | +13,8 Mo (15,88 Mo) | 0, mais Node chez l'utilisateur |
| **Taille d'un plugin** (team-rules ; slack à 1 ko près) | **1 265 886 o** (490 ko en gzip -9) ; en lien dynamique, 14 189 o (8 ko) plus `escouade-js` une fois pour tous (1,79 Mo, 565 ko) | 2 495 850 o | 12 082 028 o | 9 833 o (le bundle) + Node |
| **Compilation** du bundle | 1,15 s | 1,6 s | 4,2 à 5,3 s | — |
| **Deux compilations** | **la même empreinte** (`7c8fb54d…` pour team-rules), **sous Windows et sous Linux** (WSL Ubuntu 24.04, la Javy Linux) ; sans `-C deterministic`, deux empreintes | deux empreintes | deux empreintes | — |
| **Compilation par Cranelift** (à l'installation) | 152 ms ; précompilé : 3,98 Mo, chargé en 0,7 ms | 156 ms à chaque `Plugin::new` (12,9 ms ensuite, cache de wasmtime) | 733 ms ; précompilé : 35,6 Mo, chargé en 5,6 ms | — |
| **Démarrage à froid** (des octets à la réponse à `initialize`) | 153 ms ; **1,9 ms** depuis le précompilé | 156 ms | 1,8 ms depuis le précompilé (+ 5,6 ms de chargement) | **42 ms** (le process et Node) |
| **Démarrage à chaud** (module en mémoire : instance + `initialize`) | **1,06 ms** | 12,9 ms | 1,34 ms | — |
| **Mémoire** | **1,5 Mio** de mémoire linéaire, stable ; le working set du process gagne **0,5 Mio par instance** (65 → 70 Mio pour 10 de plus), le code compilé étant partagé | non lue par Extism ; le working set gagne **11 Mio par instance** (56 → 165 Mio pour 10 de plus : chaque `Plugin::new` a son module) | non lue ; le working set gagne **8 Mio par instance** (159 → 241 Mio), après 159 Mio pour le premier composant chargé | 37 à 44 Mio par plugin (working set de node) |
| **`permissions/decide`** (team-rules, médiane / p99) | **71 µs / 136 µs** | 91 µs / 160 µs | 81 µs / 169 µs | 129 µs / 223 µs |
| **`events/fact`** ignoré (médiane / p99) | 34 µs / 55 µs | 31 µs / 73 µs | 30 µs / 60 µs | — |
| **Un fait de slack qui poste** (secret, puis `fetch` vers le faux Slack) | 2,8 ms (test, build de debug) | non mesuré | non mesuré | — |
| **Après 24 000 appels** | mémoire inchangée (1,5 Mio) | répond toujours | répond toujours | — |

Ce qu'on en tire :

1. **La mémoire** : une instance Javy coûte un demi-mégaoctet, contre 37 à 44 Mio pour un process Node par plugin ; Extism et componentize-js en coûtent 16 à 22 fois plus que Javy.
2. **Les trois tiennent le délai de 500 ms par trois ordres de grandeur**, et décident plus vite que Node (pas de tube entre deux process). Le coût n'est pas là.
3. **La reproductibilité tranche.** Seul Javy donne la même empreinte d'une compilation à l'autre et d'un système à l'autre, condition pour que la revue sur le poste du chef retrouve ce que le store a compilé sous Linux (§5.6). Javy compile aussi sans Node, en un exécutable.
4. **Le poids** : environ 13 Mo pour wasmtime 50 avec Cranelift et WASI preview 1 (qui tire le Component Model et l'asynchrone), contre 11 Mo pour wasmtime 38 seul en ESC-170. L'app a déjà tokio et serde : le surcoût réel sera un peu moindre, à mesurer dans le premier ticket. **Mesuré dans l'app par ESC-320** (2026-11-01, Windows 11, `cargo build --release --bin escouade`, profil release de l'app, le même `dist/` et le même dossier `target` pour les deux) : `escouade.exe` du commit de départ du ticket (70324aca) **81 962 496 o**, celui du ticket **96 943 616 o**, soit **+14 981 120 o (+14,98 Mo, +18,3 %)**, dont **1 792 665 o** pour `escouade-js.wasm`, embarqué dans l'exécutable, et **13 188 455 o (+13,2 Mo)** pour wasmtime 50, Cranelift, WASI preview 1 et l'hôte : ce que le prototype annonçait (+13,3 Mo), l'app n'en partageant presque rien. L'installeur compressé en gagne moins. Pulley (l'interpréteur de wasmtime, sans Cranelift) le réduirait, au prix de la vitesse : non essayé.
5. **La taille d'un plugin** : 1,27 Mo, dont presque tout est QuickJS. Le lien dynamique la ramène à 14 ko, mais lie chaque plugin au bytecode d'une version d'`escouade-js` : à chaque nouvelle version du moteur, le store devrait recompiler et le chef resigner tous les plugins. **Retenu : le lien statique** (un module se suffit à lui-même, ce qui tourne est ce qui a été signé) ; le dynamique reste une optimisation possible plus tard.
6. **Précompiler à l'installation vaut le coup** : 152 ms de Cranelift une fois, puis 1,9 ms par démarrage. Le précompilé (`Module::serialize`, 4 Mo) est propre à la machine et à la version de wasmtime : l'app le garde dans son cache, le refait à chaque mise à jour, et ne le reçoit jamais d'ailleurs (`Module::deserialize` fait confiance à ses octets).

### 5.4 Le contrat de l'hôte

**Le même protocole, un autre transport.** Les mêmes méthodes, les mêmes délais (`schemas/plugin-protocol-1.json`) : `initialize`, `shutdown`, `events/fact`, `permissions/decide`, les points d'extension qui viendront ; et du plugin vers Escouade, `secrets/get`, `settings/get`, `log`. Les lignes ne passent plus par les tubes d'un process mais par l'entrée et la sortie standard WASI du module, que l'hôte tient en mémoire :

- l'entrée ne bloque jamais : ce qui est là, ou rien (une lecture vide est « rien pour l'instant », jamais une fin) ;
- l'hôte écrit ses lignes, puis appelle `escouade-turn` : le plugin les lit, y répond, et le tour finit quand QuickJS n'a plus aucune tâche en attente. L'hôte lit ce qu'il a écrit, sert ses demandes, et refait un tour avec les réponses, jusqu'à ce que le plugin n'ait plus rien à dire. Un tour borné par l'époque (§ limites) ; aucun fil, aucun minuteur dans le plugin ;
- la sortie d'erreur et `console.*` vont au journal du plugin (comme sa sortie d'erreur aujourd'hui), jamais dans le protocole ;
- la première ligne, avant tout, est `{"escouade/boot": {"files": {…}}}` (§ fichiers), lue par le prélude avant que le bundle ne commence.

**Les fonctions servies.**

- `secrets/get` : seulement les secrets du plugin (`plugin:<id>:<clé>`, une clé de `permissions.secrets`), comme aujourd'hui ; masqués au journal.
- `settings/get` : comme aujourd'hui, avec `project`.
- `log` : comme aujourd'hui.
- **`net/fetch`** (nouveau) : `{ url, method, headers, body }` → `{ status, headers, body }`. L'hôte n'appelle que **https**, et seulement un hôte de `permissions.network` ou l'un de ses sous-domaines (`slack.com` permet `api.slack.com`, pas `slack.com.evil.example`, ni `evilslack.com`, ni `xoxb@evil.example`, ni une adresse IP). La décision porte d'abord sur le nom d'hôte de l'URL, avant toute résolution ; **puis sur les adresses que ce nom résout, au moment de la connexion** (ajouté par la revue d'ESC-319, fait par ESC-320) : le résolveur que la connexion elle-même emploie écarte toute adresse de bouclage, privée, de lien local, non spécifiée, de multidiffusion ou de diffusion (IPv4 et IPv6, une IPv4 mappée ou traduite en IPv6 jugée comme elle-même, `plugins::wasm::forbidden`), et refuse s'il n'en reste aucune (« slack.com resolves to 127.0.0.1, an address of this machine or of the local network », `-32003`) ; la connexion va à ce qu'il a rendu, jamais à une autre résolution que le DNS pourrait contredire. Pour la même raison, `net/fetch` ne passe pas par le proxy de Réglages › Réseau : un proxy résoudrait le nom là où l'adresse n'est pas jugée. Les redirections ne sont pas suivies (le plugin reçoit la réponse 3xx, et refait un `fetch`, jugé à son tour) ; 10 s au plus, 1 Mio de réponse au plus ; un refus est une erreur JSON-RPC (`-32003`) que le plugin voit comme un `fetch` qui rejette (« slack.com is not in permissions.network of dev.escouade.slack »), et que le journal garde. Le prélude en fait le `fetch` standard (`Response`, `.json()`, `.text()`, `Headers`).
- **Les fichiers du dossier du plugin.** Au démarrage, l'hôte donne au plugin les fichiers JSON à la racine de son dossier (`rules.json`, `channels.json` ; 256 Kio chacun, 1 Mio en tout), jamais son manifeste, jamais un lien, jamais rien hors du dossier. Le prélude les sert par `readFileSync` de `node:fs` (un chemin, ou `new URL('../rules.json', import.meta.url)`), qui répond `ENOENT` pour tout le reste (`/etc/passwd`, `../../slack/channels.json`). Le SDK y ajoute `p.file(nom)` (`files/read`, asynchrone, pour tout fichier du dossier, un sous-dossier compris ; servi aux deux transports, ESC-321). Rien n'est écrit : un plugin qui veut garder un état le demande à l'hôte (un point d'extension à venir, pas de disque).
- **Les commandes** de `permissions.commands` : pas en WASM. Un plugin qui lance un programme reste un process (§5.7).

**Les limites.**

- **Mémoire** : 64 Mio de mémoire linéaire par instance (`StoreLimits`), une seule instance par plugin ; au-delà, `memory.grow` est refusé, QuickJS lève « out of memory » et l'instance s'arrête (essayé avec 32 Mio).
- **Temps** : l'époque de wasmtime, incrémentée toutes les 10 ms par un fil de l'app. Un tour a 2 s ; une décision a ses 500 ms (l'hôte répond `ask` au-delà, comme pour un process) ; une boucle sans fin est arrêtée (`Trap::Interrupt`, essayé à 300 ms : arrêtée avant 700 ms, l'hôte vivant). L'époque plutôt que le carburant : elle ne ralentit pas le code, et un plugin qui attend le réseau n'est pas en train de tourner.
- **Plantage** : un trap, une exception non rattrapée de QuickJS, une limite dépassée finissent l'instance ; l'hôte en fait une nouvelle depuis le module précompilé (1 ms) et rejoue `initialize`, avec le même budget de redémarrages que pour un process (trois fois, puis arrêté, et le journal le dit).
- **Concurrence** : une instance ne fait qu'un tour à la fois ; l'hôte sert un plugin depuis un fil à lui (ou une tâche bloquante de tokio). Un `fetch` lent n'arrête que son plugin.

**Ce qu'un plugin WASM ne peut pas faire.** Lire ou écrire un fichier hors de ce qu'on lui donne ; ouvrir une socket ; résoudre un nom ; atteindre un hôte qui n'est pas dans son manifeste, ou en http ; lancer un programme ; lire l'environnement, la ligne de commande, l'heure exacte au-delà de celle de WASI ; charger un module npm qui n'est pas dans son bundle, ni un module natif (`.node`) ; faire `import()` d'une URL ; utiliser `setTimeout` (pas de minuteur dans le prototype ; l'hôte pourra en servir), `child_process`, `net`, `worker_threads`. Son bundle n'importe que `node:fs` (pour `readFileSync`) et `node:readline` (pour le SDK) : un autre `import` refuse la compilation, avec la raison.

### 5.5 Le SDK `@escouade-dev/plugin`

Son API ne change pas : `createPlugin().onEvent(...)`, `decidePermission(...)`, `p.secret`, `p.settings`, `p.log`, `start()`. Le prototype l'a montré en faisant tourner les deux bundles tels qu'ils étaient commités, le prélude imitant Node (`process.stdout.write`, `createInterface` de `node:readline`). **Fait par WASM-2 (ESC-321), SDK 0.3.0** :

- `start()` choisit le transport sans que le plugin le sache : sous Node, l'entrée et la sortie standard ; dans l'app, l'hôte WASM d'Escouade (`globalThis.Javy` et `globalThis.__esc` du prélude : `listen` reçoit les lignes que la pompe d'`escouade-turn` lui passe, `write` les envoie, `trace` va au journal). La promesse de `start()` ne se résout jamais en WASM.
- `p.file(chemin)` lit un fichier du dossier du plugin, en texte (`files/read`, asynchrone), un sous-dossier compris (`data/words.txt`) ; l'hôte le sert **aux deux transports** (`plugins/files.rs`, le même pour un process et pour une instance) et refuse (`-32003`) un chemin qui sort du dossier (absolu, `..`, un lecteur, un lien qui mène dehors, jugé après résolution), un nom caché (`.env`, `.git`) et le manifeste ; `-32005` pour un fichier absent, `-32004` au-delà de 256 Kio ou s'il n'est pas UTF-8. `readFileSync(new URL('../rules.json', import.meta.url))` marche toujours (les JSON de la racine, donnés au démarrage).
- `fetch` reste le `fetch` standard : en WASM, le prélude le sert par `net/fetch` (`Headers` accepte un objet, un tableau ou des `Headers`) ; sous Node, c'est celui de Node.
- Le prélude (`wasm_prelude.js`) se réduit à ce que le SDK ne fait pas : lire et écrire les lignes par `Javy.IO`, `fetch`, `readFileSync`, `URL`, `console`, un `process` sans flux ni environnement. Plus de `process.stdout`, ni de `createInterface` qui marche : un bundle construit avec le SDK d'avant (0.2) ne tourne plus en WASM, et son journal dit de le reconstruire (« build the plugin with @escouade-dev/plugin 0.3 or later »). Slack et Règles de l'équipe ont leur `dist/` reconstruit avec le SDK 0.3.
- **Pas de minuteur** : le SDK n'en a plus besoin (le délai de `shutdown` n'est borné qu'en Node ; en WASM, la seconde d'Escouade le borne) ; `setTimeout` n'est pas défini en WASM, une bibliothèque qui l'appelle échoue avec la raison au lieu d'attendre sans fin.
- `create-escouade-plugin` écrit `"run": { "js": "dist/plugin.mjs" }`, sans scripts `plugin` ni `plugin.cmd`. Le banc d'`escouade-plugin` (`test`, `dev`) joue en attendant ce bundle par Node, le SDK y parlant comme dans l'app ; WASM-3 le jouera en WASM.
- `schemas/plugin-protocol-1.json` décrit `net/fetch` et `files/read` (paramètres, réponse, délais de 10 s et 5 s, erreurs `refused` -32003, `failed` -32004, `notFound` -32005) ; le TS du SDK en est généré.
- Les tests : le SDK en Vitest (le transport WASM simulé, `p.file` contre le faux Escouade), et un plugin témoin écrit pour le SDK (`tests/fixtures/plugins/sdk-both`) joué par le runtime **par Node et en WASM**, le même scénario sur les deux (`plugins_wasm_tests.rs`).

### 5.6 Le pipeline du store

**Le manifeste.** Un programme en JS/TS se déclare par **`"run": { "js": "dist/plugin.mjs" }`**, un champ de `run` à côté de `command` : le bundle commité est la source de vérité, le `.wasm` est un produit du store, jamais commité. Un `run.wasm` (un module écrit en Rust ou en Go, compilé par son auteur) n'est pas ouvert : le store ne pourrait pas le reproduire. Un manifeste a `command` **ou** `js`, pas les deux.

**À la soumission** (STORE-3, un onzième contrôle, « Compilation ») :

1. le store prend le bundle de l'archive figée, au chemin de `run.js` (un fichier, 5 Mo au plus) ; **jamais de `npm install`**, comme aujourd'hui ;
2. il le compile dans un **conteneur sans réseau** (`--network=none --read-only`, utilisateur sans droits, 2 minutes et 1 Gio au plus), qui ne contient que les outils épinglés : la Javy de la version notée (aujourd'hui 9.1.0, son archive Linux x64 de SHA-256 `a68b122d…`) et `escouade-js.wasm` (son SHA-256 noté avec l'app qui le livre), avec `-C deterministic=y -C source=omitted` ; le prélude et la réécriture des `import` sont ceux d'`escouade-plugin`, de la même version ;
3. il compile **deux fois**, dans deux conteneurs neufs, et refuse la version si les deux empreintes diffèrent (« la compilation n'est pas reproductible ») ;
4. il publie le `.wasm`, gardé sous son SHA-256 comme l'archive, et l'ajoute à `version.json` : `wasm: { sha256, size, compiler: { javy: "9.1.0", engine: "<sha256 d'escouade-js>", prelude: "<version>" } }`. L'empreinte de la version couvre donc le module.

**Le contrôle des binaires** (n° 10) le couvre : un plugin `run.js` n'a aucun binaire à fournir ni à attester ; le seul exécutable est le `.wasm`, que le store a fait lui-même. Un `run.command` vers un script (`#!`, `.cmd`, `.bat`, `.ps1`) ou un interpréteur (`node`, `deno`, `bun`, `npx`, `python`…) est refusé pour un plugin tiers (§5.7).

**La revue locale** (`npm run plugin-review`, STORE-7) recompile le bundle de l'archive avec les mêmes outils épinglés, sur le poste du chef, et compare l'empreinte à celle de `version.json` : un écart arrête tout (« Refuser », comme une archive changée). Mesuré : la Javy de Windows et celle de Linux donnent la même empreinte. Puis elle lit, comme aujourd'hui, le bundle (ce qui tourne) et `src/` (ce qu'on comprend), et vérifie que le bundle est le build de `src/` quand le plugin dit comment le faire (le `ci.mjs` des plugins de première main le fait déjà). Les pistes de lecture changent : `child_process`, `net`, un `import` autre que `node:fs` et `node:readline` ne peuvent plus tourner (la compilation les refuse) ; restent les hôtes appelés, comparés à `permissions.network`, et ce que le code fait de ses secrets.

**La signature** couvre `version.json`, donc l'empreinte du `.wasm` : ce qui est signé est ce qui tourne. L'app vérifie le `.wasm` reçu contre `version.json` avant de le précompiler.

**Le banc et son conteneur.** Le banc (`escouade-plugin test`) joue le `.wasm` publié dans l'hôte WASM. Le conteneur sans réseau de la revue ne garde plus rien pour ces plugins : le bac à sable est wasmtime, et le réseau est déjà fermé par l'hôte ; il reste pour les plugins à binaire (§5.7), et pour la compilation au store.

### 5.7 L'app, `escouade-plugin`, et ce que devient le `run` natif

**L'app** lit `run.js` : elle télécharge le `.wasm` de `version.json` (le store) ou le compile elle-même (un dossier, un dépôt git, « non vérifié »), le vérifie, le **précompile à l'installation** et garde le précompilé dans son cache (refait après une mise à jour de l'app), puis le fait tourner comme un process d'aujourd'hui : même `Runtime`, même journal, même budget de redémarrages, mêmes délais ; seul le transport change (`plugins/wasm.rs` à côté du process de `runtime.rs`). Réglages › Plugins dit « Tourne dans Escouade, isolé » au lieu de « Lance un programme sur votre ordinateur, avec vos droits », et « Se connecte à : slack.com » devient une permission **appliquée**.

**Ce que WASM-1 (ESC-320) a construit.** `src-tauri/src/plugins/wasm.rs` (l'hôte, le prélude `wasm_prelude.js`, `net/fetch`, le cache) et son transport dans `runtime.rs` : une instance par plugin sur un fil à lui, qui lit ce que l'hôte écrit, sert `net/fetch` lui-même et rend le reste au `Host` du process (mêmes `secrets/get`, `settings/get`, `log`, même journal, mêmes délais) ; une faute de l'instance (l'époque, la mémoire, une exception) finit l'instance comme un plantage finit un process, et `exited` en refait une avec le même budget (trois fois en cinq minutes). Le journal le dit (« a turn ran past 2000 ms: Escouade stopped its instance », « its instance crashed: … out of memory »). `escouade-js` est le crate `src-tauri/escouade-js/`, construit par `scripts/escouade-js.sh` (le job `escouade-js` de la CI et de la release, son SHA-256 dans le résumé du job) et **embarqué dans l'exécutable** par `build.rs` : une app construite sans lui ne lance aucun plugin `run.js` et dit pourquoi. La Javy épinglée (9.1.0) est téléchargée au premier besoin dans `<données>/tools/javy-9.1.0/` et vérifiée par le SHA-256 de son archive. Le cache est `<données>/cache/plugins-wasm/` : `<clé>.wasm` (la clé : la version de Javy, l'empreinte d'`escouade-js`, le prélude et le bundle) et `<clé>-<version de l'app>.cwasm` (le précompilé, lu en entier et non mappé : sous Windows un fichier mappé ne se remplace ni ne s'efface). L'installation (`plugin_install`) le prépare aussitôt, hors du fil de la fenêtre ; au démarrage de l'app, les précompilés d'une autre version sont retirés et refaits.

**`escouade-plugin`** (le binaire natif livré avec l'app) compile sans Node : `dev` et `test` compilent le `dist/plugin.mjs` du dossier avec la Javy épinglée, qu'il télécharge à son premier besoin depuis la release de Javy et vérifie par son SHA-256 (comme le WASI SDK de rquickjs ; `ESCOUADE_JAVY` en nomme une autre), et `escouade-js.wasm`, qu'il embarque. `javy-codegen` en bibliothèque l'éviterait, mais sa version publiée (4.0.0) est en retard sur la CLI, tire wasmtime 42 et binaryen en C++ : à revoir quand il rattrapera. Construire le bundle (`esbuild`) reste l'affaire de l'auteur, avec ses outils ; le SDK le fait par `npm run build`, mais un bundle fait autrement vaut autant.

**Ce que WASM-3 (ESC-322) a construit.** Le banc (`plugins/bench/`) joue un plugin `run.js` dans l'hôte de WASM-1 (`Session::start_wasm` du faux Escouade : `Instance` et `serve` de `plugins/wasm.rs`, sur un fil à lui), avec les scénarios d'un process : le plantage est l'entrée fermée, la fermeture finit l'instance après sa réponse à `shutdown` (comme l'app), une faute de l'instance est un plantage (code 1, la raison dans les dernières lignes). Le module est fait par `Modules::prepare`, dans le dossier de données de l'app (`~/.escouade`, ou `ESCOUADE_DATA_DIR`) : la même Javy et le même cache que l'app. `validate` refuse aussi un bundle qui ne peut pas tourner en WASM. `escouade-plugin build <dossier> [--out <dossier>] [--json]` écrit `packs/<id>-<version>.wasm` et son `.sha256` (`<empreinte>  <nom>`, comme `pack`) ; `--json` donne `file`, `size`, `sha256` et `compiler` (`javy`, `engine` = l'empreinte d'`escouade-js`, `prelude` = la version d'`escouade-plugin`), ce que le store notera dans `version.json`. `escouade-plugin --version` dit les quatre (le prélude par son empreinte), et sort en erreur s'il a été construit sans `escouade-js` : la release le vérifie pour chaque système. Le refus d'un bundle (un `import` autre que `node:fs` et `node:readline`, un `export`) est dit dans la langue de l'auteur (`wasm::Refusal`). La Javy : l'archive est vérifiée par son SHA-256 au téléchargement (une archive changée est refusée), puis le programme à chaque usage, contre l'empreinte notée à côté de lui (`javy-9.1.0.sha256`) : une Javy changée sur le disque n'est jamais lancée, elle est téléchargée à nouveau. Mesuré : `build` donne la même empreinte deux fois sous Windows et deux fois sous Linux (WSL), et la même d'un système à l'autre, avec le même `escouade-js` (la release en construit un seul, sur Linux, pour les trois systèmes). `javy-codegen` est toujours en 4.0.0 au 2026-11-01 : la CLI reste.

**Ce que WASM-4 (ESC-323) a construit.** Le onzième contrôle du store, « Compilation » (`store/server/utils/compile.ts`, `docs/STORE.md` §3.3) : le bundle de l'archive figée compilé deux fois par `escouade-plugin build`, chaque fois dans un dossier neuf, deux modules différents refusant la version ; le module gardé sous son SHA-256 et `wasm: { sha256, size, compiler }` dans `version.json` ; `plugin.wasm` servi par l'API de l'app, que `plugins/store.rs` télécharge, vérifie contre `version.json` et range dans le cache, où l'installation le précompile ; le contrôle des binaires qui refuse à un tiers un script ou un interpréteur. **Le conteneur du §5.6 n'est pas construit** : le store tourne lui-même dans un conteneur sur Coolify, sans le socket Docker de l'hôte (le lui donner ferait root sur le serveur quiconque passe par le store). La compilation y est un sous-process : sans réseau par `unshare -rn` quand le noyau le permet, 2 minutes et 1 Gio, un dossier et des données neufs, aucune variable du store ; le code d'initialisation du plugin, lui, ne tourne que dans le bac à sable WASM de wasmtime (Wizer), sans appel système. Ce que cela couvre et ne couvre pas : la décision d'ESC-323.

**Le `run` natif, tranché :**

1. **La première main le garde.** Les plugins `dev.escouade.*` qui sont des programmes Rust du dépôt de l'app (les harness, les sources du Kanban, les hébergeurs, la voix : §4), construits et signés avec l'app (`{escouade}/escouade-plugins`), restent des process : ils lancent des CLI, parlent à des API avec des comptes, et leur confiance est celle de l'app. Slack et Règles de l'équipe, eux, passent en `run.js` (§5.8) : rien ne les oblige à être des process.
2. **Les binaires tiers attestés restent permis**, inchangés : un exécutable par système, son attestation de provenance GitHub, « Lance un programme sur votre ordinateur, avec vos droits » en rouge. C'est la seule voie pour un plugin tiers qui doit lancer un programme (un harness ACP, une commande) ; ils ne seront jamais « sans crainte ». Un auteur en Go ou en Rust qui n'a besoin d'aucun droit du système est invité à `run.js`… ou à attendre `run.wasm`.
3. **`node` (et tout interpréteur) n'est plus une commande d'un plugin tiers du store.** Un `run.command` qui est un script ou qui nomme un interpréteur hors du dossier du plugin est refusé à la soumission, avec la raison (« un programme en JS/TS se publie en `run.js` : le store le compile en WASM ») : sinon, l'utilisateur sans Node aurait un plugin qui ne démarre pas, et la revue lirait un code qui n'est pas ce qui tourne (les `node_modules` de la machine). Installé depuis un dossier ou un dépôt git, « non vérifié », un tel plugin reste permis, et Réglages › Plugins dit qu'il lui faut Node.

### 5.8 La migration de Slack et de Règles de l'équipe

`dev.escouade.slack` 2.0.0 et `dev.escouade.team-rules` 2.0.0 : `"run": { "js": "dist/plugin.mjs" }` à la place de `command` et `windows`, leurs scripts `slack`, `slack.cmd`, `team-rules`, `team-rules.cmd` retirés, `engines.node` réservé au build et aux tests, `escouade.min` relevé à la version de l'app qui sert le WASM. Le code ne change pas (le prototype le prouve) ; le SDK de la version WASM se rebâtit dans leur `dist/`. Ce qu'ils gagnent : plus de Node chez l'utilisateur, et `permissions.network: ["slack.com"]` appliqué. La version 1.x reste au store pour les apps d'avant (sa compatibilité, `escouade.max`). C'est le chef qui revoit, signe et publie ces versions (`first-party-plugin-signing`).

### 5.9 Les tickets, dans l'ordre

Chacun livrable seul, chacun après le précédent :

1. **WASM-1, le runtime dans l'app** (fait, ESC-320) : `plugins/wasm.rs` (l'hôte du prototype, `escouade-js` construit en CI et livré avec l'app), `run.js` lu par les deux lecteurs du manifeste, la compilation par la Javy épinglée, la précompilation et son cache, `net/fetch` et les fichiers, les limites, le redémarrage ; le poids mesuré dans l'exécutable de l'app.
2. **WASM-2, le SDK** (fait, ESC-321) : `start()` qui choisit le transport, `p.file()`, `fetch`, le prélude réduit ; `create-escouade-plugin` qui écrit `run.js`.
3. **WASM-3, `escouade-plugin` sans Node** (fait, ESC-322) : `dev`, `test`, `validate` et `pack` sur `run.js`, la Javy téléchargée et vérifiée, le banc qui joue le WASM, `build`.
4. **WASM-4, le store** : le onzième contrôle (la compilation isolée, deux fois), `wasm` dans `version.json`, le contrôle des binaires qui refuse script et interpréteur pour un tiers, l'app qui télécharge et vérifie le `.wasm`.
5. **WASM-5, la revue locale** : la recompilation et la comparaison des empreintes, les pistes de lecture revues, le conteneur réservé aux binaires. **Fait le 2026-11-01** (ESC-324, docs/STORE.md §4.2).
6. **WASM-6, la doc du site** (fr et en) : `plugins/api` (le transport WASM, `net/fetch`, les fichiers, les limites, ce qu'un plugin ne peut pas faire), `plugins/sdk`, `plugins/publish` (`run.js`, la compilation, plus de Node), `plugins/examples`.
7. **WASM-7, la migration** de `dev.escouade.slack` et `dev.escouade.team-rules` en 2.0.0 sur `run.js`, revues, signées et publiées par le chef.

## 6. Sources

- VS Code, _Workspace Trust_ : <https://code.visualstudio.com/docs/editor/workspace-trust> et le guide des extensions <https://code.visualstudio.com/api/extension-guides/workspace-trust>.
- VS Code, les _webviews_ (iframes d'une autre origine, service workers) : <https://blog.mattbierner.com/vscode-webview-web-learnings/>.
- Zed, les extensions en WebAssembly (WIT, wasmtime, une API par version) : <https://zed.dev/extensions/wit> et « Life of a Zed extension » (blog de Zed, résumé sur <https://www.plushcap.com/content/zed/blog/life-of-a-zed-extension-rust-wit-wasm>).
- Obsidian, la sécurité des plugins : <https://obsidian.md/help/plugin-security>.
- Raycast, le manifeste, la sécurité et la publication : <https://developers.raycast.com/information/manifest>, <https://developers.raycast.com/information/security>, <https://developers.raycast.com/basics/prepare-an-extension-for-store> ; le réconciliateur React vers AppKit : <https://developers.raycast.com/faq>.
- JetBrains, les plages de versions et le _Plugin Verifier_ : <https://plugins.jetbrains.com/docs/intellij/build-number-ranges.html> ; la signature : <https://blog.jetbrains.com/platform/2020/09/plugin-signing-in-marketplace/>.
- Claude Code, le manifeste des plugins : <https://code.claude.com/docs/en/plugins-reference>.
- Tauri, les capabilities : <https://v2.tauri.app/security/capabilities/> ; les iframes et l'IPC : CVE-2024-35222.
- MCP Apps (SEP-1865), des vues en iframe isolée : <https://modelcontextprotocol.io/extensions/apps/overview>, <https://blog.modelcontextprotocol.io/posts/2025-11-21-mcp-apps/>.
- Extism, les limites de mémoire, de temps et les hôtes permis : <https://extism.org/docs/concepts/runtime-apis>.
- ESC-319 (§5) : Javy <https://github.com/bytecodealliance/javy> (v9.1.0, ses `docs/docs-using-*.md`), `javy-plugin-api` 7.1.0 <https://crates.io/crates/javy-plugin-api> ; Extism <https://github.com/extism/extism> (1.41.0) et son PDK JS <https://github.com/extism/js-pdk> (1.7.0) ; ComponentizeJS <https://github.com/bytecodealliance/ComponentizeJS> (0.23.0) et StarlingMonkey <https://github.com/bytecodealliance/StarlingMonkey> (v0.4.0) ; wasmtime 50.0.1 <https://github.com/bytecodealliance/wasmtime> ; binaryen version_133 <https://github.com/WebAssembly/binaryen>. Le prototype et ses mesures : `prototypes/wasm-plugins/`.
- Escouade : `docs/ENGINES.md` (les traits, le registre, le banc, « Ajouter un moteur »), `docs/PROTOCOL.md`, `docs/SPEC.md`.
