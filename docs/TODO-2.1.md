# Escouade 2.1 — à faire

La 2.0.0 est sortie le 2026-10-24 avec le nouveau design de l'app (voir `docs/TODO-2.0.md`). La 2.1 reprend les deux chantiers reportés de la 1.8 puis de la 2.0, et deux demandes venues pendant le redesign.

## Hébergeurs de code

GitHub, GitLab, Bitbucket et Gitea : les pull requests des tickets suivies jusqu'au merge, avec leur CI et leurs déploiements, sans quitter Escouade. Plan détaillé : `docs/TODO-1.8.md`, chapitre 2.

Depuis ESC-203, chaque hébergeur est un plugin de première main, et le plugin « GitHub » porte aussi ses Issues : voir « Tout l'externe en plugins », plus bas. Le flux des PR (ESC-59 à 68) reste au cœur.

- [x] Interface commune, comptes par instance et analyse des remotes (ESC-54).
- [x] Client GitHub : github.com, GHE.com, GitHub Enterprise Server (ESC-55).
- [x] Ouvrir une PR depuis un agent ordinaire, et la suivre sur sa carte (ESC-64).
- [ ] Clients GitLab (ESC-56), Bitbucket Cloud (ESC-57), Gitea et Forgejo (ESC-58).
- [ ] PR des tickets : ouverture par l'API, colonne « En revue », suivi jusqu'au merge (ESC-59). Puis la CI en échec corrigée par l'agent (ESC-60), les retours de relecture et les conflits (ESC-61), le merge depuis Escouade (ESC-62).
- [ ] CI de la branche cible et des branches poussées (ESC-63).
- [ ] Vue « Pull requests » du projet (ESC-65), et la relecture d'une PR par un agent (ESC-66).
- [ ] Déploiements (ESC-67).
- [ ] Notifications, vue d'ensemble et outils MCP des PR (ESC-68).
- [ ] Docs, site et statistiques (ESC-69).
- [ ] Essais : parcours enregistrés (ESC-53) et essai réel par hébergeur avant la release (ESC-70).

## Cursor CLI

Un cinquième harness, par ACP, à côté de Claude Code, Codex, OpenCode et Copilot CLI. Plan : `docs/TODO-1.8.md`, chapitre 1.

- [x] Essais réels (ESC-24, 2026-10-25) : réponses dans `docs/TODO-1.8.md`, point 0, « Cursor CLI », flux dans `tests/fixtures/cursor/`.
- [x] L'adaptateur (ESC-42, 2026-10-25) : conversation, permissions (avec le diff d'une écriture soumise, et celui de chaque écriture faite), questions, plan à approuver, tâches et sous-agents dans le bandeau Plan, modes Demander, Plan, Accepter les modifications et Bypass, modèles du compte, file des messages, interruption et reprise. « Toujours » est « Pour cet agent » (un dossier de config par agent, décidé avec l'utilisateur). Faux agent au banc de conformité, e2e, essai réel.
- [x] Ses comptes, son usage et son coût (ESC-43, 2026-10-25) : un compte de plus par sa clé d'API (trousseau, `CURSOR_API_KEY`), Principal sans clé ni jeton de l'environnement ; pas de quota lu (ACP n'en dit rien, le tableau de bord demande le jeton de la connexion) mais « Usage sur cursor.com » ; une limite atteinte finit le tour « limité » sur la carte ambre ; coût « — », les stats comptent les tours. Le serveur MCP passe par sa session (vérifié de bout en bout sur le faux), les textes rédigés par `agent acp` en mode Ask (essai réel : 21 s). Une écriture refusée par un hook se dit en échec. `fs` d'ACP annoncé à `true` : sans effet (essai réel), d'où les textes des modes et de « Ses fichiers ». Reste : un essai avec une vraie clé d'API (aucune ici).
- [x] Sa vue terminal (ESC-49, 2026-10-25) : l'interface de Cursor (`agent --resume <id>`) dans un terminal, suivie par son transcript (messages, outils sans leur résultat, fin de chaque tour), dans le dossier de config de l'agent, sans `MSYSTEM`. Cursor tient les conversations de son interface à part de celles d'ACP : décidé avec l'utilisateur, chaque côté reçoit ce qui s'est dit dans l'autre (une règle `.cursor/rules/escouade-relais.mdc` dans le dossier de l'agent, hors de git, pour l'interface ; un préambule au message suivant pour la session). La bande « Suivi limité » dit ce qu'Escouade ne sait pas : permissions et questions en attente, résultats des outils, tokens. Essayé en vrai : ni la règle d'un plugin (`--plugin-dir`) ni celle d'un dossier `--add-dir` ne sont lues, et les hooks d'un plugin arrivent ou non selon le moment où il est chargé.

## Mistral Vibe (demandé le 2026-10-28)

Un sixième harness : Mistral Vibe, l'agent de code de Mistral AI dans le terminal (open source, Apache 2.0). D'après sa doc, il parle **ACP** et figure au registre ACP, comme Cursor CLI ; ses serveurs MCP se déclarent dans `config.toml` (`./.vibe/` du projet, sinon `~/.vibe/`), et il se lance aussi sans surveillance avec une invite en argument. Ses hooks sont à vérifier : aucune source ne les décrit.

**Sur le poste de l'utilisateur** (2026-10-28) : `vibe` 3.0.3 installé en CLI (`~/.local/bin/vibe`) et authentifié. Les essais réels s'y font comme ceux des autres harness : sa config sauvegardée puis rendue à l'octet près, jamais de déconnexion, jamais un jeton lu ni copié.

- [x] Essais réels d'abord (ESC-265, 2026-10-28), comme pour Cursor CLI (ESC-24) : ACP (conversation, permissions, questions, plan, modes, modèles), dossier de config isolable, compte et clé d'API, quota ou coût lisibles, déclaration du serveur MCP d'Escouade, hooks, interface en vue terminal ; les réponses notées dans `docs/TODO-1.8.md` (point 0, « Mistral Vibe » : ACP recommandé), les flux gardés dans `tests/fixtures/vibe/`.
- [x] L'adaptateur par ACP (ESC-266, 2026-10-28), dans le cœur comme Cursor CLI (choix du chef ; le passage en plugin, ESC-178, viendra après) : `engine/vibe/` (son client ACP) et `agent/vibe.rs`, qui met ses messages dans la forme de ceux de Cursor et passe la main à sa traduction. Un `VIBE_HOME` par agent (« Pour cet agent », confiance du dossier, télémétrie et mises à jour coupées) ; mode Plan joué par Escouade (Vibe n'en a pas) ; un refus coupe le tour ; clôture des projets privés par la `denylist` de ses outils de fichiers, Bypass gardé en `ask` avec une clôture ; faux Vibe, banc, e2e et essai réel court (`real_vibe`) passés. Détail : docs/PROTOCOL.md, « Mistral Vibe ».
- [x] Ses comptes, son usage et son coût (ESC-267, 2026-10-28) : une clé d’API Mistral par compte (trousseau, `MISTRAL_API_KEY`), le coût d’un tour lu de ce dont celui de la session a grandi, coût du jour et budget par jour comme OpenCode avec un lien vers la console de Mistral, limite de débit (`-31001`) « limité » et reprise une minute plus tard, « Reprendre sur <compte> » sans copie de session, textes rédigés par `vibe -p` sans outil et à 0,05 $ au plus.
- [x] Sa vue terminal (ESC-268, 2026-10-28) : l'interface `vibe` sur la session de l'agent (le même magasin qu'ACP : reprise dans les deux sens, sans relais), suivie par le journal de la session (un segment par tour) et ses hooks (`escouade-hook` dans le `hooks.toml` du `VIBE_HOME` de l'agent, le temps de la vue), le serveur MCP d'Escouade par `VIBE_MCP_SERVERS` et `api_key_env` ; essayée pour de vrai, faux `vibe` en TUI, e2e, essai réel des harness étendu (`real-harnesses.spec.ts`). Détail : docs/PROTOCOL.md, « Vue Mistral Vibe ».

## Antigravity CLI (demandé le 2026-10-28)

Un septième harness : Antigravity CLI (`agy`), l'agent de Google dans le terminal, à côté de Claude Code, Codex, OpenCode, Copilot CLI, Cursor CLI et Mistral Vibe. Il entre **dans le cœur** comme Cursor CLI et Mistral Vibe ; le passage des harness en plugins (PLG-6, ESC-178) viendra après. **Il ne parle pas ACP** : son mode sans surveillance `agy -p= --input-format stream-json --output-format stream-json` est la voie, avec un format à lui (étapes `step_update`, pas les blocs de Claude Code).

**Sur le poste de l'utilisateur** (2026-10-28) : `agy` 1.4.9 installé (`%LOCALAPPDATA%\agy\bin\agy.exe`) et authentifié par Google ; sa config est `~/.gemini/antigravity-cli/`, **partagée avec Gemini CLI** ; ses jetons sont au Gestionnaire d'identifiants. Les essais réels s'y font comme ceux des autres harness : config sauvegardée puis rendue à l'octet près, jamais de déconnexion, jamais un jeton ni `google_accounts.json` lu ou copié.

- [x] Essais réels d'abord (ESC-269, 2026-10-28), comme pour Cursor CLI (ESC-24) et Mistral Vibe (ESC-265) : protocole, permissions et questions, config et isolement (`USERPROFILE`), compte, quota, serveur MCP, hooks, interface, modèles et modes ; les réponses notées dans `docs/TODO-1.8.md` (point 0, « Antigravity CLI » : stream-json recommandé), les flux gardés dans `tests/fixtures/agy/`.
- [x] L'adaptateur par stream-json (ESC-270 et suivants) : un processus et un `USERPROFILE` par agent, lecteur de flux à part, permissions par `denied_actions` puis règle `permissions.allow` écrite dans le `settings.json` (relu à chaud) puis renvoi du message, MCP par `.agents/mcp_config.json` (jeton écrit dans le fichier, jamais en argument), `hooks.json` pour l'`escouade-hook`.
- [x] Ses comptes (celui du trousseau ; la clé d'API Gemini pour un compte de plus), son usage (jetons seulement, quota par `/usage` de l'interface), sa vue terminal (`agy --conversation <id>`, suivie par `transcript.jsonl`) ; la clôture des projets privés ; le banc des harness, la doc et le site.
- [ ] À dire à l'utilisateur : pas de questions ni de coût en dollars ni de liste de tâches en stream-json ; le mode `auto` dépend du compte ; les modèles tiers dépendent du forfait.

## Gemini CLI (demandé le 2026-10-28)

Un huitième harness : Gemini CLI (`gemini`), à côté de Claude Code, Codex, OpenCode, Copilot CLI, Cursor CLI, Mistral Vibe et Antigravity CLI. Il parle **ACP** (`gemini --acp`) : l'adaptateur reprend le client de Cursor et de Vibe. Il entre **dans le cœur** (choix du chef, ESC-273) plutôt que comme harness ACP décrit par plugin (PLG-6, ESC-178) ; le plugin viendra avec le passage de tous les harness en plugins. Sa config `~/.gemini/` est **partagée avec Antigravity CLI** : Escouade n'y touche jamais, un `GEMINI_CLI_HOME` par agent isole tout.

**L'utilisateur n'a pas de compte Gemini à y consacrer** : l'étude s'est faite sur la doc, le code installé et des essais sans compte (le vrai binaire 0.66.0 contre un faux serveur d'API local) ; les tests de l'adaptateur se font sur un faux Gemini ; un essai réel reste écrit, ignoré, prêt pour le jour où un compte sera là.

- [x] Étude sans compte (ESC-273, 2026-10-28) : les neuf points notés dans `docs/TODO-1.8.md` (point 0, « Gemini CLI » : ACP recommandé), les flux dans `tests/fixtures/gemini/` (README : « essayé » ou « d'après la doc / le code, à vérifier avec un compte »). `~/.gemini` non lu, non écrit (empreinte identique avant et après).
- [x] L'adaptateur par ACP (ESC-274 et suivants), en reprenant `engine/cursor/` et `agent/cursor.rs` : un processus et un `GEMINI_CLI_HOME` par agent, `GEMINI_CLI_TRUST_WORKSPACE=true`, la clé d'API dans l'environnement (jamais en argument), `mcpServers` de `session/new`, `fs` non annoncé, filtre des textes « [MODE_UPDATE] … » et du contexte de session rejoué, moteur de règles (`--policy`) pour la clôture des projets privés, faux Gemini (`tests/fixtures/gemini/`) pour les tests.
- [x] Ses comptes (clés d'API au trousseau d'Escouade), son usage (jetons seulement), sa vue terminal (`gemini --resume <id>` dans le même `GEMINI_CLI_HOME`, six hooks), la clôture des projets privés ; le banc des harness, la doc et le site.
- [ ] À dire à l'utilisateur : pas de questions, de tâches ni de coût en dollars en ACP ; les sous-agents ne marchent qu'en « Tout accepter » ; Google AI Plus n'est pas pris en charge par Gemini CLI.

### Ce qui reste à vérifier avec un compte Gemini

À faire le jour où un compte (clé d'API, connexion Google ou Vertex) sera disponible, sur un `GEMINI_CLI_HOME` jetable et jamais sur `~/.gemini` (sauvegardé et rendu à l'octet près si besoin, jamais un jeton lu ni copié) ; un essai réel écrit et ignoré (`#[ignore]`, comme `real_azure_devops`) est à prévoir pour l'adaptateur.

- [ ] **Un vrai tour** : taille et rythme des fragments `agent_message_chunk` et `agent_thought_chunk`, contenu réel d'une réflexion, nombre d'appels du routeur `auto` par tour (`_meta.quota.model_usage`), `cachedReadTokens` et `thoughtTokens` réels, jetons d'entrée à vide (préambule + 15 outils), latence du premier fragment. Comparer à `21-turn-streaming-expected.jsonl`.
- [ ] **`authenticate`** par `oauth-personal` (navigateur : qu'obtient un client sans navigateur ? une URL, une erreur ?), `gemini-api-key` (`_meta.api-key`), `vertex-ai`, `gateway` ; ce que chacun écrit dans le `settings.json` de `GEMINI_CLI_HOME` ; le premier `session/new` d'un compte neuf (choix du projet Code Assist, conditions d'utilisation, case « partager mes interactions » à ne jamais valider pour l'utilisateur).
- [ ] **Les comptes** : où va la connexion Google d'un `GEMINI_CLI_HOME` jetable (trousseau global ? fichier chiffré avec `GEMINI_FORCE_ENCRYPTED_FILE_STORAGE=true` ?), donc si un deuxième compte Google est possible par dossier ; la clé d'API par `GEMINI_API_KEY` + `security.auth.selectedType` seuls (sans `authenticate`) avec un vrai service ; Vertex ; si **Google AI Plus** donne la connexion gratuite ou un refus.
- [ ] **Quota** : un 429 réel (par minute, par jour) : texte exact, nouvelles tentatives avant l'erreur, durée ; `retrieveUserQuota` (`22-user-quota-buckets-expected.json`) avec la connexion Google ; le basculement de modèle de l'interface (`handleFallback`) et ce qu'ACP en montre ; `/stats model` ; les limites réelles du niveau gratuit contre la doc (1 000 / 250 par jour).
- [ ] **Permissions avec un vrai modèle** : le modèle réessaie-t-il après un rejet (piège de Vibe) ? faut-il `session/cancel` ? `proceed_always` et ses effets d'un tour à l'autre ; `proceed_always_and_save` (`security.enablePermanentToolApproval`) et le contenu de `auto-saved.toml` ; ce qui demande une permission en `autoEdit` ; `web_fetch` et `google_web_search`.
- [ ] **Questions** : un vrai modèle sans `ask_user` pose-t-il ses questions en texte ? une consigne de préambule suffit-elle ? (ACP n'a pas de canal de question.)
- [ ] **Plan** : un vrai `enter_plan_mode` puis `exit_plan_mode` (le plan Markdown écrit dans `plans/`, le titre de la permission, le passage en `default`), le mode `plan` demandé dès `session/new` ; `general.plan.directory` dans le projet. **Tâches** : `write_todos` selon le modèle ; le suivi (`tracker_*`) et ce qu'il émet en ACP.
- [ ] **Sous-agents** : `invoke_agent` en `yolo` avec un vrai modèle (agents intégrés, ce qui revient dans le texte, jetons du sous-agent comptés ou non dans `usage`) ; une règle `allow` pour `invoke_agent` en `default` ; les agents personnalisés du projet (`.gemini/agents/`).
- [ ] **Un second `session/prompt` pendant un tour** : le code dit qu'il annule le premier ; vérifier la réponse du premier (`cancelled` ?) et l'état de la session.
- [ ] **Reprise** : `session/load` d'une session de plusieurs tours avec des outils (rejeu des résultats d'outils, des réflexions, du contexte) ; le second fichier de session de 3 lignes vu à côté de celui de l'ACP ; reprise d'une session de la **vue terminal** par ACP.
- [ ] **Moteur de règles** : la clôture des projets privés avec un vrai modèle (chemins relatifs, absolus, `..`, jokers de shell, `cat` et `type`, sous-agents) ; si `--policy` ou le niveau admin résiste à un `settings.json` de projet ; l'effet d'un `.geminiignore` dans le worktree (le `.env` était déjà refusé par un filtre de Gemini lui-même) ; `commandRegex` sous `powershell.exe -NoProfile -Command`.
- [ ] **Hooks** : `Notification` (permission demandée dans l'interface), `BeforeModel` et `AfterModel` ; les hooks de l'interface avec une session reprise (`SessionStart` `source: resume`) ; l'`escouade-hook` universel sur les six événements de l'interface ; `gemini hooks migrate`.
- [ ] **Interface** : `gemini --resume <id>` d'une session ACP avec un compte (écran d'accueil du compte : ne rien en copier), `/resume`, `/stats`, `/auth`, Maj+Tab (modes), le mode `plan` dans l'interface ; `--session-id` pour nommer la session d'une vue terminal.
- [ ] **Sans surveillance** : `gemini -p -m <petit modèle>` avec un vrai modèle (jetons d'entrée réels, durée, qualité d'un nom d'agent et d'un message de commit) ; les codes de sortie de `ExitCodes` (42, 52, 53…) ; **le code de sortie 127 et l'assertion libuv sous Windows** avec Node 22 LTS (il ne vient peut-être que de Node 24.14).
- [ ] **MCP** : l'OAuth d'un serveur MCP (`mcp-oauth-tokens.json`), les `instructions` du serveur MCP d'Escouade (passées ou non au modèle), le nombre de permissions demandées pour une session de travail réelle (`trust: true` ou règle `allow` de `mcp_escouade_*` pour les éviter).
- [ ] **Extensions et divers** : `gemini extensions`, `activate_skill` et les skills de l'utilisateur (`~/.gemini/skills`, ceux de Claude Code sont-ils lus ?), `gemini gemma`, `-w` (worktree), `--include-directories`, `--session-file`, le bac à sable (`-s`).
- [ ] **macOS, Linux, WSL** : installation (Homebrew, npm), `GEMINI_CLI_HOME`, le trousseau (macOS Keychain, libsecret, fichier chiffré sans trousseau), `ripgrep`, le shell de l'outil (`bash -c`), les chemins de la config système.

## Demandés pendant le redesign (2026-10-24)

- [ ] **Wispr Flow** pour la transcription vocale, comme service en ligne et comme app de bureau (ESC-164).
- [x] **GitLab Issues** comme source de synchro du Kanban, sur gitlab.com et les instances auto-hébergées (ESC-165). À réécrire en source du plugin « GitLab », sur le compte partagé avec ESC-56 (ESC-203).

## Plugins (demandé le 2026-10-25)

« Pour la v2.1.0, planifie pour plus tard un système de plugins, permettant à tout le monde d'apporter des fonctionnalités à Escouade, fais une analyse poussée pour voir tout ce qu'on pourrait manipuler via ces plugins, et planifie une documentation étendue sur le sujet. »

- [x] **L'analyse et le plan** (ESC-170, 2026-10-25) : `docs/PLUGINS.md` (l'inventaire de ce qu'un plugin pourrait manipuler, depuis le code ; l'architecture comparée, avec trois expériences jetables ; le modèle de confiance et de distribution), et ce chapitre (le découpage en tickets, le plan de la documentation, les questions).
- [x] Les questions qui décident, répondues par l'utilisateur le 2026-10-25 (plus bas, « Les décisions »).
- [x] Les tickets, créés le 2026-10-25 : ESC-172 puis ESC-173 à ESC-184 (plus bas, « Les tickets »), pour plus tard.

### Ce qui est recommandé

Un protocole des plugins en JSON-RPC 2.0, servi en 2.1 par un **process à côté** (stdio, comme MCP et ACP), avec une **interface déclarée** (rendue par les composants d'Escouade, sans HTML d'un tiers dans la fenêtre) et un **niveau déclaratif sans code** (thèmes, harness ACP décrits, webhooks). Un runtime **WASM** du même protocole viendra après, pour les plugins sans droits qu'un projet peut proposer ; puis des **vues libres** en iframe isolée. Les raisons : `docs/PLUGINS.md`, §2.4.

### Le découpage

Les tickets proposés s'appellent ici PLG-1, PLG-2… ; le chef d'escouade leur donne leurs clés. Chacun a un faux plugin pour ses tests ; le **banc de conformité des plugins** (PLG-3) est l'équivalent de celui des harness (`engine/bench/`) : des scénarios joués par un faux plugin, jugés sur ce qu'Escouade en fait.

#### 2.1.0 : le socle et trois points d'extension

**PLG-1. Le manifeste et le chargement.** Dépend de : rien.
- Critères : `escouade-plugin.json` lu et validé par un schéma publié dans `schemas/` (champs du §3.1 de `docs/PLUGINS.md`) ; un plugin installé depuis un dossier local ou une URL git avec une étiquette, copié dans le dossier des plugins de `DataDir`, fixé par son empreinte (le commit) ; un dossier en développement chargé en place et relu quand il change ; une API ou une version d'Escouade hors de sa plage désactive le plugin, avec sa raison (comme `Tested`, `oldest_reason`) ; un manifeste invalide ne charge rien et dit pourquoi, champ par champ ; un champ inconnu est ignoré.
- Tests : unitaires du lecteur (manifestes valides, invalides, d'une API future, champs inconnus) ; un faux dépôt git de plugin dans le dossier temporaire du test ; un e2e qui installe un plugin déclaratif depuis un dossier.

**PLG-2. Le protocole et le runtime en process.** Dépend de PLG-1.
- Critères : un process par plugin avec `run`, lancé paresseusement (à sa première `activation`), dans un Job object (`job.rs`) qui tue son arbre ; `initialize` négocie la version du protocole et les capacités (comme ACP) ; JSON-RPC 2.0, une ligne par message ; chaque appel a un délai, au-delà duquel la réponse par défaut s'applique ; un plantage redémarre le plugin trois fois en cinq minutes, puis l'arrête ; son stderr et ses messages vont au journal du plugin ; à la fermeture d'Escouade, `shutdown` puis la fin du Job ; une mémoire au-delà d'une limite arrête le plugin (lue comme `resources.rs`). Les secrets par `secrets/get`, seulement les siens (§3.5).
- Tests : un faux plugin `tests/fixtures/fake-plugin.mjs` (avec son `.cmd` et son script sans extension, comme les faux harness) dont le message choisit le comportement : répondre, se taire (délai), planter, boucler, grossir, écrire sur stderr, demander un secret d'un autre ; tests Rust du runtime sur chacun.

**PLG-3. Le banc de conformité et le faux Escouade.** Dépend de PLG-2.
- Critères : `src-tauri/src/plugins/bench/` joue des scénarios (initialisation, version refusée, délai, plantage, un fait reçu, une décision, un secret) contre un plugin quelconque, et dit ce qui manque ; le même banc, publié en CLI (`escouade-plugin test <dossier>`) pour les auteurs ; un **faux Escouade** (le SDK, PLG-9) qui envoie les faits et reçoit les réponses sans l'app.
- Tests : le banc passe sur le faux plugin et sur les exemples de PLG-10 ; il échoue, avec la bonne raison, sur un plugin qui ignore `initialize` ou dépasse son délai.

**PLG-4. Réglages › Plugins.** Dépend de PLG-1, PLG-2. Aperçu d'abord, avec le skill `impeccable` (consigne de l'utilisateur pour l'UX d'une fonctionnalité), validé avant la construction.
- Critères : l'onglet décrit plus bas (« L'onglet Réglages › Plugins ») ; la fenêtre d'installation montre l'origine, l'empreinte, les contributions et les permissions en mots, la plus forte d'abord (§3.4 de `docs/PLUGINS.md`) ; activer pour l'app ou par projet, jamais d'office pour un projet privé ; retirer une permission ; désinstaller (dossier, réglages, secrets) ; le journal ; les mises à jour proposées, jamais installées seules, et l'accord redemandé si les permissions grandissent. Les deux langues.
- Tests : Vitest des composants (installation, activation par projet, retrait d'une permission) ; un e2e qui installe le faux plugin, l'active pour un projet, lit son journal, le désinstalle ; une capture comparée à l'aperçu (comme `settings-look.spec.ts`).

**PLG-5. Les événements.** Dépend de PLG-2.
- Critères : un bus de faits typé et versionné dans le cœur, appelé là où l'état change (pas dérivé des `UiEvent`) : `agent.created`, `agent.status` (dont `waiting`), `agent.stopped`, `agent.handover`, `turn.ended` (avec `Finished`, `Interrupted`, `Limited`, `Error`), `request.permission`, `request.question`, `request.answered`, `ticket.created`, `ticket.column`, `ticket.report`, `ticket.approved`, `ticket.rejected`, `ticket.merged`, `ticket.conflict`, `project.added`, `project.closed`, `worktree.created`, `worktree.removed`, `launch.started`, `launch.ready`, `launch.stopped`, `tests.result`, `app.started`. Chaque fait dit le projet, le ticket et l'agent par leurs ids et leurs noms, jamais la conversation entière. Un plugin ne reçoit que les faits qu'il a demandés et seulement des projets où il est activé ; une file bornée par plugin, sans attendre sa réponse. Une contribution déclarative `webhooks` (une URL, des faits, un gabarit de corps) pour notifier sans code ; une contribution `commands` qui lance une commande sur un fait (après l'accord, comme les commandes de `.escouade.yml`, ESC-146).
- Tests : chaque fait émis par un test du cœur (sur les faux harness) et reçu par le faux plugin ; un projet privé dont aucun fait ne sort vers un plugin non activé pour lui ; un faux serveur HTTP qui reçoit un webhook ; une file pleine qui perd les plus vieux faits et le dit au journal.

**PLG-6. Les harness ACP décrits.** Dépend de PLG-1. Peut avancer en parallèle de PLG-2 à PLG-5 (il n'a pas besoin du runtime : c'est du déclaratif).
- Critères : un adaptateur ACP générique tiré de celui de Cursor CLI (`engine/cursor/`, `agent/cursor.rs`), ce qui est propre à Cursor (`cursor/*`) gardé à part ; `EngineId` ouvert à un harness de plugin (`plugin:<id>:<harness>`), ses données gardées à sa désinstallation et l'agent montré « harness absent » ; la contribution `harnesses` du manifeste (programme et arguments, `locate`, versions testées, variable du dossier de config, modes et leur sens, modèles, `EngineCaps`, installation, textes) ; la clôture des projets privés tenue par les lectures et écritures qui passent par le client ACP, et dite manquante quand l'agent ne les y fait pas passer ; le serveur MCP d'Escouade donné sans jeton dans les arguments.
- Tests : le banc des harness (`engine/bench/`) joué sur un faux agent ACP décrit par un manifeste, sans une ligne de Rust à lui ; `fake-cursor.mjs` rejoué par l'adaptateur générique (les flux de `tests/fixtures/cursor/` passent encore) ; `no_engine_puts_the_token_on_the_command_line` sur le harness du plugin ; un e2e d'un agent sur ce harness (message, autorisation, diff, fin de tour) ; un essai réel sur un agent ACP du marché (Gemini CLI ou Goose), noté dans le ticket.

**PLG-7. Les thèmes.** Dépend de PLG-1, et du renommage des anciens jetons (`--ok`, `--accent`, `--line2`…, dans « Restes du redesign » : à faire avant).
- Critères : la contribution `themes` (des valeurs pour la liste publiée des jetons de `src/app.css`, clair et sombre, et une palette des couleurs de projet) ; des valeurs filtrées (couleurs, longueurs ; ni `url()` ni CSS libre) ; appliqué à chaud à la fenêtre, à xterm et à CodeMirror ; un jeton manquant prend la valeur d'Escouade ; choisi dans Réglages › Application.
- Tests : Vitest du filtre des valeurs ; un e2e qui applique un thème de plugin et compare la couleur calculée de quelques surfaces ; le thème retiré, la fenêtre revient au thème d'Escouade.
- **Fait le 2026-10-29** (ESC-179) : `src/lib/plugin-themes.ts` (la liste `THEME_TOKENS`, tenue égale à `src/app.css` par Vitest, sans `--p` ni les jetons de forme ; le filtre : couleurs écrites en hex ou par `rgb()`… `color-mix()`, `var()` d'un jeton de la liste, une proportion, un nombre, trois canaux ; ni `url()`, ni autre fonction, ni `;`, accolade, guillemet) et `src/lib/plugin-theme.svelte.ts` (une feuille `<style>` sur `:root`, `data-plugin-theme` suivi par xterm, la palette à la place de `PROJECT_COLORS`, gardé dans le stockage de la page pour le premier affichage) ; le réglage `pluginTheme` (« Couleurs », Réglages › Application › Affichage) ; aucune longueur dans la liste publiée (rayons et largeurs sont hors thème). Le manifeste ne vérifie pas encore la forme d'un thème (le filtre est à l'application), reste à faire dans `escouade-plugin validate`. e2e `plugin-theme.spec.ts`, page « Les thèmes » du site à `plugins/themes` (pas `plugins/extension-points/themes` : les adresses du site n'ont qu'un niveau de section, `docs.test.ts`).

**PLG-8. Les règles d'autorisation** (si la 2.1.0 a la place ; sinon la première de la 2.1.x). Dépend de PLG-2, PLG-5.
- Critères : avant qu'une demande attende l'utilisateur (`add_pending`), les plugins qui ont la permission `permissions.decide` répondent, dans un délai court (500 ms ?) : `allow`, `deny` avec un message, ou `ask` ; un refus l'emporte ; sans réponse, la demande va à l'utilisateur comme aujourd'hui ; la réponse dit « Autorisé par <plugin> via Escouade » dans la conversation ; la permission montrée en rouge, jamais accordée par un `.escouade.yml` ; chaque décision au journal.
- Tests : le faux plugin qui autorise, refuse, se tait, plante ; deux plugins en désaccord ; le texte « via » dans la conversation ; les quatre faux harness (Claude Code, Codex, Copilot CLI, OpenCode) et le faux Cursor.
- **Fait le 2026-10-30** (ESC-180) : `src-tauri/src/plugins/decide.rs` (les plugins qui décident, `warm` au démarrage, 500 ms et 3 s une fois pour un process pas encore lancé, un refus qui l’emporte), `Effects::asked` posé par `add_pending` (ni plan ni question), `Core::plugins_decide` qui retient la notification et répond par `answer_permission_by` au nom du plugin ; `kind` de la demande tiré de l’outil au nom de Claude Code ; type « Décision » du journal (et `webhook`, `command`, oubliés par la vue). Faux plugin `--decide allow|deny|ask|silent|crash`, tests sur les huit faux harness et l’exemple « Règles de l’équipe » (`plugins_decide_tests.rs`) ; page « Les règles d’autorisation » du site à `plugins/permission-rules`.

**PLG-9. Le SDK.** Dépend de PLG-2, PLG-3 ; suit chaque point d'extension de la 2.1.0.
- Critères : `@escouade-dev/plugin` en TypeScript (les types du protocole, générés depuis un schéma unique, comme `schemas/` ; un client stdio ; les aides de chaque point) ; un gabarit (`npm create escouade-plugin`) ; `escouade-plugin` en CLI (`validate`, `test` : le banc, `dev` : le faux Escouade, `pack`) ; la compilation en un exécutable unique documentée (pour un utilisateur sans Node). Une crate Rust des mêmes types plus tard.
- Tests : le gabarit généré passe `validate` et `test` en CI ; les types générés comparés au schéma (un test qui échoue si l'un change sans l'autre).
- **Fait le 2026-10-29** (ESC-181) : `sdk/plugin/` (`@escouade-dev/plugin`, MIT : `createPlugin`, `onEvent`, `decidePermission`, `secret`, `log` ; `@escouade-dev/plugin/testing`, un faux Escouade en mémoire ; la CLI `escouade-plugin` : `validate` et `pack` en Node, `test` et `dev` passés à celle de l'app par `ESCOUADE_PLUGIN_BIN`, le `PATH` ou son dossier) et `sdk/create-escouade-plugin/` (gabarits `program` et `declarative`). Le schéma unique du protocole, `schemas/plugin-protocol-1.json` (copié dans `website/public/schemas/`) : les types TypeScript en sont générés (`sdk/plugin/scripts/generate.mjs`, `--check` joué par Vitest), le cœur y est tenu par `plugins_protocol_tests.rs` (faits, délais, erreurs, messages envoyés). La CI Linux joue `sdk/scripts/template-ci.mjs` sur les deux gabarits avec le banc construit. `deno compile` essayé sous Windows : 117 Mo, 40 Mo en archive, le banc passe sans Node. Pas encore publiés sur npm. La crate Rust des types reste à INT-4.

**PLG-10. Les plugins d'exemple.** Dépend des points qu'ils montrent. Dans un dépôt à part (`escouade-plugins`), chacun avec ses tests au banc en CI :
1. **Thème « Contraste élevé »** : déclaratif seul (PLG-7).
2. **Webhook Discord** : déclaratif seul, les tickets à tester et les agents qui attendent (PLG-5).
3. **Slack** : en TypeScript, un salon par projet, un secret au coffre (PLG-5, PLG-9).
4. **Gemini CLI** : un harness ACP décrit (PLG-6).
5. **Règles de l'équipe** : en TypeScript, des autorisations lues dans un fichier du dépôt (PLG-8).

**PLG-11. La documentation du site.** Dépend de PLG-1 à PLG-10 ; écrite au fil des tickets (chaque ticket écrit sa page), relue ici. Voir « La documentation étendue ». Critères : l'arbre ci-dessous en français et en anglais, les tests du site (`generate` puis ses tests, les NBSP), chaque exemple de code tiré d'un plugin d'exemple qui passe le banc.

**PLG-12. Les docs internes et l'essai réel.** Dépend de tout le reste de la 2.1.0. Critères : `docs/PLUGINS.md` passé de l'analyse à la référence (ce qui a été construit), `docs/SPEC.md` (« Plugins »), `docs/PROTOCOL.md` si le protocole d'un ticket change, le CHANGELOG fr/en ; un essai réel des cinq exemples sur Windows, macOS et Linux.

Ordre : PLG-1 → PLG-2 → PLG-3 → (PLG-4, PLG-5, PLG-6, PLG-7 en parallèle ; PLG-6 et PLG-7 peuvent partir dès PLG-1) → PLG-8 → PLG-9 → PLG-10 → PLG-11 → PLG-12.

#### Après la 2.1.0

- **2.1.x** : les canaux de notification (ntfy, Pushover, Telegram, à côté de ceux du système, sur les réglages de ce qui notifie) ; l'interface déclarée (menus d'un agent, d'un ticket, d'un projet ; badges des cartes ; éléments de la barre d'état ; une section de réglages ; commandes et raccourcis), rendue par les composants d'Escouade, avec son aperçu par `impeccable`.
- **2.2** : les sources du Kanban (`integrations::Client` passé en trait, comme `Forge`) ; les environnements de lancement (isola généralisé), les étapes de mise en place, les modèles de projet de l'assistant et les clés `plugins.<id>.*` de `.escouade.yml` avec `plugins.recommended` ; les textes rédigés et les consignes ajoutées (dans `instructions_room`) ; les outils ajoutés au serveur MCP et le plugin client du serveur (un jeton par plugin, la vue `Sight`) ; les hébergeurs, déploiements et CI tierce (après ESC-56 à 69) ; les statistiques en lecture et leur export ; une source de Ctrl+K ; **le runtime WASM** du même protocole (wasmtime, carburant et époque, mémoire bornée, réseau par les hôtes déclarés, disque filtré par la clôture), et les plugins WASM proposés par un projet. Le registre n'attend plus la 2.2 : c'est la place de marché de la 2.1 (plus bas).
- **2.3 et plus tard** : les vues libres (un panneau, une page) dans une iframe isolée, après un essai sous WKWebView et WebKitGTK ; les services de voix (transcription et voix, puis le direct) ; les langues (`i18n::Lang` ouvert, les textes du cœur passés en catalogues) ; les harness en programme ; un bac à sable du système pour les process (AppContainer, Seatbelt, bubblewrap).

### La documentation étendue

Une section « Plugins » du site, dans `website/content/docs/fr/plugins/` et `website/content/docs/en/plugins/`, pages de même nom dans les deux langues (comme les 39 pages d'aujourd'hui), chacune écrite avec le ticket qui construit ce qu'elle décrit :

```
plugins/
  index.md                Présentation : ce qu'un plugin apporte, les niveaux (déclaratif, programme, plus tard WASM), ce qu'il peut et ne peut pas faire
  installer.md            Pour l'utilisateur : installer, activer par projet, permissions, journal, désinstaller, projets privés
  quick-start.md          Démarrage rapide : un premier plugin en 10 minutes (le webhook Discord, puis le même en TypeScript)
  manifest.md             Le manifeste, champ par champ, et son schéma
  permissions.md          Les permissions : ce que chacune ouvre, ce qu'Escouade applique, ce qu'il ne fait que déclarer
  extension-points/
    events.md             Les événements, fait par fait, et les webhooks déclarés
    harnesses.md          Un harness ACP décrit
    themes.md             Les thèmes et la liste des jetons
    permission-rules.md   Les règles d'autorisation
    notifications.md      (2.1.x) Les canaux de notification
    ui.md                 (2.1.x) Menus, badges, barre d'état, réglages, commandes
    kanban-sources.md     (2.2) Les sources du Kanban
    launch.md             (2.2) Environnements de lancement, étapes, modèles de projet, .escouade.yml
    texts.md              (2.2) Textes rédigés et consignes
    mcp-tools.md          (2.2) Outils MCP et plugin client du serveur
    forges.md             (2.2) Hébergeurs, déploiements, CI
    stats-search.md       (2.2) Statistiques, export, recherche
  api.md                  L'API : le protocole JSON-RPC, ses méthodes et ses types (générés depuis le schéma)
  sdk.md                  Le SDK TypeScript, le gabarit, la CLI escouade-plugin, un exécutable unique
  testing.md              Tester un plugin : le faux Escouade, le banc de conformité, la CI
  publishing.md           Publier : un dépôt git, les étiquettes, puis store.escouade.dev (docs/STORE.md)
  versioning.md           La politique de versions : l'API majeure, escouade.min et max, ce qui casse et ce qui ne casse pas, l'annonce d'un retrait
  security.md             La sécurité pour les auteurs : secrets, consignes et injection, projets privés, ce que la revue refuse
  faq.md                  FAQ
  examples/
    high-contrast.md      Thème « Contraste élevé » (déclaratif)
    discord-webhook.md    Webhook Discord (déclaratif)
    slack.md              Slack (TypeScript, un secret)
    gemini-cli.md         Gemini CLI (harness ACP décrit)
    team-rules.md         Règles de l'équipe (TypeScript, autorisations)
```

Chaque exemple montre son code entier, tiré du dépôt `escouade-plugins` (pas recopié à la main : un test du site vérifie qu'il est à jour). La navigation du site gagne une section « Plugins » après « Réglages » ; la page `mcp.md` et la page `harness.md` renvoient vers leurs pages de plugins.

**Le SDK** (PLG-9) : les types TypeScript du protocole générés depuis un schéma JSON unique (la même source pour le Rust du cœur, comme `schemas/` aujourd'hui) ; un client stdio et une aide par point d'extension (`onEvent('ticket.column', …)`, `decidePermission(…)`) ; un gabarit `npm create escouade-plugin` (un plugin déclaratif, ou un programme en TypeScript avec ses tests) ; une CLI `escouade-plugin` : `validate` (le manifeste), `test` (le banc de conformité, PLG-3), `dev` (le faux Escouade : il envoie des faits choisis, montre les réponses, simule un projet privé et un délai), `pack` (l'archive et son empreinte) ; la compilation en un exécutable (`bun build --compile` ou `deno compile`) pour un utilisateur sans Node. Une crate Rust des mêmes types, puis l'interface WIT du runtime WASM, en 2.2.

**L'onglet Réglages › Plugins** (PLG-4), à concevoir avec `impeccable`, sur aperçu validé avant la construction ; dans le groupe des réglages de l'app, à côté de « Serveur MCP » :

- **La liste des plugins installés** : nom, auteur, version, origine (dossier, dépôt git, plus tard registre ; « non vérifié » hors du registre), un interrupteur pour l'app, l'état (actif, arrêté, désactivé : API trop ancienne, planté trois fois…), une mise à jour proposée.
- **Installer** : depuis un dossier, depuis une URL git (et plus tard depuis le registre). La fenêtre d'installation : origine et empreinte, ce qu'il ajoute, ce qu'il pourra faire, permission par permission, la plus forte d'abord (« Lance un programme sur votre ordinateur, avec vos droits »).
- **La fiche d'un plugin** : sa description, ses contributions (les thèmes, harness, webhooks… qu'il ajoute, et où les trouver), ses réglages et ses secrets (les champs qu'il déclare), ses permissions (chacune retirable), son activation par projet (une ligne par projet ouvert, les projets privés éteints d'office et marqués), son journal (démarrages, plantages, appels, autorisations décidées, filtrable, copiable), désinstaller (avec une case pour garder ses données).
- **Les plugins proposés par un projet** (2.2, `plugins.recommended` de `.escouade.yml`) : une bande dans l'onglet Projet, qui renvoie à l'installation.
- **Le mode développeur** : charger un dossier en place, le relire quand il change, ouvrir son journal en direct.

### Les décisions (2026-10-25)

L'utilisateur a retenu chaque recommandation :
1. **L'architecture** : un process à côté en JSON-RPC en 2.1, avec une interface déclarée et un niveau déclaratif sans code ; le WASM du même protocole en 2.2 ; les vues libres en iframe isolée en 2.3.
2. **La 2.1.0** : le socle (PLG-1 à PLG-4, PLG-9), les événements (PLG-5), les harness ACP décrits (PLG-6) et les thèmes (PLG-7) ; PLG-8 en tête de la 2.1.x s'il retarde la 2.1.0.
3. ~~**Le registre** : aucun en 2.1.0 ; un dépôt public avec revue par PR et signature par Escouade en 2.2~~ — **remplacée le 2026-10-26** (ESC-202) : la place de marché **store.escouade.dev** arrive avec les plugins de la 2.1 (soumission depuis un dépôt public, comptes, revue locale et badge « Vérifié par Escouade », plus bas). Reste vrai : l'installation depuis un dépôt git toujours permise, marquée « non vérifié ».
4. **La licence** : le SDK et le schéma sous MIT, une licence libre exigée dans le store (au choix de l'auteur), pas de vente par Escouade.
5. **Un projet qui propose un plugin** : il propose, il n'installe jamais seul ; en 2.2 avec le WASM.
6. **Les autorisations décidées par un plugin** : permises, comme une permission rouge (jamais donnée par un projet, journalisée), un refus qui l'emporte et un délai court qui rend la demande à l'utilisateur.

Compléments du 2026-10-26 (ESC-203) : tout ce qui est externe devient un plugin, et les plugins d'Escouade aussi sont des process ; PLG-6 sortirait de la 2.1.0 (à confirmer). Voir « Tout l'externe en plugins », plus bas.

### Les tickets (créés le 2026-10-25, pour plus tard)

| Plan | Ticket | Dépend de |
| --- | --- | --- |
| Renommage des anciens jetons (avant PLG-7) | ESC-172 | — |
| PLG-1. Le manifeste et le chargement | ESC-173 | — |
| PLG-2. Le protocole et le runtime en process | ESC-174 | ESC-173 |
| PLG-3. Le banc de conformité et le faux Escouade | ESC-175 | ESC-174 |
| PLG-4. Réglages › Plugins | ESC-176 | ESC-173, ESC-174 |
| PLG-5. Les événements | ESC-177 | ESC-174 |
| PLG-6. Les harness ACP décrits | ESC-178 | ESC-173 |
| PLG-7. Les thèmes | ESC-179 | ESC-173, ESC-172 |
| PLG-8. Les règles d'autorisation | ESC-180 | ESC-174, ESC-177 |
| PLG-9. Le SDK | ESC-181 | ESC-174, ESC-175 |
| PLG-10. Les plugins d'exemple | ESC-182 | ESC-177, ESC-178, ESC-179, ESC-181 |
| PLG-11. La documentation du site | ESC-183 | PLG-1 à PLG-10 |
| PLG-12. Les docs internes et l'essai réel | ESC-184 | le reste de la 2.1.0 |

## La place de marché store.escouade.dev (demandé le 2026-10-26)

« Pour le système de plugins, fais en même temps le système de marketplace qui sera sur store.escouade.dev. Chacun pourra soumettre des plugins via des repos publics, avec la documentation, etc. Une gestion de compte pour gérer ses plugins partagés. Un système de review de plugins que j'analyserai en local via un agent dans le repo d'escouade, pour avoir un flag "vérifié par escouade". »

- [x] **L'analyse et la conception** (ESC-202, 2026-10-26) : `docs/STORE.md` (le service, les comptes et le RGPD, la soumission, la revue locale et la signature, l'app, les pages, la sécurité, le plan) ; l'aperçu du store (`design/store/`, captures sombre et clair à 1440, 1028 et 390 px) et de « Parcourir le store » dans l'app (`#store*` de `design/app-redesign/index.html`) ; les choix 160 et suivants d'`UX-NOTES.md` ; le traitement 3 et 4 de `docs/registre-rgpd.md` (en projet).
- [x] Décidé par l'utilisateur le 2026-10-26 : le code vit dans ce dépôt (`store/`), et le push sur `main` le déploie par Coolify (`docs/STORE.md`, §1.1, §1.3, §1.5).
- [x] Les décisions (`docs/STORE.md`, §8), prises par l'utilisateur le 2026-10-26 : chaque recommandation (GitHub App, badge par version, non vérifiés listés et marqués, PostgreSQL de Coolify et sauvegardes Scaleway, binaires attestés) ; le design validé.
- [x] Les tickets, créés le 2026-10-26 pour jeudi (« On développera les plugins et la marketplace jeudi prochain ») : STORE-1 ESC-228, STORE-2 ESC-229, STORE-3 ESC-230, STORE-4 ESC-231, STORE-5 ESC-232, STORE-6 ESC-233, STORE-7 ESC-234, STORE-8 ESC-235, STORE-9 ESC-236, STORE-10 ESC-237.

| Plan | Modèle conseillé | Dépend de |
| --- | --- | --- |
| STORE-1. Le socle du service | Sonnet medium | — |
| STORE-2. Les comptes | Sonnet medium | STORE-1 |
| STORE-3. La soumission et les contrôles | Sonnet medium | STORE-2, ESC-173, ESC-181 |
| STORE-4. Les pages publiques | Opus medium | STORE-3 |
| STORE-5. L'espace de l'auteur | Opus medium | STORE-3 |
| STORE-6. L'API de l'app | Sonnet medium | STORE-3 |
| STORE-7. La revue locale | Opus medium | STORE-6, ESC-175, ESC-181 |
| STORE-8. La signature et la file | Opus medium | STORE-7, STORE-5 |
| STORE-9. Le store dans l'app | Opus medium | ESC-176, STORE-6, STORE-8 |
| STORE-10. Les docs et l'essai réel | Sonnet medium | STORE-1 à STORE-9, ESC-182 |

PLG-11 (ESC-183) suit : `publishing.md` décrit le store, et non plus un dépôt git seul.

## Les loops d'un projet (demandé le 2026-10-26, avancé en 2.0.x)

« Pour la 2.1, réfléchis à une façon d'afficher les loops dans la colonne de droite : analyser la loop du projet dans les paramètres du projet, pour en extraire les portes d'entrées, les différents agents, etc, montrer dans l'UI le workflow et montrer en temps réel, pour chaque agent, l'avancée, où on est, et l'historique dans cette loop. »

- [x] **La conception** (ESC-191, 2026-10-26) : l'étude dans `docs/LOOPS.md`, l'aperçu (`#loops*` de `design/app-redesign/index.html`, captures `loops*.png`), les choix 131 à 159 d'`UX-NOTES.md`.
- [x] La validation des choix par l'utilisateur, sur les captures (2026-10-26) : « Ça me va parfaitement, implémente le directement en fait pour la v2.0 ». Construit en 2.0.x, sauf LOOP-8 (les plugins, 2.1).
- [x] Les tickets, créés le 2026-10-26 : LOOP-1 ESC-192, LOOP-2 ESC-193, LOOP-3 ESC-194, LOOP-4 ESC-195, LOOP-5 ESC-196, LOOP-6 ESC-197, LOOP-7 ESC-198, LOOP-9 ESC-199 ; LOOP-8 ESC-200 pour la 2.1 (après ESC-177).

### Les tickets proposés

Les tickets s'appellent ici LOOP-1, LOOP-2… ; le chef d'escouade leur donne leurs clés. Chacun a ses fixtures (copies figées de GSD, Superpowers, Ralph, d'un projet sans workflow et d'un projet à plusieurs dépôts dans `tests/fixtures/workflows/`) et un faux agent qui joue les faits.

| Plan | Ce qu'il fait | Choix | Modèle conseillé | Dépend de |
| --- | --- | --- | --- | --- |
| LOOP-1. La lecture par règles | Le modèle d'un workflow (portes, étapes, sous-agents, transitions, retours, fichiers d'état, sortie), la lecture des dossiers de chaque harness et des `@`, ce qui est un workflow, `~/.claude` compté par trace, plusieurs dépôts, la relecture au premier plan ; gardé dans `~/.escouade/` | 133, 135, 136, 138, 155 | Sonnet medium (back) | — |
| LOOP-2. Le suivi en temps réel | Les faits de chaque harness mis en regard des étapes, l'historique par agent (heure, durée, coût, issue, retours et leur raison), hors du workflow, version gardée par étape, suivi partiel par harness, la boucle d'un prompt (tours, liste cochée) | 146–151, 153, 154 | Sonnet medium (back) | LOOP-1 |
| LOOP-3. L'onglet Workflow | Le rail, les arcs, les pastilles, replié, la liste des agents, l'historique, la boucle d'un prompt, l'état vide, les cas limites, 1028 px, les deux thèmes, le français et l'anglais ; e2e | 132, 140–148, 150–154 | Opus medium (interface) | LOOP-2 |
| LOOP-4. Réglages › Projets › Workflow | La lecture et ses sources, « Relire », la relecture automatique, les workflows trouvés, les étapes corrigées (renommer, fusionner, ajouter, deviné), la clé `workflows` de `.escouade.yml` (schéma, `KEYS`) | 133, 137–139, 152 | Opus medium (interface) | LOOP-1 |
| LOOP-5. Affiner avec un agent | L'agent court, son schéma JSON validé par les règles, modèle, compte et coût dits avant, la clôture d'un projet privé | 134, 156 | Sonnet medium | LOOP-1, LOOP-4 |
| LOOP-6. La carte, la conversation et les tickets | L'étape sur la carte, le séparateur dans la conversation, le workflow « Tickets » d'Escouade, un agent de ticket dans deux workflows | 145, 159 | Opus medium (interface) | LOOP-2, LOOP-3 |
| LOOP-7. MCP et documentation | `get_workflow`, `workflow` dans `list_agents` et `get_agent_summary`, la vue `Sight` des projets privés ; `docs/PROTOCOL.md`, `docs/SPEC.md`, une page du site en deux langues | 156, 157 | Sonnet medium | LOOP-2 |
| LOOP-8. Les faits et le lecteur de workflow des plugins | Les faits `workflow.*` sur le bus, le point d'extension « lecteur de workflow » | 158 | Sonnet medium | LOOP-2, ESC-177 (PLG-5) |
| LOOP-9. Essais réels | GSD et Ralph sur Claude Code (chat et vue), un prompt de Codex, OpenCode, Copilot CLI, Cursor CLI (chat et vue) ; les tableaux de `docs/LOOPS.md` corrigés d'après ce qui est vu | 150 | Sonnet medium | LOOP-1 à LOOP-7 |

LOOP-3 et LOOP-4 peuvent avancer en même temps une fois LOOP-1 fait (LOOP-3 sur des faits joués par le faux agent en attendant LOOP-2). LOOP-8 attend le bus de faits des plugins ; s'il n'est pas dans la 2.1.0, il passe à la 2.1.x sans rien retarder.

## Tout l'externe en plugins (demandé le 2026-10-26)

« Faudra prévoir de faire passer toutes les intégrations externes sous forme de plugins (Claude code, copilot CLI, etc, y compris la gestion des tickets externes et la gestion des PR). Ex : un plugin github pourra gérer les tickets ET les PR. »

- [x] **L'analyse et le plan** (ESC-203, 2026-10-26) : `docs/PLUGINS.md`, §4 (la décision, ce qui reste dans le cœur, les plugins de première main, un plugin à plusieurs points avec l'exemple GitHub, le contrat de chaque point, ce que coûte le « tout en process » et comment on le tient, ce qui sort du cœur, l'ordre, l'effet sur les tickets, la place de marché), et ce chapitre.
- [x] Les décisions, répondues par l'utilisateur le 2026-10-26 (plus bas, « Les décisions ») : **tout en process**, les six autres recommandations retenues.
- [ ] À confirmer : PLG-6 (ESC-178) sort de la 2.1.0 (plus bas).
- [ ] Les tickets INT-0 à INT-11 à créer, et les tickets PLG et des hébergeurs à reprendre (plus bas).

### Ce qui est décidé

**Tout ce qui est externe est un plugin, et chaque plugin est un process** : les cinq harness, les sources du Kanban (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues), les hébergeurs avec leurs PR, leur CI et leurs déploiements (GitHub, GitLab, Bitbucket, Gitea). Les intégrations d'aujourd'hui deviennent des **plugins de première main** : leur code Rust sort du cœur dans des crates, réunies dans un exécutable `escouade-plugins` à plusieurs entrées (un process par plugin), livré et signé avec l'app. Ils parlent le **même protocole** que les plugins tiers (JSON-RPC sur stdio, PLG-2) : un seul chemin, un seul contrat. **Un plugin peut tenir plusieurs points** sur un compte partagé : « GitHub » donne les Issues, les PR, la CI et les déploiements.

**Ce qui reste dans le cœur** : les agents et leur état, le lancement des process des harness (d'après la description que rend le plugin), le PTY et les vues terminal, le Kanban et sa synchro, le flux des PR d'un ticket, git, les worktrees, le serveur MCP, la clôture des projets privés, le coffre (chaque plugin n'a que ses secrets), la jauge des comptes, les mises à jour, la télémétrie et les statistiques (`docs/PLUGINS.md`, §4.2).

**Ce que ça coûte, et comment on le tient** (`docs/PLUGINS.md`, §4.6) : un passage de plus par frame et la sérialisation, tenus par des lots (64 Kio ou 8 ms) et des frames passées en chaîne brute ; la traduction de chaque harness à séparer de l'état de l'agent (41 à 61 membres d'`AgentRt` touchés aujourd'hui par harness), le gros du travail ; des mesures avant toute migration et après celle de chaque harness, avec des seuils ; le PTY jamais dans le process ; les essais réels rejoués à chaque migration ; rien de lancé avant la fenêtre ; un plugin qui plante relancé sans perdre l'agent (son harness, lancé par le cœur, vit toujours).

### Quelle version porte quoi

| Version | Étape (`docs/PLUGINS.md`, §4.8) | Tickets |
| --- | --- | --- |
| 2.0.x, dès maintenant | 0. Les mesures de référence ; 1. les contrats dans le cœur, sans rien sortir (sources, hébergeurs, `EngineId`) | INT-0, INT-1, INT-2, INT-3 |
| 2.1.0 | 2. Le socle : PLG-1, **PLG-2 (le transport)** et **PLG-3 (le banc)**, avec PLG-4, 5, 7, 9 comme décidé le 2026-10-25 ; l'exécutable `escouade-plugins` signé ; les adaptateurs en process des sources et des hébergeurs | ESC-173 à 177, 179, 181 ; INT-4, INT-5 |
| 2.1.x | 3. Les comptes partagés ; 4. les sources en process, une par une ; 5. les hébergeurs en process (GitHub avec ses Issues d'abord, puis GitLab avec ESC-56 et ESC-165, Bitbucket, Gitea, Azure DevOps avec ESC-241), au fil d'ESC-56 à 70 | INT-6, INT-7.1 à 7.4, INT-8.1 à 8.5, ESC-165, ESC-241 |
| 2.2 | 6. Le contrat des harness en process, prouvé sur le faux moteur ; 7. les harness en process, un par un, Claude Code en dernier ; PLG-6 sur ce contrat ; les plugins tiers en WASM | INT-9, INT-10.1 à 10.5, ESC-178 |
| 2.3 et plus tard | Un harness tiers par `engine/*` documenté ; les services de voix et les environnements de lancement par la même règle | — |

Chaque étape se livre seule : une intégration migrée à la fois, ses tests, ses faux (`fake-*.mjs`, `fake-trackers.mjs`, `fake-gh`), son banc et ses essais réels rejoués **inchangés**. Un ticket qui doit changer un test pour passer n'est pas une migration : il le dit et le fait valider.

### Les décisions (2026-10-26)

Posées par le chef d'escouade, répondues par l'utilisateur le 2026-10-26 :

1. **L'architecture des intégrations : tout en process.** Les plugins de première main (Claude Code, Codex, Copilot CLI, Cursor CLI, OpenCode, GitHub, Jira, Trello, Linear, ClickUp…) sont des process JSON-RPC à part, comme ceux des tiers : un seul chemin, un seul contrat ; livrés et signés avec l'app. (La recommandation était le Rust intégré pour la première main ; le coût du choix et comment on le tient : `docs/PLUGINS.md`, §4.6.)
2. **L'ordre** : sources, comptes partagés, hébergeurs, transport, harness en dernier ; revu là où le process l'impose : les mesures d'abord, puis le transport (PLG-2) et le banc (PLG-3) avant toute migration.
3. **Ce qui reste dans le cœur** : la liste ci-dessus.
4. **Un compte par plugin, partagé par ses points** (un seul GitHub pour les Issues et les PR ; un seul GitLab pour ses Issues et ses MR), avec la migration des comptes enregistrés.
5. **Les plugins de première main** : désactivables pour l'app ou par projet, jamais désinstallables, activés partout par défaut, projets privés compris ; badge « Livré avec Escouade ».
6. **Remplacer un plugin de première main par un plugin tiers** : permis, compte par compte, jamais en silence ; les ids `dev.escouade.*` réservés.
7. **Les tickets en cours** : ESC-201 (ClickUp) fini tel quel ; ESC-165 (GitLab Issues) fait dans le cœur pour la 2.0.3 (décidé le 2026-10-26), sorti avec le plugin « GitLab » par INT-8.2 ; ESC-56 à 58 écrits dans la crate de leur plugin dès le socle, sinon sortis par INT-8.n ; ESC-36 fermé (doublon d'ESC-178).
8. **Les plugins de première main restent dans le dépôt de l'app, chacun lisible seul** (2026-10-29, sur la question de l'utilisateur « tu as bien prévu de les passer sur des dépôts séparés ? ») : un dossier `plugins/<id>/` avec son manifeste, son README et ses tests, vers lequel pointe sa page du store ; pas de dépôts séparés, qui demanderaient une PR par dépôt à chaque changement du protocole et une livraison signée à part. Posé dans INT-4 (ESC-210), suivi par chaque sortie (INT-7 à INT-14). _Retenu._

**À confirmer**, conséquence de la décision 1 : PLG-6 (ESC-178, les harness ACP décrits) sort de la 2.1.0 et passe après INT-9. L'adaptateur ACP générique s'écrit alors une seule fois, en plugin de première main sur le contrat des harness, au lieu d'être écrit dans le cœur puis sorti. Recommandé ; l'autre choix garde PLG-6 en 2.1.0, dans le cœur, et ajoute sa sortie à INT-10.1.

### Les tickets à créer

Les tickets s'appellent ici INT-0, INT-1… ; le chef d'escouade leur donne leurs clés. **Une migration par intégration**, chacune avec ses tests et ses essais réels inchangés ; pour un harness, les mesures d'INT-0 avant et après, notées dans le ticket.

| Plan | Ce qu'il fait | Modèle conseillé | Dépend de |
| --- | --- | --- | --- |
| INT-0. Les mesures de référence | Un banc de flux qui rejoue des flux enregistrés de chaque harness (`tests/fixtures/<harness>/`, et un long tour réel de Claude Code de plusieurs milliers de frames) au rythme réel et au plus vite : délai de la ligne lue à l'événement de la fenêtre (médiane, 99e centile), débit maximal, CPU et mémoire ; le temps jusqu'à la fenêtre et jusqu'au premier agent prêt ; le délai d'une frappe dans une vue terminal. Les chiffres dans `docs/PLUGINS.md`, et le banc rejouable en une commande. | Sonnet medium (back) | — |
| INT-1. Le contrat des sources du Kanban | `TicketSource` et `TicketSourceKind` à la place de l'énumération `integrations::Client`, un registre comme `forge::Registry` ; `Service` ouvert, sérialisé à l'identique (`ExternalRef`) ; côté fenêtre, `Service`, `SERVICE_IDS` et `SHORT` de `src/lib/integrations.ts` lus depuis le registre ; Jira, Trello, Linear, GitHub Issues, ClickUp et GitLab Issues derrière, **sans changer de comportement**. Tests : `integrations_tests.rs`, `fake-trackers.mjs` et les e2e d'import inchangés. | Sonnet medium (back) | ESC-201, ESC-165 |
| INT-2. Le contrat des hébergeurs ouvert | `forge::Registry` construit depuis une liste qu'on pourra remplir depuis les plugins ; `Provider::kind()` et `scopes()` en chaînes possédées ; `CliToken::parse` et `Probe::matches` déclaratifs. Tests : `forge/tests.rs`, `github_tests.rs`, `fake-gh` inchangés. | Sonnet medium (back) | — |
| INT-3. `EngineId` ouvert | Un id ouvert sérialisé à l'identique (`claude`, `codex`…, aucune donnée à migrer : agents, tickets, projets, colonne `engine` des stats) ; les `match` sur `EngineId` remplacés par le registre et `EngineCaps` ; `engine::registry::all()` construit depuis une liste ; « harness absent » pour un agent dont le harness manque. Tests : toutes les suites des harness inchangées, des données d'avant relues. | Sonnet medium (back) | — |
| INT-4. Le registre des plugins de première main et `escouade-plugins` | Le manifeste à plusieurs points et la contribution `accounts` ; l'exécutable `escouade-plugins` à plusieurs entrées (`src-tauri/src/bin/`, comme `escouade-hook`), la crate `escouade-plugin-protocol` partagée avec le cœur ; l'empaquetage et la signature dans `release.yml` (Authenticode, notarisation macOS, Linux) ; l'app ne lance que le sien ; l'origine `builtin`, les ids `dev.escouade.*` réservés ; désactiver pour l'app ou un projet. D'abord avec un seul plugin de démonstration (le faux moteur minimal). Tests : le registre, un id réservé refusé, la signature vérifiée en CI sur les trois systèmes. | Sonnet medium | ESC-173, ESC-174 |
| INT-5. Les adaptateurs en process des sources et des hébergeurs | `ProcessTicketSource` et `ProcessForge` (noms du §4.5 de `docs/PLUGINS.md`), `account/me`, `secrets/get` ; délais, erreurs typées, plantage (la synchro en pause, les PR en « inconnu ») ; le faux plugin (`fake-plugin.mjs`) qui sert une source et un hébergeur ; le banc des plugins joue les mêmes scénarios que les tests des intégrations d'aujourd'hui. | Sonnet medium | ESC-174, ESC-175, INT-1, INT-2 |
| INT-6. Les comptes partagés par plugin | `AccountKind` (tiré de `forge::Provider` et de `integrations::Account`) : hôtes, façons de se connecter, champs déclarés, droits, page du jeton ; un compte sert tous les points de son plugin (la règle de github.com, `Accounts::github`, généralisée) ; la limite d'appels comptée par compte ; migration réversible d'`integrations.json` et des noms au coffre (`plugin:<id>:account:<compte>`) ; le plugin qui sert un compte, choisi à son ajout. Tests : migration aller et retour, un compte GitLab qui sert Issues et MR sur les faux, le coffre qui refuse le secret d'un autre plugin. | Opus medium (interface des comptes) | INT-1, INT-2, INT-4, ESC-54 |
| INT-7.1 à 7.4. Les sources en process | Une par ticket, dans cet ordre : **Trello**, **Linear**, **Jira**, **ClickUp**. Chacune sort en crate dans `escouade-plugins`, avec son manifeste et ses textes (ceux de `sync.rs` compris) ; ses tests passent inchangés contre le plugin en process ; l'onglet Intégrations la lit depuis le registre. | Sonnet medium | INT-5, INT-6 ; chacune après la précédente |
| INT-8.1 à 8.4. Les hébergeurs en process | Un par ticket : **GitHub** (le plugin complet du §4.4 : compte, Issues, PR, CI, relectures, déploiements ; l'essai réel par `gh`), **GitLab** (avec ESC-56, et GitLab Issues qu'ESC-165 a mis dans le cœur : sa source sort avec lui, ses tests inchangés), **Bitbucket** (avec ESC-57), **Gitea et Forgejo** (avec ESC-58). Chacun avec ses tests inchangés et sa vue des hébergeurs lue depuis le registre ; un hébergeur arrêté le dit sur la carte. | Sonnet medium ; Opus medium pour GitHub (le plus utilisé) | INT-5, INT-6 ; 8.2 à 8.4 après leur client |
| INT-8.5. Azure DevOps en process | **Azure DevOps** (dev.azure.com, `*.visualstudio.com`, Azure DevOps Server) : dans le cœur depuis ESC-241 (2.0.x : compte par PAT ou `az`, PR, checks des stratégies et d'Azure Pipelines, journaux, relance, fils, relecture) et Azure Boards comme source du Kanban depuis ESC-242 (sur le même compte : projets, états par processus, import WIQL, Acceptance Criteria, état et commentaires, PR liée à l'élément de travail) ; sorti en plugin de première main avec GitLab, Bitbucket et Gitea, Azure Boards avec lui comme GitLab Issues avec GitLab, ses tests (`forge/azure_tests.rs`, `integrations/azure_tests.rs`, `azure-devops.spec.ts`) et ses essais réels `real_azure_devops` et `real_azure_boards` inchangés. | Sonnet medium | INT-5, INT-6 ; avec INT-8.2 à 8.4 |
| INT-9. Le contrat des harness en process | `ProcessEngine` et `ProcessEngineProcess` ; `engine/launch` (la description de lancement, vérifiée par le cœur : clôture, jeton absent des arguments) et le lancement par le cœur ; `agent/frames` et `agent/items` par lots, les frames en chaîne brute ; les effets ; `agent/attach` et la file des frames pendant une relance ; `terminal/launch` et `terminal/hook` ; `engine/oneShot`. Prouvé sur le faux moteur minimal (`engine/minimal/`) devenu un plugin, au banc des harness, avec un plantage en plein tour ; mesuré contre INT-0. | Opus medium (risque) | INT-0, INT-3, INT-4 |
| INT-10.1 à 10.5. Les harness en process | Un par ticket, dans cet ordre : **Cursor CLI** (sur le plugin ACP générique de PLG-6), **OpenCode**, **Copilot CLI**, **Codex**, **Claude Code**. Chacun : sa traduction séparée de l'état de l'agent, sa vue terminal, ses textes rédigés ; le banc, les e2e et les tests de sa vue inchangés ; les mesures d'INT-0 avant et après, dans les seuils ; ses essais réels rejoués et notés (`real-harnesses.spec.ts`, `real_fence`, `real_<harness>_view`, et pour Claude Code `real_claude_models`, la passation et le changement de compte au seuil), ceux que le mode auto refuse lancés à la main avant la fin du ticket. | Opus medium | INT-9 ; 10.1 après ESC-178 ; chacun après le précédent |
| INT-11. La doc de référence | `docs/PLUGINS.md` §4 passé de l'analyse à la référence au fil des étapes ; `docs/ENGINES.md`, « Ajouter un moteur » devenu « Écrire un plugin de harness » ; les pages `kanban-sources.md`, `forges.md`, `harnesses.md` du site (PLG-11) ; le CHANGELOG fr/en à chaque étape livrée. | Sonnet medium | chaque étape |

Ordre : INT-0, INT-1, INT-2, INT-3 (dès maintenant, en parallèle) → ESC-173, ESC-174, ESC-175 → INT-4 → INT-5 → INT-6 → INT-7.1 à 7.4 et INT-8.1 à 8.5 → INT-9 → ESC-178 → INT-10.1 à 10.5 ; INT-11 suit.

### Les tickets PLG à modifier

| Ticket | Ce qui change |
| --- | --- |
| ESC-172, renommage des jetons | Rien. |
| ESC-173, PLG-1, le manifeste | Plusieurs points par plugin (`ticketSources`, `forges`, `harnesses`) et la contribution `accounts` ; `run` qui désigne `{escouade}/escouade-plugins` ; les parties déclaratives lues sans process (`locate`, hôtes, capacités) ; l'origine `builtin` donnée par l'app, les ids `dev.escouade.*` réservés. |
| ESC-174, PLG-2, le transport | Il porte maintenant les intégrations d'Escouade : les lots dans les deux sens, les chaînes brutes, un process par plugin servant plusieurs agents (multiplexés par id), la file bornée pendant une relance, `attach` ; un délai par méthode (court pour une décision, long pour un import) ; ses mesures (aller-retour, débit) comparées à INT-0. |
| ESC-175, PLG-3, le banc | Des scénarios par point (source, hébergeur, harness), rejoués sur un plugin de première main et sur le faux plugin ; le plantage en plein tour et en pleine synchro ; un mode mesure qui rejoue INT-0. |
| ESC-176, PLG-4, Réglages › Plugins | Le groupe « Livrés avec Escouade », désactiver sans désinstaller, l'état du process (actif, relancé, arrêté après trois plantages, avec « Relancer »), le plugin qui sert un compte ; les vues Harness, Intégrations et Hébergeurs lues depuis le registre ; l'aperçu `impeccable` les montre. |
| ESC-177, PLG-5, les événements | Rien ; les plugins de première main pourront s'y abonner comme les autres. |
| ESC-178, PLG-6, les harness ACP décrits | Sort de la 2.1.0 (à confirmer) : écrit en plugin de première main (`dev.escouade.acp`) sur le contrat d'INT-9 ; dépend d'INT-3 et d'INT-9 ; Cursor CLI (INT-10.1) le prend comme base. |
| ESC-179, PLG-7, les thèmes | Rien. |
| ESC-180, PLG-8, les règles d'autorisation | Rien ; la décision d'un plugin s'insère avant la demande, que le harness soit en process ou non. |
| ESC-181, PLG-9, le SDK | La crate Rust des types (`escouade-plugin-protocol`) avancée en 2.1.0, partagée par le cœur et les plugins de première main ; les types TypeScript générés depuis elle ou depuis le même schéma. |
| ESC-182, PLG-10, les exemples | Un sixième exemple : une source tierce (un CSV du dépôt, ou YouTrack) ; l'exemple Gemini CLI suit ESC-178. |
| ESC-183, PLG-11, la doc du site | Les pages des sources, des hébergeurs et des harness décrivent les plugins de première main et leur contrat. |
| ESC-184, PLG-12, docs internes et essai réel | Les essais réels des plugins de première main migrés à la date de la release, en plus des cinq exemples. |

Et ailleurs : **ESC-165** fait dans le cœur pour la 2.0.3 (GitLab Issues sur les comptes de l'hébergeur GitLab), sorti avec le plugin « GitLab » par INT-8.2 ; **ESC-56, 57, 58** écrits dans la crate de leur plugin si INT-4 est fait, sinon dans le cœur puis sortis par INT-8.2 à 8.4 ; **ESC-69, 70** : la page dit qu'ils sont des plugins de première main, l'essai réel passe aussi par « plugin arrêté puis relancé » ; **ESC-202** : le store refuse les ids `dev.escouade.*`, l'app montre « Livré avec Escouade » ; **ESC-36** à fermer (doublon d'ESC-178). ESC-201, ESC-53, ESC-59 à 63 et ESC-65 à 68 (le flux des PR, au cœur) ne changent pas.

**Les tickets INT, créés le 2026-10-26 pour jeudi** : INT-0 ESC-206, INT-1 ESC-207, INT-2 ESC-208, INT-3 ESC-209, INT-4 ESC-210, INT-5 ESC-211, INT-6 ESC-212, INT-7.1 ESC-213, INT-7.2 ESC-214, INT-7.3 ESC-215, INT-7.4 ESC-216, INT-8.1 ESC-217, INT-8.2 ESC-218, INT-8.3 ESC-219, INT-8.4 ESC-220, INT-9 ESC-221, INT-10.1 ESC-222, INT-10.2 ESC-223, INT-10.3 ESC-224, INT-10.4 ESC-225, INT-10.5 ESC-226, INT-11 ESC-227. Les modifications des tickets PLG (« Les tickets à modifier » ci-dessus) sont reportées dans leur description ; ESC-36, doublon d'ESC-178, est fermé.

## Idées pour une 2.x (notées le 2026-10-26)

Notées par l'utilisateur pour plus tard, sans ticket ni étude pour l'instant :
- [ ] **Gestion documentaire.** Devenue l’onglet Docs, conçu le 2026-10-27 avec les deux autres : voir docs/FACTORY.md.
  - Centraliser de façon standardisée la documentation du projet dans le dépôt : la spec, le design, l'architecture, les ADR…
  - Pouvoir interagir simplement avec cette spec, voir les ADR, et tout interroger facilement, aussi par le serveur MCP.
  - Une base vectorielle qui tourne dans Escouade, pour du RAG automatique sur cette documentation, ou du RAPTOR (résumés en arbre) selon ce qui marche le mieux : à comparer sur une vraie doc de projet (noté le 2026-10-27).
- [ ] **Un index du code pour les agents** (noté le 2026-10-27). Conçu le 2026-10-27 avec les deux autres : voir docs/FACTORY.md.
  - Proposer d'office, et en option, un système comme tokensave qui indexe le code du projet.
  - Les agents y trouvent le code par le serveur MCP d'Escouade, plutôt qu'en lisant à coups de grep : moins de tokens, des réponses plus justes.
  - À penser :
    - l'index tenu à jour quand le code change (worktrees des agents compris) ;
    - les projets privés (ESC-140) et les fichiers interdits (`fence.rs`) ;
    - le lien avec la base vectorielle de la gestion documentaire ci-dessus.
- [ ] **Overseer.** Conçu le 2026-10-28 avec les tâches programmées : voir docs/OVERSEER.md.
  - Un agent qu'on lance en dehors de tout projet, à la voix ou non.
  - Il a accès à tous les projets publics, pour les piloter.
  - Les projets privés (ESC-140) restent hors de sa vue.
- [x] ~~**Performance Loop.**~~ Abandonnée le 2026-10-28 : remplacée par le bouton « Planifier » de l’espace Docs, qui fait créer les tickets d’une spec modifiée par un agent planificateur (voir docs/FACTORY.md).
- [ ] **Planification d'agents.** Conçue le 2026-10-28 avec l'Overseer : voir docs/OVERSEER.md.
  - Planifier des tâches qu'Escouade lance seule : à une heure, à intervalle (tous les matins, chaque lundi…), ou à un événement.
  - Exemples : une revue de dépendances, un ticket à reprendre, un rapport.
  - À penser avec le pilote auto du Kanban, les comptes et leurs quotas, et le mode serveur (`docs/REMOTE.md`), pour que ça tourne même quand la fenêtre est fermée.
- [ ] **Mémoire partagée entre tous les harness** (notée le 2026-10-27). Conçue le 2026-10-27 avec les deux autres : voir docs/FACTORY.md.
  - Une seule mémoire, que Claude Code, Codex, Copilot CLI, Cursor CLI et OpenCode lisent et écrivent tous, au lieu de la mémoire propre à chacun.
  - Accessible par le serveur MCP d'Escouade : lire, chercher, écrire, oublier.
  - À penser :
    - la portée (par projet, globale, partagée avec l'équipe comme `.escouade/`) ;
    - les projets privés (ESC-140) ;
    - ce qui est repris des mémoires existantes (`CLAUDE.md`, `AGENTS.md`, la mémoire de Claude Code) ;
    - le lien avec la gestion documentaire ci-dessus.
- [ ] **Les tokens « chauds » et « froids »** (noté le 2026-10-27). Conçu le 2026-10-28 : la température du cache en degrés, voir docs/FEATURES.md.
  - Voir ce qui est utilisé en tokens servis par le cache (lus du cache, « chauds ») et en tokens payés plein tarif (écrits dans le cache ou hors cache, « froids »).
  - Par agent, par tour, et dans les statistiques, pour chaque harness qui le dit (Claude Code et ses `cache_read_input_tokens` / `cache_creation_input_tokens`, Codex, OpenCode…).
  - Avec ce que le cache fait gagner, et quand il expire (5 min, 1 h).
- [ ] **Choisir ce que l'interface affiche** (noté le 2026-10-27). Conçu le 2026-10-28 : des fonctionnalités à activer, présentées par un assistant, voir docs/FEATURES.md.
  - Pouvoir afficher ou cacher chaque élément de l'interface : jauges de la barre d'état, colonnes et lignes des cartes, compteurs, onglets, boutons de l'en-tête…
  - Un réglage par élément, groupé par zone, avec un aperçu, et un retour aux réglages par défaut.
  - À penser avec les thèmes et les plugins (PLG-7, ESC-179).
- [ ] **Des réglages proposés au bon moment** (noté le 2026-10-27, idée de Zied). Conçu le 2026-10-28 : proposer d'allumer une fonctionnalité coupée, voir docs/FEATURES.md.
  - Le constat : « toutes ces configurations, ça commence à être beaucoup ». Personne ne fait le tour de tous les réglages.
  - L'idée, comme Orca : Escouade détecte certains comportements et propose le réglage qui va avec, dans une petite fenêtre en bas à droite.
    - Exemples : toujours relancer à la main après la limite d'usage → la reprise automatique ; toujours choisir le même modèle → le défaut du projet ; ouvrir le panneau à chaque fois → le garder ouvert ; une branche de départ toujours changée → la régler pour le projet.
  - À penser :
    - « Activer », « Pas maintenant », « Ne plus proposer » ;
    - une seule proposition à la fois, jamais pendant un tour ni une saisie ;
    - rien d'envoyé hors de la machine ;
    - avec « Choisir ce que l'interface affiche » ci-dessus.

## Restes du redesign

Notés par la revue finale (ESC-84, `design/app-redesign/final/REVUE.md`) :
- [x] Les captures du site, prises dans la vraie app 2.0 sur une démo fictive (`npm run site-shots`), et sa démonstration au design 2.0 (ESC-187).
- [ ] La vidéo et son affiche, qui montrent encore l'app de la 1.8 (ESC-85).
- [ ] Les anciens noms de jetons (`--ok`, `--accent`, `--line2`…), encore lus à une trentaine d'endroits : un renommage mécanique.
- [ ] À 1028 px en anglais, l'en-tête d'un lancement coupe sa commande avant son dossier.
- [ ] Sur une carte d'agent étroite, avec sous-agents et plan, le plan pourrait céder avant le modèle.
- [ ] macOS : vérifier sur un Mac que les feux tombent au milieu de la barre de titre de 44 px.
