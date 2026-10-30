# Les plugins d'Escouade : analyse (ESC-170)

Demandé par l'utilisateur le 2026-10-25 : « pour la v2.1.0, planifie pour plus tard un système de plugins, permettant à tout le monde d'apporter des fonctionnalités à Escouade, fais une analyse poussée pour voir tout ce qu'on pourrait manipuler via ces plugins, et planifie une documentation étendue sur le sujet ».

Ce document est l'analyse. Il ne change pas l'app. Le découpage en tickets, le plan de la documentation et les questions qui décident sont dans `docs/TODO-2.1.md`, chapitre « Plugins ».

- §1 : l'inventaire de ce qu'un plugin pourrait manipuler, point d'extension par point d'extension, depuis le code ;
- §2 : l'architecture, comparée sur ce que font VS Code, Zed, Obsidian, Raycast, JetBrains, Claude Code, Tauri et MCP, et trois expériences jetables ;
- §3 : le modèle de confiance et de distribution ;
- §4 : tout ce qui est externe devient un plugin (ESC-203, 2026-10-26) : les harness, les sources du Kanban, les hébergeurs et leurs PR, en plugins de première main, tous en process ; un plugin qui tient plusieurs points (GitHub : tickets et PR) ; les options comparées et ce que coûte le « tout en process » décidé, l'ordre de migration, l'effet sur les tickets en cours.

**Depuis ESC-203, la règle est : tout ce qui est externe est un plugin.** Les intégrations d'aujourd'hui deviennent des plugins de première main, livrés et signés avec l'app, et, comme les plugins tiers, des process qui parlent le même protocole (§4, décidé le 2026-10-26).

## En bref

- **Un plugin est un dossier avec un manifeste** (`escouade-plugin.json`). Il apporte des **contributions déclarées** : un thème, une langue, un harness ACP, un webhook, des modèles de projet, des entrées de menu… Escouade les lit et les montre avec ses propres composants. Il peut aussi apporter un **programme**, qui parle à Escouade en **JSON-RPC 2.0 sur stdio**, comme un serveur MCP ou un agent ACP : il réagit aux événements, décide d'une autorisation, fournit une source du Kanban.
- **Pas de HTML libre dans la fenêtre en 2.1** : l'interface d'un plugin est déclarée (manifeste et blocs), rendue par Escouade. Une vue libre viendra plus tard, dans une iframe isolée (le modèle de MCP Apps).
- **WASM plus tard, pour le même protocole** : un module WASM (wasmtime) servira les plugins sans droits, ceux qu'on peut installer depuis un `.escouade.yml` sans risque. Les mêmes messages, un autre transport.
- **La 2.1.0** : le socle (manifeste, chargement, Réglages › Plugins, permissions, journal, faux Escouade pour les tests), puis trois points : les **événements** (webhooks, Slack, Discord, commandes), les **harness ACP déclarés**, les **thèmes**. Les **règles d'autorisation** suivent, si la 2.1.0 a la place. Les sources du Kanban, les hébergeurs, les outils MCP, l'interface et les langues viennent après.
- **La place de marché store.escouade.dev arrive avec la 2.1** (ESC-202, `docs/STORE.md`) : soumission depuis un dépôt public, comptes GitHub, revue en local par l'utilisateur et badge « Vérifié par Escouade » signé hors du serveur. L'installation depuis un dossier ou un dépôt git reste permise, « non vérifié ».

## 1. L'inventaire

Chaque point d'extension dit : **où** il vit aujourd'hui, **ce qu'un plugin y ferait**, **un exemple**, puis sa **valeur** pour un utilisateur, sa **difficulté** de construction et son **risque** (sécurité, stabilité, compatibilité). Valeur, difficulté et risque sont notés de 1 (faible) à 3 (fort). Le tableau du §1.13 les résume.

Ce qui rend un point facile, c'est un **trait ou un registre déjà là**, derrière lequel le cœur ne sait pas qui répond : les harness (`Engine`, `EngineProcess`, `registry`), les hébergeurs (`Forge`, `Provider`, `forge::Registry`) et le coffre (`SecretStore`) en ont un. Les sources du Kanban (`integrations::Client`, une énumération fermée), les services de voix (`voice::online::Provider`, une énumération fermée) et les langues (`i18n::Lang`, `Fr` et `En`) n'en ont pas : il faudra d'abord les ouvrir.

### 1.1 Les harness

**Où.** `src-tauri/src/engine/` : les traits `Engine` (sans état : `locate`, `tested`, `caps`, `models`, `efforts`, `spawn`, `message`, `translate`, `one_shot`, `grant_file`, `escouade_launch`, `fence_settings`, `user_config`…) et `EngineProcess` (le process vivant : `send_message`, `interrupt`, `set_model`, `set_mode`, `answer_permission`, `answer_question`, `read_usage`…), dans `engine/mod.rs` ; le registre `engine/registry.rs` (`EngineCaps`, `EngineDecl`, `Tested`, `Version`, `Range`, `EngineState`) ; un dossier par harness (`claude/`, `codex/`, `copilot/`, `cursor/`, `opencode/`, chacun avec `locate.rs`, `process.rs`, `bench.rs`, et `terminal.rs` pour une vue terminal) ; la traduction de leurs frames en conversation, `src-tauri/src/agent/<harness>.rs` ; le banc de conformité `engine/bench/` (scénarios, `replay`), et `engine/minimal/`, un faux moteur qui montre le moins qu'un moteur écrit. Le guide « Ajouter un moteur » de `docs/ENGINES.md` liste une quarantaine de cases.

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

**Où.** `src-tauri/src/forge/` : les traits `Forge` (`me`, `create_pr`, `pull_request`, les vérifications de CI `Check`, les relectures `Review`, `ReviewComment`, les déploiements `Deployment`, le merge `MergeStrategy`, la limite d'appels `RateLimit`, chaque méthode refusée par défaut `unsupported`) et `Provider` (`kind`, `name`, `request_word`, `capabilities`, `default_host`, `own_host`, `repo_of`, `probe`, `scopes`, `token_page`, `cli`, `client`), le registre `forge::Registry`, `remote.rs` (l'analyse des remotes), `accounts.rs`, `connect.rs` ; GitHub (`github.rs`, `github_api.rs`) et Azure DevOps (`azure.rs`, `azure_api.rs`, ESC-241), GitLab, Bitbucket et Gitea en cours (ESC-56 à 58). Dans la fenêtre : `settings/ForgesSection.svelte`, `modals/PrModal.svelte`, `src/lib/prs.ts`, `src/lib/forges.ts`.

**Ce qu'un plugin y ferait.** Un hébergeur de plus (`Provider` + `Forge`) ; ou seulement une partie : les déploiements d'un service qui n'est pas l'hébergeur (Vercel, Netlify, Coolify, Fly.io), la CI d'un autre service (Jenkins, Buildkite), sur une PR déjà connue.

**Exemple.** « Déploiements Coolify » : sur la carte d'un ticket en revue, l'état du déploiement d'aperçu de sa branche, et son adresse.

**Valeur 2** (GitHub, GitLab, Bitbucket et Gitea couvrent la plupart ; les déploiements et la CI tierce valent plus). **Difficulté 1** pour un hébergeur entier (le trait et le registre existent, `unsupported` permet d'en faire une partie), **2** pour des déploiements découplés de l'hébergeur (le modèle les attache à `Forge`). **Risque 2** : un jeton d'hébergeur peut écrire dans les dépôts (`scopes`) ; il va au coffre et n'est donné qu'à son plugin.

### 1.4 Les services de voix

**Où.** `src-tauri/src/voice/` : la dictée locale (Whisper, `transcribe.rs`, `models.rs`, `vad.rs`, `audio.rs`), la transcription en ligne (`online.rs` : `Provider`, une énumération de cinq services, `PROVIDERS`, `Spec`, `Net`, et le catalogue `online_catalog.rs`), le direct (`live.rs`, par WebSocket), la voix en ligne (`online_speech.rs`), la conversation vocale (`conversation.rs`, `spoken.rs`). Dans la fenêtre : `settings/VoiceTab.svelte`, `SpeechSettings.svelte`, `OnlineKey.svelte`, `OnlineModel.svelte`, `src/lib/dictation.svelte.ts`, `src/lib/online-voice.svelte.ts`.

**Ce qu'un plugin y ferait.** Un service de transcription (un fichier audio, ou un flux en direct) ou de voix (un texte, un flux audio), avec son catalogue de modèles et de voix.

**Exemple.** Wispr Flow (déjà demandé, ESC-164), AssemblyAI, Azure Speech, ou un serveur Whisper de l'équipe.

**Valeur 2.** **Difficulté 2** pour le fichier entier (une requête, une réponse), **3** pour le direct (de l'audio en continu, des délais de quelques centaines de millisecondes mesurés en ESC-136 et ESC-137 : un process par stdio tient le débit, mais le passage par un plugin s'ajoute au délai). **Risque 2** : le plugin entend ce que l'utilisateur dicte ; il a besoin du réseau et d'une clé.

### 1.5 Les outils du serveur MCP d'Escouade

**Où.** `src-tauri/src/mcp/` : `tools.rs` (la liste et les descriptions des outils : `list_projects`, `get_board`, `create_ticket`, `start_ticket`, `answer_permission`, `create_agent`, `send_message`, `get_usage`… une quarantaine), `read.rs` et `act.rs` (ce qu'ils lisent et font), `resolve.rs` (`Sight`, `may_act_on` : ce qu'un agent voit et sur quoi il agit, les projets privés), `http.rs` (le serveur et son jeton), `install.rs` (la déclaration dans chaque harness), `activity.rs` (le journal des appels, `McpActivity`). Dans la fenêtre : `settings/McpSection.svelte`, `src/lib/mcp.svelte.ts`, les noms en mots des outils dans `src/lib/tools.ts` (`ESCOUADE_TOOLS`).

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

**Construit (ESC-177, PLG-5).** Le bus est `src-tauri/src/facts.rs` : un `Body` typé par fait (34 en version 1, de `agent.created` à `app.started`, avec ceux des docs, de la mémoire, des décisions, des PR et des comptes), émis là où l’état change (`AgentRt::set_status`, `add_pending`, `Core::apply`, `edit_ticket_with`, la validation, `remove_worktree`, `move_session`, `Space::spec_write`…), les noms remplis par un fil du bus (`Resolve`). `plugins/events.rs` les distribue : `permissions.events`, une seule règle avec Réglages › Plugins (PLG-4) : les `Choices` (`Plugins::reaches` sur `active_in`, un projet privé éteint d’office, un fait sans projet connu gardé comme privé) et les permissions retirées (`granted` : `events:<e>` retiré coupe le fait, ses webhooks et ses commandes), une file de 256 par plugin, `events/fact` au programme, `contributes.webhooks` (vers un hôte de `permissions.network` seulement, écrit en clair, tiré d’un secret ou fait d’une valeur du fait ; `*.d` couvre les sous-domaines de `d`, pas `d` ; la lecture du manifeste refuse un webhook écrit en clair vers un autre hôte, `manifest::host_covered`) et `contributes.commands` (après l’accord, une empreinte rangée dans `Choices.commands_approved`, donné ou retiré dans l’onglet Permissions). La page « Les événements » du site les décrit fait par fait.

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

**Révisé par ESC-203** (§4) : les sources du Kanban et les hébergeurs ne sont plus seulement des points ouverts aux tiers en 2.2. Leur contrat est posé dès la 2.1.0 (les sources) et la 2.1.x (les hébergeurs), et les intégrations d'Escouade deviennent des plugins de première main en process, après le transport (PLG-2) et le banc (PLG-3). Les versions sont dans `docs/TODO-2.1.md`, « Tout l'externe en plugins ».

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

Pourquoi :

1. **C'est ce qu'Escouade sait déjà faire.** Ses cinq harness et son serveur MCP sont des process en JSON-RPC ou proches ; ses tests reposent sur des faux process et un banc de conformité ; ses Job objects tuent un arbre entier. Le socle coûte le moins, et le banc des plugins se calque sur celui des harness.
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

Les §1 à §3 font des plugins des **ajouts** : ils viennent à côté des intégrations écrites dans le cœur. Ce chapitre va plus loin : **chaque intégration externe devient un plugin**, celles d'aujourd'hui comprises, et **chaque plugin est un process**, ceux d'Escouade comme ceux des tiers (décidé le 2026-10-26, §4.11). Il ne change pas l'app ; le plan, version par version, est dans `docs/TODO-2.1.md`, chapitre « Tout l'externe en plugins ».

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
- **Son manifeste** est un `escouade-plugin.json` du même schéma, avec `"run": { "command": "{escouade}/escouade-plugins", "args": ["github"] }`. L'origine (`builtin`) est donnée par l'app, jamais par le manifeste. Les ids `dev.escouade.*` sont réservés : le store (ESC-202) les refuse à la soumission, l'app refuse un dossier qui les prend.
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

### 4.5 Le contrat de chaque point d'extension

Les traits Rust d'aujourd'hui deviennent, dans le cœur, des **adaptateurs** qui parlent au process : `ProcessTicketSource: TicketSource`, `ProcessForge: Forge`, `ProcessEngine: Engine` et `ProcessEngineProcess: EngineProcess`. Le reste du cœur ne voit pas la différence. Les types sont ceux de la crate du protocole, partagée par le cœur et les plugins de première main, et générés en TypeScript pour le SDK (PLG-9). Ceux des sources et des hébergeurs sont déjà sérialisables (`Container`, `ExternalIssue`, `IssuePage`, `PullRequest`, `Check`…).

| Point | Côté cœur | Méthodes JSON-RPC | Aujourd'hui |
| --- | --- | --- | --- |
| Compte | `AccountKind` (nouveau, tiré de `forge::Provider` et de `integrations::Account`) | déclaratif (hôtes, sonde, auth, champs, droits, page du jeton) ; `account/me` | `Provider::{default_host, own_host, probe, scopes, token_page, cli, needs_email}`, `Client::me` |
| Source du Kanban | `TicketSource` et `TicketSourceKind` (nouveaux, à la place de l'énumération `integrations::Client`) | `tickets/containers`, `tickets/states`, `tickets/issues` (filtre, page), `tickets/setState`, `tickets/comment` ; capacités déclarées | `Client::{containers, states, issues, set_state, comment}` (`integrations/mod.rs`) |
| Hébergeur | `Provider` et `Forge` (`forge/mod.rs`) | `forge/createPr`, `forge/pullRequest`, `forge/pullRequestsOfBranch`, `forge/pullRequests`, `forge/merge`, `forge/autoMerge`, `forge/checks`, `forge/jobLog`, `forge/rerunFailed`, `forge/reviews`, `forge/comments`, `forge/comment`, `forge/review`, `forge/reply`, `forge/deployments` ; `unsupported` par défaut ; la limite d'appels dans chaque réponse | tel quel |
| Harness | `Engine` et `EngineProcess` (`engine/mod.rs`) | déclaratif : `locate` (noms, chemins, `--version`), versions testées, `EngineCaps`, modes ; sans état : `engine/models`, `engine/efforts`, `engine/launch` (la description de lancement), `engine/oneShot`, `engine/userConfig`, `engine/fenceSettings` ; par agent : `agent/frames` (reçu), `agent/items` (envoyé), `agent/write`, `agent/sendMessage`, `agent/interrupt`, `agent/setModel`, `agent/setMode`, `agent/answerPermission`, `agent/answerQuestion`, `agent/readUsage`, `agent/attach` (après un redémarrage) ; vue terminal : `terminal/launch`, `terminal/hook` | `Engine::{locate, tested, caps, models, efforts, spawn, message, translate, one_shot, grant_file, escouade_launch, fence_settings, user_config…}`, `EngineProcess::{send_message, interrupt, set_model, set_mode, answer_permission, answer_question, read_usage…}` |

Ce que le contrat exige de chaque point, vérifié par le banc des harness (`engine/bench/`) et celui des plugins (PLG-3) : les erreurs typées (`HttpError` : 401 dit « reconnecte le compte », 404, limite atteinte), un délai par appel, la tolérance de l'inconnu ; pour un harness, la clôture (`fenceSettings`, vérifiée par le cœur), le jeton du serveur MCP jamais dans les arguments (`grant_file`, `no_engine_puts_the_token_on_the_command_line`, contrôlé sur la description de lancement), les noms d'outils normalisés.

### 4.6 Tout en process : ce que ça coûte, et comment on le tient

Ce qui existe : environ 41 000 lignes de Rust dans `engine/`, `agent/`, `integrations/` et `forge/`, tests compris ; cinq faux harness (`tests/fixtures/fake-*.mjs`), un faux service de tickets (`fake-trackers.mjs`), un faux `gh` ; le banc des harness ; des essais réels par harness (`real-harnesses.spec.ts`, `real_fence`, `real_*_view`, `real_claude_models`). La décision les garde tous, et les rejoue à chaque migration.

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
- **La mesure, avant et après.** Un banc de flux (INT-0, dans `docs/TODO-2.1.md`) rejoue des flux enregistrés de chaque harness (`tests/fixtures/<harness>/`, et un long tour réel de Claude Code avec des milliers de frames), au rythme réel et au plus vite, et mesure : le délai de la ligne lue à l'événement envoyé à la fenêtre (médiane et 99e centile), le débit maximal, le CPU et la mémoire du cœur et du plugin, le temps d'un rejeu entier. Mesuré **avant** toute migration, puis après celle de chaque harness. Les seuils proposés, à confirmer par la première mesure : au plus 5 ms de plus au 99e centile, un débit au moins dix fois le plus fort flux réel observé, au plus 30 Mo par process de plugin. Un harness qui les dépasse ne passe pas en process tant que ce n'est pas corrigé.
- **Le transport seul, mesuré avec PLG-2 (ESC-174, 2026-10-29).** `round_trips_and_throughput_are_measured` (`src-tauri/src/plugins_runtime_tests.rs`) mesure le runtime des plugins contre le faux plugin en Node, sous Windows 11 : un aller-retour en médiane de 72 µs, 146 µs au 99e centile (2 000 appels ; 43 µs au §2.2, entre un hôte Node et un plugin Node, sans Job ni file) ; 30 Mo/s de frames d'environ 1 Kio passées en chaînes brutes, par lots de 64 (30 000 frames par seconde). Bien sous les seuils proposés ; INT-0 n'existe pas encore : la comparaison au flux réel des harness se fera avec lui.
- **Le banc des plugins, avec PLG-3 (ESC-175, 2026-10-29).** `src-tauri/src/plugins/bench/` joue neuf scénarios contre le dossier d'un plugin quelconque, par un faux Escouade (`bench/fake.rs`, le pilote JSON-RPC sans l'app) : manifeste, initialisation, version refusée (la 999 offerte), délais (chaque appel mesuré, une méthode inconnue refusée par -32601), un fait reçu (`events/fact`, une notification), une décision (`permissions/decide`), un secret (seulement les siens, jamais écrits), plantage au milieu d'un appel puis relance, fermeture ; un scénario hors des permissions du manifeste n'est pas joué. La CLI `escouade-plugin` (`src/bin/`, livrée à côté de l'app et signée comme elle) en fait `test`, `validate` et `dev` ; `--measure` mesure les allers-retours (58 µs en médiane, 147 µs au 99e centile sur l'exemple Slack). Les exemples de PLG-10 (ESC-182, `plugin-examples/` : Contraste élevé, Webhook Discord, Slack, Règles de l'équipe) passent le banc dans les tests et en CI (`plugin-examples/ci.mjs`) ; les scénarios par point (source, hébergeur, harness) et le rejeu d'INT-0 viendront avec ces points et INT-0.

**2. Les terminaux et les vues TUI.** Le PTY reste au cœur (§4.2). Le plugin **décide du lancement**, rien de plus.

- **Passe par le process** : la description du lancement de la vue (`terminal/launch` : la commande, `--resume <id>`, les hooks à écrire, le dossier de config, les bandes de confiance à montrer, les variables à retirer comme `MSYSTEM` pour Cursor) ; la traduction des événements des hooks (`escouade-hook` les envoie au cœur, qui les passe au plugin par `terminal/hook` : quelques-uns par tour) ; le suivi d'un transcript quand le harness n'a pas de hook (Cursor, ESC-49), lu par le plugin qui rend des éléments.
- **Ne passe jamais par le process** : les octets du PTY vers xterm, les frappes, le redimensionnement, le copier-coller (le délai d'une frappe, mesuré de 15 à 40 ms en ESC-100 et ESC-104, ne bouge pas) ; les appels des agents au serveur MCP d'Escouade (le harness parle au cœur directement) ; le coffre entier (le plugin n'obtient que ses secrets) ; l'état d'un autre agent.

**3. Les essais réels**, à refaire à la migration de chaque harness, et notés dans son ticket : `real-harnesses.spec.ts` (ESC-52), `real_fence` (ESC-140), la vue terminal (`real_claude_view`, `real_codex_view`, `real_cursor_view`…), les modèles du compte (`real_claude_models`), la passation d'un harness à l'autre (ESC-88), le changement de compte au seuil (ESC-148). Ils demandent de vrais comptes : ceux que le mode auto refuse (ESC-12, ESC-110) se lancent à la main, et le ticket ne se ferme pas sans eux. Pour les sources et les hébergeurs : l'essai réel GitHub (`github_tests.rs`, par `gh`) à chaque migration, et le faux service pour les autres, inchangé.

**4. Le démarrage de l'app.**

- **Rien de lancé avant la fenêtre.** Ce qu'il faut à l'écran d'accueil est **déclaré** dans les manifestes et lu sans process : les harness trouvés (`locate` déclaratif), leurs capacités, les hôtes des hébergeurs (pour reconnaître un remote), les champs des comptes.
- **Démarrage paresseux** ensuite : une source à sa première synchro, un hébergeur à la première PR à suivre, un harness au premier agent qui reprend ou à la première ouverture du menu des modèles. Les agents qui reprennent au démarrage lancent leur plugin en parallèle, pas l'un après l'autre.
- **Le coût** : un process `escouade-plugins` démarre en quelques millisecondes (un exécutable Rust, pas un runtime) ; INT-0 mesure le temps jusqu'à la fenêtre et jusqu'au premier agent prêt, avant et après. Seuil proposé : la fenêtre au même temps qu'aujourd'hui, le premier agent prêt à moins de 100 ms de plus.

**5. Un plugin de première main qui plante.**

- **La relance** : celle de PLG-2, trois fois en cinq minutes, puis le plugin est arrêté.
- **L'agent, pendant la relance** : son harness vit toujours (le cœur l'a lancé). Le cœur garde les frames reçues pendant la relance (une file bornée), relance le plugin, lui redonne l'agent (`agent/attach` : sa session, son mode, son modèle, les demandes en attente que le cœur connaît) et lui rejoue les frames gardées. Ce qui peut se perdre : l'état de traduction en cours (un outil commencé avant le plantage reste « en cours » jusqu'à la fin du tour, où le cœur le ferme). La conversation dit « Le plugin Claude Code a redémarré », sans autre gêne.
- **Après le troisième plantage** : les agents de ce harness s'arrêtent proprement (interruption, puis fin du Job), passent en erreur avec « Le plugin Claude Code s'est arrêté après trois plantages », et un bouton « Relancer » ; leur session est gardée, et le message suivant les reprend (`--resume`) une fois le plugin relancé. Une notification le dit une fois, le journal du plugin garde le détail (stderr, dernier message).
- **Pour une source** : la synchro se met en pause, la carte et l'onglet Intégrations disent « GitHub Issues : arrêté », la synchro reprend au prochain passage. **Pour un hébergeur** : l'état des PR passe à « inconnu », sans être perdu, jusqu'à la relance.
- **Les tests** : le faux plugin de PLG-2 sait planter ; chaque adaptateur a son test de plantage (en plein tour pour un harness, en pleine synchro pour une source).

**6. La sécurité.** Un plugin de première main en process a les droits de l'utilisateur, comme aujourd'hui le cœur. Ce qui change : il reçoit, dans son process, les secrets de ses comptes et le jeton du serveur MCP de ses agents (que le harness reçoit déjà) ; rien d'autre. Le cœur ne lance que l'`escouade-plugins` de son propre dossier, signé avec l'app.

### 4.7 Ce qui doit sortir du cœur pour ça

Les énumérations fermées à ouvrir :

- **`EngineId`** (`model.rs`, énumération `Claude`, `Codex`, `Copilot`, `Cursor`, `Opencode`, et `Bench` en test), gardé dans les agents (`AgentMeta::engine`), les tickets, les projets et la colonne `engine` des statistiques. Il devient un id ouvert (une chaîne), **sérialisé à l'identique** (`claude`, `codex`…) : aucune donnée à migrer. Les `match` sur `EngineId` (une trentaine de fichiers hors tests) passent par le registre et les capacités (`EngineCaps`), comme l'interface le fait déjà.
- **`engine::registry::all()`**, une liste écrite en dur des cinq harness : construite depuis les plugins actifs.
- **`Service`** (`model.rs` : `Jira`, `Trello`, `Github`, `Linear`), gardé dans `ExternalRef` de chaque ticket importé : un id ouvert, sérialisé à l'identique. Côté fenêtre : le type `Service`, `SERVICE_IDS` et les noms courts (`SHORT`) de `src/lib/integrations.ts`, lus depuis le registre.
- **`integrations::Client`** (`integrations/mod.rs`, une énumération qui distribue chaque appel par `match`) : les traits `TicketSource` et `TicketSourceKind`, et un registre, comme `forge::Registry`. Les champs de `integrations::Account` propres à un service (`site`, `email`, `key`) deviennent des champs déclarés par le genre de compte.
- **`forge::Registry::standard()`** (`forge/mod.rs`), écrit en dur : construit depuis les plugins. `Provider::kind()` et `scopes()` rendent des `&'static str` : à passer en chaînes possédées ; `CliToken::parse` et `Probe::matches` sont des pointeurs de fonctions : à rendre déclaratifs (une commande et le champ lu ; un chemin et la clé attendue).

Ce qui se sépare :

- **la traduction des harness et l'état des agents** : les méthodes `claude_frame`, `codex_*`, `copilot_*`, `cursor_*`, `opencode_*` d'`AgentRt` deviennent des traducteurs à état propre qui rendent des éléments et des effets (§4.6, 1) ;
- **`Engine::spawn`** devient `engine/launch` (une description) plus le lancement par le cœur ;
- **les textes de chaque intégration** écrits dans le cœur (par exemple ceux de `sync.rs` : « GitHub : donne un jeton, ou connecte la CLI gh ») vont dans le catalogue de son plugin.

Les traits qui deviennent le contrat : `Engine` et `EngineProcess` (`engine/mod.rs`), `Forge` et `Provider` (`forge/mod.rs`), `TicketSource` et `AccountKind` (nouveaux), implémentés dans le cœur par les adaptateurs en process. `SecretStore` reste un service du cœur, que les plugins utilisent par `secrets/get`. Les faux (`integrations/fake.rs`, `fake-trackers.mjs`, `fake-gh`, les `fake-<harness>.mjs`) restent où ils sont : ils simulent le service ou le harness, pas le plugin, et servent tels quels aux tests de chaque plugin.

### 4.8 L'ordre, une étape livrable à la fois

Chaque étape est livrable seule, garde ses tests et ses essais réels inchangés, et ne migre qu'une intégration à la fois. L'ordre va du plus simple au plus risqué ; **le transport en process (PLG-2) et le banc (PLG-3) passent avant toute migration de première main**.

0. **Les mesures de référence** (INT-0), avant tout : le flux de chaque harness, le démarrage, la mémoire. Ce sont les chiffres « avant ».
1. **Les contrats dans le cœur**, sans rien sortir : `TicketSource` et `Service` ouvert (les six sources derrière, sans changer de comportement), `forge::Registry` ouvert, `EngineId` ouvert. Des refactors, livrables en 2.0.x ou avec la 2.1.0.
2. **Le socle** : le manifeste (PLG-1), **le transport en process (PLG-2)** et **le banc (PLG-3)** ; le registre des plugins de première main et l'exécutable `escouade-plugins` empaqueté et signé, d'abord vide ; les adaptateurs en process des sources et des hébergeurs, essayés sur le faux plugin.
3. **Les comptes partagés** : un genre de compte par plugin ; l'exception de github.com (`Accounts::github`) devient la règle. Migration des comptes enregistrés (`integrations.json`) et des noms au coffre, réversible.
4. **Les sources du Kanban en process**, une par ticket : Trello, Linear, Jira, ClickUp. Le plus simple : quelques appels HTTP, des types sérialisables. GitHub Issues, GitLab Issues et Azure Boards sortent avec leur hébergeur (étape 5).
5. **Les hébergeurs en process**, un par ticket : **GitHub d'abord, avec ses Issues** (le plugin « GitHub » complet), puis GitLab avec ses Issues (ESC-56 et ESC-165), Bitbucket, Gitea, et Azure DevOps (ESC-241, avec Azure Boards).
6. **Le contrat des harness en process** : la traduction séparée de l'état, `engine/launch`, les lots, `agent/attach`, la vue terminal ; prouvé sur le faux moteur minimal (`engine/minimal/`) devenu un plugin, mesuré contre INT-0.
7. **Les harness en process**, un par ticket, du moins utilisé au plus utilisé : Cursor CLI (avec le plugin ACP générique de PLG-6), OpenCode, Copilot CLI, Codex, **Claude Code en dernier**. Chacun mesuré avant et après, banc, e2e et essais réels rejoués.
8. **Plus tard** : les plugins tiers en WASM (les sources d'abord, 2.2), un harness tiers par `engine/*` documenté (2.3), les services de voix et les environnements de lancement par la même règle.

### 4.9 L'effet sur les tickets en cours ou à venir

| Tickets | Effet | Décidé |
| --- | --- | --- |
| ESC-201, ClickUp (en cours) | Il ajoute une variante à `Service` et à `integrations::Client`, comme Linear. | **Fini tel quel** : l'étape 1 le passe derrière le contrat avec les autres, l'étape 4 le sort en process. |
| ESC-165, GitLab Issues | Une source de plus, sur les comptes de l'hébergeur GitLab (`forge::accounts`) : une variante de `Service` et de `integrations::Client`, comme ClickUp. | **Fait dans le cœur pour la 2.0.3** (décidé le 2026-10-26, à la demande d'un testeur) : l'étape 1 le passe derrière le contrat avec les autres, l'étape 5 le sort avec le plugin « GitLab » (INT-8.2, ESC-218), son compte déjà partagé avec l'hébergeur. |
| ESC-56, 57, 58, clients GitLab, Bitbucket, Gitea | Le trait `Forge` existe ; les `Provider` sont écrits. | **Écrits directement dans la crate de leur plugin** si l'étape 2 est faite ; sinon dans le cœur, et sortis à l'étape 5. Pas d'attente. |
| ESC-241, Azure DevOps | Un cinquième `Provider` et son client complet (`forge/azure.rs`, `azure_api.rs`) ; Azure Boards suit sur le même compte. | **Fait dans le cœur** (demandé le 2026-10-27) ; sorti en plugin de première main avec GitLab, Bitbucket et Gitea à l'étape 5 (INT-8.5), ses tests inchangés. |
| ESC-242, Azure Boards | Une source de plus, sur les comptes de l'hébergeur Azure DevOps (`forge::accounts`) et par son client (`forge::azure_api::Azure::call`) : une variante de `Service` et de `integrations::Client`, comme GitLab Issues. | **Fait dans le cœur** (demandé le 2026-10-27) ; l'étape 1 le passe derrière le contrat avec les autres, l'étape 5 le sort avec le plugin « Azure DevOps » (INT-8.5), comme GitLab Issues avec GitLab, ses tests et son essai réel `real_azure_boards` inchangés. |
| ESC-59 à 63, 65 à 68, PR, CI, relectures, déploiements, vues | Ce sont des fonctions du cœur qui appellent `Forge`. | **Inchangés** : ils restent au cœur et marchent pour tout plugin d'hébergeur. |
| ESC-69, 70, docs et essais des hébergeurs | Les pages parlent des plugins. | La page « Hébergeurs » dit qu'ils sont des plugins de première main ; l'essai réel passe aussi par « plugin arrêté puis relancé ». |
| ESC-53, parcours enregistrés | — | Inchangé. |
| ESC-172 à 184, PLG | Le socle porte maintenant les intégrations d'Escouade. | Modifiés, ticket par ticket : `docs/TODO-2.1.md`, « Les tickets PLG à modifier ». |
| ESC-36, moteur ACP générique | Doublon de PLG-6. | **À fermer**, renvoyé à ESC-178. |
| ESC-202, place de marché (en cours) | — | Le store refuse les ids `dev.escouade.*` ; l'app montre « Livré avec Escouade » à côté de « Vérifié par Escouade ». |

### 4.10 La place de marché : ce qu'un plugin tiers peut de plus, et de moins

Avec « tout en process », un plugin tiers et un plugin de première main ont **le même transport et le même contrat**. Ce qui les sépare est la confiance, pas la technique.

**De plus qu'un plugin de première main :**

- sortir à son rythme : une API de service qui change se corrige sans attendre une release d'Escouade ;
- servir ce qu'Escouade ne tiendra pas : YouTrack, Asana, Azure Boards, un Jira Data Center derrière un proxy, une forge d'entreprise, un harness ACP de plus ;
- s'écrire dans n'importe quel langage (TypeScript avec le SDK, PLG-9).

**De moins :**

- **activé d'office nulle part dans un projet privé** (§3.3), là où un plugin de première main l'est ;
- **un harness tiers** passe d'abord par l'ACP décrit (PLG-6) ; `engine/*` lui sera ouvert quand il aura été tenu par les cinq harness d'Escouade et documenté (2.3), pas avant ;
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
- les **langues de l'index du code** (11 aujourd'hui, ni C, C++, Ruby, Kotlin ni Swift), en déclaratif : une grammaire tree-sitter et ses requêtes ;
- les **sources de documents** (Confluence, Drive, Notion), que `docs/FACTORY.md` renvoie déjà aux plugins ;
- les **faits** des Docs, de la mémoire, des PR et des comptes sur le bus (ajoutés à PLG-5, ESC-177).

Les docs elles-mêmes (§1.1, §1.5, §4.7, §4.8, §4.10, le tableau des versions de `docs/TODO-2.2.md`, `docs/STORE.md` §5.1) sont remises d'accord avec la 2.1.0 par ESC-279, avant INT-0.

## 5. Sources

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
- Escouade : `docs/ENGINES.md` (les traits, le registre, le banc, « Ajouter un moteur »), `docs/PROTOCOL.md`, `docs/SPEC.md`.
