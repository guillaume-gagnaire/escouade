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

## Plugins, place de marché et « Tout l'externe en plugins »

Les chapitres « Plugins (demandé le 2026-10-25) », « La place de marché store.escouade.dev (demandé le 2026-10-26) » et « Tout l'externe en plugins (demandé le 2026-10-26) » sont dans `docs/TODO-2.2.md`, qui en garde l'unique copie (versions, décisions et tickets). Référence : `docs/PLUGINS.md`, `docs/STORE.md`.

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
