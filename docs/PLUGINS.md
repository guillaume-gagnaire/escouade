# Les plugins d'Escouade : la référence

Ce document dit **ce qui a été construit**, et où. Il était l'analyse d'ESC-170 (2026-10-25) ; ESC-184 (PLG-12, 2026-11-04) l'a passé de l'analyse à la référence. Le texte de l'analyse reste dans l'historique git (`git show e05a5265:docs/PLUGINS.md`). Le plan et ce qui reste à faire sont dans `docs/TODO-2.2.md`, chapitres « Plugins » et « Tout l'externe en plugins » ; la place de marché dans `docs/STORE.md` ; la documentation des auteurs sur le site (`website/content/docs/*/plugins/`).

- §1 : les points d'extension, un par un, et ce qui n'en est pas encore un ;
- §2 : l'architecture retenue, et pourquoi ;
- §3 : le modèle de confiance et de distribution ;
- §4 : tout ce qui est externe est un plugin, en process (ESC-203) ; ce chapitre est encore celui de l'étude, ESC-227 (INT-11) le passe en référence ;
- §5 : les programmes en JS/TS sans Node, en WASM (ESC-319) ;
- §6 : les sources de l'étude ;
- §7 : l'essai réel (ESC-184) ;
- §8 : tous les plugins en WASM, la première main comprise : l'étude et le plan (ESC-330), la cible de la 2.2.

**La règle, depuis ESC-203 : tout ce qui est externe est un plugin.** Les intégrations d'Escouade sont des plugins de première main, livrés et signés avec l'app, et, comme ceux des tiers, des process qui parlent le même protocole (§4, décidé le 2026-10-26).

## En bref

- **Un plugin est un dossier avec un manifeste** (`escouade-plugin.json`). Il apporte des **contributions déclarées** : un thème, un webhook, un harness ACP, un compte, une source du Kanban, un hébergeur… Escouade les lit et les montre avec ses propres composants. Il peut aussi apporter un **programme**, qui parle à Escouade en **JSON-RPC 2.0**.
- **Trois genres de plugin** : **déclaratif** (rien ne s'exécute : `dev.escouade.high-contrast`, `discord`, `goose`) ; **programme en JS/TS** (`run.js`, un bundle que le store compile en WASM et que l'app fait tourner par wasmtime, dans son process, sans Node : `dev.escouade.slack` et `team-rules` depuis leur 2.0.0) ; **programme natif** (`run.command`, un process : la première main en Rust, `escouade-plugins <nom>`, et les binaires tiers attestés).
- **Pas de HTML libre dans la fenêtre** : ce qu'un plugin montre est déclaré et rendu par Escouade. Les menus, les badges et la barre d'état déclarés ne sont pas encore servis (§1.9).
- **La distribution** : la place de marché store.escouade.dev (`docs/STORE.md`), soumission depuis un dépôt public, revue en local et badge « Vérifié par Escouade » signé hors du serveur. L'installation depuis un dossier ou un dépôt git reste permise, « non vérifié ».
- **Vingt-deux plugins livrés avec l'app** à ce jour (`plugins/<id-court>/`, §8.6.1), dont le harness de démonstration des builds de développement (`tests/fixtures/plugins/minimal/`), et **cinq plugins de première main hors de l'app** (dans le même dossier) : quatre au store, signés, et Goose, écrit mais pas encore soumis.

## Les pièces

| Pièce | Où | Ce qu'elle fait |
| --- | --- | --- |
| Le manifeste | `crates/escouade-plugin-cli/src/manifest.rs`, `schemas/plugin-manifest.mjs` (deux lecteurs, un corpus : `tests/fixtures/plugins/manifests.json`), `schemas/plugin-1.json` | Lit et refuse, champ par champ, en français et en anglais |
| Le chargement et l'installation | `src-tauri/src/plugins/mod.rs` | Installe (dossier, dépôt git, store, en développement), fixe l'empreinte, tient `<data>/plugins/` et les choix de l'utilisateur |
| Les plugins livrés | `src-tauri/src/plugins/builtin.rs`, `plugins/<id-court>/` | Le registre embarqué, la vérification de `{escouade}/…` |
| Le runtime | `src-tauri/src/plugins/runtime.rs` | Un process par plugin dans un Job object, démarré à la demande, délais, relances, mémoire, fermeture |
| Le WASM | `src-tauri/src/plugins/wasm.rs`, `crates/escouade-plugin-cli/src/wasm.rs` et `wasm_prelude.js`, `src-tauri/escouade-js` | L'hôte wasmtime, `net/fetch`, les limites, le cache des précompilés dans l'app ; la compilation (Javy, `escouade-js`, le prélude) dans le crate sans Tauri que l'image du store construit (§5, ESC-328) |
| Le protocole | `schemas/plugin-protocol-1.json` (le socle, dont le TS du SDK est généré), `crates/escouade-plugin-protocol` (les types partagés avec le cœur : comptes, sources, hébergeurs, harness, voix) | Un seul protocole pour les deux transports |
| Les faits | `src-tauri/src/facts.rs`, `src-tauri/src/plugins/events.rs` | Le bus de faits typés, les webhooks et les commandes déclarés |
| Les autorisations | `src-tauri/src/plugins/decide.rs` | Les plugins qui décident, un refus qui l'emporte, le délai |
| Les adaptateurs | `src-tauri/src/plugins/adapters.rs` (sources, hébergeurs), `hosted.rs` (un client par compte), `harness.rs` et `shared.rs` (harness), `reader.rs` (workflows), `accounts.rs` (comptes), `src-tauri/src/launchenv.rs`, `src-tauri/src/voice/service.rs` | Ce que le cœur appelle, sans savoir qui répond |
| Le journal | `src-tauri/src/plugins/journal.rs` | Démarrages, plantages, appels, décisions ; `<data>/plugin-logs/<id>.log`, secrets masqués |
| Le store dans l'app | `src-tauri/src/plugins/store.rs`, `store_keys.rs` | Catalogue, liste de révocation, téléchargement et vérifications, sceau revérifié hors ligne |
| Le banc | `src-tauri/src/plugins/bench/`, la CLI `escouade-plugin` (`test`, `dev`, `validate`, `build`) | La conformité d'un plugin, livrée avec l'app |
| Le kit Rust | `crates/escouade-plugin-kit`, `crates/escouade-acp` (MIT, sur crates.io, §8.16) | La boucle des plugins Rust ; le client ACP et son service |
| Le SDK | `sdk/plugin` (`@escouade-dev/plugin`, MIT), `sdk/create-escouade-plugin` | TypeScript, faux Escouade, gabarit |
| La fenêtre | `src/components/plugins/`, `src/lib/plugins*.ts`, `src/lib/plugin-theme*.ts`, `src/lib/proposed-plugins.ts` | La vue Plugins, le store, les thèmes, les plugins proposés |

## 1. Les points d'extension

**Un point d'extension est servi** quand le cœur s'en sert ; un manifeste peut en déclarer davantage (le schéma les réserve). Le tableau du §1.14 les résume. Tout point est ouvert aux plugins de première main et aux tiers, sauf ce que dit le §4.10.

Le protocole de base (`schemas/plugin-protocol-1.json`, version 1, délai par défaut 5 s) :

| Méthode | Sens | Délai | Ce qu'elle fait |
| --- | --- | --- | --- |
| `initialize` | vers le plugin | 10 s | La poignée de main, versions et capacités |
| `shutdown` | vers le plugin | 1 s | Escouade ferme |
| `events/fact` | vers le plugin, notification | — | Un fait de `permissions.events`, d'un projet où le plugin est actif |
| `permissions/decide` | vers le plugin | 500 ms | Une demande d'un agent, avant l'utilisateur |
| `secrets/get` | vers Escouade | 5 s | Un de ses secrets (`plugin:<id>:<clé>`) |
| `settings/get` | vers Escouade | 5 s | Ses réglages ; avec un projet, ceux de son `.escouade.yml` par-dessus |
| `files/read` | vers Escouade | 5 s | Un fichier de son dossier (256 Kio, texte, jamais le manifeste ni un fichier caché) |
| `net/fetch` | vers Escouade | 10 s | Une requête https vers `permissions.network` (WASM seulement) |
| `fs/read`, `fs/write`, `fs/stat`, `fs/list`, `fs/canon` | vers Escouade | 5 s | Les fichiers de `permissions.files` (WASM seulement, §8.4.6) |
| `fs/gitProtected` | vers Escouade | 1 s pour git | Si git ne peut rien lancer de l'agent dans son dossier, pour un harness en WASM (`run.wasm`, §8.26) |
| `fs/fileOnly` | vers Escouade | 1 s pour git | Si l'écriture d'un chemin du dossier d'un agent n'atteint qu'un fichier : un fichier qui est là, ou rien et aucun sous-module de l'index de git dessous ; un booléen, sans racine (Antigravity, ESC-389) |
| `proc/spawn`, `proc/write`, `proc/kill` | vers Escouade | 5 s | Les programmes de `permissions.commands` (WASM seulement, §8.4.7) |
| `proc/out`, `proc/exit` | vers le plugin, notification | — | Ce qu'un programme écrit, et sa fin |
| `log` | vers Escouade, notification | — | Une ligne de son journal |

Les points servis par un programme ajoutent leurs méthodes (`account/*`, `tickets/*`, `forge/*`, `engine/*`, `agent/*`, `terminal/*`, `voice/*`, `launch/*`, `workflow/read`), dans `crates/escouade-plugin-protocol` ; le §4.5 les dit point par point. Les erreurs sont celles de `x-errors` : celles de JSON-RPC, puis `notYours` -32001, `keychain` -32002, `refused` -32003, `failed` -32004, `notFound` -32005.

### 1.1 Les harness

**Servi.** Trois façons, de la plus simple à la plus complète :

1. **Décrit par ACP** (`contributes.harnesses[].acp`, PLG-6, ESC-178) : un manifeste, sans code, et le plugin livré `dev.escouade.acp` (`crates/escouade-acp`) lance l'agent, lui parle et montre sa conversation, ses permissions, ses diffs. Exemple : `plugins/goose/`.
2. **Un plugin ACP avec ses extensions** (`dev.escouade.cursor-cli`, `dev.escouade.mistral`, `dev.escouade.gemini-cli`), sur le même client.
3. **Un client à lui, en process** (`dev.escouade.opencode`, `dev.escouade.copilot-cli`, `dev.escouade.codex`), par le contrat `engine/*` et `agent/*` (INT-9, ESC-221).

Dans les trois cas, le cœur garde le lancement (il vérifie la description que rend le plugin, `check_launch`), le PTY et la vue terminal (`terminal/launch`, `terminal/hook`), les comptes et ce qui n'est pas la conversation ; le moteur du registre (`engine/<harness>/`) n'en est plus que la façade. L'id d'un harness est ouvert (`EngineId`, ESC-209) : un agent d'un harness absent est gardé, « harness absent ». Le contrat, le coût mesuré et les harness un par un : §4.5 et §4.6. Antigravity CLI et Claude Code sont les derniers à sortir (ESC-283, ESC-226). `engine/*` n'est pas encore ouvert aux tiers (§4.10) : un harness tiers passe par l'ACP décrit.

Un harness est le point de plus fort risque : il lance un programme, reçoit le jeton du serveur MCP d'Escouade, et doit tenir la clôture des projets privés. L'ACP décrit la tient pour lui, par `fs/*`, et le dit quand l'agent n'y passe pas.

### 1.2 Les sources du Kanban

**Servi** (`contributes.ticketSources`, sur un compte déclaré ; `tickets/containers`, `tickets/states`, `tickets/issues`, `tickets/setState`, `tickets/comment`). Le cœur appelle `TicketSource` (`src-tauri/src/integrations/`), derrière lequel `ProcessTicketSource` parle au plugin ; la synchro des colonnes reste au cœur. Sortis du cœur : Jira, Linear, Trello, ClickUp, GitHub Issues, GitLab Issues, Azure Boards. Une source tierce (YouTrack, Asana…) est le même contrat ; un délai par appel tient un plugin lent, et un plugin coupé ou arrêté se dit dans la synchro.

### 1.3 Les hébergeurs de code

**Servi** (`contributes.forges`, sur un compte déclaré ; `forge/*`, `account/me`). `ProcessForge` répond au trait `Forge` (`src-tauri/src/forge/`), chaque méthode refusée par défaut (`unsupported`), selon les capacités déclarées. Sortis du cœur : GitHub, GitLab, Bitbucket Cloud, Gitea et Forgejo, Azure DevOps. Un plugin coupé ou arrêté se dit sur la carte de l'agent et dans Réglages › Intégrations. Ni `ci` ni `deployments` à part, pour un service qui n'est pas l'hébergeur (Jenkins, Coolify, Vercel) : pas encore (§4.4).

### 1.4 Les services de voix

**Servi** (`voice/*`, le trait `VoiceService` de `src-tauri/src/voice/service.rs`, INT-12 et INT-13). OpenAI, Groq, ElevenLabs et Deepgram sont des plugins livrés, Mistral est dans `dev.escouade.mistral` ; la clé reste au trousseau (`dictation-<service>`). Whisper et Piper, hors ligne, restent au cœur. Un plugin qui ne démarre pas laisse le service du cœur répondre, et son journal dit pourquoi. Détail, protocole et mesures : §4.13.

### 1.5 Les outils du serveur MCP d'Escouade

**Pas servi.** Un plugin n'ajoute pas d'outils au serveur d'Escouade et n'en est pas client : chacun demande un chemin d'injection à tenir et un jeton à sa portée (2.2 ou après, `docs/TODO-2.2.md`). Les outils d'un agent restent ceux du serveur (`src-tauri/src/mcp/`).

### 1.6 Les événements

**Servi** (PLG-5, ESC-177). Le bus est `src-tauri/src/facts.rs` : un `Body` typé par fait, émis là où l'état change, et non dérivé des `UiEvent` de la fenêtre. Trente-neuf faits en version 1 :

- les agents : `agent.created`, `agent.renamed`, `agent.status`, `agent.stopped`, `agent.handover` ; les tours : `turn.ended` ;
- les demandes : `request.permission`, `request.question`, `request.answered` ;
- les tickets : `ticket.created`, `ticket.column`, `ticket.report`, `ticket.approved`, `ticket.rejected`, `ticket.merged`, `ticket.conflict` ;
- les projets : `project.added`, `project.closed`, `worktree.created`, `worktree.removed` ;
- les lancements : `launch.started`, `launch.ready`, `launch.stopped`, `tests.result` ;
- l'app : `app.started` ;
- les docs : `docs.written`, `docs.filed`, `docs.gaps`, `memory.written`, `memory.forgotten`, `decision.proposed`, `decision.accepted` ;
- les PR et les comptes : `pr.opened`, `account.switched` ;
- le workflow : `workflow.step_entered`, `workflow.step_left`, `workflow.looped_back`, `workflow.left`, `workflow.changed` (ESC-200).

Un plugin les reçoit de trois façons : `events/fact` dans son programme (`permissions.events`, globs permis), un **webhook** déclaré (`contributes.webhooks` : une URL et un gabarit par fait, vers un hôte de `permissions.network`), une **commande** déclarée (`contributes.commands`, `permissions.commands`, approuvée telle quelle par l'utilisateur dans la page du plugin). Les faits d'un projet ne vont qu'aux plugins actifs dans ce projet ; ceux d'un projet privé, qu'à ceux qui y sont activés un par un.

### 1.7 Les autorisations et les questions

**Servi pour les autorisations** (PLG-8, ESC-180, `src-tauri/src/plugins/decide.rs`). Un plugin qui a `permissions.decide` reçoit `permissions/decide` avant que la demande n'arrive à l'utilisateur : autoriser, refuser avec un message, ou laisser. Plusieurs plugins répondent en parallèle, **un refus l'emporte** ; passé 500 ms (3 s une fois pour un process pas encore lancé), la demande va à l'utilisateur. La réponse dit « Autorisé par <plugin> via Escouade » et s'inscrit au journal. La permission est montrée en rouge, n'est jamais donnée par un projet, et ne s'applique ni à `ExitPlanMode` ni aux questions. Les questions, elles, restent à l'utilisateur : aucun plugin n'y répond.

### 1.8 Les textes rédigés, les consignes et la passation

**Pas servi.** Aucun plugin ne transforme un message de commit, une description de PR ou un nom d'agent, et aucun n'ajoute de consignes au protocole d'un ticket. Ces textes restent ceux du cœur (`engine/oneshot.rs`, `board.rs`, `instructions_room`) ; les textes rédigés d'un harness sont en revanche décrits par son plugin (`engine/oneShot`, `engine/oneShotRead`).

### 1.9 L'interface

**Servis** : la vue Plugins (`src/components/plugins/`), le rendu des réglages et des secrets déclarés (`contributes.settings`, `contributes.secrets`), les thèmes (§1.10), les modèles et modes d'un harness dans les menus de l'app. Ils sont rendus par les composants d'Escouade, dans les deux langues.

**Déclarés mais pas servis** : `contributes.menus`, `statusBar`, `badges`, `projectTemplates` et `escouadeYml` : le schéma les réserve (ils s'affichent dans la fiche d'un plugin comme ce qu'il ajoute), rien de la fenêtre ne les rend encore. Pas de panneau, de page ni de fenêtre apportés par un plugin ; pas de vue libre : l'iframe isolée reste à essayer sous WKWebView et WebKitGTK (§2.2).

### 1.10 Les thèmes et les langues

**Les thèmes sont servis** (PLG-7, ESC-179, `contributes.themes`, `src/lib/plugin-themes.ts`, `plugin-theme.svelte.ts`) : un thème, clair et sombre, donne des jetons de la liste `THEME_TOKENS` (tenue égale à `src/app.css` par Vitest), filtrés (les couleurs seulement, une valeur douteuse est écartée et dite) ; il touche la fenêtre, les terminaux et l'éditeur, et peut donner sa palette de couleurs de projet. Il se choisit dans Réglages › Application › Couleurs. Exemple : `plugins/high-contrast/`.

**Les langues ne sont pas servies** : le français et l'anglais sont dans `src/lib/i18n/` et `src-tauri/src/i18n.rs` ; ouvrir `Lang` reste à faire (2.3 ou après).

### 1.11 Les recettes, les worktrees, l'assistant et `.escouade.yml`

**Servis :**

- **l'environnement de lancement** (`contributes.launchEnvironments`, INT-14, ESC-286, `src-tauri/src/launchenv.rs`) : `detect`, `program`, `up`, puis `launch/ls`, `launch/down`, `launch/destroy` ; isola est le premier, `dev.escouade.isola`, et les commandes passent par l'approbation à empreinte de `.escouade.yml` ;
- **le lecteur de workflow, en contrat seulement** (`contributes.workflowReaders`, ESC-200, `src-tauri/src/plugins/reader.rs`) : `workflow/read` sur les fichiers que le manifeste déclare ; le contrat et ses tests sont là, rien dans l'app n'appelle encore un lecteur (§4.5) ;
- **les plugins proposés par un projet et leurs réglages** (PLG-14, ESC-288, §3.3) : `plugins.recommended` et `plugins.<id>` de `.escouade.yml`.

**Pas servis** : les étapes de mise en place proposées par un plugin, les modèles de projet de l'assistant (`projectTemplates`).

### 1.12 Les statistiques, les notifications et la recherche

**Pas servis comme points à part.** Un plugin lit ce qu'il lui faut par les faits (§1.6), et notifie ailleurs par un webhook ou un programme (`dev.escouade.discord`, `dev.escouade.slack`). Il n'y a ni canal de notification déclaré, ni lecture ou export des statistiques, ni source de résultats dans Ctrl+K.

### 1.13 Les comptes, et ce qui reste au cœur

**Les comptes sont servis** (`contributes.accounts`, INT-6, ESC-212, `src-tauri/src/plugins/accounts.rs`, `AccountKind`, §4.4) : les hôtes, les façons de se connecter, les champs, les droits, la page du jeton, partagés par les points d'un même plugin. Quand plusieurs plugins proposent un même genre de compte, « Connecter… » demande lequel le sert.

**Le coffre** (`integrations/secrets.rs`, `SecretStore`) n'est pas un point d'extension : c'est le service que tous les plugins utilisent (§3.5).

Restent au cœur, hors de tout plugin : les fichiers joints d'un ticket, le pilote auto et l'ordre de sa file, les commandes `/` d'Escouade, le quota et le changement de compte au seuil, et **les mises à jour d'Escouade, jamais par un plugin**.

### 1.14 Le tableau

| Point d'extension | Contribution | État | Qui s'en sert |
| --- | --- | --- | --- |
| Événements | `permissions.events`, `webhooks`, `commands` | servi | Discord, Slack |
| Autorisations décidées | `permissions.decide` | servi | Règles de l'équipe |
| Thèmes | `themes` | servi | Contraste élevé |
| Harness ACP décrit | `harnesses[].acp` | servi | Goose |
| Harness en process | `harnesses` | servi à la première main ; tiers en 2.3 | Cursor CLI, Mistral Vibe, Gemini CLI, OpenCode, Copilot CLI, Codex ; Antigravity CLI et Claude Code en cours |
| Comptes | `accounts` | servi | ceux des sources et des hébergeurs |
| Sources du Kanban | `ticketSources` | servi | Jira, Linear, Trello, ClickUp, GitHub, GitLab, Azure Boards |
| Hébergeurs | `forges` | servi | GitHub, GitLab, Bitbucket, Gitea, Azure DevOps |
| Services de voix | `voice/*` | servi | OpenAI, Groq, Mistral, ElevenLabs, Deepgram |
| Environnements de lancement | `launchEnvironments` | servi | isola |
| Lecteurs de workflow | `workflowReaders` | contrat seul, rien ne l'appelle | — |
| Réglages et secrets | `settings`, `secrets` | servi | tous |
| Menus, barre d'état, badges | `menus`, `statusBar`, `badges` | réservé | — |
| Modèles de projet, clés de `.escouade.yml` | `projectTemplates`, `escouadeYml` | réservé (`plugins.<id>` est lu) | — |
| Outils MCP, textes rédigés, langues, statistiques, Ctrl+K, canaux de notification, vues libres | — | à faire | — |

## 2. L'architecture

### 2.1 Ce que font les autres

L'étude de 2026-10-25 a comparé VS Code, Zed, Obsidian, Raycast, JetBrains, Claude Code, Tauri et MCP (le tableau, dans l'historique ; les liens au §6). Ce qu'on en a retenu :

1. **Personne ne bac-à-sable le code et l'interface libre à la fois, sauf par la séparation.** VS Code sépare le code (un process) de l'interface (des iframes d'une autre origine) ; MCP Apps fait de même. Zed bac-à-sable le code et n'offre pas d'interface ; Raycast offre une interface riche mais _déclarée_.
2. **Sans bac à sable, la sécurité vient de la confiance et de la revue** : un mode restreint, une revue humaine, des signatures, le droit de tout retirer.
3. **La compatibilité vient d'une version d'API déclarée et vérifiée**, plus un outil qui le vérifie avant la publication.
4. **Escouade parle déjà le langage des process** : ses harness et son serveur MCP sont des process, ses tests reposent sur des faux process, ses Job objects tuent un arbre entier.

### 2.2 Les trois expériences

Faites le 2026-10-25 sous Windows 11, hors du dépôt :

- **WASM dans Rust** (wasmtime 38, Cranelift) : 11 ns par appel ; une boucle infinie arrêtée par l'époque ou le carburant, une mémoire bornée (`StoreLimits`), un plantage rendu en erreur, l'hôte vivant ; 11 Mo de plus. Mesuré dans l'app depuis : §5.3.
- **Une iframe isolée sous WebView2** (`sandbox="allow-scripts"`, sans `allow-same-origin`, CSP `default-src 'none'`) : DOM, IPC de Tauri, `localStorage` et réseau refusés ; `postMessage` passe ; une boucle de 1,5 s n'a pas gelé la page. **Pas essayée sous WKWebView ni WebKitGTK**, où une iframe partage sans doute le process de la page : à vérifier avant d'offrir une vue libre.
- **Un plugin en process** (JSON-RPC sur stdio, une ligne par message) : 43 µs par aller-retour.

### 2.3 Les façons, comparées

| | Process | WASM | JS en iframe |
| --- | --- | --- | --- |
| Plantage, boucle, mémoire | relancé, délai, Job | rattrapés, bornés | dépend du moteur de rendu |
| Réseau, disque, clôture | déclarés, non appliqués | appliqués | appliqués (CSP, origine opaque) |
| Secrets, projets privés | appliqués (ce qui passe par Escouade) | appliqués | appliqués |
| Coût d'un appel | 43 µs | 11 ns | `postMessage` |
| Langages | tous | Rust, Go, C ; JS par un interpréteur | JS |
| Vit sans fenêtre | oui | oui | non |

### 2.4 Ce qui a été retenu

**Un protocole JSON-RPC 2.0, deux transports, une interface déclarée, un niveau déclaratif sans code.**

- **Le déclaratif** : thèmes, webhooks, harness ACP décrits, comptes : rien ne s'exécute, ou seulement le code d'Escouade.
- **Le process** : la première main en Rust (`escouade-plugins <nom>`) et les binaires tiers attestés. C'est ce qu'il faut pour lancer un programme ; le plugin a les droits de l'utilisateur, et Escouade le dit avant l'installation. Il n'applique que ce qui passe par lui : les faits, les secrets, les projets privés, les outils.
- **Le WASM** : les programmes en JS et en TS, pour que Node ne soit jamais un prérequis (ESC-319, décidé le 2026-11-01, §5). Il **applique** ce que le process ne fait que déclarer : le réseau, les fichiers, la mémoire et le temps. La recommandation de 2026-10-25 le gardait pour plus tard ; le prototype a montré qu'un interpréteur embarqué coûte peu (1,3 Mo par plugin, 71 µs par décision, 2 ms pour démarrer). La webview est écartée (§5.1). `node` n'est plus une commande d'un plugin tiers du store.
- **L'interface déclarée** : chaque contribution est rendue par les composants d'Escouade, dans les deux langues, aux règles de `DESIGN.md`, sans HTML d'un tiers dans la fenêtre qui a l'IPC. Les vues libres attendent d'avoir été essayées sous WebKit.

## 3. Le modèle de confiance et de distribution

### 3.1 Le manifeste

Un fichier `escouade-plugin.json` à la racine du dossier du plugin ; `schemas/plugin-1.json` en est le schéma publié (`https://escouade.dev/schemas/plugin-1.json`). La page `plugins/manifest` du site le dit champ par champ. Le manifeste de `dev.escouade.slack` :

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.slack",
  "name": "Slack",
  "version": "2.0.0",
  "description": {
    "fr": "Les tickets à tester et les agents qui attendent, dans le salon Slack de chaque projet.",
    "en": "Tickets to test and agents waiting, in each project's Slack channel."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/slack",
  "escouade": { "api": "1", "min": "2.2.0" },
  "run": { "js": "dist/plugin.mjs" },
  "activation": ["event:ticket.column", "event:agent.status"],
  "permissions": {
    "events": ["ticket.column", "agent.status"],
    "network": ["slack.com"],
    "secrets": ["token"]
  },
  "contributes": {
    "secrets": [{ "key": "token", "title": { "fr": "Jeton du bot Slack (xoxb-…)", "en": "Slack bot token (xoxb-…)" } }]
  }
}
```

- `id` : unique, en nom de domaine inversé, en minuscules. `dev.escouade.*` est réservé (§4.3, `docs/STORE.md` §3.6) ;
- `name` et `description` : un texte, ou un par langue, l'anglais par défaut ;
- `escouade.api` : la version **majeure** du protocole (1) ; `min` et `max` : les versions d'Escouade où il a été essayé. Un plugin hors de sa plage est chargé désactivé, et la raison montrée ;
- `icon` : facultatif (PLG-13, ESC-292). Un PNG carré de 256 px de côté au moins, 1 Mo au plus, relatif au dossier ; ni SVG, ni image distante. Les deux lecteurs en lisent l'en-tête sans le décoder et disent chaque refus. Le store le décode et le réencode sans métadonnées (`docs/STORE.md` §3.3) ; l'app le lit par la commande `plugin_icon`, qui ne sort jamais du dossier. Sans icône, la tuile garde ses deux lettres ;
- `run` : facultatif, sans lui le plugin est **déclaratif**. `command` (avec `windows`, `macos`, `linux`, `args`) ou `js`, jamais les deux (§5.6). `{escouade}/…` est réservé aux plugins livrés ;
- `activation` : ce qui le démarre (un fait, une commande) ; sans activation, il ne tourne pas ;
- `permissions` : `events`, `network`, `secrets`, `commands`, `decide` ; une permission inconnue n'est pas donnée ;
- `contributes` : ses points d'extension (§1.14) ;
- `store` : ce que sa page du store en dit.

Un champ inconnu est ignoré, un manifeste invalide ne charge rien et dit pourquoi, champ par champ.

### 3.2 L'installation, la signature, la mise à jour

- **Quatre origines** : un dossier local, un dépôt git à une étiquette, le store, et « En développement » (un dossier chargé en place et relu à chaque changement). Un plugin livré a l'origine `builtin`.
- **L'intégrité** : chaque version installée est fixée par son empreinte (le commit, le SHA-256 du dossier, ou `store:<SHA-256 de version.json>`) ; une copie dont les fichiers ont changé est chargée désactivée.
- **La signature** : une version revue en local (`/review-plugin`, `npm run plugin-review`) est signée par le chef d'escouade avec une clé minisign gardée sur son poste, jamais sur le serveur ; l'app la vérifie avec les clés publiques embarquées (`store_keys.rs`) : « Vérifié par Escouade », version par version, retirable par une liste de révocation signée (`docs/STORE.md` §4). `dev.escouade.*` ne s'installe que signé. Un plugin installé hors du store reste installable, « non vérifié ».
- **La mise à jour** : jamais en silence. Une nouvelle version est montrée dans la vue Plugins et s'installe sur un clic ; l'accord est redemandé si elle demande plus ; une version refusée laisse la précédente en place.

La place de marché, ses onze contrôles, la compilation et la revue : `docs/STORE.md` et §5.6.

### 3.3 L'activation, les projets privés et `.escouade.yml`

- **Installé** n'est pas **activé**. Un plugin s'active pour toute l'app, ou projet par projet ; par défaut, pour tous les projets **sauf les projets privés**.
- **Un projet privé** (ESC-140) : un plugin ne l'atteint que s'il y est activé un par un. Ses faits, ses tickets et ses textes ne vont pas aux autres plugins. Ce qu'Escouade ne peut pas garantir (un process qui lit le disque) est dit en face du projet privé.
- **Un projet qui propose un plugin** (PLG-14, ESC-288) : `plugins.recommended` (des identifiants du store) et `plugins.<id>` (des valeurs simples) sont des clés du fichier seul, jamais appliquées au projet ni exportées. L'ouverture du projet dit combien il en propose, une bande en tête de Réglages › Projets les nomme avec « Voir dans le store », « Pas pour ce projet » la fait taire sur ce poste. **Rien ne s'installe ni ne s'active seul.** `settings/get` prend un `project` : les réglages du fichier passent devant ceux de l'utilisateur, seulement les clés de `contributes.settings` d'une valeur de leur type, jamais un secret, et seulement si le plugin est actif dans ce projet.

### 3.4 Ce que l'utilisateur voit, et ce qu'il retire

Avant l'installation, une fenêtre dit en mots qui l'a fait et d'où il vient (dépôt, empreinte, sceau), ce qu'il ajoute, et ce qu'il pourra faire, permission par permission, la plus forte d'abord :

- **« Lance un programme sur ton ordinateur, avec tes droits »** (un plugin à `run.command`), ou « Tourne dans Escouade, isolé » (un `run.js`) ;
- « Décide des autorisations de tes agents » (en rouge) ;
- « Reçoit ces événements », avec chaque fait ;
- « Se connecte à », avec l'hôte et ce qu'Escouade en fait : appliqué pour un `run.js` et pour les webhooks d'un plugin déclaratif, seulement déclaré pour un `run.command` ;
- « Garde un secret : Adresse du webhook ».

Après : désactiver (pour l'app ou un projet), retirer une permission (le plugin continue sans elle, ou s'arrête s'il ne peut pas), désinstaller (son dossier, ses réglages et ses secrets effacés, avec une case pour garder ses données) ; et le **journal** de chaque plugin : ses démarrages, ses plantages, ses appels, les autorisations qu'il a décidées. Un plugin livré se coupe, mais ne se désinstalle pas.

### 3.5 Les secrets

Les secrets d'un plugin vont au coffre d'Escouade (`integrations/secrets.rs`, `SecretStore` : le trousseau du système, un fichier dans le bac à sable des tests), sous un nom à lui (`plugin:<id>:<clé>`), saisis dans les champs qu'il déclare (`contributes.secrets`), jamais dans `.escouade.yml` ni dans ses réglages en clair. Il les obtient par `secrets/get`, seulement les siens (`notYours`, -32001 sinon) ; jamais ceux d'un autre plugin, d'un compte, d'un harness ou d'une intégration. Les plugins livrés reçoivent le jeton d'un compte par le même appel (`account:<hôte>`). Le journal masque les secrets.

### 3.6 La télémétrie

Rien de neuf sans accord. Si l'utilisateur a accepté les statistiques d'usage (`telemetry.rs`), l'événement `plugin_installed` part avec `from_store` oui ou non, sans le nom du plugin ; le store compte les installations, sans identifiant, avec le même accord (`POST /api/v1/installs`, `docs/STORE.md` §5.4). Un plugin n'a pas accès à la télémétrie d'Escouade ; s'il envoie ses propres statistiques, c'est une permission réseau qu'il déclare et que l'utilisateur voit.

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
| Le lancement des process des harness | `proc.rs`, `job.rs`, `shellenv.rs`, `resources.rs`, `plugins/shared.rs` | Le cœur lance le harness d'après la description que le plugin lui rend (§4.6) : le Job object, la mesure des ressources, le lancement sous WSL et la vérification de la clôture restent les siens, et le harness survit à un plantage de son plugin. Un harness partagé par des agents (`Launch::shared`, OpenCode) reste lancé et arrêté par le cœur. |
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
- **Fait (INT-4, ESC-210, 2026-10-31).** Le registre est `src-tauri/src/plugins/builtin.rs` : la liste des dossiers `plugins/<id-court>/` (manifeste et README, embarqués), l'origine `Origin::Builtin` (`{ "kind": "builtin" }`), `Hand::Builtin`. Leur version est celle de l'app (le `version` du manifeste est remplacé à la lecture), leur empreinte `builtin:<version>`, ni mise à jour ni liste de révocation. Ils se coupent pour l'app ou un projet (les choix sont gardés dans `plugins.json`, comme ceux des autres), jamais désinstallés. **Un plugin `builtin` et le même id venu du store ou d'un dossier** : le `builtin` l'emporte toujours ; l'aperçu dit le refus (`Preview.refusal`), `install` le refuse, et une copie du même id déjà installée avant cède la place au démarrage (ses choix gardés). Les quatre exemples de `plugins/<id>/` (ESC-314) sont des plugins du store, pas des `builtin`. Le programme est `escouade-plugins` (`src-tauri/src/bin/escouade-plugins.rs`, une entrée par nom : `escouade-plugins minimal`), sur la crate `crates/escouade-plugin-protocol` (`PROTOCOL`, `Hello`, `serve`) que le cœur partage ; avant de lancer un programme `{escouade}/…` le runtime vérifie qu'il est bien celui du dossier de l'app (`builtin::check_program`) et, en release, la signature là où le système la vérifie (`codesign` sous macOS ; sous Windows, Authenticode valide dès que celle de l'app l'est). Le premier est `dev.escouade.minimal`, la démonstration, inscrit au registre des builds de développement et des tests seulement : le registre d'une release ne tient que les vrais, que INT-7/8 livrent un à un (Trello le premier, INT-7.1), tandis que `escouade-plugins` est empaqueté et signé dès maintenant. Le statut Authenticode se lit par PowerShell avec le chemin en variable d'environnement (`ESCOUADE_SIGNED_PATH`), gardé pour la session par fichier (chemin, taille, date) ; la règle (celui de l'app valide ⇒ celui du programme valide) est celle de la release. Le paquet : `escouade-plugins` est signé comme `escouade-plugin` (Authenticode via `scripts/windows-signing.mjs`, fusion universelle macOS via `scripts/universal-hook.sh`, les paquets Linux), vérifié par la CI (présence sur les trois systèmes, `codesign --verify` sous macOS) et par `release.yml` après la construction ; la notarisation et l'Authenticode ne se voient qu'au prochain Release.
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
- **Les PR** : ouvrir, suivre, merger (`Forge::create_pr`, `pull_request`, `pull_requests_of_branch`, `merge_options`, `merge`, `auto_merge`). `merge_options` dit ce que le dépôt autorise pour une PR : ses stratégies et son auto-merge ; un plugin qui ne le sert pas, les capacités déclarées de son fournisseur.
- **La CI** : les vérifications d'un commit, le journal d'un job, la relance (`checks`, `job_log`, `rerun_failed`).
- **Les relectures** : `reviews`, `comments`, `review`, `reply`.
- **Les déploiements** : `deployments`, par environnement (ESC-67).

Dans la fenêtre, l'utilisateur ajoute **un** compte GitHub ; l'import d'issues et le suivi des PR le proposent tous deux. Pour un projet, il peut couper les Issues et garder les PR.

_Fait le 2026-11-01 (INT-8.1, ESC-217)_ : **le premier plugin de première main réel**, `dev.escouade.github`, inscrit au registre des releases (`plugins/builtin.rs`, `GITHUB`). Son dossier `plugins/github/` se lit seul (règle d'INT-4) : le manifeste (`contributes.accounts` du genre `github`, avec `sourcesPublicOnly` : GitHub Issues sur le compte de github.com seulement ; `ticketSources` et `forges` `github` ; `secrets: ["account:*"]`), le README (ce qu'il fait, ses permissions en mots, son compte, comment le lancer contre le faux Escouade) et **sa crate** `escouade-plugin-github` (`src/pulls.rs`, l'ancien `forge/github_api.rs` ; `src/issues.rs`, l'ancien `integrations/github.rs` ; `lib.rs`, ses deux genres). Son programme : `escouade-plugins github`. Sa page du store pointe vers ce dossier (`source` dans `store/server/utils/builtin.ts`, testé contre les dossiers).
- **Le modèle des hébergeurs en plugin** (pour GitLab, Bitbucket, Gitea et Azure DevOps après lui) : le commun est dans la crate `crates/escouade-plugin-kit`, partagée par le cœur et les plugins Rust de première main. `serve` : la boucle en process, asynchrone (les appels répondus à mesure qu'ils finissent, plusieurs à la fois, un lot en lot ; le plugin demande `secrets/get` et écrit `log` pendant qu'il répond). `hosts` : le service de `account/me`, `forge/*` et `tickets/*` (le compte lu de l'appel, son jeton demandé par `secrets/get` de `account:<hôte>`, un client gardé par compte et par jeton pour que les lectures conditionnelles et les limites servent l'appel suivant, les erreurs typées en codes, `{ value, rateLimit }` dans chaque réponse d'un hébergeur). `forge` : le trait `Forge` (le contrat, que le cœur réexporte : `ProcessForge` et les hébergeurs encore au cœur l'implémentent), `Unsupported`, `Pace`. `errors` (`HttpError`, `Refused`, les mêmes types dans le cœur, `integrations` les réexporte), `http` (le client des réglages réseau, `send_named`, `call`, `call_page`), `text` (`criteria_of`, `criteria_in`), `i18n` (`tr!`, la langue de `ESCOUADE_LANG`). Les types de données des hébergeurs (`PullRequest`, `Check`, `RepoRef`…) sont dans `escouade-plugin-protocol::forge` (le cœur les réexporte), ceux des sources dans `::tickets` (la source y est une chaîne ; le cœur garde ses types à `Service` interné), et les codes `code::REFUSED`, `code::http(statut)`. **Ajouter un hébergeur** : une crate `plugins/<id-court>/` qui dépend du kit, un `ForgeKind` (son id, son nom, `client(http, Credentials) -> Box<dyn Forge>`) et, s'il a des tickets, un `SourceKind` (`TicketSource` du kit) ; `run()` donne `Hosts::new(forges, sources)` à `serve::run` ; une ligne dans `ENTRIES` de `bin/escouade-plugins.rs`, une dans `builtin::REGISTRY`, le membre dans le `[workspace]` de `src-tauri/Cargo.toml` ; côté cœur, `Provider::plugin()` (et `TicketSourceKind::plugin()`) rend l'id du plugin, et ses tests gardent leurs scénarios, leur client construit par `builtin::served_forge`.
- **Le jeton, prêté par le cœur** (décidé ici) : le cœur tient déjà les jetons des comptes de première main (le coffre lu avec ses anciens noms, INT-6) et celui de la CLI (`gh auth token`, qu'il sait trouver : `gh_cli`). Il les prête au plugin avant chaque appel (`Runtime::lend`, par plugin et par clé `account:<hôte>`) ; `secrets/get` donne le prêté d'abord, puis le coffre ; jamais celui d'un autre plugin, et masqué dans son journal. Le plugin ne lance donc pas `gh` (pas de `permissions.commands`) et le jeton n'est toujours dans aucun appel.
- **L'environnement d'un plugin livré** : `ESCOUADE_LANG` (la langue du cœur), `ESCOUADE_PROXY`, `ESCOUADE_NO_PROXY`, `ESCOUADE_INSECURE_TLS` (Réglages › Réseau), donnés à son lancement (`Runtime::set_builtin_env`) ; un changement arrête ses process, le suivant les prend.
- **Ce que voit l'utilisateur** : rien de changé à l'usage. Les plugins livrés sont dans Réglages › Plugins sous « Livrés avec Escouade » ; la vue des hébergeurs lit le plugin qui sert chaque genre depuis le registre (`ProviderView::served_by` : coupé, ou arrêté et pourquoi), et la carte d'un agent dit « Hébergeur arrêté : PR plus suivies » (`AgentMeta::pr_down`, posé quand la lecture des PR tombe sur `PluginDown`, levé à la suivante qui marche ; les PR lues en dernier restent).
- **Les tests, inchangés, contre le plugin** : `forge/github_tests.rs` (37 tests ; les scénarios n'ont pas changé, `GithubProvider.client` y est le client servi par le process ; ceux qui lisent le client lui-même, son allure et ses réponses gardées, le prennent dans la crate), `integrations/github_tests.rs` (les 7 de GitHub Issues, sortis de l'ancien module) et l'essai réel `real_github_reads` (github.com avec le jeton de `gh`, en lecture), plus ceux du cœur qui passent par un compte GitHub (`agentprs_tests`, `integrations_tests`) : les cœurs des tests ont le registre (`Plugins::load_for_core`). Les crates du protocole, du kit et de GitHub sont membres d'un `[workspace]` dont `src-tauri` est la racine : nextest et clippy les prennent.

_Fait le 2026-11-01 (ESC-56)_ : **le client GitLab écrit directement en plugin**, `dev.escouade.gitlab` (registre des releases, `GITLAB` ; programme `escouade-plugins gitlab`), sur le modèle de GitHub. Sa crate `escouade-plugin-gitlab` (`src/merges.rs` : MR, merge et auto-merge, jobs du pipeline de tête, journal par `/jobs/:id/trace`, relance du pipeline, approbations et relecteurs, discussions, déploiements par branche ; `lib.rs` : son genre, l'API à `<instance>/api/v4`). Le cœur garde de GitLab ce qu'il en déclare (`forge/gitlab.rs` : hôtes, droits, `glab`, `Provider::plugin`).

_Fait le 2026-11-02 (INT-8.2, ESC-218)_ : **GitLab Issues sort du cœur dans le même plugin** (`src/issues.rs`, un `SourceKind` du kit, id `gitlab`, `contributes.ticketSources` du manifeste), sur les mêmes comptes : un compte d'instance sert les MR et les issues. Le client du plugin est celui d'un compte (une instance) ; le cœur en garde un par instance (`plugins/hosted.rs`, `HostedTicketSource`) et route par l'hôte du conteneur (`<hôte>/<groupes>/<projet>`, inchangé : les liens enregistrés restent), le jeton de chaque compte prêté au plugin sous le nom de l'instance (`Runtime::lend`, celui de l'utilisateur ou de `glab`). `integrations/gitlab.rs` n'existe plus ; `GitlabKind` n'a plus que son nom et `plugin()` (`TicketSourceKind`). Les tests de l'ancien client (`integrations/gitlab_tests.rs`, même faux serveur, gitlab.com et une instance côte à côte) sont rejoués contre le process, plus l'arrêt du plugin dit par la vue des hébergeurs et par ses deux usages (`forge::tests::the_gitlab_plugin_switched_off_…`). L'essai réel (`real_gitlab_issues`, `ESCOUADE_TEST_GITLAB_*`, à la main) passe par le plugin. Tests : `forge/gitlab_tests.rs` contre le process, nourri de réponses enregistrées sur gitlab.com (`tests/fixtures/forges/gitlab/`, `record.mjs` ; discussions et déploiements d'après la doc, gitlab.com les refusant sans jeton). Pas d'essai réel (décision du 2026-10-16) : ce qui reste à essayer est dans `docs/TODO-2.2.md`.

_Fait le 2026-11-01 (INT-8.3, ESC-57)_ : **Bitbucket Cloud**, `dev.escouade.bitbucket`, sur le modèle ci-dessus (`plugins/bitbucket/`, `escouade-plugins bitbucket`, `builtin::BITBUCKET`). Le compte : e-mail et jeton d'API Atlassian (`needsEmail`), sans instance à soi. Le client (`src/pulls.rs`) n'a pas d'auto-merge (la capacité est déclarée absente par `BitbucketProvider::capabilities`), compte lui-même la limite horaire de 1000 appels (Bitbucket ne dit pas ce qui reste ; il croit `X-RateLimit-Limit`, prend `-NearLimit` pour « un cinquième restant », attend un 429) et ne suit une page suivante que sur son adresse. Ses tests : `forge/bitbucket_tests.rs` sur un faux serveur aux réponses écrites d'après la documentation (`tests/fixtures/forges/bitbucket/`) ; rien n'a été essayé sur bitbucket.org (liste des essais à faire dans `docs/TODO-2.2.md`). Le kit n'a changé que d'une ligne : `errors::said` lit aussi le `error.message` de Bitbucket.

_Fait le 2026-11-01 (INT-8.5, ESC-280)_ : **Azure DevOps et Azure Boards**, ensemble, `dev.escouade.azure-devops` (`plugins/azure-devops/`, `escouade-plugins azure-devops`, `builtin::AZURE_DEVOPS`), sur le modèle ci-dessus et sur le compte d'une organisation (celui d'une collection pour un Server) qui sert les deux points. `pulls.rs` est l'ancien `forge/azure_api.rs` et `boards.rs` l'ancien `integrations/azure.rs`, passés tels quels ; `html.rs` porte `html_to_markdown`, qui n'était qu'à Azure Boards. **Le jeton de `az`** (Entra ID, en Bearer) est lu par le cœur (`Provider::cli`, `credentials`) et prêté au plugin comme un jeton ordinaire, sous `account:<hôte>` ; le plugin ne lance aucun programme. Le cœur dit l'adresse du faux serveur (`ESCOUADE_AZURE_DEVOPS_API`) dans `Credentials::api`. **Plusieurs organisations pour Azure Boards** : un compte du cœur est un hôte, et un conteneur est `<organisation>/<projet>` ; `plugins::adapters::ByOrganization` tient un `ProcessTicketSource` par compte et envoie chaque appel à celui que nomme son conteneur (`Core::azure_client`). **Le kit** gagne `TicketSource::states_view` (par défaut rien : le cœur devine les colonnes par les noms) et `hosts::StatesView`, pour qu'Azure Boards donne les états avec les défauts de chaque colonne tirés de leurs catégories ; `tickets/states` répond alors `{ states, defaults }`, ce que `ProcessTicketSource` lisait déjà. Ses tests : `forge/azure_tests.rs`, `integrations/azure_tests.rs` et `azure-devops.spec.ts`, leurs scénarios inchangés ; les essais réels `real_azure_devops` et `real_azure_boards` restent compilés et ignorés, à jouer avec un jeton (liste dans `docs/TODO-2.2.md`).

### 4.5 Le contrat de chaque point d'extension

Les traits Rust d'aujourd'hui deviennent, dans le cœur, des **adaptateurs** qui parlent au process : `ProcessTicketSource: TicketSource`, `ProcessForge: Forge`, `ProcessEngine: Engine` et `ProcessEngineProcess: EngineProcess`. Le reste du cœur ne voit pas la différence. Les types sont ceux de la crate du protocole, partagée par le cœur et les plugins de première main, et générés en TypeScript pour le SDK (PLG-9). Ceux des sources et des hébergeurs sont déjà sérialisables (`Container`, `ExternalIssue`, `IssuePage`, `PullRequest`, `Check`…).

| Point | Côté cœur | Méthodes JSON-RPC | Aujourd'hui |
| --- | --- | --- | --- |
| Compte | `AccountKind` (nouveau, tiré de `forge::Provider` et de `integrations::Account`) | déclaratif (hôtes, sonde, auth, champs, droits, page du jeton) ; `account/me` | `Provider::{default_host, own_host, probe, scopes, token_page, cli, needs_email}`, `Client::me` |
| Source du Kanban | `TicketSource` et `TicketSourceKind` (nouveaux, à la place de l'énumération `integrations::Client`) | `tickets/containers`, `tickets/states`, `tickets/issues` (filtre, page), `tickets/setState`, `tickets/comment` ; capacités déclarées | `Client::{containers, states, issues, set_state, comment}` (`integrations/mod.rs`) |
| Hébergeur | `Provider` et `Forge` (`forge/mod.rs`) | `forge/createPr`, `forge/pullRequest`, `forge/pullRequestsOfBranch`, `forge/pullRequests`, `forge/mergeOptions`, `forge/merge`, `forge/autoMerge`, `forge/checks`, `forge/branchChecks`, `forge/jobLog`, `forge/rerunFailed`, `forge/rerunFailedOfBranch`, `forge/reviews`, `forge/comments`, `forge/comment`, `forge/review`, `forge/reply`, `forge/deployments`, `forge/environments` (ESC-67) ; `unsupported` par défaut ; la limite d'appels dans chaque réponse | tel quel |
| Harness | `Engine` et `EngineProcess` (`engine/mod.rs`) | déclaratif : `locate` (noms, chemins, `--version`), versions testées, `EngineCaps`, modes ; sans état : `engine/models`, `engine/efforts`, `engine/launch` (la description de lancement), `engine/oneShot`, `engine/userConfig`, `engine/fenceSettings` ; par agent : `agent/frames` (reçu), `agent/items` (envoyé), `agent/write`, `agent/sendMessage`, `agent/interrupt`, `agent/setModel`, `agent/setMode`, `agent/answerPermission`, `agent/answerQuestion`, `agent/readUsage`, `agent/attach` (après un redémarrage) ; vue terminal : `terminal/launch`, `terminal/hook` | `Engine::{locate, tested, caps, models, efforts, spawn, message, translate, one_shot, grant_file, escouade_launch, fence_settings, user_config…}`, `EngineProcess::{send_message, interrupt, set_model, set_mode, answer_permission, answer_question, read_usage…}` |
| Lecteur de workflow (ESC-200, choix 158) | `plugins/reader.rs` : `ProcessWorkflowReader`, `with_read` | `workflow/read` (les fichiers du projet que le manifeste déclare, `contributes.workflowReaders[].files`) → `{ workflows: [{ id, name, steps, transitions }] }` | rien : Escouade lit seule (`workflow.rs`) |
| Environnement de lancement | `launchenv::Launch` et `Env` (`launchenv.rs`, INT-14) | déclaratif (`detect`, `program`, `up`) ; `launch/ls`, `launch/down`, `launch/destroy` | `isola::{manages, configured, read_config, services, run}` |

**L’environnement de lancement (INT-14, ESC-286, 2026-11-01)** : le point `contributes.launchEnvironments` (`id`, `name`, `detect` : le fichier qui dit qu’un worktree est le sien ; `program` : la CLI, cherchée sur le PATH par le cœur ; `up` : la commande qu’un lancement de test exécute dans un terminal du projet, après l’approbation de `detect` lu en entier) et `src-tauri/src/launchenv.rs`. Le cœur fait ce qui n’a pas besoin du process : décider qu’un worktree est celui d’un environnement (`Launch::for_dir` : plugin livré avec l’app, actif pour l’app et le projet, fichier présent, programme installé), lire `detect`, lancer `up`. Le plugin, un process, fait le reste par `launch/ls` (les services et leurs adresses), `launch/down` et `launch/destroy` ; le cœur lui donne le programme trouvé, le dossier, la branche et la langue de ses messages. isola est le premier (`plugins/isola/`, entrée `isola` d’`escouade-plugins`, `src-tauri/src/bin/escouade-plugins/isola.rs`) ; `isola.rs` ne garde que la recherche de la CLI et le type `Service`. Sans le plugin (coupé pour l’app ou le projet), sans `isola` ou sans `.isola.toml`, un ticket part comme avant, sans isola. **Seuls les plugins livrés avec l’app servent ce point** : `up` est une commande du terminal du cœur, qu’on ne confie pas à un plugin d’ailleurs (Docker Compose, devcontainers, Tilt viendront avec leur propre contrat de confiance). `fake-isola` et les tests du cœur qui l’emploient passent tels quels contre le process du plugin.

**Les adaptateurs des sources et des hébergeurs (INT-5, ESC-211, 2026-10-31)** : `src-tauri/src/plugins/adapters.rs`. `ProcessTicketSource` (`account/me`, `tickets/containers`, `states`, `issues`, `setState`, `comment`) et `ProcessForge` (`account/me`, `forge/*` ; chaque réponse est `{ value, rateLimit? }` et le dernier budget se lit par `rate_limit()`) ; `ProcessTicketSourceKind` en fait une sorte du registre (le registre lira les manifestes avec INT-6). Le jeton n'est jamais dans un appel : le plugin le prend lui-même par `secrets/get` (`account:<id>`). **Erreurs typées** : le plugin dit une erreur HTTP par le code `-(32000 + statut)` (`-32401` : « reconnecte le compte », `forge::refused`), un refus qui ne changera pas par `-32010` (`Refused`), un appel que son hébergeur ne sait pas faire par `-32601` (`Unsupported`). **Plantage, arrêt, délai** : `PluginDown`, qui n'est pas un refus durable (`lasting` est faux) : la synchro garde l'opération et la retente avec ses attentes, et les PR lues en dernier restent (`agentprs` garde ce qu'il avait lu quand l'appel échoue). Délai : 60 s pour `tickets/*`, `forge/*` et `account/me`. Le faux plugin (`tests/fixtures/fake-plugin.mjs`) sert une source et un hébergeur ; `plugins_adapters_tests.rs` joue contre lui les scénarios des tests des intégrations d'aujourd'hui (liste, états, tickets, état poussé, commentaire, PR, CI, 401, 404, refus, plantage).

**Trello en plugin, en process (INT-7.1, ESC-213, 2026-11-01)** : la crate `plugins/trello/` (`escouade-plugin-trello`, son manifeste, son README), appelée par `escouade-plugins trello` et inscrite au registre (`builtin::TRELLO`). Elle pose sa source sur le kit `escouade-plugin-kit` comme GitHub y pose ses Issues : un `SourceKind` (`TrelloSource`) qui construit un `TicketSource` par compte, le kit répondant au protocole, demandant le secret (`secrets/get` de `account:trello`, ici `{"key":…,"token":…}`) et gardant un client par compte et par secret. **Pour ajouter une source** (Linear, Jira, ClickUp) : une crate `plugins/<id-court>/` (membre du workspace de `src-tauri/Cargo.toml`, dépendance de l'app, une ligne dans `ENTRIES`) avec un `SourceKind` et son client ; son manifeste (`accounts`, `ticketSources`, `permissions.network` et `secrets`, `fields[].missing` pour les textes du formulaire) et son README ; une ligne dans `builtin::REGISTRY` ; dans le cœur, `plugin()` à sa `TicketSourceKind` (qui ne garde que son nom : `Unserved`) et, dans `Core::client_for`, le secret à prêter et l'adresse de l'API (`Bases`) ; ses tests de `integrations/<service>.rs` deviennent `plugins_<service>_tests.rs`, joués contre le plugin lancé comme l'app le lance. **Le secret est prêté** (`Runtime::lend`) par le cœur à chaque appel : la clé et le jeton que le cœur tient, même quand le trousseau ne les rend pas. **Les textes de `sync.rs`** de la source (« Indique le jeton. », « Indique la clé d'API. ») sont dans le manifeste (`fields[].missing`, `builtin::missing`). **Un arrêt se dit** : un plugin arrêté ou coupé pour l'app rend `PluginDown` (« Le plugin dev.escouade.trello ne répond pas : plugin stopped / … is switched off »), la synchronisation attend et recommence. Le kit apprend le compte (`Credentials::user`, l'identifiant de « Mes cartes »). L'onglet Intégrations lit toujours le registre (`InitialState::sources`) : Trello y a son nom, la vue Plugins le montre sous « Livrés avec Escouade », et sa page du store pointe vers son dossier. Tests : les quatre de l'ancien client, mot pour mot, dans `plugins_trello_tests.rs` (le plugin lancé par le runtime sur le faux Trello de `integrations/fake.rs`) plus la langue, le secret jamais envoyé, l'arrêt et la coupure, les textes du manifeste ; les tests du cœur (`integrations_tests.rs`, `fake-trackers.mjs`, l'e2e `integrations.spec.ts`) passent inchangés à travers le plugin.

**Linear en plugin, en process (INT-7.2, ESC-214, 2026-11-01)** : la crate `plugins/linear/` (`escouade-plugin-linear`, son manifeste, son README), appelée par `escouade-plugins linear` et inscrite au registre (`builtin::LINEAR`), faite comme Trello avec la recette ci-dessus et sans toucher au kit. Son secret est la clé personnelle seule (`Core::client_for` la prête, l'adresse de l'API est `Bases::linear`, `ESCOUADE_LINEAR_API`) ; les erreurs GraphQL (`AUTHENTICATION_ERROR`, `FORBIDDEN`, `RATELIMITED`, une erreur de requête) se lisent comme les erreurs HTTP qu'elles valent (`HttpError`), un refus de mutation est un `Refused`. Le texte « Indique le jeton. » est dans son manifeste. Tests : les six de l'ancien client, mot pour mot, dans `plugins_linear_tests.rs` (le plugin lancé par le runtime sur le faux Linear), plus la langue, le secret jamais envoyé, l'arrêt et la coupure, le registre et le texte du manifeste ; `integrations_tests.rs` et les e2e passent inchangés à travers le plugin.

**Jira en plugin, en process (INT-7.3, ESC-215, 2026-11-02)** : la crate `plugins/jira/` (`escouade-plugin-jira`, son manifeste, son README), appelée par `escouade-plugins jira` et inscrite au registre (`builtin::JIRA`), faite comme Trello et Linear avec la recette ci-dessus. Le compte est le site, l'e-mail et le jeton : le site et l'e-mail ne sont pas secrets et vont dans l'appel (`account.site`, `account.email`), le jeton est le seul secret (`account:jira`, prêté par `Core::client_for`, l'API étant celle du site : pas de `Bases`). Les documents d'Atlassian (ADF lu en markdown, commentaire écrit en document) sont dans le plugin (`src/adf.rs`) et plus dans `integrations/text.rs`. Les textes du formulaire (« Indique l'adresse du site. », « Indique l'e-mail. », « Indique le jeton. ») sont dans le manifeste, dans l'ordre du formulaire (`fields[].missing`). Le cœur garde le nom de la source, l'adresse d'un site et la règle « https, sauf cette machine » (`integrations/jira.rs`, que les formulaires des comptes appliquent). **Une seule chose change dans le kit, sans rien changer pour les autres** : `SourceKind::account_name(url)`, le nom du secret de l'instance — par défaut son hôte (GitLab Issues), ici toujours `jira`, un site tapé par l'utilisateur ne faisant pas un compte de plus. Tests : ceux de l'ancien client, mot pour mot, dans `plugins_jira_tests.rs` (le plugin lancé par le runtime sur le faux Jira), plus la langue, le jeton jamais envoyé, l'arrêt et la coupure, le registre et les textes du manifeste ; `integrations_tests.rs` et les e2e passent à travers le plugin (un seul test change : celui du certificat auto-signé redémarre le plugin avec le réseau d'après, comme le fait l'enregistrement des réglages).

**ClickUp en plugin, en process (INT-7.4, ESC-216, 2026-11-02)** : la crate `plugins/clickup` (`escouade-plugin-clickup`, son manifeste, son README), appelée par `escouade-plugins clickup` et inscrite au registre (`builtin::CLICKUP`), faite comme Trello, Linear et Jira avec la recette ci-dessus, sans autre changement du kit que la clé de son cache de clients de source, qui compte désormais l'utilisateur du compte (le client fait à « Connecter », sans utilisateur, ne sert plus « Mes tâches » ensuite). Son secret est le jeton personnel seul (`account:clickup`, prêté par `Core::client_for` ; l'adresse de l'API est `Bases::clickup`, `ESCOUADE_CLICKUP_API`, dite au plugin par `account.api` ; l'id de l'utilisateur, `account.user`, filtre « Mes tâches »). **ClickUp garde sa vue propre des états** (`TicketSource::states_view`, comme Azure Boards) : les statuts de la liste dans leur ordre et les défauts des colonnes, d'après les noms puis d'après les types (`custom`, `done`) ; la règle des noms (`DOING`, `TO_TEST`) est recopiée dans le plugin, comme celle d'Azure. L'attente de la limite (`X-RateLimit-Reset`, un 429 attendu une fois si le reset est à moins de 60 s) est dans le plugin, qui a son propre `send`. Le texte « Indique le jeton. » est dans son manifeste. Tests : les tests d'appel de l'ancien client, mot pour mot, dans `plugins_clickup_tests.rs` (le plugin lancé par le runtime sur le faux ClickUp), plus la langue, le jeton jamais envoyé, l'arrêt et la coupure, le registre et le texte du manifeste ; les tests purs (ids d'une tâche, défauts par type, attente) dans la crate du plugin ; `integrations_tests.rs` et l'e2e `integrations` passent à travers le plugin, inchangés.

**Le lecteur de workflow (ESC-200, LOOP-8, 2026-11-01)** : `src-tauri/src/plugins/reader.rs`. Un plugin déclare `contributes.workflowReaders[]` (un `id`, un `name`, et `files`, des globs depuis le dossier du projet : sans `..`, sans chemin absolu) ; Escouade lui donne par `workflow/read` ceux de ces fichiers qui sont du texte (au plus 200, 256 Ko chacun, ceux du dossier de chaque dépôt pour un projet de plusieurs) et il rend `{ workflows: [{ id, name, steps: [{ name, command, description?, path?, agents?, reads?, writes? }], transitions: [{ from, to, back?, condition?, max? }], exit?, harness?, repo? }] }`. Escouade en fait un `Workflow` d'id `plugin:<plugin>:<id>` (il ne prend jamais la place d'un workflow lu par l'app), que la suite suit comme les autres : le suivi des agents, l'onglet, les faits `workflow.*`. Un graphe qui ne tient pas (sans nom, sans étape, deux étapes du même nom, une transition vers une étape absente, plus de 50 workflows) est refusé en entier, avec la raison ; un plugin qui plante est un `PluginDown` (ce qui était lu reste). Délai : 60 s. Comme les adaptateurs ci-dessus, rien dans l'app n'appelle encore un lecteur : le registre lira les manifestes avec INT-6. **Les faits `workflow.*`** (même ticket) : `workflow.step_entered`, `workflow.step_left` (durée, coût, issue), `workflow.looped_back` (sa raison), `workflow.left`, `workflow.changed` (les ids des workflows relus qui diffèrent), émis par `AgentTrack::facts_since` d'après les passages du suivi et par `Core::workflow_read` ; ils portent l'agent, le ticket et le projet comme les autres faits.

**Le contrat des harness (INT-9, ESC-221, 2026-11-01)** : les types sont dans `crates/escouade-plugin-protocol/src/engine.rs` (`escouade_plugin_protocol::engine`), l'adaptateur dans `src-tauri/src/plugins/harness.rs`, l'application des éléments à l'agent dans `src-tauri/src/agent/items.rs`.
- **Le lancement reste au cœur.** `engine/launch` reçoit l'agent (dossier, modèle, effort, mode, consignes, d'où il repart, dossiers lus, serveur d'Escouade avec son jeton, chemins clôturés tels que son programme les écrit, lecture seule, dossier de l'agent) et rend une description sans programme : arguments, variables, variables à retirer, fichiers à écrire, premières lignes, `fenceApplied`, `readOnly`. Le cœur la vérifie (`check_launch`) : le jeton jamais dans les arguments ; pour un projet privé, `fenceApplied` et chaque chemin refusé présent dans les arguments, les variables ou les fichiers ; `readOnly` tenu quand il est demandé (une discussion de l'onglet Docs) ; les fichiers seulement dans le dossier de l'agent (la variable de son compte), sans `..`. Puis il lance le programme qu'il a trouvé lui-même (`locate` déclaré ; `{escouade}/…` vérifié par `builtin::check_program`) dans son Job object, par `Program::launch` (WSL compris). Un refus empêche le lancement et dit pourquoi.
- **Les frames, par lots et en chaînes brutes.** Le cœur lit les lignes du harness et les passe au plugin dans `agent/frames` (`{ agent, frames: ["<ligne>", …] }`), sans les relire. Un lot est ce qui a été lu pendant que le précédent était chez le plugin, jusqu'à 64 Kio, **sans attente** : une ligne seule part tout de suite (l'attente de 8 ms prévue au §4.6 ajoutait jusqu'à 8 ms par ligne au rythme réel, au-delà du seuil de 5 ms). Le plugin rend des **éléments** (`start`, `text`, `thinking`, `tool`, `toolDone`, `touched`, `permission`, `question`, `turn` avec ses jetons et la limite atteinte, `notice`, `write`, et depuis Copilot CLI `permissionCard`, `questionCard`, `quota`, `limitRest`, `ready`, `usage`, `compacted`, `stop`) ; un `op` inconnu est laissé de côté. Le cœur écrit les `write` sur l'entrée du harness, dans leur ordre, et applique les autres à `AgentRt` (`apply_items`, les mêmes règles que les traductions du cœur : la session, le texte du tour, les lignes du plan, la demande en attente, le tour fermé avec ce qui restait ouvert). `agent/sendMessage`, `agent/interrupt`, `agent/setModel`, `agent/setEffort`, `agent/setMode`, `agent/answerPermission` et `agent/answerQuestion` rendent des `writes`, joués dans l'ordre des appels ; `agent/readUsage` rend le quota lu.
- **`agent/attach` et la file pendant une relance.** Le cœur donne l'agent avant son lancement, et de nouveau après chaque relance (`Runtime::attach`), avec ce qu'il sait : la session, le modèle, l'effort, le mode, les demandes en attente. Un lot resté sans réponse parce que le plugin a planté est redonné une fois le plugin relancé ; les lignes lues entre-temps attendent dans une file bornée (20 000 lignes). La conversation dit « Le plugin de Minimal a redémarré. ». Après le dernier plantage permis, le harness de l'agent est arrêté, la conversation et la fin de son process le disent (« … s'est arrêté après 3 plantages : l'agent s'arrête, sa session est gardée… »), et la session reste pour le message suivant une fois le plugin relancé.
- **`engine/oneShot`** décrit la question (arguments, variables, entrée) ; le cœur la lance avec sa limite, et `engine/oneShotRead` lit la réponse dans ce que le process a écrit.
- **La vue terminal** garde son PTY au cœur : `terminal/launch` décrit le lancement de l'interface (vérifié comme celui d'un agent, plus les bandes de confiance à montrer), et `terminal/hook` va dans les deux sens : le cœur passe le hook et ce qu'il veut y répondre (`reply` : un contexte pour le modèle, ou un refus ; l'index du code, qui devait y répondre, est retiré depuis ESC-310), le plugin rend les éléments de l'agent et la réponse dans la forme de son harness. Le branchement à `TerminalView` reste à faire : Cursor CLI, premier harness sorti (INT-10.1), garde sa vue au cœur, avec le suivi de son transcript que le contrat ne porte pas encore ; d'ici là, `caps.terminalView` d'un `ProcessEngine` reste faux.
- **Ce qui est déclaré** (`contributes.harnesses[]`, `HarnessDecl`) se lit sans lancer le plugin : le nom, le programme (`locate`), les versions testées, les capacités (modes, effort, images, `readOnly`, `accountsByKey`), les modèles, la variable du dossier des comptes, et **la reprise de la mémoire** (`memory.dir`, avec `{configDir}` et `{projectKey}` : ce que `Engine::memory_dir` rend pour `memory::claude_offer`). `plugins::harness::register` met au registre (`registry::register`) les harness d'un manifeste ; rien ne l'appelle encore au démarrage : chaque harness y viendra avec sa sortie (INT-10).
- **La preuve, sur le faux moteur minimal devenu un plugin.** `dev.escouade.minimal` déclare le harness « Minimal » ; son harness est `escouade-plugins minimal-harness` (le script du faux en process d'`engine/minimal/`, devenu un programme), et la traduction d'`agent/minimal.rs` est passée dans le plugin (`src-tauri/src/bin/escouade-plugins/minimal.rs`), qui garde seul l'état de traduction (les appels en cours). `plugins_harness_tests.rs` : le banc des harness joué par `ProcessEngine` (texte, raisonnement, approbation, fichier, question, interruption, sous-agent, reprise : les scénarios du faux en process), un plantage en plein tour (le harness continue, rien n'est donné deux fois, l'outil en cours est fermé à la fin du tour), le plugin arrêté après ses plantages, un lancement clôturé et en lecture seule avec le serveur d'Escouade, les quatre refus de `check_launch`, le one-shot, la vue terminal et ses hooks, ce qui est déclaré, l'enregistrement. Le faux en process (`engine/minimal/`) reste celui des tests de la passation et du banc écrit dans le cœur.

**Les harness ACP décrits (PLG-6, ESC-178, 2026-11-01)** : le client ACP est la crate `crates/escouade-acp` (tirée de celui de Cursor CLI ; ce qui est propre à Cursor, `cursor/*`, dans son module `cursor`), servie par le plugin livré `dev.escouade.acp` (`escouade-plugins acp`) pour chaque entrée `contributes.harnesses[]` qui a un objet `acp` (`args`, `env`, `removeEnv`, `fs`, `modes`, `readOnlyMode`, `modelOption`, `questionsInText`, `tools`, `subagentTools`, `extensions`), la description donnée avec chaque agent (`described`). Le harness entre au registre avec son plugin et en sort avec lui (`harness::sync`, « harness absent »), sous l'id `plugin:<plugin>:<harness>` (nu pour `dev.escouade.*`). La clôture tient sur les `fs/*` du client (`fenceByClient`) et est dite manquante sinon ; `"fs": false` ferme les projets privés. Le contrat gagne `delta`, `toolInput`, `todos`, `mode`, `subagent`, le `result` de `toolDone`, une permission complète (outil, description, raison, diff, « Pour cet agent », sous-agent) et le tour en erreur. Premier plugin : Goose (`plugins/goose/`), essayé pour de vrai avec Goose 1.57.0 sur Ollama (`real_goose`). Tests : `plugins_acp_tests.rs`, e2e `acp-harness.spec.ts`.

**Cursor CLI en plugin, en process (INT-10.1, ESC-222, 2026-11-02)** : le plugin de première main `dev.escouade.cursor-cli` (`plugins/cursor/`, crate `escouade-plugin-cursor`, entrée `escouade-plugins cursor`, `builtin::CURSOR`, sa page du store pointant vers son dossier) déclare le harness `cursor` avec son objet `acp` (extension `cursor`) et le sert par le service de `dev.escouade.acp` (`serve_as`, sa description à lui) et le client de `crates/escouade-acp`. Le moteur `Cursor` reste au registre du cœur (`BUILT_IN`) : ses capacités et le sens de ses modes, les comptes (connexion de la machine, comptes par clé), `list_models`, les textes rédigés (`one_shot`), la vue terminal et le suivi de son transcript, `/summarize` ; sa conversation passe par le `ProcessEngine` du plugin (`engine::cursor::plug`, posé par `harness::sync_cursor` quand le plugin est prêt et allumé, retiré sinon : l'agent dit alors au lancement « Le plugin Cursor CLI (dev.escouade.cursor-cli) est coupé ou n'est pas chargé… »). `ProcessEngine::with_native` lance le lanceur de Cursor en son `node` et son `index.js`. Le contrat gagne ce qu'il fallait pour que rien ne change : `relay` (ce qui a été dit dans sa vue, avant le message suivant) et `byKey` (le compte passe par sa clé, `keyVar` déclaré : le texte d'une clé refusée) dans `engine/launch` ; `content` dans `agent/sendMessage` (les images et fichiers texte, tels que `Cursor::message` les a faits) ; `closeGraceMs` (Cursor ne sort pas quand son entrée se ferme : tué 2 s après) ; `fenceOnPermissions` (avec `"fs": false`, la clôture tenue sur les seules permissions, comme avant, crue d'un plugin livré seulement) ; l'effort par l'option `thought_level` de chaque modèle (`cursor/list_available_models`, demandé à côté de la session) ; des ids d'éléments qui portent la fin de la clé de leur process (une session reprise ne réécrit plus la conversation d'avant, trouvé avec le vrai Cursor) ; le premier appel d'un lancement (`agent/attach`), qui peut démarrer le process du plugin, attend 30 s et non les 5 s d'un appel (`ATTACH_WITHIN` : sous la charge, l'e2e de Cursor les dépassait au redémarrage de l'app). Tests : ceux du cœur inchangés contre le plugin (`cursor_tests.rs`, `cursor_tui_tests.rs`, le banc `engine/cursor/bench.rs` dans un runtime des plugins livrés), les flux enregistrés rejoués par son client contre `replays/cursor/` (`plugins_acp_tests.rs`, qui remplace le rejeu brut par le cœur), l'arrêt dit (`a_cursor_agent_says_its_plugin_is_off_and_talks_again_once_it_is_on`), l'effort, les ids. `agent/cursor.rs` reste, pour Gemini CLI et Antigravity CLI (Mistral Vibe l'a quitté avec INT-10.6, plus bas).

**Copilot CLI en plugin, en process (INT-10.3, ESC-224, 2026-11-04)** : le plugin de première main `dev.escouade.copilot-cli` (`plugins/copilot/`, crate `escouade-plugin-copilot`, entrée `escouade-plugins copilot`, `builtin::COPILOT`, sa page du store pointant vers son dossier) est le premier harness sorti qui n'est pas ACP : son client JSON-RPC (`src/client.rs`, une machine à états sans process, chaque réponse attendue une valeur de `Ours`) et sa traduction (`src/translate.rs`, le contexte de l'agent reçu dans `Ctx`) ont quitté le cœur. Le moteur `Copilot` reste au registre (`BUILT_IN`) : capacités, comptes, quota et modèles d'un compte (par un serveur à lui, `terminal::read_quota`, `models_list`), `copilot -p`, le fichier de l'entrée d'Escouade (`grant_file`), la vue terminal ; `spawn` passe par `spawn_plugged`, et sa ligne de `BUILTIN_HARNESSES` n'a ni `native` ni `prepare`. **La vue Copilot lit le journal de sa session par la même traduction, en bibliothèque** (`agent/copilot.rs`) : deux traductions auraient divergé, et un appel au process à chaque ligne du journal n'apporterait rien, ce journal restant au cœur. Le contrat gagne, pour tous : `framing: "contentLength"` (le cœur lit et écrit les messages du harness en `Content-Length`), `closeLine` (une ligne écrite juste avant de fermer l'entrée : `runtime.shutdown`, qui sauve la session), `opens` (le plugin ouvre la session par le protocole : l'agent est prêt à l'élément `ready`, qui porte les modèles du compte, et `stop` est son refus : autre protocole, pas de connexion) ; `writes` dans les réponses d'`agent/rename` et d'`agent/compact` qu'OpenCode a ajoutés (INT-10.2), pour un harness qu'on commande par son entrée, l'élément `compacted` disant alors la fin ; `UsageAsked` (la réponse d'`agent/readUsage` quand le quota se demande au harness, l'élément `usage` rapportant sa réponse) ; `escouadeFile` dans `engine/launch` (le fichier privé du serveur d'Escouade, nommé dans `--additional-mcp-config`) ; les éléments `permissionCard` et `questionCard` (la carte écrite entière : la portée de « Toujours », un formulaire), `quota`, `limitRest` (le tour qui finit attend une limite : l'agent au repos, pas en erreur, ce que Cursor et Vibe ne disent pas). De nouveaux éléments plutôt que des champs ajoutés aux anciens : les plugins écrits avant restent tels quels. Fusionné avec INT-10.2, Copilot prend ce qu'OpenCode a mis au contrat le même jour : `unplanned` de `tool` (la table `sql`), `answered`, `context`, `model`, `entry` de `turn`, `hunks_of`, une seule alerte de fin de tour et de question. `apply_items` gagne aussi les fichiers du plan relus après un appel. `wait_gone` attend aussi le Job vide sous Windows. Les règles de git d'`engine/codex/git.rs` passent dans `crates/escouade-acp/src/git.rs` : Copilot garde `git add` et `git commit` acceptés sans demander, contrairement à Cursor et Vibe depuis leur sortie, et son plugin lance donc `git` pour savoir où sont le dépôt et ses hooks (son README le dit). Tests : ceux du cœur inchangés contre le plugin (`copilot_tests.rs`, `copilot_tui_tests.rs`, `agent/copilot_tests.rs` dont les flux réels passent par sa traduction, le banc `engine/copilot/bench.rs` par son process), plus `src/tests.rs` (le client seul) et `a_copilot_agent_says_its_plugin_is_off_and_talks_again_once_it_is_on`. Essais réels contre le plugin, le 2026-11-04, sur Copilot CLI 1.0.98 et le compte déjà connecté, sans jeton donné : `real_copilot` (conversation, commande acceptée, question, plan renvoyé ; dans un `COPILOT_HOME` d'essai, Copilot prend le compte du GitHub CLI et la conversation le dit) ; `real_copilot_through_its_plugin` (ajouté : réponse, commande acceptée, arrêt puis reprise de la même session) ; `real_fence` (le `Read` d'un fichier d'un projet privé refusé par le plugin, son contenu absent de la conversation), avec `ESCOUADE_REAL_ASK`, une demande neutre, car les modèles du compte Free déclinent la demande du test sans essayer. **Restent à lancer à la main** : `real_copilot_view` et `real_copilot_view_on_a_free_account`, qui demandent une connexion du dossier d'essai ou un `GH_TOKEN`, et `real-harnesses.spec.ts`, qui réécrit `~/.copilot/config.json`. Ces essais touchent à la connexion, que ce ticket ne devait pas toucher ; la traduction de la vue est couverte par `copilot_tui_tests.rs` et ses flux réels enregistrés.

**Mistral Vibe en plugin, en process (INT-10.6, ESC-281, 2026-11-02)** : Mistral Vibe, d'abord plugin à part puis réuni à la voix dans `dev.escouade.mistral` par INT-10.6 bis (§4.13) (`plugins/mistral/`, crate `escouade-plugin-mistral`, entrée `escouade-plugins mistral`, `builtin::MISTRAL`, sa page du store pointant vers son dossier), déclare le harness `vibe` avec son objet `acp` (extension `vibe`, `"fs": false`, `fenceOnPermissions`, `keyVar` `MISTRAL_API_KEY`, `accountVar` `VIBE_HOME`, `closeGraceMs` 2000, la télémétrie coupée dans `env`) et le sert comme Cursor CLI, par `serve_as` et le client de `crates/escouade-acp`, dont `vibe.rs` tient ce que l'ancien client du cœur (`engine/vibe/process.rs`) et sa traduction (`agent/vibe.rs`) faisaient : ses outils sous les noms de Claude Code (`agents.spawn` en `Task`, `mcp_<serveur>.<outil>`, `vibe.todo`, `ui.ask_user_question` et `search_tool_functions` tus), ses fichiers nommés depuis le dossier de l'agent, sa permission nommée par son seul appel, ses formulaires (`elicitation/create`, demandés par `elicitation.form`), le mode Plan joué par Escouade (`PLAN_MODE` à chaque message, écritures refusées, derniers mots en carte « Plan à approuver », approuvé puis mis en œuvre dans le mode choisi), un refus qui coupe le tour (`session/cancel`, le mot de l'utilisateur envoyé ensuite, jamais celui d'Escouade), « Pour cet agent » en `allow_always_permanent`, `session/fork` pour une copie, la réflexion (`thinking`) pour l'effort, un modèle que sa session n'offre pas laissé à son défaut, `auto-approve` gardé en `ask` sous une clôture, le coût d'un tour (ce dont `usage_update` a grandi) et sa limite de débit (« limité », reprise une minute après). Le moteur `Vibe` reste au registre du cœur : comptes par clé, `list_models` (son `vibe-acp` jetable), `vibe -p` pour les textes, la vue terminal par son journal et ses hooks ; `read_only` refusé (aucune discussion ne part sur lui : son manifeste ne dit pas de `readOnlyMode`).

**OpenCode en plugin, en process (INT-10.2, ESC-223, 2026-11-04)** : le plugin de première main `dev.escouade.opencode` (`plugins/opencode/`, crate `escouade-plugin-opencode`, entrée `escouade-plugins opencode`, `builtin::OPENCODE`, sa page du store pointant vers son dossier) est le premier harness qui n'est **pas un flux de lignes** : un `opencode serve` sert tous les agents d'un compte (0,7 Go le serveur), en HTTP et SSE. Sa ligne dans `BUILTIN_HARNESSES` ne dit rien de plus ; c'est le contrat qui a grandi, pour tous :
- **Un process partagé (`Launch::shared`, `plugins/shared.rs`).** Le plugin décrit le lancement comme un autre (`engine/launch`, vérifié par `check_launch`) et y ajoute `shared` : ce que dit la ligne de « prêt » (`ready`), un agent par dossier (`oneAgentAFolder`), et ce qu'il veut garder (`keep`, le mot de passe). Le cœur lance le process dans son Job object, sous WSL au besoin, dans un dossier à lui (aucun des agents : un worktree retiré le trouverait tenu), et une seule fois pour les agents du même plugin, du même programme et des mêmes variables ; il l'arrête avec son dernier agent, et ne lit que sa ligne de « prêt ». **Ce choix tient le §4.2 : le cœur lance, et le harness survit au plantage de son plugin**, là où un serveur lancé par le plugin mourrait avec lui, hors du Job et de la mesure des ressources.
- **Ce que le plugin lit seul : `agent/items`.** Le cœur ne lit pas le flux d'événements du serveur : le client HTTP et SSE est dans la crate. Le plugin envoie donc les éléments de lui-même, par la notification `agent/items` (`{ agent, items }`, à côté des réponses, dans leur ordre) ; `agent/frames` reste le chemin des flux enregistrés (le banc). Le programme d'un tel plugin ne peut pas répondre en série : `serve_later` (`escouade-plugin-protocol`) lui donne une boucle dont les réponses partent d'où il veut, et `Output::notify`.
- **Le cycle de vie d'une session.** `agent/open` (`{ agent, start, escouade }`) ouvre, reprend ou copie la session sur le process donné par `agent/attach` (`shared` : sa clé, sa ligne, `keep` ; avec le modèle, le mode, la clôture, `readOnly`, les consignes et `running`) et rend `attach` (`opencode attach`, le mot de passe en variable) et `catalog` (les modèles, comme les `Ready` du cœur) ; `initialize` d'un tel agent attend cette réponse. `agent/close` arrête l'agent, doucement (l'élément `closed` dit quand son tour est fini) ou tout de suite (son tour coupé) ; `agent/letGo` lâche le dossier dont il était le dernier agent, et `wait_gone` l'attend, avant qu'un worktree soit retiré.
- **Ce qui manquait pour tous** : `agent/compact`, `agent/rename` (sans effet pour un plugin qui ne le sert pas), `agent/control`, `cancelQueued` d'`agent/interrupt`, `questions` d'`agent/answerQuestion` ; les éléments `user` (un message tapé dans sa vue), `answered` (une demande répondue dans sa vue, sa carte le dit), `model`, `context` (ce que tient le contexte, la fenêtre du modèle), `closed` ; `entry` de `turn` (où une copie la fourche), `unplanned` de `tool`, `hunks`, `alwaysByHarness` et `endsStream` de `permission`, `parent` de `question`. `apply_items` applique ce que les traductions du cœur faisaient seules : **l'alerte de fin de tour et celle d'une question** (les agents de Cursor CLI et de Mistral Vibe n'en avaient plus depuis INT-10.1 : corrigé pour eux aussi), les messages en file marqués partis, l'activité d'un outil, le dernier texte d'un tour, un élément redit qui n'ajoute rien.
- **Un plantage et un arrêt se disent sans appel en cours.** Aucun lot n'est chez le plugin pour le révéler : chaque agent regarde l'état de son plugin (250 ms). Relancé, il dit « Le plugin de OpenCode a redémarré. » et se redonne (`agent/attach`, le serveur retrouvé par `shared`, sa session réécoutée, les variantes relues) ; un tour fini pendant ce temps est fermé d'après `GET /session/status`. Arrêté après ses plantages ou coupé, l'agent s'arrête en le disant, sa session gardée, et le serveur reste à ses autres agents jusqu'au dernier.

Le moteur `Opencode` reste au registre du cœur : ses capacités et le sens de ses modes, les comptes, `opencode models` (`listed`, dans la crate), les textes rédigés (`opencode run`, `run.rs`), la déclaration MCP de l'utilisateur. La règle des modes, la clôture et la lecture seule sont des règles de la session, tenues par le plugin (`fenceByClient`). Tests : ceux du cœur contre le plugin (`opencode_tests.rs`, `fence_tests.rs`, `agent/opencode_tests.rs` par sa traduction, le banc `engine/opencode/bench.rs` sur le faux serveur, les flux de `replays/opencode/`), ceux de la crate (les règles, le catalogue, l'adresse et les blocs SSE, ôtés d'`engine/opencode/`), et l'arrêt : `an_opencode_agent_says_its_plugin_is_off_and_talks_again_once_it_is_on`, `a_crash_of_the_opencode_plugin_leaves_its_server_and_the_agents_turn_goes_on`, `an_opencode_agent_stops_with_its_plugin_and_says_why_its_session_kept`.

Le branchement des harness livrés n'est plus propre à Cursor : `plugins::harness::BUILTIN_HARNESSES` est une table (l'id du plugin, son `native` s'il en a un, son `prepare` s'il en a un), que `sync` met d'accord avec les interrupteurs des plugins et dont `spawn_plugged` dit l'arrêt (« Le plugin Mistral (dev.escouade.mistral) est coupé ou n'est pas chargé… ») ; Gemini CLI n'y ajoutera qu'une ligne. `ProcessEngine::with_prepare` écrit avant chaque lancement ce que le cœur garde dans le dossier de l'agent : pour Vibe, la confiance de son dossier et le `denylist` d'un projet privé dans le `config.toml` de son `VIBE_HOME` (à côté des règles qu'y écrit « Pour cet agent »), comme avant. Le contrat gagne l'élément `session` (la session ouverte avant tout tour, sans passer l'agent « en cours » : une copie a la sienne dès son lancement) ; `apply_items` compte le coût d'un tour dans l'agent et termine une reprise prévue, comme les traductions du cœur. Une différence : en « Accepter les modifications », un `git add`/`git commit` du worktree que le cœur acceptait sans demander (la règle de Codex) demande désormais, comme pour Cursor CLI depuis INT-10.1 : la règle lit le dépôt, ce qu'un plugin ne fait pas. Tests : ceux du cœur contre le plugin (`vibe_tests.rs`, `vibe_tui_tests.rs`, `agent/vibe_tests.rs` dont les essais sur le faux Vibe branchent les plugins livrés et attendent la session, le banc `engine/vibe/bench.rs`), les six flux enregistrés de Vibe 3.0.3 rejoués par son client contre `replays/vibe/` (`plugins_acp_tests.rs`, un rejeu commun avec Cursor ; dans un rejeu, les règles d'Escouade n'accordent rien à la place de l'essai, comme l'ancien cœur sans process), l'arrêt dit (`a_vibe_agent_says_its_plugin_is_off_and_talks_again_once_it_is_on`).

**Gemini CLI en plugin, en process (INT-10.7, ESC-282, 2026-11-03)** : le plugin de première main `dev.escouade.gemini-cli` (`plugins/gemini/`, crate `escouade-plugin-gemini`, entrée `escouade-plugins gemini`, `builtin::GEMINI`, sa page du store pointant vers son dossier) déclare le harness `gemini` avec son objet `acp` (`--acp`, `GEMINI_CLI_TRUST_WORKSPACE` dans `env`, extension `gemini`, `"fs": false`, `fenceOnPermissions`, `questionsInText`, `readOnlyMode` `default`, `accountVar` `GEMINI_CLI_HOME`, `closeGraceMs` 2000) et le sert comme Cursor CLI et Mistral Vibe, par `serve_as` et le client de `crates/escouade-acp`, dont `gemini.rs` tient ce que l'ancien client du cœur (`engine/gemini/process.rs`, retiré) et sa traduction (`agent/gemini.rs`) faisaient : les mises à jour lues pendant un prompt seulement (`session/load` rejoue l'historique après sa réponse), ses appels sans `rawInput` (l'outil lu dans leur id, une commande dans leur titre, un outil MCP et un sous-agent dans leur titre), ses fichiers nommés depuis le dossier de l'agent, son changement de mode dit par un texte (« [MODE_UPDATE] … »), la permission d'`exit_plan_mode` en carte « Plan à approuver » (le plan lu dans le fichier qu'elle nomme, le mode choisi posé une fois Gemini sorti de son mode plan), un tour fini sur un bloc `<escouade_question>` tenu jusqu'aux réponses (ses jetons additionnés, un message attendant, l'arrêt le finissant), un refus qui coupe le tour, un modèle que sa session n'offre pas laissé à son défaut, ses jetons (`thoughtTokens` dans la sortie) et sa limite d'usage (`limit_of`, avec celle d'Antigravity, `crates/escouade-acp/src/limits.rs`). Le moteur `Gemini` reste au registre du cœur : comptes Google par le fichier du dossier du compte (`GOOGLE_APPLICATION_CREDENTIALS`) ou par clé, `list_models` (un `gemini --acp` jetable, un petit échange `initialize` puis `session/new`), `gemini -p` pour les textes, la vue terminal par ses hooks seuls.

La table des harness livrés grandit pour tous : `Prepare` reçoit `Preparing` (le dossier de l'agent, son dossier de travail, la clôture, `read_only`, les variables données par le cœur) et répond les arguments qu'il ajoute au lancement. Gemini y écrit `settings.json`, retire la connexion rafraîchie, écrit ses règles (`--policy`) ; Vibe n'ajoute rien. `close_turn` montre tel quel un bloc qui n'est pas une question, et l'arrêt d'un tour tenu par une question le finit, pour tout harness `questionsInText`. Comme pour Cursor et Vibe, `git add` et `git commit` du worktree en « Accepter les modifications » demandent désormais (la règle lit le dépôt, ce qu'un plugin ne fait pas), et le menu des modèles vient de `list_models` seul : `Ready.catalog` n'existe pas sur le contrat.
**Codex en plugin, en process (INT-10.4, ESC-225, 2026-11-04)** : le plugin de première main `dev.escouade.codex` (`plugins/codex/`, crate `escouade-plugin-codex`, entrée `escouade-plugins codex`, `builtin::CODEX`, sa page du store pointant vers son dossier) n'est pas ACP : son client de `codex app-server` (`src/client.rs`, JSON-RPC une ligne par message, une machine à états sans process, chaque réponse attendue une valeur de `Ours`) et sa traduction (`src/translate.rs`, le contexte de l'agent reçu dans `Ctx`) ont quitté le cœur, avec la grille d'OpenAI (`src/prices.rs`, que `pricing::price` interroge pour un modèle de Codex parmi les estimations du cœur). Le moteur `Codex` reste au registre (`BUILT_IN`) : capacités et sens des modes, comptes (`CODEX_HOME`), quota et modèles d'un compte sans agent (`engine/codex/process.rs`, un app-server à lui, lancé puis arrêté), `codex exec`, la vue terminal et la confiance de ses hooks ; `spawn` passe par `spawn_plugged`, et sa ligne de `BUILTIN_HARNESSES` n'a ni `native` ni `prepare`. **La vue Codex lit son rollout par la même traduction, en bibliothèque** (`agent/codex.rs`) : deux traductions auraient divergé. Il se sert de ce qu'OpenCode (INT-10.2) et Copilot CLI (INT-10.3) ont mis au contrat, sans le refaire : `opens` et les éléments `ready` et `stop`, `agent/compact` et ses `writes`, `agent/rename`, `UsageAsked` et `usage`, `permissionCard` (la portée « pour cette session » de Codex, les hunks), `quota`, `answered`, `context`, l'entrée portée par `turn`, `endsStream` pour la carte d'un plan, `limitRest` (Codex attend sa limite), `hunks_of`. Il ajoute ce qui lui manquait, pour tous : `turn` dans la réponse d'`agent/compact` (la compaction écrite se poursuit en un tour du harness, qui la finit : pas d'élément `compacted` à attendre) ; `entry` (la dernière entrée quand aucun `turn` ne la dit : le tour fini sur un plan) ; `activity` (ce que fait l'agent quand l'élément suivant ne le dit pas : une commande qui ne fait que lire est un `Read`, un raisonnement commencé) ; `took` (la durée du tour selon le harness, à côté de `turn` pour que les plugins existants restent tels quels) ; une limite de 3 minutes à l'ouverture d'une session `opens` (`SESSION_WITHIN`, à la place des 60 s par requête de l'ancien client). `check_launch` compare les chemins de la clôture quelles que soient les barres : Codex lit `C:/code/secret`. Les règles de git d'`engine/codex/git.rs` passent dans `crates/escouade-acp/src/git.rs` : Codex garde `git add` et `git commit` acceptés sans demander, et son plugin lance donc `git rev-parse` ; il lit aussi le `config.toml` du compte, seulement quand le serveur d'Escouade est refusé (une entrée de l'utilisateur est alors éteinte). Après un plantage du plugin, les demandes en attente restent répondables par leur id (`Attach::pending`) ; le tour en cours n'est plus connu avant la fin de ce tour, et un message envoyé entre-temps est refusé par Codex, ce que la conversation dit. Tests : ceux du cœur inchangés contre le plugin (`codex_tests.rs`, `codex_tui_tests.rs`, `agent/codex_tests.rs` et ses flux rejoués, `engine/codex/bench.rs` dans un runtime des plugins livrés), l'arrêt dit (`a_codex_agent_says_its_plugin_is_off_and_talks_again_once_it_is_on`), le client seul (`src/tests.rs`), et les arguments d'un serveur refusé, descendus du test de `engine/codex/mod.rs` dans la crate avec le code qu'ils vérifient. **Essais réels non rejoués** (décidé par le chef le 2026-11-04) : le seul Codex de ce poste est celui de l'app de bureau, en 0.144.5, qu'Escouade refuse (la 0.163.0 au moins), et une 0.166 aurait tourné sur le `~/.codex` qu'elle partage. `real-harnesses.spec.ts` lancé s'arrête sur ce refus, `real_fence`, `real_codex_view` et `real_codex_compact` n'ont pas tourné : ils restent à rejouer sur une 0.163 ou plus récente.

**Antigravity CLI en plugin, en process (INT-10.8, ESC-283, 2026-11-04)** : le plugin de première main `dev.escouade.antigravity-cli` (`plugins/antigravity/`, crate `escouade-plugin-agy`, entrée `escouade-plugins agy`, `builtin::ANTIGRAVITY`, sa page du store pointant vers son dossier) n'est pas ACP : son client d'`agy -p` en stream-json (`src/client.rs`, une machine à états sans process) a quitté le cœur, avec la traduction de ses événements (`src/translate.rs`) et ses mots (`src/words.rs` : lancement, règles, dossier de l'agent, appels en forme d'ACP). Cette traduction garde sa forme d'avant : ses événements sont mis dans la forme d'ACP, puis lus par le client ACP de `crates/escouade-acp` (extension `agy`, qui prend les requêtes de Cursor sans ses refus par le texte), que le plugin conduit par `session_opened`, `prompt_begun`, `heard`, `text_whole`, `forget` et `prompt_ended`. Le moteur `Agy` reste au registre : capacités et sens des modes, comptes, `agy models`, `agy -p` des textes, la vue terminal (sa ligne de table n'a qu'un `prepare`, qui retire les hooks de la vue) ; `spawn` passe par `spawn_plugged` et retire le jeton du serveur d'Escouade du dossier de l'agent à la fin du process. **La vue lit les mots du plugin en bibliothèque**, pour les mêmes règles et les mêmes appels des deux côtés.

Il se sert de ce que le contrat avait déjà (`opens`, `ready`, `stop`, `closeGraceMs`, `keyVar`, `mode`, `Launch::files`, `prepare`) et le fait grandir pour tous, car un agent d'`agy` a **plusieurs programmes à la suite** : `agy` ne prend modèle, effort et mode qu'au lancement, relit mal à temps une règle écrite, et n'a pas de message qui arrête un tour. Le manifeste dit `relaunches`, et :
- `relaunch` (une `Launch` entière) arrête le programme et en démarre un autre, sa description **vérifiée comme la première** (`check_launch` : jeton, clôture, lecture seule, fichiers dans le dossier de l'agent) ; les lignes écrites ensuite vont au nouveau ;
- `pause` arrête le programme et garde l'agent, jusqu'au `relaunch` suivant ;
- `agent/programGone` dit au plugin, derrière ce que le programme a écrit, qu'il a fini de lui-même : `relaunch` continue (une conversation à reprendre disparue en ouvre une neuve), `stop` dit pourquoi, rien finit le process ;
- `file` écrit un fichier pendant que l'agent tourne, dans son dossier seul et avec le même contrôle que ceux d'un lancement : les règles écrites à chaud ;
- la réponse à tout appel qui rend des `writes` peut rendre des `items`, dans leur ordre (`Items`) : un message qui relance est `relaunch` puis `write` ;
- `stderrLeavesOut` écarte des lignes de sa sortie d'erreur (ses journaux nomment le compte) ;
- `agent/attach` redonne `home`, `readDirs`, `byKey`, la clôture, la lecture seule et les consignes à tout harness, pour un plugin qui redémarre et écrit des règles.

`check_launch` lit la clôture dans un fichier JSON, où les `\` d'un chemin Windows sont doublés, et quelle que soit la barre finale. Les fichiers sont décrits par le plugin, qui lit le dossier de l'agent pour garder le reste de `settings.json`. Deux écarts, sans effet à l'usage normal : la lecture du transcript, quand il est en retard sur l'événement, attend dans le fil du plugin (600 ms au plus par lot, là où l'ancien process n'attendait que pour son agent) ; et un `agy` qui reste muet à l'ouverture sans sortir n'est plus relancé sur une conversation neuve au bout de 90 s : l'ouverture s'arrête à la limite commune des harness `opens` (3 min, `SESSION_WITHIN`), et le dit. Une carte montrée avant un redémarrage du plugin ne peut pas continuer le tour : sa réponse le finit. Coupé, un agent d'Antigravity le dit au lancement et garde sa conversation.

**Claude Code en plugin, en process (INT-10.5, ESC-226, 2026-11-04)** : le plugin de première main `dev.escouade.claude-code` (`plugins/claude/`, crate `escouade-plugin-claude`, entrée `escouade-plugins claude`, `builtin::CLAUDE`, sa page du store pointant vers son dossier) tient son client (`src/client.rs`, stream-json et protocole de contrôle, une machine à états sans process) et sa traduction (`src/translate.rs`), qui ont quitté le cœur avec `localcmd`, et ce qui lit une connexion (`src/signin.rs` : ce que Claude Code garde dans `.credentials.json` et le trousseau, son empreinte, les mots d'une connexion perdue). Le moteur `Claude` reste au registre, façade : ses capacités, ses comptes, les modèles d'un compte, les textes rédigés (`claude -p`), la vue terminal, la déclaration du serveur MCP. Il ne reste au cœur, de `signin.rs` et `usage.rs`, que ce qui décide : relancer un agent une fois sa connexion bonne, l'une après l'autre (`RefreshGate`), lire le quota au point d'accès OAuth avec les réglages de proxy de l'app, les fenêtres de quota de tous les harness. La vue lit le transcript par la même traduction, en bibliothèque (`agent/claude.rs`). L'état de l'agent, son plan et ses demandes restent au cœur : c'est lui qui applique les éléments.

Le contrat grandit pour tous. **Une réponse du harness attendue** : une méthode peut répondre `Asked { writes, ask }`, le cœur écrit `writes` et attend `Item::Answer` sous `ask`, sans bloquer les appels suivants, ce qui vaut pour `agent/interrupt`, `agent/setModel`, `agent/setEffort`, `agent/setMode`, `agent/rename` et `agent/control`. **Trois méthodes** : `agent/stopTask`, `agent/taskOutput`, `agent/remote`, que le cœur refusait (« not supported ») ; un plugin qui ne les a pas reste refusé. **`LaunchParams`** gagne `takesEffort` (ce que le harness du compte a dit du modèle), `settings` et `refusalFile` (les fichiers que le cœur écrit). Un lancement qui nomme `settings` tient la clôture par ce fichier : le cœur ne cherche plus les chemins dans les arguments, et cette dispense ne vaut que pour le fichier qu'il a écrit lui-même. **`AnswerPermission`** porte `call` et `rules`, **`AnswerQuestion`** `input`. **Les éléments** : `conversation` (l'`init` d'un harness, sa session et ce que dit un changement), `message` et `messageOutput` (l'estimation du tour en cours), `tasks` (les tâches de fond pour le plan), `event`, `output` (la sortie d'une commande locale), `call` (un appel entier, le plan prévenu, sa ligne faite ou complétée), `toolResult` (comme `toolDone`, sur une ligne même finie), `replayed` (un message qui revient), `ask` (une carte avec ses `suggestions`, son titre et son défaut à non ; le cœur répond seul à la lecture d'un fichier joint), `result` (la fin d'un tour compté par modèle, les interruptions restant au cœur), `limit`, `overage`, `remote`, `told` (les commandes et les modèles du premier échange), `answer`, `said` (une ligne hors protocole, gardée pour ce que dit sa fin). Le contrôle d'un agent attend la fin de son lancement avant son premier appel : un message envoyé tout de suite n'est plus perdu. **Un plugin relancé sans plantage** (la langue ou le réseau changés relancent les plugins livrés) retrouve l'agent avant son prochain appel, pour tout harness (`known`) : une réponse à une carte n'est plus perdue, et un plugin coupé pendant un tour arrête l'agent en le disant. Un lot d'éléments s'applique en plusieurs fois, chaque carte et chaque fin de tour à part : `Effects` ne garde qu'une notification et qu'une fin de tour. stderr se lit en octets, une ligne qui n'est pas de l'UTF-8 n'arrête plus sa lecture, et les 30 dernières lignes sont gardées.

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

  **Avec INT-10.1 (ESC-222, 2026-11-02), Cursor CLI en process.** Le banc de flux rejoue les flux de Cursor par le process de son plugin (`Bench::play_plugged` : ce que Cursor a écrit passé au plugin par `agent/frames`, ses éléments à `Core::on_frame` ; les réponses enregistrées données sous les ids que le plugin a donnés à ses requêtes), `npm run bench:int0` construisant d'abord `escouade-plugins` en release. Mesuré deux fois le même soir, avant (le cœur) et après (le plugin), sous la même charge d'autres worktrees qui compilaient (d'où des 99e centiles de plusieurs ms pour tous les harness, Claude Code compris, loin de ceux du tableau plus haut) :

  | Cursor CLI | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Au plus vite : médiane | 54 µs | 1 163 µs |
  | Au plus vite : 99e centile | 7 972 µs | 7 214 µs |
  | Au plus vite : débit | 36 552 lignes/s | 9 032 lignes/s |
  | À son rythme : médiane, 99e centile | 22,2 ms, 31,3 ms | 21,2 ms, 32,2 ms |
  | Lignes sans événement (sur 471) | 99 | 107 |
  | Le process du plugin, au repos (release) | — | 10 Mo |

  Dans les seuils : moins de 5 ms de plus au 99e centile, au plus vite (moins, ici) comme à son rythme (+0,9 ms), et plus de 2 200 lignes/s ; 10 Mo pour le process (seuil : 30 Mo). Le prix est l'aller-retour vers le process, environ 1,1 ms par ligne au plus vite dans ce banc (chaque ligne y attend sa réponse, là où l'app envoie par lots) ; il disparaît dans le regroupement des deltas au rythme réel. Les 8 lignes de plus sans événement sont les réponses aux réglages (mode, modèle) que le client garde pour lui.

  **Avec INT-10.6 (ESC-281, 2026-11-02), Mistral Vibe en process.** Le banc de flux rejoue les six flux de Vibe par le process de son plugin (`plugged_by` : Cursor et Vibe ; les flux de Vibe en `in`/`out`, ses réglages d'ouverture laissés au client, les réponses à ses formulaires données par `agent/answerQuestion`). Avant : une copie propre de HEAD, au calme ; après : le worktree, au calme (une première mesure prise pendant nextest complet est écartée) :

  | Mistral Vibe | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Au plus vite : médiane | 116 µs | 1 635 µs |
  | Au plus vite : 99e centile | 1 357 µs | 13 855 µs |
  | Au plus vite : débit | 53 825 lignes/s | 13 898 lignes/s |
  | À son rythme : médiane, 99e centile | 23,2 ms, 32,2 ms | 24,3 ms, 32,4 ms |
  | Lignes sans événement (sur 902) | 167 | 529 |

  Au rythme réel, dans les seuils (+1,1 ms en médiane, +0,2 ms au 99e centile) ; au plus vite, le débit reste au-dessus de 2 200 lignes/s, mais le 99e centile dépasse le seuil (+12,5 ms pour 5 ms permis). Comme pour Cursor, chaque ligne y attend la réponse du plugin (l'app envoie par lots) ; les 362 lignes de plus sans événement par passe sont surtout des morceaux de raisonnement que le cœur regroupe avant de les peindre. Écart à reprendre (regroupement des deltas côté client ou banc par lots) avant de compter ce point comme tenu.

  **Avec INT-10.7 (ESC-282, 2026-11-03), Gemini CLI en process.** Le banc de flux rejoue les 140 lignes des dix cas de Gemini par le process de son plugin (`plugged_by` : Cursor, Vibe, Gemini ; ses flux sont le message lui-même, avec son `dir`). Avant : une copie propre du commit de départ ; après : le worktree. Deux passes de chaque côté, sous la charge variable des autres worktrees (la seconde passe « avant » l'a prise de plein fouet : un harness inchangé comme Antigravity y monte aussi à 3,7 ms) :

  | Gemini CLI, au plus vite (20 passes) | Avant (le cœur), 2 passes | Après (le plugin), 2 passes |
  | --- | --- | --- |
  | Médiane | 37 µs, 35 µs | 128 µs, 132 µs |
  | 99e centile | 871 µs, 5 562 µs | 761 µs, 834 µs |
  | Débit | 12 699, 7 187 lignes/s | 9 628, 9 158 lignes/s |
  | Lignes sans événement (sur 2 800) | 1 668, 1 674 | 2 008, 1 985 |
  | Le process du plugin, au repos (release) | — | 10 Mo |

  Dans les seuils : pas plus de 5 ms de plus au 99e centile (moins, contre la meilleure passe « avant »), plus de 2 200 lignes/s, 10 Mo pour le process (seuil : 30 Mo). Le prix est l'aller-retour vers le process, environ 0,1 ms par ligne en médiane. Les 16 lignes de plus sans événement par passe sont celles que le client garde pour lui (réglages, rejeu hors d'un prompt). Ses flux n'ont pas d'horodatage : il n'y a pas de rejeu à leur rythme, ni avant ni après. Le démarrage n'a pas été remesuré : un plugin de plus n'est lancé qu'avec son premier agent.
  **Avec INT-10.4 (ESC-225, 2026-11-04), Codex en process.** Mesuré avant la fusion des contrats d'OpenCode et de Copilot CLI, le client et la traduction de Codex n'ayant pas changé depuis que dans la forme de leurs éléments. Le banc de flux rejoue les quatre flux de Codex par le process de son plugin (`plugged_by`, `turn/start`, `turn/interrupt` et `thread/compact/start` demandés comme le cœur les demande, les réponses aux cartes par `agent/answerPermission` et `agent/answerQuestion`). Avant : deux mesures de 76c7c38a prises le même soir par ESC-223 et ESC-224 sur leurs copies de main ; après : le worktree, nextest arrêté, d'autres worktrees compilant :

  | Codex | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Au plus vite : médiane | 29 et 32 µs | 581 µs |
  | Au plus vite : 99e centile | 394 et 418 µs | 3 739 µs |
  | Au plus vite : débit | 95 970 et 81 991 lignes/s | 13 155 lignes/s |
  | À son rythme : médiane, 99e centile | 9,5 ms, 31,0 ms ; 8,6 ms, 31,7 ms | 7,1 ms, 31,5 ms |
  | Lignes sans événement (sur 11 660, sur 583) | 6 380, 319 | 6 380, 319 |
  | Le process du plugin, au repos (release) | — | 11 Mo |

  Dans les seuils : moins de 5 ms de plus au 99e centile, au plus vite (+3,3 ms) comme à son rythme (aucune différence au-delà de l'écart entre les deux mesures d'avant), plus de 2 200 lignes/s, 11 Mo pour le process (seuil : 30 Mo). Comme pour Cursor, le prix au plus vite est l'aller-retour de chaque ligne dans ce banc ; il disparaît dans le regroupement des deltas au rythme réel.

  **Avec INT-10.8 (ESC-283, 2026-11-04), Antigravity CLI en process.** Le banc de flux rejoue les six flux d'`agy` par le process de son plugin (`plugged_by`, `only_said` : ses lignes sont ses événements seuls, sans `dir`). Avant : une copie propre de e05a5265 ; après : le worktree, dans la même cible de compilation, `ESCOUADE_BENCH_ONLY=agy`, d'autres worktrees compilant dans les deux cas. Ses flux n'ont pas d'horodatage : il n'y a pas de rejeu à leur rythme, ni avant ni après.

  | Antigravity CLI, au plus vite (20 passes) | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Médiane | 36 µs | 161 µs |
  | 99e centile | 1 010 µs | 1 015 µs |
  | Débit | 9 462 lignes/s | 4 739 lignes/s |
  | Lignes sans événement (sur 1 380) | 280 | 280 |
  | Le process du plugin, au repos (release) | — | 11 Mo |

  Dans les seuils : +5 µs au 99e centile (seuil : 5 ms), plus de 2 200 lignes/s, 11 Mo pour le process (seuil : 30 Mo). Le prix est l'aller-retour vers le process, environ 0,1 ms par ligne en médiane. Le démarrage n'a pas été remesuré : un plugin de plus n'est lancé qu'avec son premier agent.

  **Avec INT-10.5 (ESC-226, 2026-11-04), Claude Code en process.** Le banc de flux rejoue les trois flux de Claude Code et son long tour par le process de son plugin (`plugged_by`, `only_said` : ses lignes sont ses frames, sans `dir`, passées telles quelles à `agent/frames`). Avant : une copie propre de e05a5265, en release, sur le poste au repos, juste avant le passage après (`ESCOUADE_BENCH_ONLY=claude`, puis `claude/long-turn`). Après : la branche fusionnée avec main (b0ffbe2a).

  | Claude Code | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Ses trois flux, au plus vite : médiane | 12 µs | 110 µs |
  | Ses trois flux, au plus vite : 99e centile | 447 µs | 835 µs |
  | Ses trois flux, au plus vite : débit | 22 137 lignes/s | 6 573 lignes/s |
  | Le long tour, à son rythme : médiane | 24,6 ms | 24,1 ms |
  | Le long tour, à son rythme : 99e centile | 32,3 ms | 32,2 ms |
  | Le long tour, à son rythme : CPU | 375 ms | 1 250 ms |
  | Le long tour, au plus vite : médiane | 4,5 ms | 13,8 ms |
  | Le long tour, au plus vite : 99e centile | 8,4 ms | 30,5 ms |
  | Le long tour, au plus vite : débit | 339 870 lignes/s | 17 320 lignes/s |
  | Lignes sans événement (sur 2 120, 3 371, 67 420) | 560, 98, 1 960 | 560, 98, 1 965 |
  | Le process du plugin, au repos (release) | — | 11 Mo |

  Au rythme réel, dans les seuils : le 99e centile ne bouge pas, 32 ms étant le regroupement des deltas. Le débit reste près de 8 fois au-dessus des 2 200 lignes/s, le process sous ses 30 Mo. **Au plus vite, le long tour dépasse le seuil** (+22,1 ms au 99e centile pour 5 ms permis), comme OpenCode et Mistral Vibe : 67 420 lignes rejouées d'affilée, là où ce tour réel en écrit 26 par seconde. Le CPU triple au rythme réel, soit 0,9 s de plus pour un tour de 2 min. Les 5 lignes de plus sans événement au plus vite tombent sur des lignes de delta regroupées.

  **Avec INT-10.3 (ESC-224, 2026-11-04), Copilot CLI en process.** Le banc de flux rejoue les quatre flux de Copilot par le process de son plugin (`plugged_by`). Les enregistrements ne suivent pas l'ouverture pas à pas d'un plugin `opens` : chaque session lui est donnée par `agent/attach`, comme après un redémarrage, et ses réponses arrivent aussi ligne à ligne. Les flux de Copilot n'avaient pas de ligne à leur rythme, leur dernière ligne n'ayant pas de `t` : `times_of` la prend avec la précédente, et `ESCOUADE_BENCH_ONLY=copilot` mesure ce harness seul. Avant : `main` à 76c7c38a ; après : la branche fusionnée avec Gemini CLI, en release sur ce poste, partagé avec d'autres agents.

  | Copilot CLI | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Au plus vite : médiane | 196 µs | 2 109 µs |
  | Au plus vite : 99e centile | 600 µs | 7 546 µs |
  | Au plus vite : débit | 158 123 lignes/s | 18 170 lignes/s |
  | À son rythme : médiane, 99e centile | 13,7 ms, 31,2 ms | 14,5 ms, 30,9 ms |
  | Lignes sans événement (sur 2 954) | 1 897 | 1 923 |
  | CPU à son rythme (2 954 lignes) | 47 ms | 1 000 ms |
  | Mémoire du cœur, pendant le banc | 58 à 65 Mo | 58 à 66 Mo |
  | Le process du plugin, au repos (release) | — | 10 Mo |

  Au rythme réel, dans les seuils (+0,8 ms en médiane, 99e centile inchangé), et le débit reste très au-dessus de 2 200 lignes/s ; 10 Mo pour le process (seuil : 30 Mo). Au plus vite, le 99e centile prend 6,9 ms, au-delà des 5 ms permis au rythme réel : comme pour Vibe, chaque ligne y attend seule la réponse du plugin, là où l'app envoie par lots. L'écart reste ouvert, avec celui de Vibe. Le CPU à son rythme est multiplié par vingt, sur un flux qui n'en prend qu'une seconde : c'est le prix de l'aller-retour vers le process.

  **Avec INT-10.2 (ESC-223, 2026-11-04), OpenCode en process.** Le banc de flux rejoue ses quatre flux par le process de son plugin (`plugged_by`, les lignes enregistrées telles quelles par `agent/frames`). Avant : une copie propre de HEAD ; après : le worktree, chacun seul sur le poste pendant la mesure d'OpenCode :

  | OpenCode | Avant (le cœur) | Après (le plugin) |
  | --- | --- | --- |
  | Au plus vite : médiane | 144 µs | 2 843 µs |
  | Au plus vite : 99e centile | 656 µs | 17 480 µs |
  | Au plus vite : débit | 277 458 lignes/s | 18 733 lignes/s |
  | À son rythme : médiane, 99e centile | 16,6 ms, 31,5 ms | 16,1 ms, 31,5 ms |
  | Lignes sans événement (au rythme réel, sur 5 147) | 1 213 | 1 212 |
  | Le process du plugin, une session ouverte (release) | — | 12,9 Mo |

  Au rythme réel, dans les seuils (rien de plus au 99e centile) ; le débit reste 8 fois au-dessus des 2 200 lignes/s, et le process sous ses 30 Mo. **Au plus vite, le 99e centile dépasse le seuil** (+16,8 ms pour 5 ms permis), comme celui de Mistral Vibe : à ce rythme du banc, une ligne attend la réponse du plugin avant la suivante, et 17 ms est de l'ordre de la fenêtre de 16 ms du regroupement des deltas, que le flux traversait avant en moins d'une fenêtre. L'explication est déduite des chiffres, pas mesurée à part. Ce banc n'emprunte pas le chemin de l'app (`agent/items`, sans réponse attendue) : écart ouvert, avec celui de Vibe.

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
| ESC-165, GitLab Issues | Une source de plus, sur les comptes de l'hébergeur GitLab (`forge::accounts`) : une variante de `Service` et de `integrations::Client`, comme ClickUp. | **Fait dans le cœur pour la 2.0.3** (décidé le 2026-10-26, à la demande d'un testeur) : l'étape 1 le passe derrière le contrat avec les autres, l'étape 5 le sort avec le plugin « GitLab » (INT-8.2, ESC-218 ; _fait le 2026-11-02_), son compte déjà partagé avec l'hébergeur. |
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

**Depuis P6 (ESC-350, §8.28), ces cinq plugins sont des modules** et le code des services est la crate `escouade-voice` : ce qui suit reste vrai du protocole `voice/*` et du compte Mistral, plus du programme `escouade-plugins`, de `plugin_guest.rs` ni du service du cœur qui répondait à la place d'un plugin.

**Ce qui est livré.** OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont cinq plugins de première main (`plugins/<service>/` : manifeste, README), au registre des plugins livrés avec l'app (`plugins/builtin.rs`), chacun un process `escouade-plugins voice-<service>` (`src-tauri/src/bin/escouade-plugins/voice.rs`). Le cœur ne les appelle que par le contrat d'INT-12 : `ProcessVoiceService: VoiceService` (`voice/plugin_host.rs`, sur le modèle de `ProcessTicketSource`) répond à la dictée, au direct, à la synthèse et au catalogue par JSON-RPC ; le côté plugin (`voice/plugin_guest.rs`) sert `voice/*` avec le code des cinq services tel qu'il était dans le cœur (`online.rs`, `live.rs`, `online_speech.rs`, `online_catalog.rs`), sans le changer. **Écart assumé avec §4.3** : le programme `escouade-plugins` n'est donc plus sans `escouade_lib` pour ces cinq entrées. Sortir ce code dans une crate à part (la forme de §4.3) demanderait de défaire son lien avec les réglages, le trousseau et la conversation, pour un gain de taille du seul exécutable ; elle reste à faire quand INT-10 aura fixé la forme des crates.

**Le contrat s'est ouvert** (revue d'INT-12) : `Provider` n'est plus une énumération mais l'identifiant du service, interné (`Provider::of(id)`, `Copy`) ; les cinq du cœur en sont les constantes (`Provider::OpenAi`…) et ni `Spec` ni `OnlineError` n'en dépendent plus. Le registre (`voice/service.rs`) liste aussi les services des plugins : `register`, `unregister`, `hide`, `services`, `find`, `of`. Un plugin d'un id du cœur prend sa place ; un service dont le plugin est parti répond par une erreur (`Gone`), jamais par un plantage. `core.rs` met le registre d'accord avec les interrupteurs des plugins (`plugin_host::sync_builtin`) au démarrage et à chaque changement : un plugin coupé retire son service (la dictée repasse en local), un plugin qui ne démarre pas laisse le service du cœur répondre, le journal du plugin disant pourquoi.

**Le protocole `voice/*`** (`voice/wire.rs`), de l'app vers le plugin : `voice/describe` (id, nom, modèle par défaut, direct ou non, modèle du direct, longueur maximale d'un passage), `voice/transcribe` (le son en flottants 32 bits, `Spec` : modèle, clé, adresse, réseau), `voice/live/open`, `voice/live/audio` (notification), `voice/live/finish` (le texte définitif), `voice/live/close`, `voice/speak` (le son revient par la notification `voice/audio`, en 16 bits), `voice/catalog`, `voice/list`, `voice/cancel` (notification : un appel que `cancel` a coupé). Du plugin vers l'app : `voice/partial` (le texte compris jusqu'ici) et `voice/audio`. Les notifications d'un plugin que le journal ne connaît pas vont à un écouteur (`Runtime::on_notification`), dans l'ordre où le plugin les a écrites. Une erreur du service traverse en refus JSON-RPC (codes `-32101` à `-32106` : pas de clé, clé refusée, réseau, service, quota, voix) et revient la même `OnlineError`. La clé reste au trousseau de l'app (`dictation-<service>`, l'entrée d'avant) et part dans chaque appel, vers le seul plugin de ce service, qui ne la garde ni ne l'écrit.

**Le compte Mistral (tranché avec INT-6, fait par INT-10.6 bis, ESC-327, 2026-11-03).** Le plugin « Mistral » porte la voix et Mistral Vibe, comme « GitHub » porte les Issues et les PR. Le process est un seul, `escouade-plugins mistral` : `voice/*` va à `voice/plugin_guest.rs`, tout le reste, dans son ordre, à la crate `escouade-plugin-mistral` (`handler`, le client de `crates/escouade-acp` avec son extension `vibe`), et `initialize` dit les deux. Son manifeste déclare le compte (`contributes.accounts`, id `mistral`) ; le harness le nomme (`account`), la voix aussi (`voice` du compte, car elle n'est pas un point de `contributes`) : `points` dit `harnesses:vibe` et `voice:mistral`. La clé ne part jamais par `secrets/get` : le plugin n'a pas de `permissions.secrets`, la voix la reçoit à chaque appel et Vibe par `MISTRAL_API_KEY`, donnée par le cœur.

**Plusieurs comptes Vibe, un compte Mistral (choix de l'utilisateur du 2026-11-03, sur captures).** Une clé unique aurait fait perdre une clé à qui a deux organisations Mistral : les clés restent, et se partagent. Le compte `mistral` est la clé de Réglages › Voix (`plugin:dev.escouade.mistral:account:mistral`, ancien nom `dictation-mistral`). Un compte de Vibe qui a sa clé la garde (`plugin:dev.escouade.mistral:account:vibe-<id>`, ancien nom `vibe:<id>`). Chaque ancien nom reste tenu et lu d'abord (la migration réversible d'INT-6 : `read_entry`, `accounts::revert_mistral` pour le retour) : personne ne ressaisit sa clé. Chaque côté se replie sur l'autre : un compte de Vibe sans clé va par celle du compte Mistral (la modale d'ajout le propose, champ vide) ; la voix sans clé va par celle du premier compte de Vibe actif, et Réglages › Voix le dit (« Celle du compte Vibe « Équipe » · clé …wxyz », avec « Donner une autre clé », sans « Effacer » : elle n'est pas à la voix). Le Principal de Vibe reste la clé de `vibe --setup`, qu'Escouade ne lit jamais. Le plugin coupé retire les deux côtés à la fois.

**Les mesures** (`voice/plugin_tests.rs`, `cargo nextest run --run-ignored only int13_measure --no-capture`, faux serveurs WebSocket, Windows 11, build de debug, 20 phrases de 0,3 s par service, le son poussé au rythme du micro : une trame de 20 ms toutes les 20 ms) : le délai du texte final après la fin de la phrase, en médiane et au 99e centile. Dans le cœur : 0,5 ms (p99 0,7) pour OpenAI, Mistral et ElevenLabs, 0,6 ms (p99 0,9) pour Deepgram. **Par le plugin** : 1,0 ms (p99 1,4), 1,0 (1,6), 1,1 (1,5) et 1,2 (1,7) : **moins de 1 ms de plus en médiane comme au 99e centile**, sous le seuil d'INT-0 (5 ms). Poussé d'un coup (une demi-seconde de son sans attendre, ce que le micro ne fait pas), le plugin ajoute 8 à 10 ms en médiane et 20 à 60 ms au 99e centile : le temps d'encoder, de passer le tuyau et de décoder les 25 trames en attente. Le seuil est la mesure à la main ; la CI ne garde que le passage du direct par le plugin, avec des bornes larges (`the_live_transcription_goes_through_the_plugin_and_says_what_the_core_says`).

**L'essai réel ElevenLabs, rejoué par le plugin** (`int13_real_elevenlabs_through_the_plugin`, la clé de la machine donnée à ce seul process) : la dictée d'une phrase française en 1,0 s et une mauvaise clé refusée ; le direct (textes partiels pendant que la phrase se dit, définitif 165 ms après la fin) ; une phrase lue (premier son en 2,5 s, 34 morceaux) ; 9 modèles et 31 voix listés.

**Wispr Flow (ESC-164) n'est pas à faire** pour l'instant (décision de l'utilisateur du 2026-11-01) : il s'écrira, le jour venu, comme un sixième plugin sur ce protocole.

## 5. Les programmes en JS/TS sans Node : un runtime WASM (ESC-319)

Demandé par l'utilisateur le 2026-11-01, en apprenant qu'un plugin en TypeScript (`dev.escouade.slack`, `dev.escouade.team-rules`) se lance par `node dist/plugin.mjs` : « je veux pas rendre node obligatoire, donc soit tu les exécutes dans la partie navigateur de tauri, soit tu prends en compte wasm directement et chaque plugin ts qui est soumis sera buildé en wasm ».

Ce chapitre est l'étude, avec un prototype mesuré (`prototypes/wasm-plugins/`, son README dit comment refaire chaque mesure), et ce que les tickets du §5.9 en ont construit : tout est dans l'app, dans la CLI et dans le code du store. Reste la mise en production de la compilation au store (ESC-328). Il a révisé le choix du §2.4.

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
- **Concurrence** : une instance ne fait qu'un tour à la fois ; l'hôte sert un plugin depuis un fil à lui. Depuis ESC-331, un `fetch` lent n'arrête plus rien : il est servi à côté du fil, 16 au plus à la fois par plugin (§8.11).

**Ce qu'un plugin WASM ne peut pas faire.** Lire ou écrire un fichier hors de ce qu'on lui donne ; ouvrir une socket ; résoudre un nom ; atteindre un hôte qui n'est pas dans son manifeste, ou en http ; lancer un programme ; lire l'environnement, la ligne de commande, l'heure exacte au-delà de celle de WASI ; charger un module npm qui n'est pas dans son bundle, ni un module natif (`.node`) ; faire `import()` d'une URL ; utiliser `child_process`, `net`, `worker_threads`. Son bundle n'importe que `node:fs` (pour `readFileSync`) et `node:readline` (pour le SDK) : un autre `import` refuse la compilation, avec la raison.

### 5.5 Le SDK `@escouade-dev/plugin`

Son API ne change pas : `createPlugin().onEvent(...)`, `decidePermission(...)`, `p.secret`, `p.settings`, `p.log`, `start()`. Le prototype l'a montré en faisant tourner les deux bundles tels qu'ils étaient commités, le prélude imitant Node (`process.stdout.write`, `createInterface` de `node:readline`). **Fait par WASM-2 (ESC-321), SDK 0.3.0** :

- `start()` choisit le transport sans que le plugin le sache : sous Node, l'entrée et la sortie standard ; dans l'app, l'hôte WASM d'Escouade (`globalThis.Javy` et `globalThis.__esc` du prélude : `listen` reçoit les lignes que la pompe d'`escouade-turn` lui passe, `write` les envoie, `trace` va au journal). La promesse de `start()` ne se résout jamais en WASM.
- `p.file(chemin)` lit un fichier du dossier du plugin, en texte (`files/read`, asynchrone), un sous-dossier compris (`data/words.txt`) ; l'hôte le sert **aux deux transports** (`plugins/files.rs`, le même pour un process et pour une instance) et refuse (`-32003`) un chemin qui sort du dossier (absolu, `..`, un lecteur, un lien qui mène dehors, jugé après résolution), un nom caché (`.env`, `.git`) et le manifeste ; `-32005` pour un fichier absent, `-32004` au-delà de 256 Kio ou s'il n'est pas UTF-8. `readFileSync(new URL('../rules.json', import.meta.url))` marche toujours (les JSON de la racine, donnés au démarrage).
- `fetch` reste le `fetch` standard : en WASM, le prélude le sert par `net/fetch` (`Headers` accepte un objet, un tableau ou des `Headers`) ; sous Node, c'est celui de Node.
- Le prélude (`wasm_prelude.js`) se réduit à ce que le SDK ne fait pas : lire et écrire les lignes par `Javy.IO`, `fetch`, `readFileSync`, `URL`, `console`, un `process` sans flux ni environnement. Plus de `process.stdout`, ni de `createInterface` qui marche : un bundle construit avec le SDK d'avant (0.2) ne tourne plus en WASM, et son journal dit de le reconstruire (« build the plugin with @escouade-dev/plugin 0.3 or later »). Slack et Règles de l'équipe ont leur `dist/` reconstruit avec le SDK 0.3.
- **Les minuteurs** : `setTimeout` et `clearTimeout` existent en WASM depuis ESC-332 (§8.4.5), servis par `timer/set`.
- `create-escouade-plugin` écrit `"run": { "js": "dist/plugin.mjs" }`, sans scripts `plugin` ni `plugin.cmd`. Le banc d'`escouade-plugin` (`test`, `dev`) joue en attendant ce bundle par Node, le SDK y parlant comme dans l'app ; WASM-3 le jouera en WASM.
- `schemas/plugin-protocol-1.json` décrit `net/fetch` et `files/read` (paramètres, réponse, délais de 10 s et 5 s, erreurs `refused` -32003, `failed` -32004, `notFound` -32005) ; le TS du SDK en est généré.
- Les tests : le SDK en Vitest (le transport WASM simulé, `p.file` contre le faux Escouade), et un plugin témoin écrit pour le SDK (`tests/fixtures/plugins/sdk-both`) joué par le runtime **par Node et en WASM**, le même scénario sur les deux (`plugins_wasm_tests.rs`).

### 5.6 Le pipeline du store

**Le manifeste.** Un programme en JS/TS se déclare par **`"run": { "js": "dist/plugin.mjs" }`**, un champ de `run` à côté de `command` : le bundle commité est la source de vérité, le `.wasm` est un produit du store, jamais commité. Un `run.wasm` (un module écrit en Rust ou en Go, compilé par son auteur) n'est pas ouvert : le store ne pourrait pas le reproduire. Le §8.5 l'ouvre, autrement : le store compile les sources en Rust. Un manifeste a `command` **ou** `js`, pas les deux.

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

**Le `run` natif, tranché** (ce qui suit est revu par le §8.8, dès l'acceptation des décisions d'ESC-330) **:**

1. **La première main le garde.** Les plugins `dev.escouade.*` qui sont des programmes Rust du dépôt de l'app (les harness, les sources du Kanban, les hébergeurs, la voix : §4), construits et signés avec l'app (`{escouade}/escouade-plugins`), restent des process : ils lancent des CLI, parlent à des API avec des comptes, et leur confiance est celle de l'app. Slack et Règles de l'équipe, eux, passent en `run.js` (§5.8) : rien ne les oblige à être des process.
2. **Les binaires tiers attestés restent permis**, inchangés : un exécutable par système, son attestation de provenance GitHub, « Lance un programme sur votre ordinateur, avec vos droits » en rouge. C'est la seule voie pour un plugin tiers qui doit lancer un programme (un harness ACP, une commande) ; ils ne seront jamais « sans crainte ». Un auteur en Go ou en Rust qui n'a besoin d'aucun droit du système est invité à `run.js`… ou à attendre `run.wasm`.
3. **`node` (et tout interpréteur) n'est plus une commande d'un plugin tiers du store.** Un `run.command` qui est un script ou qui nomme un interpréteur hors du dossier du plugin est refusé à la soumission, avec la raison (« un programme en JS/TS se publie en `run.js` : le store le compile en WASM ») : sinon, l'utilisateur sans Node aurait un plugin qui ne démarre pas, et la revue lirait un code qui n'est pas ce qui tourne (les `node_modules` de la machine). Installé depuis un dossier ou un dépôt git, « non vérifié », un tel plugin reste permis, et Réglages › Plugins dit qu'il lui faut Node.

### 5.8 La migration de Slack et de Règles de l'équipe

**Fait dans le dépôt** (WASM-7, ESC-326, 2026-11-02) ; **pas encore au store** : leur 2.0.0 attend que le store sache compiler en production (ESC-328), et les versions publiées, `slack` 1.1.0 et `team-rules` 1.1.1, se lancent par Node (§7). `dev.escouade.slack` 2.0.0 et `dev.escouade.team-rules` 2.0.0 : `"run": { "js": "dist/plugin.mjs" }` à la place de `command` et `windows`, leurs scripts `slack`, `slack.cmd`, `team-rules`, `team-rules.cmd` retirés, `engines.node` réservé au build et aux tests, `escouade.min` relevé à la version de l'app qui sert le WASM. Le code ne change pas (le prototype le prouve) ; le SDK de la version WASM se rebâtit dans leur `dist/`. Ce qu'ils gagnent : plus de Node chez l'utilisateur, et `permissions.network: ["slack.com"]` appliqué. La version 1.x reste au store pour les apps d'avant (sa compatibilité, `escouade.max`). C'est le chef qui revoit, signe et publie ces versions (`first-party-plugin-signing`).

### 5.9 Les tickets, dans l'ordre

Chacun livrable seul, chacun après le précédent :

1. **WASM-1, le runtime dans l'app** (fait, ESC-320) : `plugins/wasm.rs` (l'hôte du prototype, `escouade-js` construit en CI et livré avec l'app), `run.js` lu par les deux lecteurs du manifeste, la compilation par la Javy épinglée, la précompilation et son cache, `net/fetch` et les fichiers, les limites, le redémarrage ; le poids mesuré dans l'exécutable de l'app.
2. **WASM-2, le SDK** (fait, ESC-321) : `start()` qui choisit le transport, `p.file()`, `fetch`, le prélude réduit ; `create-escouade-plugin` qui écrit `run.js`.
3. **WASM-3, `escouade-plugin` sans Node** (fait, ESC-322) : `dev`, `test`, `validate` et `pack` sur `run.js`, la Javy téléchargée et vérifiée, le banc qui joue le WASM, `build`.
4. **WASM-4, le store** (fait, ESC-323 ; en production avec ESC-328) : le onzième contrôle (la compilation isolée, deux fois), `wasm` dans `version.json`, le contrôle des binaires qui refuse script et interpréteur pour un tiers, l'app qui télécharge et vérifie le `.wasm`.
5. **WASM-5, la revue locale** : la recompilation et la comparaison des empreintes, les pistes de lecture revues, le conteneur réservé aux binaires. **Fait le 2026-11-01** (ESC-324, docs/STORE.md §4.2).
6. **WASM-6, la doc du site** (fait, ESC-325, fr et en) : `plugins/api` (le transport WASM, `net/fetch`, les fichiers, les limites, ce qu'un plugin ne peut pas faire), `plugins/sdk`, `plugins/publish` (`run.js`, la compilation, plus de Node), `plugins/examples`.
7. **WASM-7, la migration** de `dev.escouade.slack` et `dev.escouade.team-rules` en 2.0.0 sur `run.js` (faite dans le dépôt, ESC-326) ; leur revue, leur signature et leur publication par le chef suivent ESC-328.

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

## 7. L'essai réel (ESC-184, 2026-11-04)

Il remplace « l'essai réel des cinq exemples » du plan : les `dev.exemple.*` ont été retirés du store sans jamais servir (décision de l'utilisateur du 2026-11-01). Il porte sur les plugins de première main publiés et signés, et sur les plugins livrés avec l'app.

### 7.1 Sous Windows, depuis store.escouade.dev

**Comment.** Une app de debug construite sur main (e05a5265, elle embarque les clés publiques depuis f5b67f76), sur ses propres dossiers, sans `ESCOUADE_STORE_TEST_KEY` : les sceaux sont vérifiés contre les vraies clés. Le scénario est `tests/e2e/real-store.spec.ts`, sauté sans `ESCOUADE_E2E_REAL_STORE` ; il se rejoue à la release, et il écrit son rapport et ses captures dans `ESCOUADE_E2E_REAL_OUT`. Aucun webhook ni jeton réel, statistiques coupées : rien n'a été compté au store.

| Plugin | Sceau et fenêtre | Ce qu'il fait | Désactivé, désinstallé |
| --- | --- | --- | --- |
| Contraste élevé 1.1.0 (`store:ca76…2cba`) | « Vérifié par Escouade », commit a2480d7, archive identique, signature vérifiée ici ; « Sans programme » | Choisi dans Réglages › Application › Couleurs, il change la fenêtre (`--text` #15171c → #000000) ; « Escouade » rend les couleurs d'avant | désinstallé : dossier et preuves partis |
| Discord 1.1.0 (`store:ece7…ce53`) | Le même ; deux faits, `discord.com`, un secret | **Arrêté à la configuration** : le champ « Adresse du webhook Discord » au trousseau, aucune adresse donnée | désinstallé : dossier et preuves partis |
| Slack 1.1.0 (`store:6014…d6f3`) | Le même ; « Lance un programme », `slack.com`, un secret | Son programme démarre (`slack.cmd`) et répond à `initialize` ; **arrêté à la configuration** : le champ du jeton, aucun jeton donné | désinstallé : dossier et preuves partis |
| Règles de l'équipe 1.1.1 (`store:68e7…7d98`) | Le même, commit 3f3485d ; « Lance un programme », « Décide des autorisations » en rouge | Le `rm -rf build` d'un agent (le faux Claude Code des e2e) est refusé : « Refusé par Règles de l'équipe via Escouade — The team's rules forbid `rm -rf` » | désactivé : la même commande arrive à l'utilisateur ; désinstallé : dossier et preuves partis |

Après un redémarrage de l'app, les quatre sont là et leur sceau tient, relu de ses preuves. Le refus du tableau est celui du plugin relancé avec l'app ; à froid, voir plus bas.

**Ce qu'il a trouvé et corrigé.** La fenêtre d'installation disait de Discord qu'« Escouade ne filtre pas encore son réseau », alors qu'Escouade applique les hôtes déclarés aux webhooks qu'elle envoie elle-même (`plugins/events.rs`). Un plugin déclaratif a maintenant son texte. La même fenêtre disait « 2 webhooks : » sans rien après, un webhook n'ayant pas de nom : elle dit ce qui le déclenche. Et « store.escouade.dev · store.escouade.dev » : une fois suffit.

**Ce qu'il n'a pas pu faire.**

- **Une mise à jour** : il n'y en a pas à jouer. Chaque plugin n'a qu'une version installable : la seule autre version du catalogue, `team-rules` 1.1.0, n'est pas signée, donc refusée pour un `dev.escouade.*`. La mise à jour reste couverte par le faux store (`plugins_store_tests.rs`, et `plugins.test.ts` pour la fenêtre).
- **La révocation** : aucune liste n'est publiée (`/api/v1/revocations.json` répond 404, « No revocation list yet », et sa copie sur escouade.dev aussi) ; l'app le tient sans erreur. Sa lecture reste couverte par le faux store. `docs/STORE.md` §9.1, étape 8, est encore à jouer pour de vrai.
- **Couper le réseau** puis relancer : le redémarrage a été fait réseau ouvert.

**À savoir pour la release.**

- `slack` 1.1.0 et `team-rules` 1.1.1 se lancent par Node (`slack.cmd`, `team-rules.cmd`) : sans Node sur la machine, ils ne démarrent pas. Leurs 2.0.0 en `run.js` attendent ESC-328, qui attend elle-même la release 2.2.0 : **une 2.2.0 proposera donc un moment des versions à Node**, à fermer juste après la release.
- **Le premier refus de `team-rules` 1.1.1 peut arriver trop tard.** Juste installé, son process démarre avec la première demande, et Node par `team-rules.cmd` a pris plus de 3 s dans plusieurs passages, le poste chargé (journal : « no answer within 3000 ms », le process démarré 5 s après la demande ; quand il répond, c'était en 2,3 s). La demande reste alors à l'utilisateur : **rien n'est autorisé**, c'est le repli voulu. Le plugin relancé avec l'app répond à temps : `decide::warm` le démarre. Depuis ESC-329, `decide::warm_one` le démarre aussi dès qu'il devient prêt et activé (installation, mise à jour, réactivation, `decide` rendue), avant toute demande. La 2.0.0 en `run.js` lève la question de Node.
- Les plugins de voix (`openai`, `groq`, `elevenlabs`, `deepgram`) disent `escouade.min` 2.2.0 : une app qui se dit 2.1.2, comme ce build, les charge désactivés et le service du cœur répond à leur place.

### 7.2 Les plugins livrés avec l'app

Sous Windows, le même build, plugin par plugin de la vue Plugins, onglet Journal : sur les 22 de ce build de développement, **18 démarrent** (« En marche », leur programme `escouade-plugins <nom>` a répondu à `initialize`) :

- la démonstration : Minimal ;
- la voix : Mistral ;
- le lancement : isola ;
- les hébergeurs et les sources : GitHub, GitLab, Bitbucket, Gitea / Forgejo, Azure DevOps, Trello, Linear, Jira, ClickUp ;
- les harness : ACP, Cursor CLI, Gemini CLI, OpenCode, Copilot CLI, Codex.

**4 ne démarrent pas dans ce build, comme prévu** : OpenAI, Groq, ElevenLabs et Deepgram, qui demandent Escouade 2.2.0 (§7.1). À vérifier sur l'app de la release.

Cet essai dit qu'un programme démarre et parle le protocole, rien de plus : aucun compte réel de service n'y a servi. Les essais avec de vrais comptes sont ceux de leurs tickets.

### 7.3 Les harness : les essais de chaque ticket

Ils n'ont pas été rejoués ici : chaque ticket INT-10 l'a fait sur ce poste, contre son plugin.

| Harness | Ticket | Rejoué et passé | Ce qui manque |
| --- | --- | --- | --- |
| Cursor CLI | ESC-222 | `real_cursor` (reprise comprise), puis `real-harnesses.spec.ts` par ESC-282 | `real_cursor_view` : la limite du compte Free atteinte, à relancer après sa remise à zéro |
| Mistral Vibe | ESC-281 | `real_vibe`, `real_vibe_writes_a_text`, `real-harnesses.spec.ts`, puis `real-harnesses.spec.ts` par ESC-282 | — |
| Gemini CLI | ESC-282 | aucun | **Pas de compte sur ce poste** : `real_gemini` reste écrit et ignoré, jamais joué |
| OpenCode | ESC-223 | `real_opencode`, `real_fence`, `real_opencode_view`, `real-harnesses.spec.ts`, sur Ollama, avant la fusion avec main | — |
| Copilot CLI | ESC-224 | `real_copilot`, `real_fence`, `real_copilot_through_its_plugin` | **La vue**, qui demande un jeton : `real_copilot_view`, `real_copilot_view_on_a_free_account` ; et `real-harnesses.spec.ts` |
| Codex | ESC-225 | aucun | **Le Codex de ce poste est en 0.144.5, sous le minimum 0.163, et refusé** : `real-harnesses.spec.ts`, `real_fence`, `real_codex_view`, `real_codex_compact` à jouer sur une 0.163 ou plus |
| Antigravity CLI | ESC-283 | en cours | à reprendre ici quand il sera dans main |
| Claude Code | ESC-226 | `real-harnesses.spec.ts`, `real_fence`, `real_claude_view`, `real_claude_models`, sur Haiku, le compte Principal, Claude Code 2.1.317 et 2.1.318 | **Le changement de compte au seuil entre deux vrais comptes** : un seul compte Claude sur ce poste, ses tests restent sur les faux ; **la passation réelle vers Codex** : le Codex de ce poste est refusé. `real_claude_view` a passé deux fois sur quatre, et les deux dernières fois seul ; sous charge, une fois son attente de 60 s dépassée, une fois un sous-agent encore « en cours » à la fin du tour |

### 7.4 macOS et Linux

Il n'y a pas de Mac sur le poste de l'essai, et WSL n'y répond pas. **Aucun essai réel n'a donc été fait sous macOS ni sous Linux.** Ce qui tient lieu, à la CI de main (verte sur e05a5265, le 2026-11-04) :

| | Linux (ubuntu-22.04) | macOS (macos-latest) |
| --- | --- | --- |
| clippy, tests Rust (runtime, WASM, banc, installation, store contre son faux) | oui | oui |
| La page d'un lancement dans une vraie webview | oui | oui |
| Les paquets portent `escouade-hook`, `escouade-plugin`, `escouade-plugins` | oui | oui, signés |
| `plugins/ci.mjs` : tests, `validate`, banc des plugins de première main | oui | non |
| Les gabarits du SDK (`validate`, banc) | oui | non |

Les e2e, et donc la vue Plugins, le store dans l'app, les thèmes et les sceaux vus dans la fenêtre, ne tournent que sous Windows.

**À faire à la release, sur un Mac et sous Linux** : `docs/STORE.md` §9.1, étape 7 : installer depuis store.escouade.dev, le sceau et la fenêtre des permissions, le thème dans WKWebView et WebKitGTK, un programme par plugin (les scripts `slack` et `team-rules` des versions à Node, qui doivent être exécutables), le lien `escouade://`, les plugins livrés et leur signature (`codesign` sous macOS), puis la révocation (étape 8).

## 8. Tous les plugins en WASM, la première main comprise (ESC-330)

Décidé par l'utilisateur le 2026-11-04 : « je veux que tous les plugins, même les builtins, passent par le même système : on livre un plugin, il est compilé pour qu'il soit utilisable sur tous les systèmes, et il fait ce qu'il a besoin de faire », puis « en 2.2, tous les plugins, rust ou TS, seront compilés en WASM, c'est acté ». Le même jour, l'état de fin : « A la fin, je veux que tous les plugins builtins soient dans /plugins/* (exemple plugins/claude), tous publiés sur le store, vérifiés et signés. »

Ce chapitre est l'étude, faite avant le portage. **A1 à A8, A9, P3b, P1, P2, P3 (Codex, Copilot CLI, Cursor, Gemini), P4, P5, P6, P7 et D1 sont faits** (ESC-331, §8.11 ; ESC-332, §8.12 ; ESC-334, §8.13 ; ESC-338, §8.14 ; ESC-333, §8.15 ; ESC-341, §8.16 ; ESC-335, §8.17 ; ESC-336, §8.18 ; ESC-344, §8.19 ; ESC-351, §8.22 ; ESC-349, §8.23 ; ESC-348, §8.24 ; ESC-345, §8.25 ; ESC-346, §8.26 ; ESC-350, §8.28 ; ESC-337, §8.20 ; ESC-342, §8.21 ; ESC-347, §8.27 ; ESC-354, Z2, §8.32) ; le reste du plan ne l'est pas. Le prototype, ses tests et ses mesures sont dans `prototypes/wasm-all/` (son README dit comment tout refaire). Il revoit le §5.7 (« le `run` natif, tranché ») et le §5.6 (« un `run.wasm` n'est pas ouvert ») : ces deux passages tiennent jusqu'à l'acceptation des décisions proposées par ESC-330. Sans numéro tant qu'elles ne sont pas acceptées, elles se citent par leur identifiant : `run-wasm-le-rust-des-plugins-compile-par`, `les-capacites-de-l-hote-ce-qu-un-plugin`, `la-premiere-main-dans-plugins-id-court-p`, `plus-aucun-run-command-le-store-le-refus`.

**En bref.**

- **Le portage coûte peu sur ce qui a été essayé.** Le fichier source de Linear compile **sans changer une ligne** ; il reçoit seulement l'export du tour, ajouté par `prepare.mjs`. Codex change cinq endroits (`prepare.mjs` les imprime). Les deux répondent exactement comme leur natif, ligne à ligne, sur les flux enregistrés.
- **Le coût à l'exécution est nul ou favorable** : voir les mesures du §8.9. Un plugin passe de 7 à 11 Mo par process à moins de 2 Mo, et son démarrage de 14 à 18 ms à 2 ou 3 ms. Le seul recul mesuré : un appel de Linear prend 0,2 ms de plus.
- **La reproductibilité tient, à une condition.** Le même rustc 1.98.0 donne **les mêmes octets sous Windows et sous Linux** (`5ae4aabb…` pour Linear, `c5ab5a0a…` pour Codex), à condition de passer par un enveloppeur de `rustc` et une chaîne d'outils minimale. Sans lui, les empreintes diffèrent.
- **Ce qui reste difficile n'est pas la compilation.** C'est, dans l'ordre : OpenCode (flux SSE, serveur local), la voix (WebSocket, sons binaires, code encore dans le crate de l'app), Antigravity (attente active d'un fichier) et les hébergeurs auto-hébergés et derrière un proxy, que `net/fetch` refuse aujourd'hui (§8.4.1).
- **Un risque neuf au store.** Compiler du Rust exécute du code natif de l'auteur : les `build.rs` et les macros procédurales. Javy n'en avait pas. Le bac à sable d'ESC-323 ne suffit pas pour un tiers (§8.5.4).
- **Le store ne lit que le dossier du plugin** : `path = "../../crates/…"` n'y existe pas. Le kit doit sortir sur crates.io (§8.6.3), ce qui est une publication, à accepter.

### 8.1 L'inventaire : ce que chaque plugin fait hors du protocole

Relevé sur `main` au 2026-11-04, et sur `ticket/esc-226` (c291625b) pour Claude Code, le code étant celui de ce ticket en cours. « Rien » veut dire : aucun process, aucun fichier, aucun fil, aucun minuteur.

**Le socle commun.**

| Pièce | Ce qu'elle fait hors du protocole | Où |
| --- | --- | --- |
| `escouade-plugin-kit` | tokio à 2 fils, une tâche par appel | `crates/escouade-plugin-kit/src/serve.rs:150-222`, `:225-229` |
| | reqwest, proxy et TLS lus dans l'environnement (`ESCOUADE_PROXY`, `ESCOUADE_NO_PROXY`, `ESCOUADE_INSECURE_TLS`) | `http.rs:22-41` |
| | la langue lue dans l'environnement (`ESCOUADE_LANG`) | `i18n.rs:47` |
| `escouade-plugin-protocol` | rien : `serve` prend un `BufRead` et un `Write` | `src/lib.rs:133` |
| `escouade-acp` | lit et écrit les fichiers de l'agent (`fs/read_text_file`, `fs/write_text_file`) | `crates/escouade-acp/src/agent.rs:1195`, `:1227-1231`, `:1551` |
| | résout les chemins, lit `.git`, `gitdir`, `commondir`, le dossier temporaire | `git.rs:340-348`, `:383-393` |
| | le plan de Gemini, par le chemin que Gemini nomme | `gemini.rs:76` |
| | l'heure | `agent.rs:215` |

**Les hébergeurs et les sources.** Tous passent par `net/fetch` sans nouveauté de fond. Le minuteur est leur seule capacité en plus.

| Plugin | Hôtes (`permissions.network`) | Hors du protocole | Où |
| --- | --- | --- | --- |
| `dev.escouade.linear` | `api.linear.app` | rien | `plugins/linear/src/lib.rs` |
| `dev.escouade.trello` | `api.trello.com` | rien | `plugins/trello/src/lib.rs` |
| `dev.escouade.jira` | `*.atlassian.net` | rien ; le site est tapé par l'utilisateur | `plugins/jira/src/lib.rs` |
| `dev.escouade.clickup` | `api.clickup.com` | une attente de limite d'appels, l'heure | `src/lib.rs:84`, `:692` |
| `dev.escouade.github` | `api.github.com`, `*.ghe.com` ; toute instance Server | une pause par limite d'appels, l'heure | `src/pulls.rs:160`, `:107` |
| `dev.escouade.gitlab` | `gitlab.com` ; toute instance | une pause, l'heure | `src/merges.rs:317`, `:62` |
| `dev.escouade.bitbucket` | `api.bitbucket.org` | une pause, l'heure | `src/pulls.rs:172`, `:93` |
| `dev.escouade.gitea` | `codeberg.org`, `gitea.com` ; toute instance | rien | `src/pulls.rs` |
| `dev.escouade.azure-devops` | `dev.azure.com`, `*.dev.azure.com`, `*.visualstudio.com` ; toute instance Server | une pause, l'heure | `src/pulls.rs:347`, `:46` |

Le jeton de la CLI (`gh auth token`, `glab auth status --show-token`, `az account get-access-token`) est déjà demandé par le cœur (`src-tauri/src/forge/github.rs:87`, `gitlab.rs:88`, `azure.rs:203`) : **aucun hébergeur ne lance de process**.

**Les harness.** Le cœur lance tous les CLI (`engine/launch`, `check_launch`) : aucun plugin de harness ne lance son harness.

| Plugin | Hors du protocole | Où |
| --- | --- | --- |
| `dev.escouade.codex` | `std::process::id()` (qui **plante** sur WASI, §8.9) | `src/client.rs:167` |
| | `dirs::home_dir()`, lit `config.toml` du dossier du compte | `src/client.rs:204`, `:212` |
| `dev.escouade.copilot-cli` | `std::process::id()` | `src/client.rs:183` |
| `dev.escouade.cursor-cli`, `dev.escouade.gemini-cli`, `dev.escouade.acp` | rien d'eux-mêmes : tout est dans `escouade-acp` ; `ESCOUADE_VERSION` lu dans l'environnement | `plugins/acp/src/lib.rs:76`, `:87` |
| `dev.escouade.mistral` | Mistral Vibe : comme les précédents ; sa voix : comme la voix, plus bas | `src/lib.rs` |
| `dev.escouade.antigravity-cli` | lit `settings.json`, le fichier MCP, le plan, ce qui est toujours permis | `src/client.rs:179`, `:266`, `:503` ; `src/words.rs:171` |
| | **attend un fichier en boucle dans un tour** : le transcript, 40 ms entre deux lectures, 600 ms au plus | `src/client.rs:283-298`, `:30` |
| | lit des variables et `dirs::home_dir()` pour le dossier de l'agent | `src/words.rs:546-548` |
| `dev.escouade.opencode` | tokio à 2 fils, 3 tâches par session | `src/lib.rs:78-80`, `:172`, `:263` ; `src/session.rs:274`, `:299`, `:717` |
| | joint `opencode serve` sur `127.0.0.1`, le port lu dans ce que le serveur écrit | `src/lib.rs:324` ; `src/server.rs:49`, `:279` |
| | **un flux SSE sans fin** (`/global/event`), relancé 200 ms après une coupure | `src/server.rs:228-248` |
| | des appels HTTP bornés par un délai | `src/session.rs:766`, `src/server.rs:77` |
| `dev.escouade.claude-code` (ESC-226) | son client et sa traduction : rien, comme Codex | `plugins/claude/src/client.rs`, `translate.rs` |
| | `signin.rs` lit `.credentials.json` et lance `/usr/bin/security` (macOS), en tokio : du code **que le cœur appelle en bibliothèque**, pas le plugin | `src/signin.rs:110`, `:180` |
| `dev.escouade.minimal` | le plugin écrit un fichier d'essai ; son faux harness a un fil et dort | `src-tauri/src/bin/escouade-plugins/minimal.rs:390`, `minimal_harness.rs:53`, `:206` |

**Les services de voix.** `dev.escouade.openai`, `.groq`, `.elevenlabs`, `.deepgram` et la voix de `.mistral` n'ont **pas de crate** : leur programme est `escouade_lib::voice_plugin`, le crate de l'app tout entier (`src-tauri/src/bin/escouade-plugins/voice.rs:7`, `src-tauri/src/lib.rs:414`). Le code est dans `src-tauri/src/voice/`.

| Besoin | Où |
| --- | --- |
| un fil par requête (`std::thread::spawn`), un runtime tokio à chaque appel | `src-tauri/src/voice/plugin_guest.rs:190`, `:209` |
| WebSocket `wss` (`tokio-tungstenite`), TLS et tunnel de proxy faits à la main, en-têtes d'authentification | `voice/live.rs:31`, `:647-692` |
| le son en direct : un morceau toutes les 100 ms, en base64 sur `voice/live/audio` | `voice/live.rs:38`, `voice/wire.rs:15`, `:65` |
| envoi d'un WAV en multipart ou brut : **un corps binaire** | `voice/online.rs:316`, `:353-385` |
| réponses en flux : SSE base64 (Mistral), PCM brut (ElevenLabs), lues par morceaux | `voice/online_speech.rs:121`, `:306`, `:422` |

**isola.** C'est **le seul plugin qui lance un programme lui-même** : `isola ls --json`, `isola down`, `isola destroy`, 60 s au plus, ses deux flux vidés par deux fils, tué au-delà (`src-tauri/src/bin/escouade-plugins/isola.rs:121-168`). `isola up` est lancé par le cœur (`contributes.launchEnvironments`). Docker n'est pas appelé par le plugin : c'est `isola` qui le fait.

**Ce qui revient partout.** L'environnement (`ESCOUADE_LANG`, le proxy, la version) doit devenir une donnée d'`initialize`, WASI n'ayant pas d'environnement ; `std::process::id()` plante ; `dirs::home_dir()` rend `None`. Trois points, un seul ticket (A4 du plan).

### 8.2 Ce que le cœur fait déjà, et ce qui lui reste

**Déjà au cœur, et qui y reste :**

- trouver un CLI (`locate`), décrire sa version, le lancer dans son Job object, sous WSL au besoin (`plugins/harness.rs:866`) ; vérifier sa description de lancement (`check_launch`, `:633`) ;
- le PTY de la vue terminal : ni ses octets ni son écran ne passent par un plugin (`terminal/launch`, `terminal/hook`) ;
- lire ce que le harness écrit et le lui passer par lots de 64 Kio (`agent/frames`), écrire ce que le plugin demande (`writes`), appliquer les éléments ;
- écrire les fichiers d'un lancement dans le dossier de l'agent (`Launch::files`, `check_launch:677-702`) ;
- le processus partagé d'OpenCode (`Launch::shared`, `plugins/shared.rs`) ;
- le coffre (`secrets/get`, `plugins/runtime.rs:1587`), les jetons des CLI, la connexion des comptes ;
- le serveur MCP d'Escouade, les hooks, les autorisations, le journal, les redémarrages ;
- la voix locale (Whisper, Piper, la capture du micro) : ce n'est pas externe (§4.2).

**Ce qui passe au cœur avec le WASM.** Ces trois points ne peuvent pas rester dans un module :

- `signin.rs` de Claude Code, pour ses lectures (`.credentials.json`, le trousseau de macOS) : le cœur les fait déjà en l'appelant en bibliothèque. Sa règle pure peut rester dans la crate, ses entrées-sorties vont au cœur. ESC-226 a déjà séparé les deux (c291625b).
- le jugement de la clôture des projets privés sur les fichiers, que `escouade-acp` tient aujourd'hui lui-même (`paths.rs`) : avec `fs/*`, c'est l'hôte qui le tient (§8.4.6). C'est plus sûr qu'aujourd'hui.
- le tunnel de proxy et le TLS du WebSocket, aujourd'hui dans `voice/live.rs` : l'hôte les fait pour tous (§8.4.4).

**Ce qui est sorti du cœur.** Le cœur liait des crates de plugins **en bibliothèque**, pour ses « Vue X » et ses comptes : un correctif publié au store n'aurait pas corrigé la vue, restée à la version de l'app. C'est fini (P8, §8.30) : les vues passent par `view/translate`, le cœur lisant le fichier de la session et le plugin le traduisant, et `src-tauri/Cargo.toml` ne dépend plus d'aucun dossier de `plugins/`. Le cœur garde les crates de `crates/` (le protocole, le kit, `escouade-acp`, `escouade-voice`), qui ne sont pas des plugins.

### 8.3 Le modèle d'exécution

**Une instance par plugin, sur un fil, réveillée par des événements.** C'est le modèle de `run.js` (§5.4), étendu sur un point : les réponses lentes ne bloquent plus le fil.

- **Le module est un réacteur** : `crate-type = ["cdylib"]`, pas de `_start`, et l'export `escouade-turn`, le même nom que pour `run.js`. Il garde son état entre deux tours. Vu en le branchant (ESC-331) : rustc 1.98 n'exporte pas `_initialize` d'un cdylib `wasm32-wasip1`, ni dans le prototype ni dans le module de test ; l'hôte l'appelle s'il est là, sinon il passe au premier tour.
- **Le fil attend sur un seul canal** ce qui peut arriver : une ligne du cœur, la fin d'un `net/fetch`, un morceau de flux, un message de WebSocket, la sortie d'un programme, un minuteur. Il écrit tout ce qui est arrivé sur l'entrée du module, puis lance **un** tour, puis sert ce que le module a écrit. Plusieurs événements font un seul tour.
- **Rien n'attend dans un tour.** Une demande à l'hôte est une ligne ; sa réponse arrive au tour suivant. Une tâche Rust qui attend rend la main (`Poll::Pending`) ; `escouade-plugin-kit` a un exécuteur d'une centaine de lignes, sans tokio, qui sonde ses tâches à chaque tour, puis celles qu'un réveil a relancées (`crates/escouade-plugin-kit/src/serve/tasks.rs`, §8.29). Un harness, machine à états sans attente, garde son `Plugin::handle` tel quel (`crates/escouade-plugin-protocol/src/turn.rs`).
- **Deux portes vers l'hôte :**
  - **les lignes JSON-RPC**, pour tout ce qui dure : c'est la voie normale, la même qu'en `run.js`, le même journal ;
  - **un appel synchrone** (`escouade.call`, un import du module, requête et réponse en JSON), pour ce qu'une machine à états doit avoir avant de répondre, sans changer sa forme : les fichiers (§8.4.6). Il est servi sur le fil du plugin et compte dans le temps du tour.
- **Les limites** restent celles du §5.4 : 64 Mio, 2 s par tour, 500 ms pour une décision, le même budget de redémarrages. Le temps d'attente du réseau n'est pas compté : il n'est pas dans un tour. Une instance qui dépasse est finie et refaite ; ce qu'elle tenait (les agents attachés) lui est redonné par `agent/attach`, comme après un redémarrage d'aujourd'hui.
- **Ce que cela change à `wasm.rs`** : `serve` reçoit ses événements d'un canal au lieu de `rx` et sert `net/fetch` à côté, sur le runtime de l'hôte. Aujourd'hui, un `fetch` lent bloque son plugin (`wasm.rs:1184`) ; pour Linear ou GitHub, qui font plusieurs appels à la fois, ce serait une régression. Le test `a_slow_fetch_holds_back_neither_the_next_nor_the_thread` prouve le nouveau modèle : un appel tenu 400 ms laisse passer le suivant.

**Pourquoi pas un `main` qui bloque sur son entrée.** Un programme Rust ordinaire aurait gardé `serve` tel quel, l'hôte bloquant dans `fd_read`. Mais le temps du tour n'a alors plus de bord, l'hôte ne sert plus `net/fetch` depuis le même fil, et `run.js` et `run.wasm` auraient deux hôtes. Le tour garde **un seul hôte pour les deux** : seule la première ligne change (`_start` ou `_initialize`). Le prix : trois lignes dans le `run()` de chaque harness.

### 8.4 Les capacités de l'hôte

Sur le modèle de `net/fetch` : une méthode du protocole, déclarée au manifeste, appliquée par l'hôte, dite dans la fenêtre d'installation et gardée au journal. **Ce que le manifeste ne déclare pas, le module ne peut pas l'atteindre**, l'import n'étant servi qu'avec la permission. Une capacité non déclarée répond `-32003`.

Les « coûts » sont ceux du prototype quand ils ont été mesurés, sinon une estimation dite comme telle.

#### 8.4.1 Le réseau : `net/fetch` à revoir

`net/fetch` existe, mais quatre de ses règles empêchent la première main d'y passer. Sans elles, personne ne migre.

| Ce qui bloque | Qui | Ce qui est proposé |
| --- | --- | --- |
| **Une instance auto-hébergée** : GitHub Enterprise Server, GitLab, Gitea, Azure DevOps Server, Jira. Son hôte n'est pas au manifeste, et `forbidden` refuse tout hôte de réseau local (`wasm.rs:839`). | 5 plugins | `"network": ["{account}"]` : l'hôte de l'adresse **que l'utilisateur a saisie** pour ce compte, et lui seul. Il est dit dans la fenêtre et sur le compte. L'interdiction du réseau local et de http saute pour cet hôte seul : c'est l'utilisateur qui l'a nommé. |
| **Réglages › Réseau** : `net/fetch` ignore le proxy (`wasm.rs:964`), alors que le kit le suit (`http.rs:32`). Un poste derrière un proxy d'entreprise perdrait tous ses plugins. | tous | L'hôte suit le proxy réglé, comme le kit aujourd'hui. Avec un proxy, les adresses résolues ne peuvent pas être jugées : seul le nom l'est, et le journal le dit. `ESCOUADE_INSECURE_TLS` passe à l'hôte. Revoit la décision 32. |
| **Un corps binaire** : `body` est du texte, la réponse passe par `from_utf8_lossy` (`wasm.rs:1090`). | voix | `bodyBase64` dans les deux sens. |
| **10 s et 1 Mio** : une transcription lit plus longtemps, un WAV pèse plus. | voix, journaux de CI | Des limites par plugin, déclarées : `"limits": { "fetchSeconds": 120, "fetchBytes": 33554432 }`, avec un plafond fixé par l'hôte (120 s, 32 Mio). |

**Texte de la fenêtre.** Inchangé pour un hôte ; pour `{account}` : « Se connecte aux instances que tu lui donnes » / « Connects to the instances you give it ».
**Coût.** Mesuré sur Linear, deux `fetch` et un secret par appel : 677 appels/s contre 795 en natif, 99e centile 2,35 ms contre 2,03 ms. Négligeable devant une vraie API, qui répond en centaines de millisecondes.
**Surface d'attaque.** `{account}` permet à un plugin de joindre une adresse du réseau local, mais seulement celle que l'utilisateur a tapée. Avec un proxy, un DNS menteur n'est plus jugé par Escouade ; le proxy l'est par l'entreprise.

#### 8.4.2 Joindre un serveur local que le cœur a lancé

Pour OpenCode seulement : `opencode serve` écoute sur `127.0.0.1`, sur un port qu'il choisit.

- **Déclaration** : `"network": ["local:shared"]`, et `Launch::shared.listens`, le gabarit de la ligne où le serveur dit son adresse (`"opencode server listening on http://{host}:{port}"`).
- **Ce qu'Escouade applique** : `net/fetch` et `net/stream` en **http** vers `127.0.0.1:<port>`, seulement si `<port>` a été lu par le cœur dans la sortie d'un processus partagé que le cœur a lancé pour ce plugin, et tant que ce processus vit. Le plugin ne nomme jamais un port que le cœur n'a pas lu. Tout autre port local reste refusé.
- **Texte** : « Parle au serveur local du harness, lancé par Escouade » / « Talks to the harness's local server, started by Escouade ».
- **Coût** : aucun de plus que `net/fetch`.
- **Surface d'attaque** : seulement ce serveur. Un harness qui écrirait une fausse ligne d'adresse pourrait désigner un autre port local : le harness est déjà un programme de confiance, lancé par le cœur, et le gabarit n'accepte que `127.0.0.1`.

#### 8.4.3 Les réponses en flux et le SSE

- **Déclaration** : aucune de plus que `network` (ou `local:shared`) : le flux suit les mêmes hôtes.
- **Méthodes** : `net/stream { url, method, headers, body }` → `{ stream, status, headers }`, puis `net/chunk { stream, data | dataBase64 }`, `net/end { stream, error? }` ; `net/credit { stream, bytes }` et `net/cancel { stream }` du plugin vers l'hôte. Le découpage du SSE reste au plugin (`block_end`, `sse_data` d'OpenCode ne changent pas).
- **Ce qu'Escouade applique** : les règles de `net/fetch` à l'ouverture ; **un crédit** : l'hôte ne lit plus la socket au-delà de 256 Kio non consommés, donc un plugin lent ralentit le serveur et non la mémoire de l'app ; 8 flux par plugin ; pas de limite de durée, mais 120 s sans octet ferment un flux de réponse, jamais un flux déclaré `"sse": true`.
- **Texte** : aucun de plus.
- **Coût** : une ligne JSON par morceau, un tour par lot d'événements ; le prototype ne l'a pas mesuré. Estimé par le débit du §8.9 (14 Mo/s en lignes isolées, bien plus en lots), très au-dessus d'un flux de harness ou de voix.
- **Surface d'attaque** : un flux ouvert tient une connexion. D'où les 8 flux et le crédit.

#### 8.4.4 Le WebSocket

- **Déclaration** : `network` (les mêmes hôtes) ; `wss` seulement.
- **Méthodes** : `ws/open { url, headers }` → `{ socket }`, `ws/send { socket, text | dataBase64 }`, `ws/close { socket }` ; `ws/message { socket, text | dataBase64 }`, `ws/closed { socket, code, reason }`.
- **Ce qu'Escouade applique** : l'hôte de `network`, jugé à l'ouverture comme un `fetch`, le proxy et le TLS de Réglages › Réseau ; 4 sockets par plugin ; 1 Mio par message ; une file bornée à 256 Kio, au-delà de laquelle `ws/send` est refusé. Les en-têtes (la clé du service) ne vont pas au journal.
- **Texte** : aucun de plus que « Se connecte à ».
- **Coût** : un morceau de 100 ms à 16 kHz fait 3,2 Ko, 4,3 Ko en base64. C'est le cas le plus serré de tout le chantier : le délai de bout en bout est à mesurer dans le ticket de la voix (P6) contre le « moins de 1 ms ajoutée » du §4.13. Un tour coûte 18 µs en médiane, 72 µs au 99e centile, traduction comprise (§8.9) : rien n'annonce de problème, mais rien ne l'a prouvé.
- **Surface d'attaque** : une connexion longue vers un hôte déclaré, comme le SSE.

#### 8.4.5 Les minuteurs

- **Déclaration** : aucune. Comme `log`, c'est un droit de base, borné.
- **Méthodes** : `timer/set { ms }` → `{ timer }`, `timer/cancel { timer }` ; `timer/fired { timer }`. En JS, le prélude en fait `setTimeout` ; en Rust, le kit en fait `sleep`, `timeout`.
- **Ce qu'Escouade applique** : 10 ms au moins, 64 minuteurs par plugin. Coupés avec l'instance.
- **Texte** : aucun.
- **Coût** : un réveil de tour, quelques dizaines de microsecondes. Un plugin arrêté n'en a aucun.
- **Surface d'attaque** : un plugin qui se réveille sans cesse consomme du CPU, borné à 100 tours par seconde par le plancher de 10 ms. Il n'y a pas d'heure cachée de plus que WASI.

#### 8.4.6 Les fichiers d'un dossier déclaré

- **Déclaration** : `"permissions": { "files": [{ "root": "agent", "access": "write" }, { "root": "agentHome", "access": "read" }] }`. Les racines sont une liste fermée, résolues par l'hôte : `plugin` (son dossier, ce qui existe), `agent` (le dossier de travail de l'agent), `agentRepo` (son dépôt git, pour un worktree), `agentHome` (le dossier du compte de l'agent), `temp` (un dossier à lui). Aucun chemin libre, aucun `~`.
- **Méthodes**, par l'appel synchrone : `fs/read`, `fs/write`, `fs/stat`, `fs/list`, `fs/canon`. Essayé dans le prototype : `fs/read`.
- **Pourquoi synchrone, et pas des préouvertures WASI** : un agent naît après l'instance, et WASI fixe ses dossiers à l'instanciation ; une machine à états lit au milieu d'un calcul (`agent.rs:1195`) ; et les chemins sont ceux de l'hôte (`C:\wt`), que WASI ne sait pas nommer. La voie asynchrone aurait forcé à réécrire ces lectures en attentes.
- **Ce qu'Escouade applique** : le chemin résolu, liens compris, doit être sous une racine de cet agent-là ; la clôture du projet privé est appliquée **ici, par l'hôte**, quel que soit le code du plugin ; 256 Kio au plus en lecture sans `limits`, 4 Mio au plus ; le journal garde chaque écriture et chaque refus. Un test du prototype : `config.toml` lu dans le dossier déclaré, refusé dans un autre.
- **Texte** : « Lit les fichiers de tes agents » / « Reads your agents' files » ; « Écrit dans les dossiers de tes agents » / « Writes in your agents' folders », en orange ; « Lit le dossier de compte de tes agents » / « Reads your agents' account folder ».
- **Coût** : un aller-retour dans la mémoire du module, puis la lecture elle-même ; rien de mesurable devant un disque.
- **Surface d'attaque** : `agentHome` contient les identifiants d'un harness (`.credentials.json`, `auth.json`). Un plugin de harness a déjà ce pouvoir aujourd'hui, en mieux. Ici il est déclaré ; à la revue, ce droit se justifie.

**Antigravity** gardait une attente de 600 ms dans un tour : réécrite avec un minuteur (§8.24) : `frames` répond sans le transcript, puis un `agent/items` envoie la suite quand il arrive. C'est le seul harness à changer de forme.

#### 8.4.7 Lancer un programme nommé

Pour isola seulement, à ce jour, et pour les tiers.

- **Déclaration** : `permissions.commands`, qui existe : `["isola ls --json", "isola down", "isola destroy"]`. Le premier mot est un programme **que le cœur trouve** (`locate`), jamais un chemin.
- **Méthodes** : `proc/spawn { command, args, dir, agent, project, env }` → `{ proc }`, `proc/write { proc, data, close }`, `proc/kill { proc }` ; `proc/out { proc, stream, data }`, `proc/exit { proc, code }`.
- **Ce qu'Escouade applique** :
  - la commande et ses arguments doivent **commencer par** une entrée déclarée, mot pour mot, le reste n'étant accepté que si l'entrée finit par `*` ;
  - `dir` est une racine (`agent`, `project`), jamais un chemin libre ; un projet privé la refuse sans activation explicite ;
  - l'environnement est celui d'un agent, sans les secrets d'Escouade, plus les seules variables que le plugin nomme ;
  - le programme tourne dans le Job object du plugin : il meurt avec lui, ou à `proc/kill` ; 4 à la fois, sortie bornée par un crédit ; pas de shell ;
  - chaque lancement va au journal, avec ses arguments, secrets masqués.
- **Texte** : une ligne par commande, comme les hôtes de « Se connecte à », pour pouvoir en retirer une : « Lance cette commande sur ton ordinateur » / « Runs this command on your computer », puis la commande, en rouge, et appliqué.
- **Coût** : celui d'un process.
- **Surface d'attaque** : la plus forte de toutes. **Le module est isolé, le programme qu'il lance ne l'est pas.** `isola` a les droits de l'utilisateur. Une entrée en `*` sur un programme généraliste (`git *`, `docker *`) rouvre tout (`git -c core.sshCommand=…`). Le store refuse `*` pour un tiers vérifié, sauf justification relue.

#### 8.4.8 Ce que le module reçoit à son démarrage

`initialize` porte en plus ce que l'environnement portait : `lang` (ajouté par ESC-334, pour tout plugin), `escouade.version` (déjà là), et pour la première main rien du réseau, l'hôte le gardant. Le dossier de l'agent et le dossier temporaire arrivent dans `agent/attach`. L'identifiant d'une instance vient de WASI (`random_get`, `turn::instance_id`), `std::process::id()` étant remplacé dans Codex (fait) et dans Copilot (P3).

### 8.5 `run.wasm` : du Rust compilé par le store

#### 8.5.1 La cible : `wasm32-wasip1`

**Retenu : `wasm32-wasip1`, un module cœur en réacteur.** Rejeté pour l'instant : `wasm32-wasip2` et le modèle de composants.

- **Un seul hôte.** Javy sort un module cœur WASI p1 : `run.js` et `run.wasm` partagent `Instance`, `serve`, les limites, le journal.
- **WASI p2 n'apporterait rien ici.** Ses `wasi:http` et `wasi:sockets` sont ce qu'Escouade ne veut pas donner à un module : le réseau passe par `net/*`, jugé par l'hôte. Le contrat reste le JSON-RPC, qu'un WIT ne remplacerait qu'à grand prix pour trente méthodes déjà écrites et testées.
- **Le composant ne coûte pas cher en Rust, mais il ne sert à rien ici** : Linear compile aussi en `wasm32-wasip2`, 487 155 o contre 459 364 o, sans changer de source. Il demanderait la partie composants de wasmtime et un second chemin dans l'hôte, pour rien de plus. Sa reproductibilité n'a pas été essayée. En JS, ESC-319 avait vu un composant 10 fois plus lourd (§5.3).
- **Le risque** : WASI p1 est figé. wasmtime le sert et l'app épingle wasmtime (=50.0.1) ; `wasm32-wasip1` est une cible de niveau 2 de Rust. Si elle disparaissait un jour, la voie serait `wasi-sdk` et un adaptateur p1 → p2, sans toucher aux plugins.

**Le manifeste.** `"run": { "wasm": "Cargo.toml" }` : le manifeste Cargo du plugin, jamais un binaire. Le `.wasm` n'est pas commité. Rust seulement pour commencer ; le nom laisse la place à d'autres langages.

#### 8.5.2 La chaîne d'outils, épinglée

| Pièce | Version | Comment elle est tenue |
| --- | --- | --- |
| rustc, cargo, std de `wasm32-wasip1` | 1.98.0 (la même que l'app) | les archives de static.rust-lang.org, épinglées par empreinte, dans un dossier à part : **pas de `rust-src`** (le prototype passait par rustup, profil `minimal`, §8.16) |
| les dépendances | `Cargo.lock` commité, `--locked`, sommes de contrôle de crates.io | |
| le profil | `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true` | imposé par l'outil (`--config`) ; le `[profile]` de l'auteur est ignoré |
| l'enveloppeur `rustc-canon` | embarqué dans `escouade-plugin`, qui se relance lui-même comme `RUSTC_WRAPPER` | son empreinte, celle de sa source, notée dans `version.json` |

**Trois pièges, trouvés en le faisant.** Deux compilations sur un même système donnaient déjà la même empreinte ; d'un système à l'autre, non (642 460 o contre 643 804 o pour Codex). Trois causes :

1. **`-C metadata`** : cargo y mêle le `host` de `rustc -vV` et le chemin relatif d'une dépendance, avec ses `\`. Ce sel entre dans chaque symbole, et l'ordre du code suit les symboles. `rustc-canon` le recalcule sur le seul nom du crate, sa version, ses fonctionnalités et sa cible.
2. **Les emplacements de panique** gardent le chemin de chaque source. `--remap-path-prefix` retire le début, pas les `\` que Windows met après (`src\de\mod.rs`). `rustc-canon` donne donc **une règle par fichier source**, de son chemin entier à son chemin canonique (`/src/…`, `/cargo/…`).
3. **`rust-src`** : quand ce composant est installé, rustc écrit les chemins de la bibliothèque standard là où il la trouve sur la machine. D'où un `RUSTUP_HOME` à part, en profil `minimal`. Un `rust-toolchain.toml` qui demande `clippy` casse aussi la compilation hors ligne.

Une source qui atteint un fichier par `#[path = "../../…"]` garde ses `..` dans son emplacement : le prototype copie donc ses sources (`prepare.mjs`), et un plugin n'en aura pas.

**Le résultat** (`sh reproducible.sh`, rustc 1.98.0, le même `Cargo.lock` de 67 paquets) :

| Compilation | `linear.wasm` | `codex.wasm` |
| --- | --- | --- |
| Windows 11, deux fois, dossiers `target` neufs | `5ae4aabb…` | `c5ab5a0a…` |
| Linux (image `rust:1.98.0-bookworm`, Podman), deux fois, sans réseau, `--read-only`, utilisateur 1000 | `5ae4aabb…` | `c5ab5a0a…` |
| Sans `rustc-canon` : Windows / Linux | `c488693b…` / `61a82e7e…` | `acff2243…` / `c1037cdc…` |

Empreintes complètes : `5ae4aabbeb48ce8d18ad8fd4181c5769b828c40fa808a684500afadbb5dddb18`, `c5ab5a0a044e731e81f39fbd6f864382c826bc05838dda926e4f0a5af198eb79`. **macOS n'a pas été essayé** : rien ne l'y distingue de Linux, mais il faudra le voir une fois, en CI. Un autre dossier de dépôt : un autre dossier de dépôt et un autre `CARGO_HOME` donnent aussi la même empreinte (essayé sous Linux, `/tmp/ailleurs/un/autre/depot`).

**Taille et temps** (machine chargée par d'autres agents) :

| | Linear | Codex |
| --- | --- | --- |
| Module | 459 364 o, 165 162 o en gzip -9 | 642 140 o, 223 483 o en gzip -9 |
| Natif équivalent | 3 693 568 o | 1 164 800 o |
| Compilation à froid (Linux, sans réseau, les deux modules et leurs 67 paquets) | 12 s | |
| Précompilation par Cranelift | 50 ms, 1,48 Mo | 93 ms, 1,96 Mo |
| Chargement du précompilé | 0,41 ms | 0,54 ms |

#### 8.5.3 Ce que fait le store, et l'image d'ESC-328

Un douzième contrôle n'est pas nécessaire : **le onzième, « Compilation », s'étend**. Pour un `run.wasm` :

1. **Récupérer** les dépendances : `cargo fetch --locked`, le réseau limité à `index.crates.io` et `static.crates.io`, sans exécuter une ligne du plugin. Essayé.
2. **Vérifier** avant de compiler (§8.5.4).
3. **Compiler deux fois** par `escouade-plugin build --json` (`compileTwice` d'aujourd'hui), chacune dans un bac à sable sans réseau. Deux empreintes refusent la version.
4. **Publier** le `.wasm` comme aujourd'hui : `wasm: { sha256, size, compiler: { rust: "1.98.0", target: "wasm32-wasip1", canon: "<empreinte>", lock: "<empreinte de Cargo.lock>" } }`.

**L'image** (ESC-328 prépare son étape Rust) doit garder dans sa dernière étape, en plus de ce que ce ticket y met : rustc, cargo, la std `wasm32-wasip1`, et `gcc` pour lier `build.rs` et macros, qui sont natifs. **678 Mo** mesurés pour le `RUSTUP_HOME`, plus environ 60 Mo de dépendances par `Cargo.lock`. La limite d'aujourd'hui (2 minutes, 1 Gio) doit monter : 1 Gio tue la compilation (`tmpfs` compris, dans le conteneur de l'essai), 1,5 Gio passe. **Proposé : 2 Gio et 5 minutes par plugin**.

#### 8.5.4 Ce qui change au store : du code natif de l'auteur s'exécute

**Javy n'exécute le plugin que dans WASM. Cargo, lui, exécute en natif chaque `build.rs` et chaque macro procédurale.** Les deux modules du prototype en ont déjà : `anyhow`, `num-traits`, `parking_lot_core`, `proc-macro2`, `quote`, `serde`, `serde_core`, `serde_json`, `zmij` pour les scripts, `serde_derive` pour les macros. La décision d'ESC-323 (décision 43) disait : « le code du plugin ne tourne que dans le bac à sable WASM ». Ce n'est plus vrai.

`unshare -rn` coupe le réseau, pas le reste : le sous-process tourne sous le même utilisateur que le store, dont il pourrait lire `/proc/<pid>/environ`, donc **les secrets**. Pour un tiers, il faut donc :

- **Un vrai bac à sable** : `bubblewrap` (`--unshare-all --clearenv --new-session --die-with-parent`), un espace de PID et de montage neufs, seuls la chaîne d'outils, les dépendances et le dossier du plugin en lecture seule. Sans espaces de noms utilisateur sur le serveur, `run.wasm` tiers reste refusé (« pas d'isolation : version en attente »). `STORE_COMPILE_ISOLATION=require` pour `run.wasm`, sans repli.
- **Une liste fermée**, en plus : pas de `build.rs` dans les crates du plugin ; pas de dépendance `git` ni `path` hors du dossier ; pas de `[patch]`, pas de `.cargo/config.toml`, pas de `rust-toolchain.toml` ; des dépendances de crates.io seulement ; un script ou une macro procédurale tiers seulement s'il est sur une liste relue (`serde_derive`, `thiserror-impl`, `proc-macro2`…), par nom, version et somme de contrôle.
- **`include_bytes!("/etc/passwd")`** devient inoffensif : le bac à sable ne voit rien d'autre.

Un plugin tiers qui passe tout cela est compilé ; la revue relit le reste. **La première main** a les mêmes règles : le store ne fait pas d'exception, la revue du chef est la même.

#### 8.5.5 Ce que fait la revue locale (ESC-324)

`npm run plugin-review` recompile déjà un `run.js` et compare. Pour un `run.wasm` :

- `recompile.ts` lance `escouade-plugin fetch` puis `build` et compare au `compiler` de `version.json` ; **une autre empreinte arrête tout**, comme aujourd'hui. Le poste du chef est sous Windows : c'est exactement l'essai ci-dessus. Il lui faut Visual Studio Build Tools pour lier `build.rs` et macros (déjà là pour l'app). La chaîne d'outils s'installe à son premier besoin dans `<données>/tools/rust-1.98.0/`.
- **Pour un tiers, cette recompilation se fait dans un conteneur** (`scripts/plugin-review/container.ts`, §8.20), jamais en natif : les `build.rs` ne tournent pas sur le poste du chef. `fetch` a le réseau et n'exécute rien du plugin ; `build` tourne en `--network=none --read-only`. Podman y est (STORE-7).
- Les pistes de lecture : `Cargo.lock` en plus, dépendance par dépendance, et son diff avec la version d'avant, le code de `unsafe`, les `extern`, `#[link]`, les inclusions qui sortent du dossier, les imports du module comparés aux capacités.
- Le banc joue le module publié dans l'hôte WASM, hors ligne, comme un `run.js`.
- Une dépendance nouvelle se relit : ce que le lock ajoute est du code qui tourne.

### 8.6 Les dossiers, la CI et le store de la première main

#### 8.6.1 Les noms courts

Chaque plugin quitte `plugins/builtin/dev.escouade.<id>/` pour `plugins/<id-court>/`, comme `plugins/slack/`. Il y garde son manifeste, son README, sa LICENSE, son CHANGELOG et ses tests. **Les identifiants ne changent pas** : les comptes, les réglages et le trousseau en dépendent (`plugin:<id>:…`).

| Dossier | Identifiant | Programme | Note |
| --- | --- | --- | --- |
| `plugins/claude/` | `dev.escouade.claude-code` | `run.wasm` | l'exemple de l'utilisateur ; suit ESC-226 |
| `plugins/codex/` | `dev.escouade.codex` | `run.wasm` | prototype |
| `plugins/copilot/` | `dev.escouade.copilot-cli` | `run.wasm` | |
| `plugins/cursor/` | `dev.escouade.cursor-cli` | `run.wasm` | |
| `plugins/gemini/` | `dev.escouade.gemini-cli` | `run.wasm` | |
| `plugins/antigravity/` | `dev.escouade.antigravity-cli` | `run.wasm` | |
| `plugins/opencode/` | `dev.escouade.opencode` | `run.wasm` | |
| `plugins/acp/` | `dev.escouade.acp` | `run.wasm` | sert aussi `plugins/goose/` |
| `plugins/mistral/` | `dev.escouade.mistral` | `run.wasm` | la voix et Vibe |
| `plugins/openai/` | `dev.escouade.openai` | `run.wasm` | voix, crate à créer |
| `plugins/groq/` | `dev.escouade.groq` | `run.wasm` | voix, crate à créer |
| `plugins/elevenlabs/` | `dev.escouade.elevenlabs` | `run.wasm` | voix, crate à créer |
| `plugins/deepgram/` | `dev.escouade.deepgram` | `run.wasm` | voix, crate à créer |
| `plugins/github/` | `dev.escouade.github` | `run.wasm` | |
| `plugins/gitlab/` | `dev.escouade.gitlab` | `run.wasm` | |
| `plugins/bitbucket/` | `dev.escouade.bitbucket` | `run.wasm` | |
| `plugins/gitea/` | `dev.escouade.gitea` | `run.wasm` | |
| `plugins/azure-devops/` | `dev.escouade.azure-devops` | `run.wasm` | |
| `plugins/linear/` | `dev.escouade.linear` | `run.wasm` | prototype |
| `plugins/jira/` | `dev.escouade.jira` | `run.wasm` | |
| `plugins/trello/` | `dev.escouade.trello` | `run.wasm` | |
| `plugins/clickup/` | `dev.escouade.clickup` | `run.wasm` | |
| `plugins/isola/` | `dev.escouade.isola` | `run.wasm` | le seul à lancer un programme |

`dev.escouade.minimal` n'est pas un produit : il va dans `tests/fixtures/plugins/minimal/`, sans publication. `discord`, `goose`, `high-contrast`, `slack` et `team-rules` ne bougent pas. `plugins/builtin/` disparaît, et `plugins/README.md` avec sa section.

**Fait (ESC-339, 2026-11-05)** : les dossiers sont à leur place, sans changement de code ni d'identifiant. `plugins/ci.mjs` les vérifie depuis M1 (§8.6.4).

**Le nom du dossier n'est pas le dernier mot de l'identifiant** pour six d'entre eux : il sert seulement à l'étiquette, et le store relie un dossier à un id par son manifeste (§3.6 de `docs/STORE.md`).

**La licence** : les plugins déménagés restent en GPL-3.0-or-later, leur code venant du cœur ; `plugins/README.md`, qui dit MIT, doit le dire plugin par plugin. Chacun a sa `LICENSE` dans son dossier : le contrôle 5 la lit.

**Les versions** : chacun a la sienne, à partir de `2.2.0`, et un `CHANGELOG.md`. Son `escouade.min` est `2.2.0`. `manifest_text` ne remplace plus sa version par celle de l'app.

#### 8.6.2 Le passage par le store

Chacun passe **comme un tiers** :

1. l'étiquette `plugins/<id-court>/vX.Y.Z`, posée par le chef ;
2. la soumission en dossier, par le MCP du store (`submit_version`) ;
3. les contrôles, dont la compilation en WASM par le store ;
4. la publication, non vérifiée, en demandant la revue (`publish_version`) : le store ne relit et ne signe qu'une version publiée (`docs/STORE.md` §4.3), et l'app ne prend aucun `dev.escouade.*` sans signature ;
5. la revue locale, qui recompile et compare ; le rapport ;
6. la signature par le chef (`npm run plugin-review -- sign`, sa clé sans mot de passe : mémoire `first-party-plugin-signing`) : la version porte le sceau.

**La liste `BUILTINS` de `store/server/utils/builtin.ts`** est vide (ESC-353) : chacun est une vraie ligne de plugin, à sa place dans le catalogue, avec la marque « Escouade » (le préfixe la donne, et sa tuile : marque ou glyphe de sa catégorie) et ses versions. Il n'y a plus rien sur le store pour un plugin tant qu'il n'est pas publié. Le préfixe `dev.escouade.*` reste réservé au dépôt de l'app (`STORE_FIRST_PARTY_REPO`, décision 25) et l'app ne l'installe que signé.

Pour 23 plugins, ces gestes tiennent en une commande du chef (ESC-353, `store/scripts/plugins-publish/`) ; un agent de ticket ne la lance jamais :

```sh
npm run plugins:publish -- --changed        # tout dossier dont la version n'est pas encore signée au store
npm run plugins:publish -- claude trello    # ceux-là
```

- **Qui passe** : les dossiers de `plugins/` dont le manifeste porte un `dev.escouade.*`, un par un, dans l'ordre des noms. `--changed` laisse de côté une version `0.0.0` (pas encore portée), et nommer un tel dossier est refusé.
- **Avant tout** : l'arbre propre et le commit sur `origin/main`, sinon rien n'est fait (code 2).
- **Pour chaque plugin** : l'étiquette sur ce commit, poussée ; `submit_version` ; `get_version` jusqu'à la fin des contrôles ; `publish_version` avec `request_review`, et pour une première version les `store.categories` de son manifeste (Cursor CLI, Gemini CLI et Mistral : celles de l'outil, voir plus bas) ; `plugin-review <id>@<version>` ; puis, sur le seul verdict du rapport, `plugin-review report`, `plugin-review sign`, et la lecture du catalogue qui doit la dire vérifiée.
- **Il s'arrête** (code 1, avec la raison et ce qu'il reste à faire) au premier contrôle rouge, à une version retenue, à la limite de 20 soumissions par heure, à un rapport sans la ligne `**Verdict final : Vérifier**`, ou à un autre verdict : rien n'est déposé ni signé. L'outil n'écrit jamais ce verdict. C'est la lecture du chef (`/review-plugin`) qui le pose ; la lecture humaine reste celle des diffs.
- **Il reprend là** : il ne garde rien à lui. Ce qui est au catalogue, signé ou non, n'est ni étiqueté ni soumis de nouveau ; un rapport présent n'est pas récrit ; une version signée n'est pas refaite.
- **Un manifeste ne change pas pour ça** quand il entre dans le module : Cursor CLI, Gemini CLI et Mistral lisent le leur (`include_str!`, `harnesses_of`), et une clé `store` y change l'empreinte (+64 octets, mesuré). Leurs catégories restent donc dans l'outil (`EMBEDDED_CATEGORIES`), jusqu'à leur prochaine version. Les 21 autres modules les disent au manifeste, sans effet sur leur empreinte, et `goose` aussi. Un test tient cette règle.
- **Ce qui est déjà au store ne repart pas** : une version signée ou retirée au catalogue n'est pas touchée (`discord`, `high-contrast`, `slack`, `team-rules`, tant que leur version ne bouge pas) ; une version publiée non signée va à sa revue, sans nouvelle soumission. Seule la version compte : un dossier changé sans version nouvelle ne repart pas.
- **Il ne lit jamais la clé** : `sign` est `npm run plugin-review -- sign`, son propre process. Seul le jeton d'administration lui sert, celui du trousseau ou d'`ESCOUADE_STORE_TOKEN`, sur `/mcp`, limité à ces trois outils (`docs/STORE.md` §2.5). Aucun secret dans son journal.

#### 8.6.3 Le kit sort sur crates.io

Le store ne lit que le dossier du plugin (`docs/STORE.md` §3.6), à raison : c'est ce qu'il archive et ce que le chef relit. Or `path = "../../crates/escouade-plugin-kit"` sort du dossier.

**Retenu** : `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` sont publiés sur crates.io. Un plugin en dépend par version, son `Cargo.lock` en fixe la somme de contrôle. Dans le dépôt, un `[patch.crates-io]` à la racine du workspace qui tient les plugins (`src-tauri/Cargo.toml`) les ramène sur les chemins locaux pour le développement ; le store, qui ne voit pas cette racine, compile ce qui est publié. **Les plugins tiers en Rust ont besoin de ces crates de toute façon.**

Écarté : copier `crates/` dans chaque dossier (`vendor/`), des milliers de lignes dupliquées 23 fois ; ou faire lire `crates/` au store pour la seule première main, une exception à la règle « comme un tiers ». **Publier sur crates.io est un geste public et sans retour** (un numéro publié ne s'efface pas) : c'est au nom de l'utilisateur, à faire par lui ou sur son accord. **Accordé le 2026-11-04** : l'utilisateur a choisi crates.io et s'est connecté par `cargo login` sur le poste. Le chef publie les trois crates au ticket A9 ; aucun agent de ticket ne lance `cargo publish`.

#### 8.6.4 La CI de `plugins/`

`plugins/ci.mjs` connaît deux sortes de plugins, avec et sans `package.json`. Il en gagne une troisième, avec `Cargo.toml` :

- `cargo test` en natif : les tests unitaires ne changent pas ;
- `escouade-plugin build`, puis `validate` et `test --first-party` : le banc joue le module dans l'hôte de l'app ;
- `--changed` : seulement les dossiers touchés, plus tous quand `crates/` bouge.

**Fait (M1, ESC-340, 2026-11-05).** Pour chaque plugin de `plugins/` :

- sa `LICENSE` et son `CHANGELOG.md`, présents et non vides, quelle que soit sa sorte ;
- pour un crate, la version du manifeste face à celle de `Cargo.toml`, et `cargo test`, lancé une seule fois de `src-tauri/` sur tous les crates retenus (`-p` répétés, `--no-fail-fast`) : une compilation, et la chaîne d'outils épinglée de `src-tauri/rust-toolchain.toml`, qu'un `cargo` lancé du dossier du plugin ne verrait pas ;
- `validate`, en `--builtin` tant que le programme est `{escouade}/escouade-plugins`, y compris pour les quatre sans crate (voix) ;
- le banc pour les `run.js`, et pour un `run.wasm` : `build --out`, puis `test --wasm` sur ce module, pour ne pas le compiler deux fois. `build` compile un `run.wasm` depuis A5. Aucun plugin n'a de banc tant que `escouade-plugins` le sert.

**La version d'un plugin que l'app stampe.** Les 23 manifestes disent `0.0.0` et les crates `1.0.0` : `manifest_text` y met la version de l'app. La comparaison admet donc `0.0.0` tant que le programme est celui de l'app, et plus du tout dès qu'un plugin passe en `run.wasm` : c'est le portage qui lui donne sa version (§8.6.1).

**`--changed [<révision>]`** compare à la révision donnée, ou à l'endroit où la branche a quitté `main` (`origin/main`, sinon `main`), arbre de travail et fichiers nouveaux compris. Sont touchés les dossiers modifiés et ce qui en dépend par `path` (`mistral` sur `acp`). **Tous passent** si bouge ce qui juge ou construit tous les plugins : `crates/`, `schemas/`, `sdk/plugin/`, `src-tauri/src/plugins/`, `src-tauri/src/bin/escouade-plugin.rs`, `src-tauri/Cargo.lock`, `src-tauri/rust-toolchain.toml` et le script lui-même. Si git ne sait pas comparer (clone trop court), tous passent aussi : une branche ne doit pas être verte faute de test. La logique est dans `plugins/ci-lib.mjs`, testée par `scripts/plugins-ci.test.ts`.

**En CI**, le job Linux lance tout sur `main` et `--changed HEAD^1` sur une pull request, `fetch-depth: 2` donnant la fusion et sa base. Le job Windows ne le lance pas : les tests des crates y passent déjà (`default-members`), et le reste ne dépend pas du système ; la comparaison entre systèmes est le job reproductible ci-dessous, à poser quand un plugin sera en `run.wasm`.

Un job **reproductible** compare sous Windows et Linux, sur `main` : le même plugin, la même empreinte. Un écart est rouge. Un job de plus sur macOS le fait une fois par release.

Les tests de l'app (`forge/*_tests`, `plugins_harness_tests`, le banc de flux) lancent `escouade-plugins`, ou le module pour un plugin porté (§8.18), construit une fois par CI (`scripts/plugins-wasm.sh`), le `target` étant partagé : quelques minutes, pas 23 fois 12 s.

### 8.7 La distribution dans l'app

**Ce que l'utilisateur doit garder** : l'app marche dès son installation et hors ligne, et un harness n'attend aucun téléchargement.

**Retenu : les deux.** Une **amorce** dans l'app, puis le store.

- **L'amorce.** L'installeur embarque un dossier `plugins/` (les ressources de Tauri) : pour chaque plugin de première main, **ce que le store a publié pour sa dernière version** : `version.json`, `version.json.minisig`, l'archive du dossier et `plugin.wasm`. Rien de construit à part : les mêmes octets, la même signature.
- **Au premier lancement**, ou après une mise à jour de l'app, Escouade **installe de l'amorce ce que la configuration de l'utilisateur utilise**, par le chemin d'une installation du store, sans réseau : mêmes vérifications, mêmes dossiers, origine `store`, une marque `seed`. La précompilation (50 à 93 ms par module, soit deux secondes environ pour tous sur un cœur) se fait à la première activation de chacun, hors du fil de la fenêtre, pas au démarrage.
- **Le tri** (décidé par l'utilisateur le 2026-11-07, « je veux un tri », ESC-373). Il se lit dans les réglages, les projets, les agents et les comptes, jamais sur le réseau (`plugins/sort.rs`) :
  - **un harness** (Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, Antigravity CLI, OpenCode, Mistral Vibe, Goose ; `dev.escouade.acp` pour un harness décrit) : un compte de ce harness dans Réglages › Comptes, un agent sur lui, le harness par défaut d'un projet ou celui des textes rédigés ;
  - **une source de tickets ou un hébergeur** (Trello, ClickUp, Linear, Jira, GitHub, GitLab, Bitbucket, Gitea, Azure DevOps) : un projet qui l'a pour source, un projet dont un remote est chez lui (lu dans `.git/config`, par le nom de l'hôte ou par le compte connecté pour cette instance), un compte de ce service déjà enregistré (sauf s'il est servi par le plugin d'un autre auteur) ;
  - **un service de voix** (Deepgram, ElevenLabs, OpenAI, Groq, Mistral) : choisi « En ligne » pour la dictée ou pour la voix, ou dont une clé est gardée (sa présence, jamais sa valeur) ;
  - **isola** : un projet (ou l'un de ses dépôts) avec un `.isola.toml` ;
  - **Discord, Slack, Contraste élevé, Règles de l'équipe** : ce qui est installé le reste ; sinon, proposés.
  - **Ce qui ne se lit pas sûrement s'installe** : un fichier de réglages, d'état ou de comptes qui ne se lit pas installe toute l'amorce ; un projet hors d'atteinte (disque non monté) ou dont la configuration git ne se lit pas veut tous les hébergeurs et isola ; un identifiant de l'amorce que la règle ne connaît pas s'installe. Le tri ne fait jamais perdre une fonction de la 2.1.
- **Une mise à jour depuis la 2.1.** Une entrée `builtin` que le tri désigne prend la version de l'amorce, ses choix gardés ; une que le tri ne désigne pas est retirée proprement, ses choix gardés comme « Garder ses réglages » les garde, ses secrets laissés au trousseau, et proposée. Elle n'est pas notée « désinstallée » : l'utilisateur ne l'a pas retirée.
- **Une première installation** n'installe rien d'office. L'accueil propose les harness ; « Ajouter un compte… » sur l'un d'eux installe son plugin depuis l'amorce, sans réseau, puis ajoute le compte.
- **Le geste qui demande un plugin absent** l'installe depuis l'amorce, sans réseau, et le dit dans une ligne (« Plugin Codex installé ») : ajouter un compte d'un harness, choisir une source de tickets ou un harness par défaut pour un projet, ajouter un projet (l'hébergeur de ses remotes, isola), connecter un compte d'une source ou d'un hébergeur, choisir un service de voix « En ligne » ou lui donner sa clé, préparer ou lancer le test d'un worktree avec un `.isola.toml`. Le geste vaut accord : pas de fenêtre de plus. C'est `Core::need_plugin`. Un plugin désinstallé par l'utilisateur n'est remis que sur un tel geste, jamais au lancement.
- **Les listes de choix** (harness d'un compte, source de tickets, hébergeur, service de voix) gardent tous les choix de l'amorce, installés ou non : leurs registres sont ceux du cœur. La vue Plugins montre ceux de l'amorce non installés (« Proposés par Escouade », « Dans l'app, sans téléchargement »), installables d'un clic.
- **Ensuite, le store.** Les plugins de première main se mettent à jour par lui, sans attendre une release de l'app, comme tout plugin : la mise à jour est proposée, jamais faite seule (voir plus bas). L'amorce n'installe que ce qui n'est pas installé, et que le tri désigne. Une version de l'amorce plus récente que celle installée est proposée comme une mise à jour du store. On ne redescend jamais de version.
- **Sans réseau**, rien ne change : l'amorce, ou ce qui a été téléchargé, tourne. Le catalogue n'est lu que pour proposer. **Sans store** (panne, entreprise fermée), tout marche à la version de l'amorce.

**Le mode de mise à jour : celui de tout plugin.** Décidé par l'utilisateur le 2026-11-04 : « pas d'auto update, faut vraiment considérer les plugins portés depuis les builtins comme des plugins à part entière ». La règle du §3.2 (« jamais en silence ») vaut donc pour la première main comme pour les tiers : une nouvelle version est proposée dans la vue Plugins, et elle s'installe au clic de l'utilisateur, avec la même fenêtre quand ses permissions grandissent. Un agent en cours garde sa version jusqu'à son prochain lancement : un plugin de harness ne change pas sous un tour.

**Ce que l'app vérifie avant de lancer**, qu'il vienne de l'amorce ou du store :

1. la signature `minisign` de `version.json` contre les clés publiques embarquées (`store_keys.rs`), son commentaire de confiance (`escouade-verified-v1 <id>@<version>`) ;
2. l'empreinte de `plugin.wasm` et de l'archive contre `version.json` ;
3. `dev.escouade.*` : signé, sinon refusé, quelle que soit l'origine ;
4. la liste de révocation : celle du store si elle est là, sinon **celle de l'amorce**, gardée dans l'installeur, à laquelle les plugins livrés n'échappent plus ;
5. `escouade.min` et `max`.

Le précompilé reste local, jamais reçu : `Module::deserialize` fait confiance à ses octets.

**Une révocation qui frappe l'amorce** : un `dev.escouade.*` révoqué sans remplaçant laisse le plugin coupé, et sa raison dite. C'est le prix d'avoir une seule règle ; un correctif publié par le store, qui ne demande pas de release, est la parade.

**Ce que devient `dev.escouade.*`.** Il reste réservé. L'origine `builtin`, son `{escouade}/…`, `check_program`, `builtin:<version>` et la règle « l'identifiant de l'app l'emporte » disparaissent. Le registre de `plugins/builtin.rs` devient la seule liste des identifiants de l'amorce. Plugins à part entière (décision du 2026-11-04 plus haut), ceux de la première main **se coupent et se désinstallent** comme les autres. L'amorce ne réinstalle pas au lancement un plugin que l'utilisateur a désinstallé : elle le note, la vue Plugins le propose à nouveau, et le geste qui le demande le remet. L'app dit ce qui manque quand un harness ou une source n'a plus son plugin. Un plugin d'un autre auteur peut toujours prendre la place d'un service de voix, à son identifiant à lui.

**Les fichiers.** La liste est embarquée à la compilation (`plugins/seed.json`) ; les octets sont dans `src-tauri/seed/`, que Tauri copie dans les ressources (`<ressources>/seed/<id>/`). Rien de ce dossier n'est commité sauf son `README.md`.

**L'ordre de la release.** L'amorce fige des versions publiées : **le chef publie d'abord, la release ensuite**. `release.yml` télécharge l'amorce nommée par `plugins/seed.json` (un fichier commité : `id`, `version`, empreinte de `version.json`), vérifie chaque signature, et échoue sinon. Un build de développement n'a pas de signature : `debug_assertions` accepte l'amorce de `scripts/seed-dev.sh`, comme il accepte aujourd'hui un programme non signé.

**Le poids.** `escouade-plugins.exe` pèse **34,5 Mo** aujourd'hui (34 479 104 o, release de `ticket/esc-226`) et disparaît. L'amorce pèse à peu près 23 × 0,5 Mo, soit **11 à 14 Mo** en clair d'après les deux modules mesurés, moins dans l'installeur compressé (165 Ko et 223 Ko en gzip pour les deux mesurés). Les plugins qui ne servent pas ne se précompilent pas. Le précompilé, 1,5 à 2 Mo par module actif, va dans le cache. **Mesuré par Z2** (§8.32) : 26 plugins, 17,4 Mo en clair, **2,5 Mo de plus dans l'installeur** ; la précompilation de Claude Code à sa première activation, une centaine de millisecondes en release.

### 8.8 Les binaires natifs des tiers

Aucun n'est publié aujourd'hui : la décision ne casse rien.

- **Au store** : `run.command` est refusé à la soumission, pour tous, avec la raison (« un plugin se publie en `run.js` ou `run.wasm` : le store le compile pour tous les systèmes »). Le contrôle n° 10 perd ses attestations et ses binaires par système. `gh attestation verify` et `binaries/` sortent de la revue.
- **Dans l'app** : `run.command` n'est plus lancé, quelle que soit l'origine. Le manifeste reste lisible, et la fiche dit pourquoi le plugin ne démarre pas.
- **Pour un auteur** en Go ou en C : rien ne s'ouvre en 2.2, le compilateur devant être reproduit par le store. `run.wasm` reste Rust au début. Son besoin de lancer un programme passe par `permissions.commands`.
- **Les versions publiées qui se lancent par Node** (`slack` 1.1.0, `team-rules` 1.1.1) restent pour les apps d'avant : `escouade.max` les borne, et leur 2.0.0 prend le relais.
- **Un harness décrit** (`contributes.harnesses[].acp`, Goose) ne change pas : le cœur lance le CLI, et `dev.escouade.acp` le sert.

Le transport par process de `runtime.rs` part en dernier, avec ses tests (`tests/fixtures/fake-plugin.mjs`) : ticket Z du plan.

Le prix : **un outil d'un tiers qu'on ne peut pas compiler en WASM ne s'intègre que par un programme qu'il installe lui-même**, et que le plugin nomme. C'est le sens de la décision du 2026-11-04.

### 8.9 Le prototype et ses mesures

`prototypes/wasm-all/` n'est branché nulle part. Il contient :

- `guest/` : les modules. `linear` et `codex` compilent **les fichiers du dépôt** (`prepare.mjs` les copie), sur `kit` (les fichiers du vrai kit, sauf `serve`), `reqwest` (la part de reqwest dont ils se servent, sur `net/fetch`) et `escouade-turn` (le tour et l'appel synchrone) ;
- `rustc-canon/`, `reproducible.sh` : la compilation reproductible ;
- `host/` : l'hôte (`src/lib.rs`), les deux plugins natifs sur leurs vraies crates (`src/bin/`), les tests (`tests/parity.rs`) et le banc (`examples/measure.rs`).

**Ce qu'a coûté le portage de ces deux-là :**

| | Changé dans le code du plugin | Ce qui a changé |
| --- | --- | --- |
| Linear | 0 ligne | l'export du tour, ajouté ; `reqwest` remplacé par une façade de 230 lignes |
| Codex | 5 endroits | `run()` ; un `use` ; l'export du tour ; `std::process::id()` (qui plante sur WASI) ; `std::fs::read_to_string` remplacé par `fs_read` |
| Le kit | `serve.rs` réécrit (170 lignes, sans tokio) | tous ses autres fichiers tels quels |

**Les tests** (`cargo test --release`, 5 passés) : Linear répond **exactement** comme son natif à `account/me`, `tickets/issues`, `tickets/states` sur un faux Linear ; Codex traduit **les mêmes éléments** que son natif, ligne à ligne, sur 479 lignes de `full-turn.jsonl` et `plan-mode.jsonl` ; un hôte hors du manifeste est refusé ; un appel lent ne retient pas le suivant ; un fichier se lit dans le dossier déclaré et pas à côté.

**Les mesures**, faites comme INT-0, avec une différence : **elles s'arrêtent à la réponse du plugin**. INT-0 compte aussi ce que le cœur fait de la réponse, inchangé ici. Ce sont les mêmes lignes vers le même code, par les tubes d'un process ou par les tours d'un module, sous Windows 11, au profil release de l'app, en release pour le module, trois passes avec d'autres agents qui compilaient sur le poste ; **c'est la médiane des trois passes**, et l'écart d'une passe à l'autre est grand (le débit de Codex en module va de 21 000 à 47 000 lignes/s, en natif de 21 000 à 31 000) : seuls les ordres de grandeur comptent. Bruts : `prototypes/wasm-all/results/`. La mémoire d'un module vient d'une quatrième passe. Le passage par `ProcessEngine` et par le cœur reste à rejouer par `npm run bench:int0` à chaque harness porté : le seuil d'INT-0 (5 ms de plus au 99e centile, 2 200 lignes/s) reste le juge.

| | Codex natif | Codex module | Linear natif | Linear module |
| --- | --- | --- | --- | --- |
| **Au plus vite : débit** | 27 562 lignes/s, 9,3 Mo/s | 42 561 lignes/s, 14,4 Mo/s | 795 appels/s | 677 appels/s |
| **Au plus vite : médiane, 99e centile** | 32 µs, 80 µs | 18 µs, 72 µs | 1,17 ms, 2,03 ms | 1,37 ms, 2,35 ms |
| **En lots de 64 Kio** | 199 153 lignes/s | 191 309 lignes/s | — | — |
| **Au rythme réel : médiane, 99e centile** | 69 µs, 254 µs | 65 µs, 164 µs | 1,74 ms, 2,65 ms | 1,64 ms, 2,43 ms |
| **Mémoire, à vide** | 7,5 Mo (working set) | 1,18 Mio de mémoire linéaire, 0,28 Mo de working set en plus par instance | 6,7 Mo | 1,18 Mio, 0,25 Mo |
| **Mémoire, après le banc** | 10,9 Mo | 1,57 Mio | 10,4 Mo | 1,64 Mio |
| **Démarrage jusqu'à `initialize`** | 18,4 ms | 3,4 ms ; 93 ms la première fois, précompilation comprise | 14,4 ms | 2,2 ms ; 50 ms la première fois |
| **99e centile du démarrage** | 537 ms | 5,1 ms | 575 ms | 4,4 ms |
| **Taille** | 1 164 800 o, ou sa part des 34,5 Mo d'`escouade-plugins` | 642 140 o | 3 693 568 o | 459 364 o |

Le flux de Codex est celui de `full-turn.jsonl` et `plan-mode.jsonl`, 479 lignes rejouées 20 fois au plus vite, une fois à son rythme (60 s, un silence de plus de 2 s raccourci à 2 s, comme INT-0). Linear répond à `tickets/issues`, une page de 50 issues, deux `fetch` et un secret, sur un faux Linear local : 2 000 appels au plus vite, 200 toutes les 20 ms. Le natif équivalent est un exécutable pour ce seul plugin, sur sa vraie crate, au profil de l'app ; `escouade-plugins` les porte tous.

Ce qu'on en tire :

1. **Le flux d'un harness est aussi rapide en module qu'en process**, plus rapide ligne à ligne (pas de tube), un peu moins en gros lots (la traduction elle-même est plus lente en WASM). Au rythme réel, il n'y a pas de différence qui compte : quelques dixièmes de milliseconde, très en dessous des 16 ms de `conv_flusher`.
2. **Le réseau ajoute un peu** : environ 0,2 ms par appel à Linear, au plus vite, et rien de visible au rythme réel. C'est le seul recul mesuré. Il vient du JSON de `net/fetch` et de l'aller-retour vers le fil de l'hôte.
3. **La mémoire et le démarrage changent d'échelle** : plus de process par plugin. 23 plugins actifs coûteraient 150 à 250 Mo en process, une quarantaine en modules, le code compilé étant partagé.
4. **Le risque n'est pas là où on l'attendait** en 2.1 (§4.6 : « W : impossible pour un harness »). Le harness est lancé par le cœur depuis INT-9 : le plugin n'est plus qu'une machine à états, et elle compile.

### 8.10 Le plan

Les tickets du portage, dans l'ordre. **Les lettres sont des noms de travail**, le chef crée les tickets. Chacun passe ses suites locales ; chaque harness porté rejoue INT-0.

**A. Ce qui débloque tout, à la suite :**

| Ticket | Ce qu'il fait | Ce qu'il touche | Après |
| --- | --- | --- | --- |
| **M0. Le déménagement** | `plugins/builtin/*` → `plugins/<id-court>/`, LICENSE, CHANGELOG, `minimal` en fixture ; aucun changement de code | `src-tauri/Cargo.toml` (ses chemins), `plugins/builtin.rs` (`include_str!`), `store/server/utils/builtin.ts` (`source`), `website/`, docs, `plugins/README.md` | ESC-226 fusionné : sinon conflit |
| **M1. La CI de `plugins/`** | `plugins/ci.mjs` lit `Cargo.toml` : `cargo test`, `--changed` | `plugins/ci.mjs`, `.github/workflows/` | M0 |
| **A1. `run.wasm` dans l'app** | `run.wasm` au manifeste, le réacteur, `serve` par événements, `fetch` concurrent, `escouade.call` | `plugins/wasm.rs`, `manifest.rs`, `runtime.rs`, `schemas/` | |
| **A2. Le réseau** | `net/fetch` revu (§8.4.1) ; `net/stream` ; `ws/*` ; `local:shared` ; `timer/*` ; `setTimeout` du prélude | `plugins/wasm.rs`, `wasm_prelude.js`, `schemas/plugin-protocol-1.json`, `sdk/` | A1 |
| **A3. Les fichiers et les commandes** (fait, §8.15) | `fs/*`, racines, clôture à l'hôte ; `proc/*` | `plugins/wasm.rs`, `plugins/files.rs`, `manifest.rs`, `src/lib/i18n/*/plugins.ts`, la fenêtre | A1 |
| **A4. Le kit en WASM** | `escouade-turn` et `serve` dans les crates, `http` sans reqwest sur wasm, `initialize` au lieu de l'environnement, `std::process::id` | `crates/*` | A1 |
| **A5. `escouade-plugin build`** (fait, §8.17) | `rustc-canon` et la chaîne d'outils privée, `run.wasm` au banc, `plugin-cli` d'ESC-328 | `plugins/bench/`, le crate d'ESC-328 | A4, ESC-328 |
| **A6. Le store** (fait, §8.18) | le onzième contrôle pour `run.wasm`, bubblewrap, la liste fermée, l'image | `store/server/utils/{compile,checks}.ts`, `store/Dockerfile`, `docs/STORE.md` | A5, ESC-328 |
| **A6b. Les profils du conteneur** (fait, `docs/STORE.md` §3.3) | seccomp et AppArmor étroits, pour que bubblewrap tourne sous Docker ; le geste sur l'hôte reste à l'utilisateur | `store/deploy/`, `store/scripts/image-test.mjs` | A6 |
| **A7. La revue locale** (fait, §8.20) | `recompile.ts`, les pistes, le conteneur pour un tiers | `store/scripts/plugin-review/` | A6 |
| **A8. L'amorce** (fait, §8.14) | `plugins/seed.json`, l'installation sans réseau, la révocation, plugins à part entière, `release.yml` | `plugins/seed.rs`, `plugins/store.rs`, `plugins/builtin.rs`, `plugins/mod.rs`, `src-tauri/tauri.conf.json`, `.github/workflows/release.yml` | A1 |
| **A9. crates.io** (fait, §8.16 : publiées le 2026-11-05) | publier les trois crates ; `[patch.crates-io]` | `crates/*/Cargo.toml` ; **geste de l'utilisateur** | A4 |

**P. Le portage, en parallèle une fois A1 à A5 faits** (rien n'empêche d'en porter en attendant A6, le banc suffit) :

| Ticket | Plugins | Ce qui le distingue |
| --- | --- | --- |
| **P1** (fait, §8.19) | Linear, Trello, Jira, ClickUp | le kit seul ; le plus simple |
| **P2** (fait, §8.25) | GitHub, GitLab, Bitbucket, Gitea, Azure DevOps | `{account}`, proxy, instances auto-hébergées ; essais contre `fake-trackers` |
| **P3** (Codex, Copilot CLI, Cursor, Gemini : fait, §8.26 ; ACP, Claude Code : fait, §8.27) | Codex, Copilot CLI, Cursor, Gemini, ACP, Vibe, Claude Code | les machines à états ; `fs/*` pour ACP ; `real-harnesses` rejoué |
| **P4** (fait, §8.24) | Antigravity | l'attente du transcript réécrite (§8.4.6) |
| **P5** (fait, §8.23) | OpenCode | SSE, `local:shared`, ses trois tâches ; le plus risqué des harness |
| **P6** (fait, §8.28) | OpenAI, Groq, Mistral, ElevenLabs, Deepgram | sortir `voice/online*`, `live.rs`, `wire.rs` du crate de l'app dans une crate commune ; WebSocket ; INT-13 rejoué |
| **P7** (fait, §8.22) | isola | `proc/*` |
| **P8** (fait, §8.30) | les « Vue X » | `view/translate` ; le cœur ne lie plus aucun plugin |

**Z. La fin, dans cet ordre :**

| Ticket | Ce qu'il fait |
| --- |--- |
| **Z1. Les contrôles et la première publication** | `npm run plugins:publish` (fait, §8.6.2, ESC-353) ; le chef publie chaque plugin, revue et signature une par une la première fois |
| **Z2. L'amorce réelle** (fait, §8.32) | `plugins/seed.json` rempli, un essai d'installation hors ligne |
| **Z3. Le retrait** | `escouade-plugins`, `run.command`, le transport par process, `check_program`, l'origine `builtin`, `BUILTINS`, les attestations ; `npm run bench:int0` complet |

**L'ordre demandé par l'utilisateur** se lit ainsi : **le déménagement** (M0), **la CI de `plugins/`** (M1), **les contrôles du store pour un `run.wasm`** (A6, A7), **la publication de chacun** (Z1).

**Ce qui part en parallèle** : M1 avec A1 ; A2, A3, A4 après A1 ; A8 après A1 ; tout P après A5, un ticket par famille, P1 d'abord pour éprouver le kit.

**Le sort des tickets en cours :**

- **ESC-226 (Claude Code natif)** se termine tel quel, et passe avant M0. Son client et sa traduction sont déjà des machines à états. Il garde `tokio` pour `signin.rs` : P3 le retire du plugin, les lectures allant au cœur. Rien à changer maintenant.
- **ESC-328 (l'image du store)** continue : c'est un prérequis. Il reste juste **à prévoir sa dernière étape pour Rust** (§8.5.3) : le crate sans Tauri d'`escouade-plugin` (`crates/escouade-plugin-cli`) devient celui qui porte `rustc-canon`, `bubblewrap` entre dans l'image, et sa limite de temps monte. Son test d'empreinte (slack, team-rules) reste. Sa décision de ne dépendre d'aucune release tient.
- **ESC-227 (la doc de référence)** passe **après Z3** : §4 et §8 deviennent la référence, `docs/ENGINES.md` (« Écrire un plugin de harness ») dit le WASM, et les pages `plugins/api`, `publish` et `manifest` du site disent `run.wasm` et les capacités. La revue du 2026-10-29 fait qu'elle couvrait déjà « tout en process » : ce texte-là est à revoir.

**Ce que la décision du 2026-11-04 ne tranche pas et que ce plan suppose** : la publication sur crates.io (§8.6.3) et la licence GPL des plugins déménagés (§8.6.1). Les deux sont dans les décisions proposées, et se refusent séparément.

### 8.11 Ce qu'A1 a construit (ESC-331)

- **Le manifeste.** `"run": { "wasm": "Cargo.toml" }` : les deux lecteurs (`manifest.rs`, `schemas/plugin-manifest.mjs`, mêmes cas dans `tests/fixtures/plugins/manifests.json`) et le schéma. Un programme est `command`, `js` ou `wasm`, un seul des trois (`exclusive`) ; `wasm` nomme un `Cargo.toml` du dossier, jamais un binaire (`cargo`, nouveau code).
- **Où est le module.** `plugin.wasm`, à la racine du dossier du plugin. Une installation depuis le store y écrit le module de `version.json`, à la place de tout ce que l'archive pourrait avoir sous ce nom, avant l'empreinte du dossier ; une version sans module n'installe rien. Depuis un dossier, c'est celui de l'auteur ; sans lui, le plugin s'arrête et le journal dit pourquoi. `build` viendra avec A5.
- **Un seul hôte.** `Instance` et `serve` servent les deux. `_start` exporté : un bundle, sa première ligne `escouade/boot`, puis `_start`. Sinon : un réacteur, sans première ligne, `_initialize` s'il existe. `wasm::reactor` refuse un module qui exporte `_start` ou qui n'a pas `escouade-turn`.
- **`serve` par événements.** Le fil attend sur **un seul canal**, celui où le cœur écrit déjà. La fin d'un `net/fetch` y revient comme une ligne de plus : pour le module, tout est ligne, et A2 et A3 n'auront qu'à y écrire. Ce qui est là fait un tour. L'hôte ferme le canal : le fil s'arrête aussitôt et les `fetch` en cours sont abandonnés (le retour du `fetch` tient une référence faible, qui ne garde pas le canal ouvert).
- **`net/fetch` à côté**, sur un runtime de l'hôte à deux fils (`plugins-net`), pour `run.js` comme pour `run.wasm`. **16 appels à la fois par plugin** (pas dans l'étude) : avant, un plugin n'en avait qu'un, et rien ne bornait plus ce qu'il pouvait ouvrir. Les suivants attendent leur tour hors de son fil. Le temps d'attente de ce tour n'est pas dans les 10 s d'un appel.
- **`escouade.call`.** L'import `escouade`/`call` (requête, longueur, sortie, capacité), lié pour tous les modules. Il répond `{"result":…}` ou `{"error":"…"}` ; une réponse trop grande rend l'opposé de sa taille, sans rien écrire ; un pointeur hors de la mémoire du module est une faute du module. **Aucune méthode n'est branchée** : `Instance::start_with` prend ce qu'il sert (`Calls`), et l'app n'en donne pas encore ; A3 y met les fichiers. Une réponse trop grande refait l'appel : A3 devra s'en souvenir pour tout ce qui écrit.
- **Le cache.** `<empreinte du module>-<version de l'app>.cwasm` ; le module reste dans le dossier du plugin. Précompilé à l'installation, retiré par `prune`, comme pour `run.js`.
- **Le banc et `validate`.** Dans l'app, `escouade-plugin test` joue `plugin.wasm`, ou le module de `--wasm`. `validate` accepte un `run.wasm` et juge la forme de `plugin.wasm` s'il est là, dans `crates/escouade-plugin-cli` (`wasm::reactor_form`), sans wasmtime : l'image du store a donc la même règle, et l'hôte reste dans l'app, qui prend cette règle au crate.
- **Les preuves.** `tests/fixtures/plugins/reactor`, un réacteur sans dépendance, ses octets commités (`sh build.sh`) ; ses tests sont dans `plugins_reactor_tests.rs`. Linear et Codex du prototype, contre `escouade-plugins`, passent par cet hôte dans `plugins_wasm_parity_tests.rs`. Depuis A4, ces modules viennent des crates du dépôt (§8.13). Ils ne sont pas commités : ces tests se sautent sans eux, et échouent avec `ESCOUADE_PARITY=1`. **La CI les construit sans les jouer** (`plugin-kit-wasm`) ; M1 ou P1 les joueront.
- **Pas fait ici** : le test de lecture de fichier du prototype (A3) ; le store accepte déjà le champ par le lecteur commun, mais ne compile pas (`dev.exemple.reactor` n'existe que dans le faux store) : A6.

### 8.12 Ce qu'A2 a construit (ESC-332)

- **Les fichiers.** `plugins/beside.rs` : ce qui tourne à côté du fil d'un plugin (`Beside`), et `request_of`, qui prend au module ce que l'hôte sert lui-même. `plugins/wasm.rs` garde `Fetcher` et `serve`, qui ne fait plus que passer les lignes. Tout est dans un `JoinSet` du fil : fermer l'instance coupe flux, sockets et minuteurs.
- **`{account}`.** L'hôte tapé pour un compte que ce plugin sert (`accounts::hosts_of` : `ForgeAccount::host()`, le `site` d'un compte de source), **et sur ce port** : l'étude dit « l'hôte », un port non saisi vaut celui du schéma, et un autre port du même hôte est refusé. Aucun sous-domaine. La liste est demandée à chaque appel (`Net::accounts`) : un compte retiré ferme l'hôte sans redémarrage. http et https passent, `wss` seul pour un socket.
- **`local:shared`.** `SharedLaunch::listens`, lu dans `shared.rs` (`port_in`, `ports_of`) ; le port est celui du process vivant, par runtime et par plugin. Il contourne le proxy. OpenCode s'en sert depuis P5 (§8.23).
- **Le proxy.** `Net::network` est partagé et lu à chaque appel : un changement de Réglages › Réseau atteint les instances qui tournent, sans les relancer. Le proxy système n'est jamais suivi, seul celui des réglages ; le nom du proxy échappe au jugement des adresses. Le journal dit, sur chaque ligne passée par le proxy, que le nom seul a été jugé. `localhost` et `*.localhost` sont refusés par le nom quand un proxy les emmènerait.
- **Le journal.** Chaque appel y va par sa méthode et son hôte (`Kind::Message`), jamais son chemin, sa requête ni ses en-têtes. Un message d'erreur ne cite plus l'URL : `only https` dit l'origine, et `reqwest` perd son URL (`without_url`) ; avant, une requête en échec écrivait sa clé de requête au journal.
- **`body` ou `bodyBase64`.** Une réponse UTF-8 vient dans `body`, une autre entière dans `bodyBase64` : plus rien n'est remplacé, et un lecteur de `body` doit regarder les deux. Dans le prélude, `Response` a `bytes()` et `arrayBuffer()`, et `fetch` prend un `Uint8Array` ou un `ArrayBuffer`.
- **`net/chunk`.** `data` si les octets sont UTF-8, un caractère coupé par la lecture attendant le morceau suivant ; `dataBase64` sinon. Le crédit compte les octets lus. Il est découpé au besoin : le plugin n'a jamais plus de 256 Kio non consommés, et `net/credit` ne rend pas plus que ce qui a été donné.
- **Deux décisions de ce ticket que l'étude ne tranchait pas.** Un `ws/send` long passe quand rien n'attend : sinon, la file de 256 Kio rendrait impossibles les messages entre 256 Kio et 1 Mio. Un message reçu attend d'être écrit sur l'entrée du module avant que le suivant soit lu (le `oneshot` de `Frame`) : le WebSocket n'a pas de crédit, et un serveur rapide aurait rempli la mémoire de l'app.
- **`timer/set`** ramène à 10 ms ce qui est dessous au lieu de refuser, pour que `setTimeout(f, 0)` marche. `net/credit`, `net/cancel` et `timer/cancel` sont des notifications (avec un `id`, l'hôte répond). Après `net/cancel`, aucun `net/end`.
- **Le SDK.** `openStream` et `openSocket`, dans `sdk/plugin/src/net.ts`, à côté de `Plugin` : ils passent par `handle` et `notify`, et le crédit est rendu quand un morceau est lu. Rien n'en reste dans le bundle d'un plugin qui ne s'en sert pas : slack et team-rules gardent leurs octets. La description de l'erreur `refused` n'a pas bougé pour la même raison : esbuild la garde dans le bundle.
- **Le banc.** `escouade-plugin test --account <adresse>` donne à `{account}` ce qu'aurait saisi l'utilisateur. Les limites du manifeste s'y appliquent. Pas de proxy ni de `local:shared` au banc : aucun process partagé n'y tourne.
- **Le prélude a changé** : `goldens.json` est refait, et les modules déjà compilés se refont par leur clé.
- **Les preuves.** `plugins_net_tests.rs` (chaque règle et chaque refus, sur faux serveurs, le module `reactor` jouant `probe/host` ; TLS auto-signé et tunnel réels), `plugins/beside_tests.rs` (crédit, morceaux, file), et `tests/fixtures/plugins/sdk-net`, un `run.js` du SDK joué dans `plugins_wasm_tests.rs`. Le délai de bout en bout de la voix reste à mesurer en P6.
- **Pas fait ici** : aucun plugin n'est porté ; le store ne compile pas encore `run.wasm` (A6).

### 8.13 Ce qu'A4 a construit (ESC-334)

- **Les crates compilent pour les deux cibles.** `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp`, en natif comme pour `wasm32-wasip1`. tokio et reqwest ne sont des dépendances du kit qu'hors wasm. `src-tauri/rust-toolchain.toml` porte la cible ; le job `plugin-kit-wasm` de la CI passe clippy, compile Linear et Codex et lance `cargo publish --dry-run` sur les trois crates.
- **Le tour est dans le protocole, pas dans le kit**, écart assumé avec le mot du ticket : Codex et les harness ne dépendent que du protocole, et leur machine à états n'a besoin que de lui. `escouade_plugin_protocol::turn` tient `run` (le `serve` d'une machine à états, sur les deux cibles), `instance_id` et `read_file` (`std::process::id` et `std::fs` pour une source commune) et, sur wasm, `lines`, `write`, `flush`, `set_turn` et `host_call`.
- **`module!(run)`** est l'unique ligne d'un plugin qui exporte son tour, et ne produit rien en natif. Le premier tour appelle `run`, qui garde le plugin. Le prix d'un harness est de deux lignes : `turn::run` et `module!`.
- **Un seul `Cx`.** `serve.rs` tient `Plugin`, `Cx` et ce que les deux boucles partagent (les lots, les erreurs, `initialize`) ; `serve/process.rs` est la boucle de tokio d'avant, `serve/reactor.rs` l'exécuteur. Les réponses du cœur ne passent plus par `tokio::sync::oneshot` mais par une attente à `Waker`, commune aux deux. Le réacteur est compilé pour les tests natifs : il se teste ligne à ligne, sans wasm.
- **`http::reqwest`** est ce qu'un plugin importe : reqwest lui-même en natif, en wasm la façade `http/fetch.rs`. Elle a la surface dont les plugins se servent, sauf les flux, qu'OpenCode lit par `net/stream` avec son propre réacteur (§8.23). Proxy, certificats et délai s'acceptent et ne font rien : c'est l'hôte qui les tient. **Essayé** : Azure DevOps, Bitbucket, ClickUp, Gitea, GitHub, GitLab, Jira et Trello compilent pour wasm avec une ligne `use` de plus et leur `tokio::time::sleep` remplacé.
- **`time::sleep`, `time::timeout`** : tokio en natif, `timer/set`, `timer/cancel` et `timer/fired` en wasm. Un minuteur lâché avant de sonner est annulé. Un hôte qui refuse un minuteur le dit au journal et le `sleep` finit aussitôt : une tâche sans fin retiendrait l'appel pour toujours. **Le côté hôte est celui d'A2** (§8.12) : l'interface est aussi testée contre un hôte simulé.
- **`initialize`.** Le cœur envoie `lang` à tout plugin ; le kit et `serve` gardent `lang` et `escouade.version` (`protocol::greeting()`), et le kit règle sa langue dessus. `read_file` passe par `fs/read`, que l'hôte refuse jusqu'à A3 : Codex y lit `config.toml` pour le seul cas d'un serveur refusé.
- **La preuve.** `sh scripts/plugins-wasm.sh` compile Linear et Codex depuis `plugins/<id-court>/`, sans fork, par `cargo rustc --crate-type cdylib` : leur crate reste une `rlib` pour `escouade-plugins` jusqu'à Z3, et leur manifeste ne change pas. `ESCOUADE_PARITY=1 cargo nextest run --lib plugins_wasm_parity`, après `cargo build --bins`, les joue dans l'hôte de l'app. Linear : un `use` à la place de sa dépendance reqwest, `module!`. Codex : `run()`, deux lignes de `client.rs`, `module!`. Ce n'est pas une compilation reproductible : c'est A5.
- **Le prototype s'arrête là.** `prepare.mjs` cherche le code d'avant A4 et échoue : pour refaire les mesures du §8.9, partir du commit `de8eba05`.
- **Prêtes pour A9.** Licence, dépôt, `readme`, `include`, une copie de LICENSE et un README par crate. `cargo publish --dry-run -p escouade-plugin-protocol -p escouade-plugin-kit -p escouade-acp` passe. Le chef publie (§8.16).

### 8.14 Ce qu'A8 a construit (ESC-338)

- **`plugins/seed.json`** : `{ "format": 1, "plugins": [{ "id", "version", "sha256" }] }`, `sha256` étant celui de `version.json`. Il était **vide jusqu'à Z2** : aucune version de la première main n'était publiée. Avec une liste vide, rien ne change pour l'utilisateur. Z2 l'a rempli (§8.32).
- **`plugins/seed.rs`.** `Seed::take` lit une version dans `<ressources>/seed/<id>/` et rend un `Fetched`, comme `Store::fetch`. Les contrôles sont les mêmes fonctions, sorties de `fetch` : `check_version_json` (l'empreinte embarquée, le dossier, l'id, la version), `check_seal`, `check_archive`, `check_module`. Le reste est `inspect_store` puis `install`, l'unique chemin d'une installation du store.
- **Le premier lancement.** `Plugins::sync_seed`, dans `core`, après `apply_store` et avant le démarrage de tout plugin : installe ce que l'amorce a, qui n'est pas installé et que le tri désigne (`sort::sort`, ESC-373, §8.7), sauf ce que l'utilisateur a désinstallé. Le premier lancement et celui d'après une mise à jour de l'app sont le même code : seul compte ce qui est installé. L'origine est `Store { seed: true, .. }`.
- **Ce qu'une version refusée laisse.** Rien d'installé, et sa raison dans la vue (« Proposés par Escouade ») : signature fausse, fichier changé, `escouade.min`, `escouade.max`, sceau retiré par la liste (`unverify`). **Une version retirée par `disable` s'installe**, coupée, sa raison dite : c'est la règle du §8.7, et le plugin reste visible dans « Ajoutés » avec le bandeau rouge. Sur un clic, en revanche, elle est refusée, comme une version retirée du store.
- **La liste de révocation.** `<ressources>/seed/revocations.json` et son `.minisig`, vérifiés avec les clés de l'app. `Store::with_seed_list` la prend si son numéro dépasse celui de la liste gardée ; la liste du store la remplace dès qu'elle est plus récente. Elle n'est pas recopiée dans les données : l'amorce reste où elle est. `ListView.from` vaut alors `seed`.
- **Les mises à jour.** `store_update` rend la plus récente de l'offre du store et de la version de l'amorce, jamais plus ancienne que l'installée, jamais une version de l'amorce révoquée. `Update.seed` dit d'où elle s'installe : `plugin_inspect_seed`, sans réseau, puis la fenêtre habituelle. Rien ne s'installe seul. `inspect_seed` refuse de redescendre.
- **Désinstaller.** `remove` note le plugin dans `<plugins>/.seed.json` (sa version, son nom). `seed_offers` le propose, et la note s'efface à la réinstallation. La vue prend le store si son `best` dépasse la version de l'amorce, sinon l'amorce, sans réseau. Les choix gardés par « Garder ses réglages » reviennent avec lui.
- **Un harness ou une source sans son plugin.** Un agent dit « Le plugin … n'est plus installé : la vue Plugins propose de l'installer » (`harness::UNINSTALLED`, relu à chaque `sync`) ; une source ou un hébergeur dit la même chose dans son message habituel (`Runtime::host`).
- **Le passage de `builtin` à l'amorce.** `Plugins::given` : un id du registre que `seed.json` nomme n'est plus donné par l'app. Il quitte la liste de `with_builtin`, `install` et `remove` l'acceptent. Une entrée `Origin::Builtin` déjà au fichier des plugins reste jusqu'à ce que `sync_seed` mette la version de l'amorce à sa place, choix gardés ; refusée, elle continue de tourner comme avant, jusqu'à Z3. `builtin_contributions` prend aussi une version de l'amorce signée : isola garde son environnement de lancement. `builtin::seeds` dit quels ids l'amorce peut tenir ; la liste embarquée n'en prend pas d'autre.
- **La précompilation.** `prepare_all` saute les plugins de l'amorce : leur module se précompile à la première activation, sur le fil du plugin. Une mise à jour par un clic le précompile aussitôt, comme toute installation (`plugin_install`).
- **La release.** `store/scripts/seed.ts fetch`, à côté de `copy-revocations.ts` et sur les mêmes fonctions : il télécharge, vérifie la signature (clés de `store/keys/`), le commentaire, les empreintes et le module, et échoue sur une version absente, retirée, révoquée, non signée, hors de `escouade.min`/`max`, ou sans liste de révocation. Avec une liste vide il ne demande rien au store : une release avant Z2 passe. **Chaque job de construction le lance**, avant Tauri.
- **Un build de développement.** `Seed.unsigned` vaut `cfg!(debug_assertions)`. `sh scripts/seed-dev.sh` construit les modules des plugins en Rust de `plugins/` (ou des dossiers donnés), et écrit `src-tauri/seed/` non signé avec son propre `seed.json`, que seul un build de développement lit (`ESCOUADE_SEED_DIR` en donne le chemin). Une signature fausse reste refusée partout : seule son absence est tolérée.
- **Les preuves.** `plugins_seed_tests.rs` (15 tests), sur les octets et les signatures du faux store (`POST /_fake/seed`), `store/tests/unit/seed.test.ts` pour la release, `tests/e2e/plugins-seed.spec.ts` pour la vue.
- **Le tri (ESC-373).** `plugins/sort.rs` lit la règle du §8.7 dans `Settings` (`accounts`, `voice`, `writer_engine`), les projets (`integrations.links`, `engine`, `repos`, leur `.git/config`, `.isola.toml`), les harness des agents, `integrations::Accounts` (`jira`… `clickup`, `forges`, leur `plugin`) et les clés de voix gardées (`voice::online::usable_keys`). `sort::reads` dit si `settings.json`, `state.json` et `integrations.json` se lisent ; sinon tout est voulu. `sync_seed(lang, wants)` prend la règle ; `dev.escouade.acp` est voulu aussi quand un plugin installé décrit un harness par ACP (`contributes.harnesses[].acp`). Une entrée `builtin` non voulue sort par `retire_builtin` (ses choix dans `.kept/`). Le geste passe par `Plugins::need`, puis `Core::need_plugin` (harness et services de voix resynchronisés, `UiEvent::PluginSeeded`, une ligne dans la fenêtre, la vue relue) ; « Installer » de la vue est `plugin_install_seed`, sans fenêtre de permissions, comme le geste. Preuves : `plugins_sort_tests.rs` et `tests/e2e/plugins-sort.spec.ts`.
- **Pas fait ici** : Z2 remplit la liste et fait l'essai hors ligne réel ; le temps du premier lancement avec les 23 plugins n'est pas mesuré (`sync_seed` tourne au chargement du cœur, et n'écrit rien aux lancements suivants).

### 8.15 Ce qu'A3 a construit (ESC-333)

- **Le manifeste.** `permissions.files` (`root`, `access`), à côté de `permissions.commands` et non à la racine du manifeste : c'est une permission comme les autres, avec son mot (`files:agent:write`), que la fenêtre dit et que l'utilisateur peut retirer. `plugin` ne s'écrit pas (`readOnly`, nouveau code) : ce que le store a signé ne change pas. Une racine n'y vient qu'une fois. `limits.fileBytes` dit la lecture voulue, l'hôte la borne à 4 Mio. Une ligne de `permissions.commands` commence par un nom de programme, jamais un chemin, et `*` n'y vient qu'en dernier mot (`command`, nouveau code). Les deux lecteurs ont les mêmes cas dans `tests/fixtures/plugins/manifests.json`.
- **`plugins/files.rs`.** `Access` sert `fs/*`. On nomme une racine et un chemin relatif, ou un chemin de l'hôte sous une racine de cet agent (ce que lit une machine à états). Le chemin est jugé après ses liens, jusqu'où il existe, avant de dire s'il existe : aucun refus ne renseigne sur ce qui est hors de portée. Un `..`, un `~`, un flux NTFS (`a.txt:b`), un lien qui ne mène nulle part sont refusés. La clôture est celle de l'agent (`Spawn::fence`), lue par l'hôte à chaque appel. `fs/read` rend `{ text }`, ou `{ dataBase64 }` pour ce qui n'est pas de l'UTF-8.
- **`agent/attach` porte `roots`**, les seules racines déclarées. Le cœur les résout (`Runtime::place_agent`, depuis `ProcessEngineProcess`) : le plugin les apprend, il ne les donne jamais. `agentRepo` vient du `.git` du dossier (`files::repo_of`) ; `agentHome` de la variable de compte du harness, et il manque tant que l'agent tourne sur le compte par défaut : **P3 devra donner le dossier par défaut de chaque harness**. `temp` est `<données>/plugin-temp/<id>`, le même pour tous ses agents.
- **Seuls les plugins de harness reçoivent un agent.** `agent/attach` n'existe que pour eux ; un autre plugin a `plugin`, `temp`, et `proc/spawn` avec un `project`. C'est tout ce que le §8.1 demande. Le jour où un plugin d'une autre sorte voudra le dossier d'un agent, il faudra lui donner les agents qui arrivent.
- **`plugins/procs.rs`.** `Procs` sert `proc/*`. Le programme vient du PATH, dossiers absolus seuls (`locate`). **Pas de `.cmd` ni de `.bat` sous Windows** : ils ne tournent que par un shell. Un outil installé par npm n'est donc pas lançable : c'est voulu (isola n'a pas de release Windows, §8.22). Sans `dir`, le programme tourne dans `temp`. Une variable `ESCOUADE_*`, `PATH`, `LD_*`, `DYLD_*`, `NODE_OPTIONS` ne se nomme pas : elle ferait tourner autre chose que le programme déclaré.
- **Un Job object par programme, tenu par l'instance**, et non un seul par plugin : un groupe Unix se nomme par son chef, et une instance n'a pas de process. L'effet est celui du §8.4.7 : tout meurt avec l'instance (`Capabilities::end`, à la sortie de son fil), ou à `proc/kill`.
- **Lancer un programme prend du temps sur le fil.** `proc/spawn` crée le process et le met dans son Job sur le fil du plugin : par `escouade.call`, ce temps compte dans le tour. Vu dans un test en debug : plus de 200 ms. Les 2 s d'un tour suffisent, mais un tour de décision (500 ms) n'a pas à lancer de programme.
- **Le crédit est implicite.** Pas de `proc/credit` : l'hôte ne lit plus le tuyau au-delà de 256 Kio qui ne sont pas encore sur l'entrée du module, et un tour lit tout ce qui y est. Un module n'a donc rien à faire pour avancer. `proc/out` peut arriver avant la réponse à `proc/spawn` : le SDK le garde pour le programme qu'on va lui nommer.
- **Deux portes, un seul service.** `Capabilities` sert les deux : `escouade.call` pour `run.wasm`, une ligne pour `run.js` (`p.fs`, `p.runProgram` du SDK). La ligne prend le chemin des capacités d'A2 : `request_of` la reconnaît, `Beside` la sert, la réponse revient au tour suivant sur le canal unique. **`run.js` n'a pas la porte synchrone** : il faudrait un import de plus dans escouade-js, donc de nouvelles empreintes pour tous les modules, pour une API qui est de toute façon en promesses. Mêmes refus, même journal ; un test joue chaque porte. Un plugin lancé par une commande reçoit `methodNotFound`.
- **`escouade.call` ne sert jamais deux fois.** La réponse trop grande est gardée avec sa requête : la même requête la reçoit sans être rejouée, donc un `fs/write` avec `append` n'écrit qu'une fois. Une erreur porte son code (`{ "error", "code" }`).
- **Le journal** dit la méthode et la racine (`fs/write agent: wrote 17 bytes`), jamais le chemin ni le contenu, comme `net/fetch` dit la méthode et l'hôte. Une commande y va avec ses arguments, secrets masqués, comme le demande le §8.4.7.
- **Le banc** sert `fs/*` et `proc/*` par le même code, autour d'un agent `bench` et d'un dossier jetable. Hors ligne (la revue), aucun programme ne tourne.
- **Les preuves.** Sur `files.rs` et `procs.rs`, chaque racine et chaque refus ; `plugins_reactor_tests.rs` : un `run.wasm` lit le dossier d'un agent et lance une commande ; `plugins_wasm_tests.rs` : `sdk-both` fait de même en `run.js`.

### 8.16 Ce qu'A9 a préparé (ESC-341)

**Publiées le 2026-11-05 par le chef**, en `0.1.0` : [escouade-plugin-protocol](https://crates.io/crates/escouade-plugin-protocol), [escouade-plugin-kit](https://crates.io/crates/escouade-plugin-kit) et [escouade-acp](https://crates.io/crates/escouade-acp), après le « oui » de Florian Gagnaire sur #22 (fermée) et la vérification de l'adresse e-mail du compte, que crates.io exige en plus de la connexion par GitHub. Le propriétaire est le compte `guillaume-gagnaire`. Un crate neuf qui dépend de `escouade-plugin-kit = "0.1.0"` compile pour `wasm32-wasip1` depuis crates.io. Ce qui suit est ce qu'A9 avait préparé.

- **Les noms sont libres** : `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` n'étaient pas dans l'index de crates.io le 2026-11-05.
- **`0.1.0`**, pas `1.0.0` : la version d'un crate n'est pas celle du protocole (`PROTOCOL`, 1), et son API bouge encore avec le portage. `"0.1.0"` veut dire `^0.1` : un changement cassant passe à `0.2.0`, avec l'exigence des plugins, dans le même commit.
- **MIT**, choisi par l'utilisateur le 2026-11-05, comme le SDK (décision 4 du 2026-10-25). Les plugins et l'app restent en GPL-3.0-or-later. Un tiers garde ainsi le choix de sa licence : en GPL, le `run.wasm` qu'il publie, lié au kit, aurait été distribué sous GPL.
- **La condition de la publication** : une dizaine de lignes de `forge.rs` du protocole et du kit (les champs de dépôt, la méthode `parent`) viennent de la PR #17 de Florian Gagnaire, sous la GPL de l'app, et le dépôt n'a pas de CLA. **Son accord est demandé par l'issue [#22](https://github.com/guillaume-gagnaire/escouade/issues/22) : le premier `cargo publish` attend son « oui » en commentaire de #22.** La recherche est faite ligne à ligne, sur les lignes ajoutées par Florian Gagnaire et Eric Pommereuil : du code reformaté a pu lui échapper. Aucune ligne d'Eric Pommereuil n'a été trouvée au-delà de formes banales.
- **Les plugins dépendent par version** (`escouade-plugin-kit = "0.1.0"`). `[patch.crates-io]`, dans `src-tauri/Cargo.toml`, racine du workspace qui les tient, les ramène sur `crates/`. Le cœur garde ses `path` : il n'est pas publié.
- **Entre crates, `path` et `version` à la fois.** `cargo publish` retire le `path` du manifeste publié. Avec la version seule, l'empaquetage cherche le crate sur crates.io et échoue tant qu'il n'y est pas : `[patch.crates-io]` ne l'atteint pas.
- **Le README est la doc du crate** (`#![doc = include_str!("../README.md")]`), en anglais, avec un exemple que `cargo test --doc` compile. docs.rs construit les deux cibles (`[package.metadata.docs.rs]`). Le job `plugin-kit-wasm` passe les doctests et `cargo doc` sans avertissement. `scripts/crates-io.test.ts` garde les champs, la licence et le `[patch.crates-io]`.
- **Il reste, pour que le store compile un plugin de la première main** : son manifeste porte `workspace = "../../src-tauri"`, que `cargo` refuse dans un dossier seul (essayé sur `plugins/linear`), et il n'a pas de `Cargo.lock` à lui, ce qu'un membre du workspace ne peut avoir et qui demande de toute façon les crates publiées. Sans cette ligne, `cargo metadata` cherche bien `escouade-plugin-kit` sur crates.io. **Réglé au portage** : §8.18.

**Publier, dans cet ordre** (depuis `src-tauri/`, sur `main` à jour, arbre propre, « oui » de Florian Gagnaire en commentaire de #22, adresse e-mail du compte vérifiée sur crates.io) :

```sh
cargo publish --dry-run --locked -p escouade-plugin-protocol -p escouade-plugin-kit -p escouade-acp
cargo publish --locked -p escouade-plugin-protocol
cargo publish --locked -p escouade-plugin-kit
cargo publish --locked -p escouade-acp
```

Le deuxième et le troisième attendent seuls que le premier soit dans l'index. **Après** : sur crates.io, chaque page dit `0.1.0`, MIT, le dépôt et le README ; le propriétaire est le compte de l'utilisateur. Sur docs.rs, la construction de `https://docs.rs/crate/<nom>/0.1.0/builds` est verte pour les deux cibles, et `escouade_plugin_kit::serve` se lit. Depuis un dossier vide, un crate qui dépend de `escouade-plugin-kit = "0.1.0"` compile pour `wasm32-wasip1`. Une version publiée ne s'efface pas : une erreur se corrige par `0.1.1`, et `cargo yank` retire seulement la version des nouvelles résolutions.

### 8.17 Ce qu'A5 a construit (ESC-335)

- **`escouade-plugin build` compile un `run.wasm`.** Même sortie que `run.js` : `packs/<id>-<version>.wasm`, son `.sha256`, et en `--json` `file`, `size`, `sha256` et `compiler`, `{ rust, target, canon, lock }` : ce que le store notera (§8.5.3). `--version` y ajoute `rust`, `rust-target` et `rustc-canon`. Tout est dans `crates/escouade-plugin-cli` : `rust.rs` et `canon.rs`.
- **La chaîne d'outils, sans rustup.** Quatre archives `.tar.gz` de static.rust-lang.org (`rustc`, `cargo`, `rust-std` de l'hôte, `rust-std` de la cible) sont téléchargées au premier besoin. Chacune est refusée si son SHA-256 n'est pas celui écrit dans `rust.rs` (les six systèmes de Javy, relevés dans `channel-rust-1.98.0.toml`). On n'en garde que `bin` et `lib`, dans `<données>/tools/rust-1.98.0/` : 654 Mo, 53 s à la première fois sur ce poste. **Chaque fichier est revérifié à chaque usage**, contre les sommes notées (`.escouade-sums`) : moins de 3 s, sur tous les cœurs ; une chaîne changée est téléchargée à nouveau, comme Javy. `ESCOUADE_RUST_TOOLCHAIN` nomme une chaîne déjà là, pour l'image du store (A6) : elle doit être de cette version, avoir la cible et ne pas avoir `rust-src`.
- **`rustc-canon` est `escouade-plugin` lui-même** : `cli::main` commence par regarder `ESCOUADE_CANON_ROOT` et, s'il est posé, sert d'enveloppeur. Pas de second programme à livrer, ni à lier à la première utilisation. Son empreinte est celle de **sa source** (`canon.rs`, en LF), la même d'un système à l'autre, et le profil imposé y est écrit pour que l'empreinte le couvre. Par rapport au prototype, une règle de plus : ce qu'un `build.rs` a généré, sous `OUT_DIR`, devient `/out/<crate>/…`. Une conséquence pour les tests : un test qui compile doit lancer le vrai programme, pas appeler `cli::main` depuis le binaire de test.
- **La compilation se fait sur une copie du dossier**, sous `<données>/cache/plugins-rust/`, sans `target`, `.git`, `node_modules` ni `packs`. Rien de ce qui entoure le plugin ne compte : ni un workspace, ni le `[patch.crates-io]` de la racine des plugins (§8.6.3), ni un `.cargo/config.toml` d'un dossier parent du plugin. `CARGO_HOME` est à part aussi (`<données>/tools/cargo-home`). Les variables `CARGO*` et `RUST*` de la machine sont retirées, sauf `CARGO_HTTP_*` et `CARGO_NET_*`. `--locked`, `CARGO_ENCODED_RUSTFLAGS` vide, et le profil passe sur le `[profile]` de l'auteur par `CARGO_PROFILE_RELEASE_*`, plutôt que par `--config` : même effet. Un `~/.cargo/config.toml` reste lu quand le dossier de données est sous le dossier personnel, cargo n'a pas d'option contre ; ses `rustflags` et son profil n'ont pas d'effet, et le store ne l'a pas.
- **Les refus, fr et en** : sans lancer rien, `rust::check`, que `validate` dit aussi. Pas de `Cargo.lock`, `[patch]`, `[replace]`, un `path` qui sort du dossier (toutes les tables de dépendances, `workspace.members`, `package.workspace`), `git` ou un autre registre, une source du lock hors crates.io, une cible qui n'est pas `cdylib`, un `build.rs`, une macro procédurale, `.cargo/config.toml`, `rust-toolchain`, `[profile.*.package]` et `build-override`, que le profil imposé ne couvre pas. Ensuite, sur `cargo metadata --filter-platform wasm32-wasip1`, `rust::check_packages` : **tout ce qui tournerait en natif doit être sur la liste fermée**, `rust_native.json`, par nom, version et somme de contrôle.
- **La liste fermée va plus loin que la lettre du §8.5.4** : un script ou une macro, et aussi tout ce dont ils dépendent (`syn`, `unicode-ident`, `autocfg`), car ce code-là tourne en natif de la même façon. Elle part de ce qu'il faut au kit, à ACP, à Linear, à Codex et à la fixture : `anyhow`, `autocfg`, `num-traits`, `parking_lot_core`, `proc-macro2`, `quote`, `serde`, `serde_core`, `serde_derive`, `serde_json`, `syn`, `unicode-ident`, `zmij`. Ajouter une version est une relecture. **Une dépendance qui ne tourne qu'en WASM n'a pas besoin d'y être.** Un `Cargo.lock` de plugin qui monte `serde` d'une version sera refusé tant que la liste ne suit pas : c'est le prix de la règle, à revoir à A6 si le coût est trop haut.
- **`["cdylib", "rlib"]` passe**, écart assumé avec le mot du ticket : l'`rlib` n'ajoute aucune cible qui tourne, et sans elle aucun plugin de la première main ne pourrait passer ici tant qu'`escouade-plugins` les lie (Z1 vient avant Z3). Une `rlib` seule, une `staticlib`, un `[[bin]]`, `src/main.rs` et `src/bin` sont refusés. `module!` manquant : `wasm::reactor_form` refuse le module après compilation.
- **`test` et `dev` compilent le `run.wasm`**, et ne lisent plus le `plugin.wasm` du dossier, qui pourrait ne plus suivre les sources. `--wasm` joue toujours un module donné. `validate` juge toujours la forme de `plugin.wasm` s'il est là.
- **La preuve.** `tests/fixtures/plugins/rust-build` : serde, ses macros et ses scripts, un module en sous-dossier, une panique. Son empreinte est dans `goldens.json` et `plugins_rust_build_tests.rs` la compare, dans deux dossiers neufs avec deux `target` neufs : chaque système de la CI la prouve, sans comparer deux machines. Changer `canon.rs` fait échouer ce test : c'est voulu, chaque module publié changerait. `sh scripts/plugin-build-linux.sh [dossier]` compile sous Linux depuis n'importe quel poste avec un moteur de conteneurs, sans monter de dossier. Mesuré : `538b34fd334aada1a2f2a1c86fdd15b232c7d8cc31706bf7e7ddf079068e4bfc`, 119 315 o, sous Windows 11 et sous Linux (`rust:1.98.0-bookworm`, Podman), un autre dossier et un autre `CARGO_HOME` compris. Sur un vrai plugin : une copie de Linear, le kit dans son dossier en attendant crates.io (A9), un `Cargo.lock` de 159 paquets, donne `00f840b469dd2ffec969815f03594e4be30eea32fdaa4ad48ed14f3f5d7d38ca`, 489 907 o, deux fois sous Windows et deux fois sous Linux, et passe la liste fermée telle quelle. **macOS n'a toujours pas été essayé.**
- **Pas fait ici** : le store, sa liste et son bac à sable (A6), la revue (A7), le job `reproductible` de `plugins/ci.mjs` (M1, §8.6.4). Le `cargo fetch` séparé du §8.5.3 ne passe pas par `build` : A6 le lancera avec le même `CARGO_HOME`, ou posera un dossier de données commun.

### 8.18 Ce qu'A6 a construit (ESC-336)

- **Le onzième contrôle s'étend**, comme le §8.5.3 le prévoit : `server/utils/compile.ts`, `checks.ts`, `submissions.ts`. Le détail, les limites et les messages sont dans `docs/STORE.md` §3.3. Le `compiler` de `version.json` est celui d'A5.
- **`escouade-plugin fetch <dossier>`, nouveau.** Le §8.17 laissait au store un `cargo fetch` à lancer lui-même. Une commande vaut mieux : elle passe `rust::check` avant cargo, retire les variables `CARGO*` et `RUST*`, travaille sur la même copie et le même `CARGO_HOME`, et dit `✗` pour la faute du plugin (`Refusal::StaleLock`, ou `BadManifest` quand cargo ne lit pas le manifeste), rien pour un réseau en panne. `canon.rs` ne bouge pas : `goldens.json` tient.
- **Écart avec le §8.5.3 : le réseau de `fetch` n'est pas filtré par hôte.** Il faudrait un proxy dans le conteneur. C'est `rust::check`, passé avant cargo, qui l'enferme : pas de `.cargo/config.toml`, pas de `[patch]`, un lock sur crates.io seul, et rien du plugin ne s'exécute dans cette étape.
- **Écart avec le §8.5.3 : `fetch` passe avant chaque compilation**, dans le dossier neuf de celle-ci. Deux téléchargements par version, et deux compilations sans rien de commun.
- **Le bac à sable a sa propre racine**, `/opt/sandbox`, tirée de `debian:bookworm-slim`, plutôt que le `/usr` du store : Node, le code du store et ses variables n'y sont pas. Pas une couche de plus dans la dernière étape, qui ne lance aucune commande : `gcc` y est apporté par des `COPY`.
- **La liste fermée vient d'abord, le bac à sable en plus** : un `build.rs` ou une macro hors liste sont refusés avant que rien ne tourne.
- **`run.command` est refusé à tous** (décision 72), au contrôle n° 10, et la lecture des releases sort de la soumission. Un `.exe` ou un script ne se distinguent plus. Reste pour Z3 : `attest.ts`, `release`, `asset` et `attestations` de `source.ts`, le service et la revue des binaires déjà publiés.
- **La preuve.** `store/tests/unit/compile.test.ts`, `tests/db/submissions.test.ts` (un faux GitHub, le faux `escouade-plugin`) et `scripts/image-test.mjs` (le vrai, dans le vrai bac à sable). `rust-build` donne `538b34fd…`, l'empreinte de `goldens.json`, deux fois, dans l'image sous Podman et sous Docker 27, et sous Windows par `STORE_TEST_ESCOUADE_PLUGIN`. Quatre choses que seuls le test d'image et la CI ont montrées : un `--proc /proc` que le runner refuse, d'où un bac à sable **sans `/proc`** (`argv[0]`, `LD_LIBRARY_PATH` et un `ld.lld` en script dans l'image, `docs/STORE.md` §3.3) ; `/usr/bin/cc` passe par `/etc/alternatives`, absent du `/etc` vide (l'image pose un lien direct) ; **le pid 1 de bubblewrap garde l'environnement de son démarrage, lisible en `/proc/1/environ`, malgré `--clearenv`** : `inSandbox` lance donc bubblewrap avec `PATH` seul ; et la nouvelle session sort la compilation du groupe de process (la mémoire se lit sur l'arbre).
- **Il reste, pour la production** : sous le Docker par défaut de Coolify, aucun `run.wasm` ne compile, et chaque version attend en `error`. ESC-356 (A6b) écrit les deux profils qu'il faut, `store/deploy/seccomp-bubblewrap.json` et `store/deploy/apparmor-bubblewrap`, essayés sur une VM Ubuntu 24.04 ; leur mode d'emploi est dans `docs/STORE.md` §3.3, et le geste sur l'hôte reste à faire par l'utilisateur. Ce ticket ne touche pas à Coolify. Les chemins surveillés de Coolify ne changent pas de ce fait : ceux du §1.3 de `docs/STORE.md` restent à ajouter à la main.
- **Il reste, pour la première main** : les manifestes de `plugins/` gardent `workspace = "../../src-tauri"` et n'ont pas de `Cargo.lock` (§8.16). Ce ticket n'y touche pas : ils seront refusés au store, par `rust::check` (`package.workspace` sort du dossier), jusqu'à leur portage. Les trois crates sont sur crates.io en `0.1.0` depuis le 2026-11-05 : `cargo fetch --locked` peut les tirer dès qu'un plugin a son `Cargo.lock` (P1).

### 8.19 Ce que P1 a construit (ESC-344)

Linear, Trello, Jira et ClickUp sont les premiers plugins portés, en `2.2.0`, `escouade.min` `2.2.0`, `"run": { "wasm": "Cargo.toml" }`, `crate-type = ["cdylib"]`. Ils sortent d'`escouade-plugins` et du workspace de `src-tauri`. Le kit n'a pas changé, ni l'hôte, à une règle de test près (plus bas) : c'est ce que ce ticket devait éprouver.

**La forme : un plugin porté a deux compilations** (le point ouvert du §8.16, décision proposée au ticket).

- **Dans le dépôt**, contre `crates/`. `plugins/Cargo.toml` est un second workspace, qui ne tient que les plugins portés, avec le même `[patch.crates-io]` et le profil d'un module. Son `Cargo.lock` est commité. `plugins/rust-toolchain.toml` est la copie de celui de `src-tauri`, que `scripts/plugins-ci.test.ts` garde égal. `plugins/.cargo/config.toml` partage `src-tauri/target` et range ses compilations intermédiaires dans `src-tauri/target/plugins` (`build-dir`, §8.23) : le cache de la CI, et les modules là où les tests les cherchent.
- **Au store**, contre crates.io. Chaque plugin porté a son propre `Cargo.lock`, dans son dossier, que cargo ignore dans le workspace et que `escouade-plugin build` lit, sur sa copie du dossier seul (§8.17). `sh scripts/plugin-lock.sh <id-court>` l'écrit : il part du `Cargo.lock` de `src-tauri`, ce qui garde les versions de la liste fermée, hors de tout workspace. **Aucun ajout à `rust_native.json`.**
- Ce que le store publiera est donc le second. Les deux modules diffèrent : même source, chemins autres.
- Le lock fixe le kit publié. Un changement de `crates/` n'atteint un plugin qu'après une publication et un nouveau `plugin-lock.sh`. Le dépôt peut donc tester un kit que le store ne compile pas encore : c'est `ci.mjs`, qui fait les deux, qui le voit.

**Porter un plugin.**

1. Dans son `Cargo.toml` : version, `crate-type = ["cdylib"]`, plus de `workspace`, plus de `reqwest` ni de `tokio`. Dans `src-tauri/Cargo.toml`, le retirer des dépendances, de `members` et de `default-members`, et de `ENTRIES` d'`escouade-plugins.rs`. L'ajouter aux `members` de `plugins/Cargo.toml`, et son crate du kit à `[patch.crates-io]` s'il n'y est pas (`escouade-acp`, par exemple).
2. Dans le manifeste : `version`, `escouade.min`, `run.wasm`, et `{account}` s'il joint une adresse que l'utilisateur tape.
3. Dans ses sources : `use escouade_plugin_kit::http::reqwest;`, `escouade_plugin_kit::time::sleep`, `escouade_plugin_kit::protocol::module!(run);`.
4. `sh scripts/plugin-lock.sh <id-court>`, puis `sh scripts/plugins-wasm.sh` et les tests de l'app.

**Un plugin du registre qui est un module.** L'installeur n'en a pas : l'amorce le donnera (Z2). D'ici là `builtin::folder` le prend, pour un build de développement et les tests seulement, dans `<target>/plugins-wasm/<id-court>/plugin.wasm`, que `scripts/plugins-wasm.sh` écrit (tous les `run.wasm` de `plugins/`, plus Codex tant qu'il est une `rlib`). Le manifeste lu en `Hand::Builtin` ne demande plus que son `Cargo.toml` soit dans ce dossier. **Une release sortie de `main` avant Z2 n'aurait pas ces quatre plugins** : leur page dirait que le manifeste n'est pas lisible, et `release.yml` ne l'arrête pas, puisque `seed.json` vide passe. Z2 passe avant la prochaine release.

Lu en `Hand::Builtin`, son `escouade.min` devient la version de l'app, comme sa `version` : `main` dit encore `2.1.2` et ces manifestes `2.2.0`, que le store lira.

**Les tests.** Les suites `plugins_{linear,trello,jira,clickup}_tests` n'ont pas changé d'une assertion sur le fond et jouent le module par le runtime. Deux choses les y aident :

- **Un faux service par son adresse.** Le cœur donne à un plugin `account.api`, une adresse `http://127.0.0.1:…` en test, que `net/fetch` refuserait à juste titre. `Fetcher::fake` laisse passer une URL de même origine qu'une route. Une route n'existe que dans les tests et, par `Bases::faked` (`ESCOUADE_TRELLO_API`, etc.), dans un build de développement : les e2e s'en servent, une release n'a pas ce chemin.
- **`{account}` pour Jira.** Son manifeste ne disait que `*.atlassian.net`, alors qu'un process joint le site que l'utilisateur tape. C'est un droit de plus dans la fenêtre, et la seule différence de permissions du portage. Sans lui, un site hors d'Atlassian, le faux service des e2e compris, serait refusé. **Un compte qu'on vérifie n'est pas encore enregistré** : le cœur prête donc son hôte au plugin (`Runtime::lend_host`), comme il lui prête le jeton. Le fetch `{account}` le voit le temps qu'un autre hôte le remplace. P2 s'en sert tel quel.

**La parité.** `tests/fixtures/plugins/parity/<nom>.json` tient, pour chacun, ce que le natif de `d3c0024c` a répondu à neuf appels et ce qu'il a demandé au service : méthode, cible, en-têtes et corps, nominal, 401, 404, 429 et méthode inconnue. `plugins_wasm_parity_tests.rs` rejoue le module sur le même faux et compare tout, à l'adresse du faux près. Aucun écart : ni dans les réponses, ni dans les requêtes. `ESCOUADE_PARITY_RECORD=1` et `ESCOUADE_NATIVE_PLUGINS=<programme>` les réécrivent, depuis un programme d'avant le portage.

**Une seule correction de fond, pour le banc.** Sans site, Jira disait une erreur non typée (`not a URL`) ; il répond maintenant comme à un jeton refusé (`-32401`), ce qui se répare. Le banc n'avait jamais joué Jira, qui n'avait pas de banc tant qu'il était un process.

**Les empreintes** (compilées depuis crates.io, refaites sur le kit `0.1.1`, §8.29, `escouade-plugin build`, deux fois sous Windows, deux dossiers sous Linux, `rust:1.98.0-bookworm`, Podman) :

| Plugin | SHA-256 | Taille |
| --- | --- | --- |
| Linear | `9f6d4c20d128deeafb8ca4e97ffa9bd078b5f327d2489e5b73922d19e53ce513` | 495 974 o |
| Trello | `42b0d2029c45c1d94ae057b381110b7793d306788d11cbbe26720dd8d8d371b7` | 480 578 o |
| Jira | `8e8c4a1c8d50dad22815baded6cda09e97856f2304eecb1ba01a6d75f876466d` | 524 404 o |
| ClickUp | `3a52c75b2d94d1945e5ff8cf72452fb9cbb1f73a77d2176dbd1a866392fa0192` | 569 666 o |

Les quatre sont identiques sous Windows et sous Linux, dans deux dossiers (`sh scripts/plugin-build-linux.sh plugins/<id-court> --first-party`), `canon` `4d3e106a…`. Le §8.17 avait mesuré `00f840b4…` pour Linear, le kit copié dans son dossier : avec le kit publié le module change, et c'est celui-ci que le store publiera. macOS n'a pas été essayé.

**INT-0.** `npm run bench:int0 -- --linear-only`, avec `ESCOUADE_NATIVE_PLUGINS` pour comparer au natif : 2 000 appels de `tickets/issues` (un `fetch`, un secret) sur un faux service sans délai, en release, sous Windows, avec d'autres compilations en cours sur le poste : **module 1,23 ms au 50e centile et 1,94 ms au 99e, natif 1,54 ms et 2,42 ms**. Le seuil (5 ms de plus au 99e centile) est tenu, et l'écart en faveur du module tient au bruit du poste et aux deux process à traverser : il ne dit pas que le WASM est plus rapide. Le test échoue au-delà du seuil. `npm run bench:int0` sans option rejoue aussi le reste.

**Pas fait ici.** Le job `reproductible` du §8.6.4, qui compare Windows et Linux à chaque `main`. Ni étiquette ni soumission (Z1). La compilation au store, dans `bubblewrap`, est celle d'A6 (§8.18).

### 8.20 Ce qu'A7 a construit (ESC-337)

- **La revue recompile un `run.wasm`.** `recompileRust` (`store/scripts/plugin-review/recompile.ts`) : même empreinte, « ce qui a été lu est ce qui tourne » ; autre empreinte ou autre `lock`, « Refuser » ; autre `rust`, `target` ou `canon`, pas recompilé. Le `sign` d'ESC-324 s'applique tel quel. Le détail est dans `docs/STORE.md` §4.2.
- **La première main sur le poste, un tiers en conteneur.** `review.ts` choisit par `firstPartyBench`, l'identifiant réservé et le dépôt de l'app. `container.ts` reprend les étapes du §8.18 dans quatre conteneurs : `prepare`, `fetch` avec réseau, `build` sans réseau, `read`. Le conteneur ne voit ni `HOME`, ni `~/.escouade-signing`, ni le dépôt, et **aucun dossier du poste ne lui est ouvert en écriture** : le module sort par la sortie standard d'un dernier conteneur. Le volume des outils est en lecture seule pendant `build` : un script ne laisse rien à la revue suivante.
- **Écart avec le §8.5.5 : pas un conteneur, quatre.** Le réseau de `fetch` et le `--network=none` de `build` ne tiennent pas dans un seul.
- **Pas de bubblewrap ici**, contrairement au store : sous un vrai Docker il ne tourne pas sans lever seccomp et AppArmor (ESC-356), et sur le poste de la personne qui relit le conteneur suffit.
- **Une autre image que le banc** : `node:22-trixie-slim` n'a pas `cc`. `rust:1.98.0-trixie` l'a, et sa glibc lance l'`escouade-plugin` d'Ubuntu 24.04. Il faut dans `ESCOUADE_PLUGIN_LINUX` un binaire qui a `fetch` : ceux d'avant A6 n'ont pas cette commande.
- **Les pistes de Rust** : `rust-leads.ts`, §8.5.5. Les imports réels d'un module sont toujours `wasi_snapshot_preview1` et `escouade.call` : le §8.4 passe par les méthodes du protocole. La piste compare donc aussi les noms de méthodes écrits dans le module à `permissions.network`, `files` et `commands` ; un nom calculé lui échappe, l'hôte le refuse quand même (`-32003`).
- **La preuve.** Le faux store sert `dev.exemple.rust-lines` et `dev.escouade.rust-first`, le faux moteur joue les quatre étapes (`plugin-review-rust.test.ts`, `review-admin.test.ts`). Le vrai, sous Windows 11 et Podman 5.8 : `rust-build` redonne `538b34fd…` en 9 s une fois les outils là ; `npm run plugin-review` jusqu'au rapport, en tiers comme en première main, à la même empreinte.
- **Le banc d'un `run.wasm` tiers suit la règle d'un `run.js`** : joué comme l'outil du poste. Tant que l'outil dit 2.1.2, un plugin qui demande `min: 2.2.0` échoue au scénario `manifest`, et le rapport le dit : la première main passe par `--escouade` (§8.7), et un tiers attendra la 2.2.

### 8.21 Ce que D1 a construit (ESC-342)

- **Le modèle Rust** : `sdk/create-escouade-plugin/templates/rust/`, `--kind rust`, dit la même chose que le gabarit TypeScript. Il dépend du kit par version, a son `Cargo.lock` et un `[workspace]` à lui : il se compile comme chez un tiers, depuis crates.io. Le nom du paquet est `escouade-plugin-<dossier>`, qui reste valide quel que soit le dossier, et la `lib` s'appelle `plugin`. `[target.'cfg(not(target_family = "wasm"))'.dev-dependencies]` porte tokio, pour les tests. Un test tient le lock sur la liste fermée.
- **Le contrôle** : aucun exemple n'est écrit dans une page. Les blocs de TypeScript et de Rust sont des lignes de fichiers compilés : `sdk/doc-examples/{ts,rust}`, deux plugins qui font la même chose, un modèle, ou un plugin de `plugins/`. `website/tests/docs.test.ts` tient chaque bloc à son fichier. `sdk/doc-examples/doc-examples.test.ts` refuse un bloc sans chemin, passe chaque morceau de manifeste à `validateText` et lance `tsc`. `sdk/scripts/template-ci.mjs` passe le modèle et `rust/` à `cargo test`, `cargo check --target wasm32-wasip1`, `validate` et au banc, et `ts/` à esbuild, `validate` et au banc. Le banc de `rust/` compile sur les crates de crates.io : ce job demande le réseau.
- **Trois pages** : `plugins/capabilities`, `plugins/calls`, `plugins/rust`. `plugins/api` garde la forme des messages, et sa section WASM s'appelle maintenant « Un programme en WASM ».
- **Vu en le faisant.** Un `cargo update` monte vite `serde` hors de la liste fermée : la page du kit le dit. `settings/get` n'est pas servi par le banc et répond `-32601` : la page de tests le dit, et le corriger reste à décider. `fs/list` refuse un chemin vide, `.` nomme la racine. Le `timer/fired` d'un plugin du kit ne passe pas par `Plugin::call` : `arrived` le garde pour `time::sleep`. Un plugin du kit n'appelle donc pas `timer/set` lui-même.
- **Écrit pour l'état de fin, en avance sur le code** : la revue qui recompile un `run.wasm` (A7) et le bac à sable en production. `docs/STORE.md` §3.3 dit où ils en sont.

### 8.22 Ce que P7 a construit (ESC-351)

isola est en `run.wasm`, en `2.2.0`, `escouade.min` `2.2.0`, sorti d'`escouade-plugins` et du registre des programmes natifs. C'est le premier plugin de la première main qui lance un programme par `proc/*`. La recette du §8.19 s'applique telle quelle, à trois choses près.

**Le plugin ne tient pas de chemin : le cœur lui prête le worktree.** `launch/*` nomme un worktree, et `dir` de `proc/spawn` n'est jamais un chemin, or isola n'est pas un harness : il n'a pas d'agent. `Env::call` pose le temps de l'appel une place sous un nom de l'appel (`worktree-<uuid>`, `Runtime::place_agent`), donne ce nom dans `agent` et la retire ensuite (`detach`) : `proc/spawn { dir: "agent", agent }`, sans racine nouvelle. Le plugin ne reçoit plus ni `program` ni `dir`. La place ne porte pas de projet : le cœur a jugé le projet dans `Launch::for_dir`, et un projet qu'on ferme doit encore voir ses services s'arrêter.

**Un projet privé n'a isola qu'une fois activé.** `Launch::decls` passait `private = false` : en process, isola touchait tout projet. Il suit maintenant le §3.3 et le §8.4.7 (`Runtime::reaches`). Sans activation, un worktree de ce projet n'est à personne, et le ticket part comme sans isola. **C'est le seul changement de comportement du portage**, et la décision proposée au ticket le dit.

**Ce que le kit `0.1.0` ne fait pas pour `proc/*`, et ce que le plugin fait à sa place.** Le kit n'a pas d'API de programmes : `proc/out` et `proc/exit` arrivent à `Plugin::call` comme des notifications, sans `id`, et le plugin garde lui-même ce qu'il reçoit, `proc/out` pouvant précéder la réponse à `proc/spawn`. Surtout, le réacteur (`Reactor::turn`) polle les tâches dans l'ordre : l'appel qui attend avant la notification qui le réveille. Rien ne suivrait `proc/exit`, et l'appel resterait endormi. Le plugin demande donc un minuteur vide après `proc/exit` : sa réponse est la ligne qui fait le tour suivant. **Changer le kit n'aurait rien réglé** : le `Cargo.lock` du plugin fixe le `0.1.0` publié, les tests du dépôt auraient passé sur `crates/` et le module du store serait resté bloqué. **Le kit `0.1.1` règle cela** (§8.29) : le minuteur vide est retiré.

**Rien d'autre ne se lance.** `isola_runs_what_its_manifest_names_and_nothing_else` (`plugins/procs.rs`) lit le manifeste réel et joue treize refus : un autre programme, `sh -c`, `cmd /c`, `isola up`, un argument de plus, un chemin, une casse, un mot pour deux. Rien n'a tourné, les treize sont au journal, et `isola ls --json` passe.

**`locate` décide de ce qui est installé.** `isola::cli` et `launchenv::find` cherchent comme l'hôte. Un `.cmd` qui s'appelle `isola` n'est pas lançable par le module : il ne compte plus comme installé, et le ticket part sans isola plutôt que d'échouer à `launch/ls`. Sous test seulement, `locate("isola")` rend le double, un script. Une release n'a pas ce chemin.

**La parité.** `tests/fixtures/plugins/parity/isola.json` tient ce que le natif de `6dbd1223` a répondu à quinze appels (`launch/ls`, `down`, `destroy`, `launch/up`, en français et en anglais, nominal, autre branche, sortie illisible, échec) et les commandes qu'il a lancées. `plugins_isola_tests.rs` rejoue le module par le runtime, sur le même double : aucun écart. Trois différences sont hors de cette transcription. Le texte d'un programme qui ne démarre pas vient de l'hôte (`no program isola on this machine`), et non du système. « *dossier* is not a folder » est dit par le cœur (`Env::services`). Les paramètres ont changé : `agent` à la place de `program` et `dir`, donc le message d'un appel sans paramètres aussi. Les 60 s (`proc/kill`, puis le même message) se jouent dans les tests du plugin, à l'horloge arrêtée.

**Le banc** `escouade-plugin test isola --first-party --escouade 2.2.0` : 6 réussis, 0 échoué, 5 sans objet. Il ne lance aucun `isola` : `launch/*` demande un worktree que seul le cœur prête.

**L'empreinte** (compilée depuis crates.io, refaite sur le kit `0.1.1`, §8.29, deux fois sous Windows, deux dossiers sous Linux, `canon` `4d3e106a…`) :

| Plugin | SHA-256 | Taille |
| --- | --- | --- |
| isola | `67c8b84a58aaedfc9ad35987e24f35b8d408a5a2d63dc561cf739ff1ef630573` | 239 368 o |

macOS n'a pas été essayé.

**Le vrai isola n'a pas été lancé.** Le ticket supposait un moteur de conteneur : isola n'en demande pas (« No Docker required », des process et un proxy). Il n'a **pas de release pour Windows** (0.5.0 : Linux et macOS), et le poste de ce portage est sous Windows. Podman 5.8.9 y répond, mais le lancement du binaire de la release dans un conteneur a été refusé par le contrôle automatique des permissions de l'agent, et ne s'est pas fait. Reste vérifié par lecture, dans `cmd/ls.go` de la 0.5.0 : les champs de `ls --json` sont ceux du double (`worktree`, `service`, `port`, `status`, `pid`, `url`, `direct_url`), et une liste vide sort en `null`, que le plugin lit. **À faire sous Linux ou macOS avant Z1** : `isola up`, puis « ▶ Tester » et « Tout arrêter » dans l'app.

**Pas fait ici.** Ni étiquette ni soumission (Z1). Comme pour P1, une release sortie de `main` avant Z2 n'aurait pas ce plugin : il ne tourne que dans un build de développement ou les tests.

### 8.23 Ce que P5 a construit (ESC-349)

OpenCode est un module : `2.2.0`, `escouade.min` `2.2.0`, `"run": { "wasm": "Cargo.toml" }`, retiré d'`escouade-plugins` et du workspace de `src-tauri`, membre de celui de `plugins/`. Ses trois tâches tokio sont devenues des événements de l'hôte : il n'y a plus ni `tokio` ni `reqwest` dans la crate, sur aucune cible.

**Pourquoi pas le réacteur du kit.** Celui de `crates/escouade-plugin-kit` crée une tâche par appel du cœur, et rien d'autre : la lecture du flux, qu'aucun appel ne commence, n'y a pas de place. Il n'a pas non plus de client de `net/stream`. Et le kit `0.1.0` est publié : y ajouter `spawn` aurait demandé une publication avant de pouvoir compiler le module du store. `src/host.rs` est donc un réacteur d'environ 300 lignes, sur `escouade-plugin-protocol` seul : `spawn`, `request`, `fetch`, `stream`, `sleep`, `timeout`. Les `async fn` du client gardent leur forme : un `tokio::spawn` devient `host.spawn`, `reqwest` devient `net/fetch`. **Le kit n'a pas changé, ni l'hôte**, à une méthode de test près (`Runtime::crash_instance`). P6 a repris ce code, et le kit `0.1.1` l'a fait monter : `plugins/opencode/src/host.rs` n'existe plus (§8.29).

**Le tour.** `Reactor::turn(lignes) -> lignes` est du code ordinaire : `cargo test` le joue sans module, ligne à ligne (`src/tests.rs`, `src/host.rs`). Chaque tour sonde toutes les tâches ; comme rien d'autre qu'un tour ne les réveille, un changement qu'une autre tâche attend dans le même tour (le flux qui s'ouvre, le catalogue lu) passe par `stir`, qui refait un passage. Le crédit de `net/stream` est rendu une fois par tour et par flux, pas une fois par morceau.

**Sa crate reste liée au cœur** (§8.2) : `crate-type = ["cdylib", "rlib"]`, `escouade-plugin-opencode` restant une dépendance par chemin de `src-tauri`, pour `translate` et `rules`, jusqu'à P8. Cargo accepte qu'un chemin soit membre d'un autre workspace, et le `[patch.crates-io]` de `src-tauri` s'y applique. `cargo build` de l'app produit donc aussi une DLL dont personne ne se sert. `getrandom` ne reste que hors de WASM, pour ses tests : le module tire le mot de passe par `random_get`. **Aucun ajout à `rust_native.json`.**

**`local:shared`.** `SharedLaunch::listens` vaut `opencode server listening on http://{host}:{port}`. Rien ne change au lancement : le cœur lit le port, et le module ne peut joindre que celui-là, tant que le process vit.

**Les limites, nouvelles pour OpenCode.**

- `limits` : 60 s par requête, soit le `REQUEST_LIMIT` du natif ; 16 Mio par réponse, parce qu'une instance a 64 Mio et qu'une réponse y passe en quatre copies (l'entrée, la ligne, le JSON, le texte). `a_long_session_is_copied_within_the_memory_of_the_module` copie une session de 15 Mio. Plus lourde, la copie échoue en le disant, là où le natif n'avait pas de limite. Seule `GET /session/:id/message` peut s'en approcher.
- 8 flux par plugin (§8.4.3), donc 8 serveurs à la fois. Le neuvième n'a pas de flux : son agent dit au bout d'une minute que le flux ne répond pas.

**Une seule différence avec le natif, voulue.** Le dernier agent parti, ou le serveur changé d'adresse, le flux est annulé tout de suite (`net/cancel`). Le natif le gardait ouvert jusqu'à l'arrêt du serveur, sans s'en servir.

**La parité.** `tests/fixtures/plugins/parity/opencode.json` tient ce que le natif de `6dbd1223` a fait, sur le vrai `fake-opencode.mjs`, en 21 étapes : ouverture, conversation, raisonnement, tâches, sous-agent, deux appels, fichier, commande acceptée puis refusée, question, interruption avec un message gardé, plan, compaction, renommage, requête refusée, coupure du flux entre deux tours et dans un tour, copie, reprise, dossier lâché, agent inconnu. Pour chaque étape : les réponses, les éléments dans leur ordre, les requêtes au serveur. `opencode-again.json` tient un plugin relancé après la fin d'un tour. **Aucun écart.** Le faux serveur gagne `/fake/readers` et `/fake/cut`, hors de l'API d'OpenCode. Les ids, le port, le dossier et les durées sont normalisés ; les requêtes d'une même étape sont triées, le natif les envoyant côte à côte.

**Les tests de l'app** n'ont pas changé d'une assertion : `opencode_tests.rs`, `fence_tests.rs`, `engines_tests.rs` et le banc passent par le module. Deux tests tuaient le process par son pid : un module n'en a pas, ils passent par `Runtime::crash_instance`.

**L'essai réel, sur ce poste** (OpenCode 1.19.3, Ollama, `qwen3-coder:30b-32k`) : `real_opencode` (conversation, permission, question, tâches, sous-agent), `real_fence` (le `Bash` et le `Read` d'un fichier du projet privé refusés par les règles de la session, son contenu absent de la conversation) et `real_opencode_view` passent. `real-harnesses.spec.ts`, joué pour OpenCode, passe aussi. Sur un poste saturé, `real_fence` et `real_opencode_view` ont attendu le modèle plus de 10 minutes sans réponse ; seuls, ils passent en 2 minutes.

**INT-0.** `npm run bench:int0 -- --opencode-only`, avec `ESCOUADE_NATIVE_PLUGINS` pour comparer. `int0_opencode_stream_bench` mesure ce que la lecture du flux coûte, hors du banc de flux, qui passe par `agent/frames` et ne voit pas `net/stream` : un vrai socket, un événement écrit, son élément lu à la sortie du plugin. En release, sous Windows, d'autres compilations en cours :

| | Module | Natif |
| --- | --- | --- |
| Un événement à la fois, 2 000 fois : 50e centile | 0,038 ms | 0,033 à 0,055 ms |
| 99e centile | 0,11 ms | 0,09 à 4,39 ms |
| 20 000 deltas d'un coup | 17 550 à 21 690 lignes/s | 109 600 à 149 500 lignes/s |
| 20 fois un vrai tour (`full-turn.jsonl`, 23 360 lignes) | 21 800 à 22 140 lignes/s | 122 500 à 149 100 lignes/s |

Deux mesures, les fourchettes vont de l'une à l'autre. **Le seuil de latence est tenu** : le 99e centile n'a pris que 0,02 ms de plus sur la mesure la plus calme. **Le JSON de `net/stream` se voit au débit** : le module lit 6 à 7 fois moins vite que le natif, et reste à 8 fois le seuil de 2 200 lignes/s. Le test vérifie que chaque élément est bien celui de son événement, et échoue sous le seuil.

**L'empreinte** (compilée depuis crates.io, refaite sur le kit `0.1.1`, §8.29, `escouade-plugin build`) :

| Plugin | SHA-256 | Taille |
| --- | --- | --- |
| OpenCode | `eb47097b3696d6d40fa01c46ad175f85e5cd21c7f4925ef1af3fb39878277bdf` | 756 021 o |

Identique sous Windows, compilé deux fois, et sous Linux, dans deux dossiers (`sh scripts/plugin-build-linux.sh plugins/opencode --first-party`, `rust:1.98.0-bookworm`, Podman), `canon` `4d3e106a…`. macOS n'a pas été essayé. Le banc `--first-party --escouade 2.2.0` passe sur ce module : 6 réussis, 0 échoué, 5 sans objet.

**Un `build-dir` pour le workspace de `plugins/`.** OpenCode est une crate des deux workspaces, sous le même nom et le même hash, et ils partageaient tout `src-tauri/target` : un `cargo test` natif depuis `plugins/` écrasait l'`rlib` que l'app lie, compilée contre l'autre `serde_json`, et l'app ne compilait plus (« expected `Value`, found `Value` »). `plugins/.cargo/config.toml` garde `target-dir`, où sortent les modules, et met les compilations intermédiaires dans `target/plugins`. Il en ira de même des harness que le cœur lie encore.

**Le réacteur et l'ordre des tâches.** Celui du kit `0.1.0` sonde chaque tâche une fois par tour : une tâche qui attend ce qu'une plus récente vient de poser dort jusqu'à la ligne suivante (vu par P7). `host.rs` n'y est pas exposé : `run` refait un passage tant qu'une tâche est née ou que `stir` a été appelé, et les deux états qu'une autre tâche attend (le flux ouvert, le catalogue lu) l'appellent. Les morceaux du flux sont rangés avant le passage. `a_task_waiting_for_another_goes_on_in_the_same_turn` le prouve.

**Pas fait ici.** Ni étiquette ni soumission (Z1). La vue OpenCode et les comptes restent liés à la crate (P8). Le banc de flux complet (`fluxbench`, par `agent/frames`) n'a pas été rejoué : le disque du poste était plein, et sa compilation release a dû être retirée.


### 8.24 Ce que P4 a construit (ESC-348)

Antigravity CLI est le premier harness porté de bout en bout, en `2.2.0`, `escouade.min` `2.2.0`, `"run": { "wasm": "Cargo.toml" }`, `crate-type = ["cdylib", "rlib"]`. Il sort d'`escouade-plugins` et des membres du workspace de `src-tauri`. La `rlib` reste pour le cœur, dont la « Vue Antigravity » lit `words.rs` et, en test, `translate.rs` : le chemin `../plugins/antigravity` reste dans ses dépendances jusqu'à P8.

**L'attente du transcript.**

- Rien n'attend dans un tour. `Client::frames` lit ce qu'il peut. Une ligne dont le transcript manque (`needs` : la fin d'un texte, ou la fin d'une écriture) est gardée dans `held`, avec tout ce qui la suit, dans l'ordre : `agent/frames` répond ce qui passait avant elle.
- `lib.rs` pose un seul minuteur pour tout le plugin dès qu'un agent attend (`timer/set` en notification, sans `id` : quel minuteur a sonné n'importe pas, et un `serve` de machine à états ne lit pas une réponse). À `timer/fired`, chaque agent relit le transcript (`tick`), toutes les 40 ms, 15 fois par lot : les 600 ms du natif. Ce que cela laisse passer sort en `agent/items`.
- **Plus rien n'est perdu de ce que le natif lisait** : la patience épuisée, les lignes passent sans le transcript, comme le natif.
- Un lot qui arrive pendant l'attente se range derrière et `frames` rend vide. `interrupt` et un message hors tour lisent aussitôt ce qui est gardé (`hurry`) : il passerait sinon derrière ce qu'ils font. Un message pendant un tour va dans sa file, comme avant, et ne presse rien.
- Une sonnerie arrivée dans le tour même qui a commencé à garder ne relit rien (`fresh`) : `turn::write` sort avant les réponses du tour, et elle ferait passer des `agent/items` devant la réponse de `agent/frames`.

**Les fichiers.** Le manifeste déclare `agentHome` en lecture et `limits.fileBytes` à 4 Mio : c'est une ligne de plus dans la fenêtre d'installation (« Lit le dossier de compte de tes agents »). Chaque agent d'Antigravity a son dossier (`agent_home_var`), la racine existe donc toujours. Tout passe par `host.rs`, parce que `turn::read_file` n'envoie pas l'agent et que l'hôte refuse alors une racine d'agent : **P3 a le même trou pour Codex**, dont `read_file` rend `None`. Le module appelle `turn::host_call` avec `agent`. Aucune écriture : `Item::File` et les fichiers du lancement restent au cœur, qui les vérifie.

**Ce que le cœur a gagné, tous pour un plugin de harness.**

- `agent/items` n'arrivait qu'aux agents d'un process partagé. Il arrive maintenant à tout agent (`hear_what_it_tells`). Pour un programme à lui, il passe par `pump` (`Heard::Items`), derrière la réponse du lot d'avant : sinon le fil qui lit le plugin aurait pu le livrer avant elle.
- **`fs/read` prend `tail`.** Un fichier au-delà de la limite rend sa fin et `cut`, à partir d'un caractère entier, au lieu d'échouer. Le transcript grandit sans fin, et le natif le lisait entier : sans cela, un agent aurait perdu les arguments de ses appels passés 4 Mio de conversation. Le schéma, `protocol.ts` et sa copie du site suivent. **L'écart :** le module ne cherche une étape que dans les 4 derniers Mio.
- **Le dossier de l'utilisateur.** `home_env` gardait `GIT_CONFIG_GLOBAL`, `CARGO_HOME` et `RUSTUP_HOME`, que seul l'hôte peut connaître. `words::tools_env` en fait une fonction à part : vide en module, et `Agy::spawn` les ajoute à l'environnement. `AGY_CLI_DISABLE_AUTO_UPDATE` reste posé par `home_env`, donc par le plugin, à chaque lancement.

**Une limite.** Si l'instance plante pendant l'attente, ce qu'elle gardait est perdu, y compris un `result` : `agent/frames` a déjà répondu, et le cœur ne le redonne pas. Le tour reste alors ouvert jusqu'au message suivant ou à l'arrêt. Il faut un plantage dans une fenêtre de 600 ms ; ce n'est pas traité.

**L'empreinte** (compilée depuis crates.io, refaite sur `escouade-acp` `0.1.2`, §8.30, `escouade-plugin build`, deux fois sous Windows, deux dossiers sous Linux, `rust:1.98.0-bookworm`, Podman, `canon` `4d3e106a…`) : `c215c7e1cb36a45bcef5144eb146289479c9edeb862ee513fa609b1368aea531`, 981 294 o, la même partout. C'est celle du module que le store publiera. macOS n'a pas été essayé.

**INT-0.** `ESCOUADE_BENCH_ONLY=agy npm run bench:int0 -- --flux-only`, 20 passes au plus vite sur les six flux d'`agy`, en release, sous Windows, d'autres worktrees compilant : **médiane 104 µs, 99e centile 1 058 µs, 6 231 lignes/s**, 280 lignes sans événement sur 1 380 comme avant. Le process du §4 donnait 161 µs, 1 015 µs et 4 739 lignes/s, le cœur 36 µs, 1 010 µs et 9 462 lignes/s. Le seuil (5 ms de plus au 99e centile) est tenu : +48 µs sur le cœur. Le flux enregistré n'a pas de dossier d'agent, il ne mesure donc pas la lecture du transcript, dont le coût ne compte que par ligne qui le lit.

**Les preuves.**

- `plugins_wasm_agy_tests.rs` : les treize flux d'`agy` 1.4.9 de `tests/fixtures/agy/`, chacun avec un transcript fabriqué à côté (raisonnement, appels, une écriture refusée, le plan), cartes répondues, joués par le module et comparés à `tests/fixtures/plugins/parity/antigravity.json`, que le natif du commit `6dbd1223` a écrit. L'environnement ne diffère que des trois variables du dossier de l'utilisateur, que le test retire.
- Le même fichier : un transcript en retard arrive derrière, par `agent/items`, sans tenir le tour ; un transcript jamais écrit lâche après sa patience.
- `the_fake_agy_passes_the_bench_with_its_transcript_late` : le banc des harness sur le faux `agy`, chaque ligne de transcript écrite 150 ms en retard (`FAKE_AGY_LATE_MS`), par le vrai `spawn_plugged`.
- `plugins/antigravity/src/tests.rs` : `tick` à la main, sans minuteur.

### 8.25 Ce que P2 a construit (ESC-345)

GitHub, GitLab, Bitbucket, Gitea et Azure DevOps suivent la marche à suivre du §8.19 : `2.2.0`, `escouade.min` `2.2.0`, `"run": { "wasm": "Cargo.toml" }`, hors d'`escouade-plugins` et du workspace de `src-tauri`, dans celui de `plugins/`, un `Cargo.lock` à chacun. L'hôte n'a pas changé, le kit non plus. **Aucun ajout à `rust_native.json`.** Ce que P1 ne pouvait pas voir :

**Les redirections.** L'hôte rend les 3xx au plugin (`Policy::none`), pour juger chaque adresse. La façade `http::reqwest` du kit ne les suit pas, là où reqwest le faisait sans rien dire : un dépôt renommé (301, 307), une instance passée en https ou un journal de GitHub Actions (302) auraient fini en erreur. Le kit publié ne bouge pas sans une publication, chaque hébergeur a donc `src/redirect.rs`, la règle de reqwest à une différence près : dix sauts, un GET sans corps après 301, 302 et 303, le même appel après 307 et 308. Les identifiants restent sur le même hôte, port compris, et jamais de https vers http. **Un corps ne quitte pas le service** : si un saut sort de cet hôte, ou passe de https à http, et que l'appel a encore son corps, quel que soit le 3xx, `follow` s'arrête, rend ce 3xx comme dernière réponse et n'envoie rien à l'autre hôte ; `success` en fait une erreur qui dit le statut. reqwest renvoyait le corps : `*.blob.core.windows.net` est ouvert à tout compte Azure, et un commentaire ou un merge n'a rien à y faire. Un appel sans corps, ou dont un 301, 302 ou 303 sur le service a retiré le corps, suit toujours, sans les identifiants. Les redirections de lecture ne changent pas. **Monté dans le kit `0.1.1`** (§8.29) : `src/redirect.rs` n'existe plus.

**Ce que joint un saut.** Il est jugé comme tout `net/fetch`. GitHub déclare donc où il range ses journaux : `*.actions.githubusercontent.com` et `*.blob.core.windows.net`, joints sans le jeton. C'est une permission large, mais elle ne porte aucun droit : le jeton n'y va pas, et rien de ce qu'on y lit ne sort ailleurs que dans le journal affiché. **Limite connue** : une Enterprise Server qui garde ses journaux sur un autre hôte qu'elle-même n'y arrive pas, puisqu'un hôte d'instance ne vient que du compte. Le refus se dit, le reste marche. Azure DevOps ne suit pas sa page de connexion (`_signin`, `login.microsoftonline.com`), hors de ses hôtes : la redirection se lit comme un jeton refusé, et c'est ce que le natif finissait par dire.

**Les limites.** `"limits": { "fetchSeconds": 15, "fetchBytes": 8388608 }`. 15 s est le délai qu'avait le process (`http::Network::client`). Pour la taille, le plafond de l'hôte (32 Mio) ne dit pas ce qu'un module tient : 64 Mio de mémoire et trois copies d'une réponse (la ligne, le JSON, la réponse au cœur). Mesuré sur GitHub : 15,3 Mo passent, 16,6 Mo tuent le module (`the plugin ended before it answered`). 8 Mio laissent de la marge à ce qu'il garde par ailleurs (500 réponses conditionnelles). Un journal plus long, que le process lisait, se dit trop long (`an answer over 8 MiB`). `net/stream` lèverait cette limite, mais la façade du kit ne le propose pas.

**`rustc-canon` et Windows.** Il passe un `--remap-path-prefix` par fichier source d'un crate. Les fichiers de `futures-util` dépassent les 32 767 caractères d'une ligne de commande (`os error 206`), dans le seul dossier d'Azure DevOps : c'était son seul usage, deux `join_all`. Il a maintenant `src/joined.rs`, vingt lignes. Corriger `canon` par un fichier d'arguments changerait son empreinte, donc celles déjà notées : **ce n'est pas fait, et un portage qui tirerait une aussi grosse dépendance le rencontrera.**

**Le banc.** Il envoie un compte vide. Gitea n'avait pas d'adresse par défaut et Azure Boards visait `https:///_apis` : Gitea répond maintenant depuis Codeberg, Azure comme à un jeton refusé (`-32401`). Dans l'app, un compte a toujours son adresse.

**Les tests.**

- Les suites `forge/{github,gitlab,bitbucket,gitea,azure}_tests`, `integrations/{github,gitlab,azure}_tests` jouent le module par le runtime, sans changer d'assertion. Une exception : le journal redirigé va maintenant vers un hôte du manifeste, par une route (`builtin::served_forge_on`), puisqu'un faux serveur sur une autre origine serait refusé à juste titre.
- Les tests qui appelaient un client directement (son allure, ce qu'il attend, `merge_options_of`, `real_azure_*`) ne peuvent plus lier un `cdylib`. Ils sont dans la crate : `src/pulls_tests.rs`, `merges_tests.rs`, `boards_tests.rs`, avec `plugins/fake_service.rs`, la part de `integrations/fake.rs` qui leur sert, pris par `#[path]` sous `cfg(test)` : le store ne le voit pas.
- `Bases::faked` s'étend à `ESCOUADE_GITHUB_API` et `ESCOUADE_AZURE_DEVOPS_API`, posées au démarrage du cœur : les e2e et `fake-trackers` passent par là, en build de développement. `ESCOUADE_BITBUCKET_API` disparaît : un module n'a pas d'environnement, et aucun essai ne s'en servait.

**La parité.** `plugins_wasm_hosts_parity_tests.rs` joue, sur le module et sur le programme de `6dbd1223`, tout `forge/*` sur les fixtures de `tests/fixtures/forges/` (ESC-56, 57, 58, 217 à 220, 280), puis une lecture redirigée, une écriture redirigée, 404, 401 et 429, et neuf appels de GitHub Issues, GitLab Issues et Azure Boards : huit transcriptions dans `tests/fixtures/plugins/parity/<nom>-host.json` et `<nom>-source.json`. Mêmes réponses et mêmes requêtes, méthode, cible, en-têtes et corps, à deux différences près, que le test écarte : reqwest ajoutait un `Referer` en suivant, et Bitbucket compte un saut dans son budget de l'heure, ce qui est plus juste. Certains pas restent des 404 ou des refus : ils ne prouvent que l'égalité des deux côtés, pas le nominal, que les suites ci-dessus couvrent.

**Les empreintes** (compilées depuis crates.io, refaites sur le kit `0.1.1`, §8.29, `escouade-plugin build --first-party`) :

| Plugin | SHA-256 | Taille |
| --- | --- | --- |
| GitHub | `6d2101591831738033c4df6f02318760d4dfdd4644c91892c89a53a53ed50ef0` | 622 270 o |
| GitLab | `a984ce48ed7551b75b1e2666cd5707c6f5732162f82173e511666d5fd032eb7f` | 607 201 o |
| Bitbucket | `e0584248908e12e72b5a0c130fc2a9257c061cb05e8d3e9f5714dd00dddbd5d1` | 569 705 o |
| Gitea | `8e97c0d31ce96844d9d6adb4cfa3b87cf6bbd72533d6f4cae87d38408ec86572` | 545 132 o |
| Azure DevOps | `edc70b1076ff077ddd53af11ffeaf373f9bc9e1730c8e195e054dd8dd467156e` | 725 849 o |

Les cinq sont identiques sous Windows et sous Linux, dans deux dossiers (`sh scripts/plugin-build-linux.sh plugins/<id-court> --first-party`, `rust:1.98.0-bookworm`, Podman), `canon` `4d3e106a…`. macOS n'a pas été essayé.

**Pas fait ici.** Ni étiquette, ni soumission, ni crates.io. Une release sortie avant Z2 n'aurait plus ces cinq hébergeurs (§8.19). INT-0 n'a pas été rejoué : un hébergeur fait un `fetch` et un secret par appel, comme Linear, mesuré au §8.19.

### 8.26 Ce que P3 a construit (ESC-346)

Codex, Copilot CLI, Cursor CLI et Gemini CLI sont des modules, en `2.2.0`, `escouade.min` `2.2.0`, `"run": { "wasm": "Cargo.toml" }`, sortis d'`escouade-plugins` et du workspace de `src-tauri`. Le cœur les lance toujours et ne lie plus que Codex et Copilot, pour leurs vues.

**Ce qui a dû changer, en dehors des cinq endroits du prototype.**

- **`escouade-acp` passe à `0.1.1`.** Tous les quatre en dépendent : la version change avant de pouvoir écrire leurs `Cargo.lock` (plus bas). P3b (ESC-347) a sa part dans la même publication.
- **Le service ACP entre dans la crate** (`escouade_acp::plugin`). Cursor, Gemini et Mistral dépendaient de `plugins/acp` par un chemin, qu'un dossier seul ne compile pas. `plugins/acp` n'est plus qu'une façade, jusqu'à P3b. La version d'Escouade se lit dans `greeting()` à l'arrivée d'un agent : à la création du service, `initialize` n'est pas passé en module.
- **`turn::read_file` ne suffit pas.** Il n'envoie pas d'agent, et l'hôte refuse un chemin d'agent sans lui. `escouade_acp::plugin::agent_file(agent, path)` l'envoie : Codex y lit `config.toml`, Gemini son plan. Le protocole ne change pas, il n'y a rien à publier de ce côté.
- **`fs/gitProtected`, une nouvelle méthode de l'hôte.** La garde qui laisse passer `git add` et `git commit` sans carte lit `.git` et lance `git rev-parse`, ce qu'un module ne peut pas. Le prototype ne l'avait pas vu : ses flux enregistrés ne passent pas par cette garde, et le module aurait posé la question à chaque commit, sans erreur. `git::accepts` demande à l'hôte en module, par `escouade.call`, à chaque demande de permission comme avant. Elle prend le dossier d'un agent de ce plugin, demande la racine `agentRepo`, et ne rend qu'un booléen. Refusée ou trop lente (1 s, un tour en a 2), la réponse est non : la carte s'affiche. Une entrée de `proc/*` n'allait pas : c'est asynchrone, et il aurait fallu `git *` en commande déclarée (§8.4.7).
- **Les chemins.** En module, `cfg!(windows)` est faux sous Windows. `names()`, le garde-fou de la clôture de Cursor et Gemini, serait devenu sensible à la casse sous Windows : `c:\prive` n'aurait plus couvert `C:\Prive`. En wasm, il décide d'après le chemin. Le reste de Codex et Copilot compare des chaînes, déjà insensibles aux barres et à la casse.
- **Le dossier de compte par défaut**, que le §8.15 laissait à P3. Sans variable de compte, `AgentPlace.home` prend celui du harness : `CODEX_HOME` ou `~/.codex`, `~/.copilot`, `~/.cursor`, et `~/.gemini` plutôt que `~`, qui aurait donné tout le dossier de l'utilisateur.
- **Copilot perd `url`**, remplacé par 20 lignes (`words::host_of`). La crate tirait `displaydoc`, `yoke-derive`, `zerofrom-derive`, `zerovec-derive`, `synstructure` et `syn` 2, hors de `rust_native.json`. Une adresse que cette fonction ne sait pas lire n'a pas d'hôte, et sa permission vaut pour une fois : c'est le seul écart possible, sur une adresse non ASCII. **Aucun ajout à `rust_native.json`.** Le module passe de 903 291 à 691 448 o.

**Repris d'ESC-347** : `limits.memoryBytes` et `limits.turnSeconds` (les quatre manifestes demandent 256 Mio et 10 s, un message de 18 Mio ferait tomber un module à 64 Mio, et avec lui tous les agents du harness), et le précompilé gardé à côté des modules de `plugins-wasm/` en debug et en test.

**Les modules du registre se précompilent à leur première activation**, comme ceux de l'amorce : en debug, les huit à chaque démarrage sur des données neuves faisaient passer « Nouveau projet » de quelques secondes à 12 s et 23 e2e dépassaient leurs 15 s.

**Les permissions, désormais dites à l'installation.** Codex : `agentRepo` et `agentHome` en lecture. Copilot : `agentRepo`. Gemini : `agentHome`. Cursor : aucune. Avant, un process pouvait tout, sans le dire.

**Deux formes de crate.** Cursor et Gemini sont des `cdylib`, comme P1. Codex et Copilot sont `["cdylib", "rlib"]` : la vue du cœur lit leur traduction jusqu'à P8. Ils sont membres du workspace de `plugins/` et restent une dépendance `path` de `src-tauri`, ce que cargo accepte : une dépendance n'a pas à être membre. `src-tauri/Cargo.toml` les garde donc, sans les `members`. `scripts/plugins-ci.test.ts` accepte les deux formes.

**Les compilations de `plugins/` sont à part.** Un crate avec un `cdylib` n'a pas d'empreinte dans le nom de ses fichiers : `libescouade_plugin_copilot.rlib` était le même pour les deux workspaces, et l'app reprenait la `rlib` bâtie par `plugins/` contre un autre `serde_json` (E0308, « two versions of the same »). `plugins/.cargo/config.toml` garde `target-dir` et ajoute `build-dir = "../src-tauri/target/plugins"`, la forme de P5 (ESC-349) : les modules restent où `scripts/plugins-wasm.sh` et la CI les cherchent.

**La parité.** `plugins_harness_parity_tests.rs` remplace la comparaison en direct de Codex, puisque le natif n'existe plus. `tests/fixtures/plugins/parity/<court>.jsonl` tient ce que chaque natif a répondu, enregistré sur le programme de `6dbd1223` : 8 lancements (modes, lecture seule, clôture, reprise, serveur d'Escouade), puis les fichiers de `tests/fixtures/<harness>/` joués ligne à ligne, un message envoyé là où le client de l'enregistrement parlait, une permission acceptée là où il répondait, et tous les appels du contrat à la fin. Soit 377 pas pour Codex, 999 pour Copilot, 717 pour Cursor et 163 pour Gemini, **sans écart**.

- Deux enregistrements successifs du natif sont identiques.
- Un enregistrement répond aux ids de son client, pas à ceux-ci : la relecture donne à chaque réponse l'id que le plugin a écrit pour cette méthode. Sans ça, aucune session ne s'ouvre et on compare du vide : Copilot passait de 50 pas à 999.
- Copilot demande `auth.getStatus` et d'autres que ses enregistrements n'ont pas : `Harness::answers` y répond.
- Trois choses ne se comparent pas : le début des ids des requêtes (`instance_id`), l'ordre de `writes` (une table de hachage), et `clientInfo.version` de Codex, celle de sa crate, seul écart réel : `2.2.0` au lieu de `1.0.0`.
- Le plan de Gemini nomme un fichier du poste de l'enregistrement, que le natif lisait pour de vrai. Le chemin est faussé pour les deux côtés : la lecture du plan se vérifie dans l'e2e.
- `ESCOUADE_PARITY_RECORD=1 ESCOUADE_NATIVE_PLUGINS=<programme d'avant>` les réécrit. Il faut un programme d'avant le portage : celui-ci n'a plus ces entrées.

**INT-0.** Le banc de flux (`fluxbench::int0_stream_bench`) en release, sous Windows, en trois passes (5, 20 et 20 tours) : la seule dernière sur un poste calme, les deux autres avec d'autres agents qui compilaient. Le 99e centile est donné passe par passe, comparé au process mesuré à la sortie de chaque harness (§4.6). Claude Code, qui reste un process ici, sert de témoin du bruit : 2,4, 2,0 et 1,5 ms au plus vite pour 0,84 ms le jour de sa mesure, et son long tour à son rythme, à 55 et 81 ms sur la première et la dernière passe, pour 32 ms.

| 99e centile | Process | Module, passes 1, 2, 3 | Débit, passes 1, 2, 3 |
| --- | --- | --- | --- |
| Codex, à son rythme | 31,5 ms | 31,7, 31,3, 31,6 ms | |
| Copilot, à son rythme | 30,9 ms | 46,5, 62,0, 32,1 ms | |
| Cursor, à son rythme | 32,2 ms | 33,0, 58,5, 54,4 ms | |
| Codex, au plus vite | 3,7 ms | 9,7, 4,3, 10,1 ms | 8 416, 14 660, 17 841 lignes/s |
| Copilot, au plus vite | 7,5 ms | 27,7, 15,2, 26,5 ms | 18 097, 15 795, 20 080 lignes/s |
| Cursor, au plus vite | 7,2 ms | 6,7, 6,4, 2,6 ms | 9 576, 14 699, 23 712 lignes/s |
| Gemini, au plus vite | 0,8 ms | 30,6, 5,9, 7,2 ms | 4 032, 5 415, 9 084 lignes/s |

- **Le débit tient partout** : plus de 2 200 lignes/s, et il dépasse le process pour Codex et Cursor sur la meilleure passe.
- **À son rythme**, ce que le seuil vise, Codex tient à chaque passe et Copilot sur la passe calme (+1,2 ms). Cursor ne tient que sur une passe sur trois (+0,8 ms) : son 99e centile porte sur 5 lignes de 471, et le témoin monte autant. **Ce n'est pas tranché : à remesurer sur un poste vide.**
- **Au plus vite**, Copilot et Gemini restent au-dessus de 5 ms de plus, et Codex deux fois sur trois, même sur la passe calme. Leur médiane ne bouge pas : ce sont quelques lignes. Ce rythme n'arrive pas en vrai, et 32 ms au moins séparent déjà deux envois de la conversation, mais **l'écart n'est pas expliqué**.
- Gemini n'a pas de flux à son rythme au banc.

**Second passage.** `escouade-acp` `0.1.1` publiée, les quatre plugins ont leur `Cargo.lock` et leur empreinte : §8.29.

**Les harness réels.** Cursor CLI 2026.10.22 (`real_cursor`) et Copilot CLI 1.0.98 (`real_copilot_through_its_plugin`) ont tourné, par leur module, sur les comptes du poste. **Pas rejoués** : Codex, le poste n'a que la 0.144.5, refusée (0.163 au moins) ; Gemini, pas de compte ; `real-harnesses.spec.ts` pour Copilot, qui réécrit `~/.copilot/config.json`. Aucun jeton pris, aucun fichier de connexion touché.

**Le piège du `Reactor` du kit** (ESC-351) ne les touche pas : ces quatre plugins ne dépendent pas du kit, et `turn::run` traite chaque ligne jusqu'au bout, sans tâche qui attend.

**Pas fait ici.** Le portage d'ACP, de Claude Code (P3b) et de Vibe (P6) : `plugins/acp` et `plugins/mistral` restent natifs. Les vues « Vue X » (P8). Ni étiquette ni soumission.

### 8.27 Ce que P3b a construit (ESC-347)

ACP (`dev.escouade.acp`) et Claude Code (`dev.escouade.claude-code`) sont en `run.wasm`, `2.2.0`, `escouade.min` `2.2.0`, membres du workspace de `plugins/`, sortis d'`escouade-plugins`. Goose reste un manifeste sans code, servi par ACP : il tourne donc sur le module. La recette du §8.19 a suffi, à cinq écarts près.

**Claude Code reste aussi une `rlib`.** Le cœur lit encore `translate`, `words`, `localcmd` et les arguments de `client` pour « Vue Claude Code » : `crate-type = ["cdylib", "rlib"]`, ce que `rust::check` permet, et `src-tauri` le prend par chemin. Il sort du workspace de `src-tauri`, pas de ses dépendances, jusqu'à P8. Cargo accepte qu'une dépendance par chemin soit membre d'un autre workspace.

**`signin.rs` est au cœur** (`src-tauri/src/engine/claude/signin.rs`), avec `tokio`, `sha2`, `unicode-normalization` et `anyhow`, qui quittent le plugin. Le plugin n'a jamais lu une connexion : c'était le cœur qui lisait ce fichier en bibliothèque. Il n'y a donc ni `escouade.call` ni `fs/*` à ajouter, et le manifeste de Claude Code ne déclare aucune permission. Seul `lost`, les mots d'un tour qui a perdu sa connexion, reste au plugin (`translate::lost`) : c'est de la traduction. Ce qui ne sert que sous macOS (`KeychainGate`, `keychain_answer`) est compilé pour macOS et les tests.

**Les fichiers d'ACP** (`crates/escouade-acp/src/files.rs`). En natif, rien ne change. Cursor, Gemini, Vibe et Goose déclarent `"fs": false` : leurs harness ne passent pas par ces fichiers, en module comme en process ; ils servent les harness décrits qui les demandent (`"fs": true`). En module, le chemin va à l'hôte avec la clé de l'agent :

- le client retire `.` et `..` par le texte (`plain`), selon le système du chemin et non celui de la cible, un module ne sachant pas lire `C:\wt` : `Path::is_absolute` y est faux ;
- `fs/canon` juge le chemin, liens compris, sous les racines de cet agent et hors de sa clôture ;
- `fs/read` et `fs/write` servent ensuite, `dataBase64` redevenant du texte remplacé comme en natif.

Les réponses à l'agent gardent leurs codes et leurs textes : `-32000` pour un refus, `-32002` pour un échec, `fenced_file` pour la clôture, reconnue aux mots de l'hôte (`host_call` perd le code). **Une seule différence de comportement, voulue** : le client natif atteignait tout chemin hors clôture ; le module lit dans `agent` et `agentRepo` et écrit dans `agent` (`permissions.files`), 4 Mio par fichier (`limits.fileBytes`). Le reste est refusé en `-32000`, nouveau texte `out_of_reach`. `agentRepo` est en lecture seule : le ticket le demande comme racine, et rien n'écrit dans le dépôt principal depuis un worktree. Goose (`"fs": false`) n'est touché que par le diff d'un `write`, lu au même endroit.

**`limits.memoryBytes` et `limits.turnSeconds`, nouveaux.** Un message peut porter 18 Mio de fichiers, soit 24 Mio en base64, envoyés puis renvoyés par `--replay-user-messages`. Dans 64 Mio de mémoire linéaire, le module s'arrêtait, et avec lui tous les agents du harness : un plugin n'a qu'une instance. Essayé : `the_biggest_message_passes_through_the_module` tombe à 64 Mio. Lire ce message en entier prend 1,1 s à l'aller et 0,8 s au retour sur un poste chargé, trop près des 2 s d'un tour. Le manifeste peut donc demander plus, jamais moins que le défaut : 256 Mio (`wasm::MEMORY_MAX`) et 10 s (`wasm::TURN_MAX`) au plus. Un tour de décision garde ses 500 ms. Claude Code et ACP demandent les deux plafonds ; les autres harness portés en ont le même besoin. La mémoire d'une instance ne redescend pas : le pic d'un gros PDF reste jusqu'à son arrêt.

**Le précompilé d'un module de `scripts/plugins-wasm.sh`** se garde à côté de lui (`<target>/plugins-wasm/<id-court>/`) dans les tests : sans ça, chaque process de nextest recompilait chaque module, et Claude Code sert presque tous les tests d'un agent. C'est le chemin de `main` (`Modules::prepare_reactor`), arrivé par une autre voie en même temps ; `plugins-wasm.sh` retire les précompilés périmés.

**`escouade_acp::plugin`** (ESC-346) porte le service qui était dans `plugins/acp` : Cursor, Gemini et Mistral en dépendent sans chemin vers un autre plugin. `plugins/acp` tient en trois lignes.

**La parité** (`plugins_claude_acp_parity_tests.rs`, `tests/fixtures/plugins/parity/{claude,acp}.json`) compare chaque ligne de chaque flux enregistré au natif de 6dbd1223 : toutes les `real-run-*.jsonl` et le long tour pour Claude Code (3 725 lignes), ceux de Cursor, Vibe et Gemini pour ACP, y compris par la description de Goose, plus chaque méthode du contrat, à l'élément près (un condensé par ligne, les genres d'éléments à côté pour lire un écart). Écart trouvé : aucun, hormis deux non-écarts écartés du test. L'ordre des réponses d'une interruption vient d'une `HashMap`, dans le natif aussi ; le plan de Gemini `plan-exit-approval` se lit à un chemin qui n'existe que sur le poste qui l'a enregistré. Les fichiers d'ACP, qui demandent un agent connu de l'hôte, sont joués sur le module par `plugins_acp_tests` (`fs/*`, clôture, `..`, lien, lecture seule).

**INT-0.** Le banc de flux rejoue Claude Code par le module (`ESCOUADE_BENCH_ONLY=claude`, puis `claude/long-turn`), en release sans LTO, poste chargé, face aux chiffres du process (ESC-226) :

| Claude Code | Le process (ESC-226) | Le module |
| --- | --- | --- |
| Ses trois flux, au plus vite : médiane | 110 µs | 90 µs |
| Ses trois flux, au plus vite : 99e centile | 835 µs | 655 µs |
| Ses trois flux, au plus vite : débit | 6 573 lignes/s | 4 800 lignes/s |
| Le long tour, à son rythme : médiane | 24,1 ms | 24,1 ms |
| Le long tour, à son rythme : 99e centile | 32,2 ms | 32,5 ms |
| Le long tour, à son rythme : CPU | 1 250 ms | 1 562 ms |
| Le long tour, au plus vite : 99e centile | 30,5 ms | 30,7 ms |
| Le long tour, au plus vite : débit | 17 320 lignes/s | 15 493 lignes/s |

Dans les seuils : +0,3 ms au 99e centile, plus de 2 200 lignes/s. Le module coûte un peu de débit et de CPU, pas de délai. La mémoire du process de tests reste à 56 Mo.

**L'essai réel.** `real-harnesses` a rejoué le vrai Claude Code 2.1.318 sur le module : un ticket jusqu'à sa fusion, et sa vue terminal. Goose n'est pas installé sur ce poste : `real_goose` n'a pas tourné. Il tourne sur le module par `acp-harness.spec.ts` et `plugins_acp_tests`, sur un faux agent.

**L'empreinte de Claude Code**, compilée depuis crates.io, `escouade-plugin build` : `013dba0ea81041981d2b572bf589ac26552d1f8322e81ee407a2e5653a96540b`, 528 940 o. La même sous Linux a été vérifiée au §8.30.

**Le piège de `Reactor::turn`** (P7) ne les touche pas : ni ACP ni Claude Code n'utilisent le kit. Une machine à états répond dans le tour, par `turn::run`.

**Second passage.** `escouade-acp` `0.1.1` publiée, ACP a son `Cargo.lock` et son empreinte : §8.29.

### 8.28 Ce que P6 a construit (ESC-350)

OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont des modules, en `2.2.0`, `escouade.min` `2.2.0`, `"run": { "wasm": "Cargo.toml" }`, sortis d'`escouade-plugins`. Mistral porte sa voix et Vibe dans la même instance. L'app ne garde de la voix que ce qui est du cœur : la capture, Whisper, Piper, le trousseau, le choix du service dans les réglages et le client de `voice/*` (`plugin_host.rs`).

**La crate `crates/escouade-voice` (`0.1.0`, MIT).** Elle tient `voice/online*`, `live.rs` et `wire.rs`, sans tokio ni reqwest :

- `online`, `speech`, `catalog` : ce que chaque service demande et ce que ses réponses veulent dire, en code pur. Une requête est une valeur (`Call`), un flux des trames (`Frame`).
- `wire` : le protocole `voice/*`, que l'app lit aussi.
- `host` : le réacteur de P5 (`plugins/opencode/src/host.rs`), avec les sockets de `ws/*`.
- `serve` : le service, `Reactor::turn(lignes) -> lignes`, joué par `cargo test` sans module.

Les quatre plugins de voix tiennent en une ligne (`escouade_voice::serve::run("deepgram", …)`). Elle dépend du kit pour `i18n` et le protocole, et l'app la lie pour `wire`, `Provider`, `OnlineError` et les catalogues livrés : le registre doit décrire les cinq services sans démarrer leur module. **Elle est en MIT**, comme les trois crates du kit, alors que son code vient du cœur : `scripts/crates-io.test.ts` l'exige de toute crate publiée, et `escouade-acp` a fait pareil.

**Le réacteur et l'ordre des tâches** (§8.22, §8.23). Le kit `0.1.0` n'a ni flux ni socket, et ne sait pas lancer une tâche qui survit à son appel : la voix n'y passe pas. Elle n'est pas exposée au piège du kit : `Host::run` refait un passage tant qu'une tâche est née ou que `stir` a été appelé, comme chez P5. Les trois états qu'une autre tâche attend l'appellent : la commande d'un flux, la fin d'un flux, l'annulation. `a_task_waiting_for_another_goes_on_in_the_same_turn` le prouve. **Rien de ce ticket ne doit monter dans le kit pour fonctionner** ; ESC-358 peut y faire monter `host.rs`, dont les deux copies diffèrent par `ws/*` et par ce que laisse un appel abandonné.

**Ce qui a dû changer, en dehors du déplacement.**

- **La dictée passe en PCM 16 bits.** Dix minutes de son (`MAX_LISTEN`) en flottants faisaient tomber le module, par manque de mémoire, là où le natif n'avait pas de limite. `voice/transcribe` porte maintenant `pcm16` : les mêmes octets au final dans le WAV, deux fois moins lourds, et chaque copie est lâchée avant la suivante. `a_long_dictation_goes_through_the_module` joue les dix minutes. Le direct reste en flottants : ses morceaux sont courts.
- **`ws/open` dit ce que le service a répondu.** L'hôte ne donnait que `HTTP 429`, or le natif montre le message du service. Il donne maintenant `HTTP 429: <réponse>`, 512 caractères au plus, les valeurs des en-têtes (donc la clé) retirées, parce que ce texte va aussi au journal.
- **Un appel abandonné referme ce qu'il ouvrait.** Une attente dépassée sur `ws/open` ou `net/stream` aurait laissé un socket ouvert dont personne ne sait le numéro. `Host` retient l'appel, et ferme ce que sa réponse nomme quand elle arrive.
- **Un flux sans texte final se referme.** En natif, le socket restait ouvert jusqu'à ce que le service le ferme. Un plugin n'en a que 4 : le flux se ferme quand plus personne n'attend.
- **`ws/send` attend sa réponse.** Un flux qui s'ouvre avec du son en attente peut remplir la file de 256 Kio : la trame est renvoyée 10 ms plus tard, à sa place.
- **Les services démarrent à leur premier appel.** Le registre décrit les cinq par les tables de la crate (`ProcessVoiceService::described`), sans démarrer de module : sinon chaque lancement de l'app compilerait cinq modules pour une voix que personne n'utilise, le problème de P3. `a_service_of_the_registry_starts_its_module_at_its_first_call_only` le vérifie, et compare cette description à celle des modules.
- **Plus de service dans le cœur.** `CoreService` est retiré, et avec lui le repli quand un plugin ne démarre pas : le service le dit à son premier appel (`Gone`, ou l'erreur du runtime), et le journal du plugin dit pourquoi. Tant qu'un des cinq n'a pas de plugin, ses réglages et sa clé restent atteignables. Un plugin coupé ou désinstallé retire son service.
- **Les limites, déclarées par les cinq** : `fetchSeconds` 120, `fetchBytes` 32 Mio, `memoryBytes` 256 Mio, `turnSeconds` 10. Le natif n'avait pas de limite de durée totale ; une dictée qu'un service met plus de 120 s à lire échoue maintenant en le disant.

**Ce qui diffère encore du natif, tout le reste étant au mot près.**

- Le texte d'une erreur réseau vient de l'hôte et non de reqwest. Son type ne change pas : la phrase part par requête, ou le service dit qu'il ne répond pas.
- 4 sockets et 8 flux par plugin, au lieu de pas de limite. Au-delà, le flux dit qu'il n'a pas ouvert et la phrase part par requête.
- Une dictée annulée n'interrompt pas sa requête, `net/fetch` n'a pas d'annulation : l'hôte la mène à son terme et sa réponse se perd. Une lecture, elle, coupe son flux.
- Le proxy n'est plus dans `Spec` : l'hôte suit Réglages › Réseau (§8.4.4). Les tests de proxy de la voix sont retirés, `plugins_net_tests.rs` les couvre pour `net/fetch`, `net/stream` et `ws/open`. Les fonctions du tunnel sont passées de `voice/live.rs` à `plugins/socket.rs`.
- Vibe dit sa vraie version à `initialize` : le natif de `mistral` envoyait `clientInfo.version` vide, sa boucle n'appelant pas `greeted`.

**La parité.** `tests/fixtures/plugins/parity/voice-<id>.json` tient ce que le natif de `6e0203a5` a fait, service par service, en 20 pas pour Groq et 28 pour les autres : la description, le catalogue, une dictée avec ses refus (401, 403, 429, 500, la clé citée par le service, un corps illisible, une réponse sans texte, le réseau coupé), une lecture à deux allures avec ses refus, la liste de l'API et le catalogue qui en sort, et un flux jusqu'à chacune de ses fins (parle, refusé, quota, échec, coupé, coupé à sa fin, muet). Pour chaque pas : la réponse, les requêtes au faux service, le son entendu, les textes partiels et ce que le socket a reçu. **Aucun écart**, requêtes comprises. Les bords de multipart, les temps et les messages réseau sont normalisés, et aucun chemin ni port n'y entre : elle tient sous Linux et macOS. `mistral.jsonl` ajoute Vibe à `plugins_harness_parity_tests.rs`, sur 11 enregistrements de `tests/fixtures/vibe/`. Sa seule retouche est le `clientInfo.version` ci-dessus.

**Les tests de l'app.** `online_tests`, `live_tests`, `online_speech_tests`, `online_catalog_tests`, `conv_tests` et `tests` gardent leurs faux services et jouent le module : `service::of` passe, sous test, par `voice/plugged.rs`. Une route sans port (`Fetcher::fake`) laisse un plugin joindre tout port de `127.0.0.1`, en http et ws ; seuls les tests et un build de développement ont des routes. Il faut donc `sh scripts/plugins-wasm.sh` avant ces suites, ce que la CI fait déjà.

**Le délai du direct, contre le « moins de 1 ms » du §8.4.4.** `int13_measure`, build de debug, Windows, 20 phrases par service : le texte final après la fin de la phrase, en médiane (99e centile).

| Service | Cœur, avant | Process, avant | Module |
| --- | --- | --- | --- |
| OpenAI | 0,5 ms (0,8 à 3,2) | 1,0 ms (1,2 à 1,8) | 0,9 ms (1,4) |
| Mistral | 0,5 ms (0,7 à 1,2) | 1,0 ms (1,3 à 1,7) | 0,9 ms (1,5) |
| ElevenLabs | 0,5 ms (0,7 à 1,1) | 1,0 ms (1,2 à 2,3) | 0,8 ms (1,1) |
| Deepgram | 0,6 ms (0,7 à 0,9) | 1,0 ms (1,5 à 1,8) | 1,6 ms (2,1) |

La colonne « avant » vient de sept passes du même test sur un autre build, et le module d'une seule : **ce n'est pas une mesure à conditions égales**. Elle suffit à dire que le module tient la promesse pour OpenAI, Mistral et ElevenLabs. **Deepgram dépasse d'environ 1 ms le cœur d'avant.** Il envoie deux trames à la fin de la phrase et chaque `ws/send` attend sa réponse, ce qui est l'explication probable, pas une cause mesurée. Le seuil d'INT-0 (5 ms) reste tenu partout.

**Le banc** `escouade-plugin test <court> --first-party --escouade 2.2.0 --wasm`, sur le module du dépôt : 6 réussis, 0 échoué, 5 sans objet pour chacun des cinq.

**L'essai réel** (`int13_real_elevenlabs_through_the_plugin`, la clé d'ElevenLabs du poste, donnée à ce seul process), par le module : la dictée d'une phrase en 2,1 s, la mauvaise clé refusée, quatre textes partiels et le final 188 ms après la fin de la phrase, une phrase lue (premier son en 1,3 s, 41 morceaux), 9 modèles et 31 voix listés. **Pas rejoués** : OpenAI, Groq, Deepgram et Mistral, faute de clé sur ce poste, comme Vibe réel.

**Second passage.** `escouade-voice` `0.1.0` publiée, les cinq plugins ont leur `Cargo.lock` et leur empreinte : §8.29. **Aucun ajout à `rust_native.json`.**

**Pas fait ici.** Ni étiquette ni soumission (Z1). Le kit n'a pas changé.

### 8.29 Le kit 0.1.1 (K1, ESC-358)

Ce que les portages avaient contourné chacun de leur côté remonte dans `escouade-plugin-kit` `0.1.1`. Le protocole ne change pas. **Tout est un ajout** : un plugin qui dit `"0.1.0"` compile tel quel, et rien ne bouge pour lui tant que son `Cargo.lock` fixe le `0.1.0`.

**Le réacteur (`serve/tasks.rs`).** Un tour sonde chaque tâche une fois, comme avant, parce que la plupart attendent ce que le tour vient de poser (une réponse, un minuteur) et qu'un `poll_fn` sans waker doit continuer de marcher. Il sonde ensuite celles qui ont été réveillées entre-temps : un vrai `Waker`, par tâche. Une tâche qui attend ce qu'une plus récente pose repart dans le même tour, sans autre ligne : c'est le cas de l'appel d'isola qui attend `proc/exit`, que le `sleep(Duration::ZERO)` du §8.22 contournait. `a_task_woken_by_a_later_one_goes_on_in_the_same_turn` échoue sur le code du `0.1.0`, identique à la source publiée, et passe sur le `0.1.1`.

**Pas de boucle sans fin.** Un tour fait 128 passages au plus. Passé cela, ce qui est encore réveillé est une tâche qui se réveille elle-même, ou deux qui se répondent. Le tour rend la main et demande un minuteur vide (`timer/set`, `ms` 0, sans `id`) : la sonnerie fait le tour suivant. Le module n'est pas tué par sa limite de temps par tour, les autres lignes passent, et rien ne reste endormi sans raison.

**Les redirections (`http::redirect`, `http/fetch.rs`).** `redirect.rs` des cinq hébergeurs entre dans le kit tel quel, en public : `follow`, `reply`, `success`, `send_named`, pour un client qui a son mot à dire à chaque saut (ses identifiants, son budget, son allure). Et la façade les suit maintenant toute seule : dans un module, `send()` fait ce que fait reqwest en process, aux mêmes règles (dix sauts, un GET après 301, 302 et 303, le même appel après 307 et 308, un corps qui ne quitte pas le service, jamais de https vers http). `http::send_named`, `call` et `call_page` en profitent, donc Jira, Linear, Trello et ClickUp. Hors du service, seuls `Accept`, `Accept-Language` et `User-Agent` restent : n'importe quel autre en-tête peut porter une clé, `PRIVATE-TOKEN` par exemple, ce que le nom seul ne dit pas. `RequestBuilder::once` envoie un seul appel, et `redirect::reply` passe par lui : un hébergeur garde la main sur chaque saut. En process, `reply` reste `send`, donc les redirections y restent celles de reqwest.

**`Host` (`crate::host`).** Les deux copies de `host.rs` (OpenCode, voix) en deviennent une, avec tout ce qu'elles avaient à elles : `net/stream`, `ws/*`, l'appel abandonné qui referme ce qu'il ouvrait, `spawn`, `stir`. C'est la voie d'un plugin qui lit un flux, garde un socket ou lance une tâche qu'aucun appel n'a commencée. Elle tourne sur les mêmes `Tasks` que le réacteur, donc avec les mêmes wakers et la même borne ; `stir` reste pour ce qui attend sans waker. Un minuteur refusé va au journal (`log`) et non plus sur stderr. `escouade-voice` s'en sert et demande donc le kit `0.1.1`.

**La façade lit `bodyBase64`.** Un corps qui n'est pas de l'UTF-8 arrivait vide ; il arrive avec ses caractères de remplacement, comme `Host::fetch`.

**Ce qui ne monte pas.**

- **Une API `proc/*`** : seul isola lance un programme. Le réacteur réglé, son code est le code naturel, et l'exemple de la doc (`sdk/doc-examples/rust/src/procs.rs`) reçoit les notifications dans `Plugin::call` : les retenir dans le kit casserait ce contrat au sein de `^0.1`.
- **`Cx::spawn` et un flux dans `Cx`** : `Host` les donne à qui en a besoin, et aucun plugin sur `serve::run` n'en demande.
- **`agent_file`** (`escouade_acp::plugin` et `plugins/antigravity/src/host.rs`) : huit lignes, et sa place serait `turn` dans le protocole, à republier pour si peu.
- **`src/joined.rs` d'Azure DevOps** : un seul plugin.

**La publication.** L'utilisateur a publié le 2026-11-06, depuis `ebc130a5`, `escouade-plugin-kit` `0.1.1`, `escouade-acp` `0.1.1` et `escouade-voice` `0.1.0`. Le protocole reste en `0.1.0`.

**Le second temps.** Les cinq `redirect.rs`, le `sleep(Duration::ZERO)` d'isola et `plugins/opencode/src/host.rs` sont retirés. Les hébergeurs prennent `http::redirect`, OpenCode prend `host::Host` du kit. Les plugins du kit, OpenCode, le modèle `create-escouade-plugin` et les exemples de la doc disent `0.1.1`. `sh scripts/plugin-lock.sh` a refait le `Cargo.lock` des 23 plugins en `run.wasm`, et `AWAITING_PUBLICATION` n'existe plus, ni dans `plugins/ci-lib.mjs`, ni dans `ci.mjs`, ni dans les tests.

**Les empreintes**, compilées depuis crates.io par `escouade-plugin build --first-party`, deux fois sous Windows, deux dossiers sous Linux (`rust:1.98.0-bookworm`, Podman, `canon` `4d3e106a…`), sans aucun écart. macOS n'a pas été essayé. Celle de Claude Code ne bouge pas : il n'utilise pas le kit. C'est sa première vérification sous Linux.

| Plugin | SHA-256 | Taille |
| --- | --- | --- |
| ACP | `a4c1c4faa5e9f9c559e27fc90a53647e55d8c8bf4d4005ce61eeb6dc90409149` | 758 256 o |
| Antigravity CLI | `c215c7e1cb36a45bcef5144eb146289479c9edeb862ee513fa609b1368aea531` | 981 294 o |
| Azure DevOps | `edc70b1076ff077ddd53af11ffeaf373f9bc9e1730c8e195e054dd8dd467156e` | 725 849 o |
| Bitbucket | `e0584248908e12e72b5a0c130fc2a9257c061cb05e8d3e9f5714dd00dddbd5d1` | 569 705 o |
| Claude Code | `013dba0ea81041981d2b572bf589ac26552d1f8322e81ee407a2e5653a96540b` | 528 940 o |
| ClickUp | `3a52c75b2d94d1945e5ff8cf72452fb9cbb1f73a77d2176dbd1a866392fa0192` | 569 666 o |
| Codex | `2af09ad1c2a5305236b0f12bb7f6e9959294ec71fe4684c534c9bf7fe6ad0d6e` | 673 553 o |
| Copilot CLI | `08b46076ddc4bb438592f2b368372a61ab042fd70e26e9953b0406f06b59e6d1` | 752 915 o |
| Cursor CLI | `09ee107c99ca7ffa0c7064911733b15599bbed2dc66b217e49e183edc19ae07c` | 853 575 o |
| Deepgram | `5ea971014abbe346b90aca6f14e921d811f50e20eea52166cfa509c62ce18b9e` | 531 268 o |
| ElevenLabs | `ec5e71af06044e9167b26770eede85f350b3985e8124a46e06a5299777ac97ce` | 531 268 o |
| Gemini CLI | `4dfa2efb4b7d97a9ccc09c27bfb5ad27ae673545e4819cee4cd167a59651705f` | 853 575 o |
| Gitea | `8e97c0d31ce96844d9d6adb4cfa3b87cf6bbd72533d6f4cae87d38408ec86572` | 545 132 o |
| GitHub | `6d2101591831738033c4df6f02318760d4dfdd4644c91892c89a53a53ed50ef0` | 622 270 o |
| GitLab | `a984ce48ed7551b75b1e2666cd5707c6f5732162f82173e511666d5fd032eb7f` | 607 201 o |
| Groq | `e10f0d8a012f4f178fb3cdbc27e83d655c7b7af6c5b1b1969192e087241de76d` | 531 268 o |
| isola | `67c8b84a58aaedfc9ad35987e24f35b8d408a5a2d63dc561cf739ff1ef630573` | 239 368 o |
| Jira | `8e8c4a1c8d50dad22815baded6cda09e97856f2304eecb1ba01a6d75f876466d` | 524 404 o |
| Linear | `9f6d4c20d128deeafb8ca4e97ffa9bd078b5f327d2489e5b73922d19e53ce513` | 495 974 o |
| Mistral | `21e0d78f6121728796d350b9599ecea74cb14532600e273a7b7436214c82fd25` | 1 200 832 o |
| OpenAI | `e0f11f6bac475a86d3ce7cc5404075ff1789f423019883ac8902e8202630f28c` | 531 268 o |
| OpenCode | `eb47097b3696d6d40fa01c46ad175f85e5cd21c7f4925ef1af3fb39878277bdf` | 756 021 o |
| Trello | `42b0d2029c45c1d94ae057b381110b7793d306788d11cbbe26720dd8d8d371b7` | 480 578 o |

Huit lignes de ce tableau sont celles de P8 (§8.30) : Antigravity CLI, Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, Mistral et OpenCode.

**Le banc** `escouade-plugin validate`, puis `test <court> --first-party --escouade 2.2.0 --wasm`, sur ces modules-là : 23 sur 23, aucun échec.

**Rien n'a été réenregistré.** Les parités rejouent les transcriptions d'avant le portage, et aucune fixture n'a changé. Ce ticket n'ajoute ni calcul flottant ni `cfg` de système : le kit et la voix n'ont que `target_family = "wasm"`.

**Pas fait ici.** Ni étiquette, ni soumission, ni signature (Z1, Z2).

### 8.30 Ce que P8 a construit (ESC-352)

**Le cœur ne lie plus aucun plugin.** `src-tauri/Cargo.toml` ne dépend d'aucun dossier de `plugins/`. Claude Code, Codex, Copilot, Antigravity et OpenCode redeviennent des `cdylib` seules. `escouade-plugins` reste pour le plugin de démonstration, jusqu'à Z3.

**`view/translate`.** Le cœur garde le PTY, le fichier que l'interface écrit, ses hooks et l'état de l'agent ; le plugin dit ce que les lignes veulent dire. Le cœur lui passe les lignes (`ViewParams`, `schemas/plugin-protocol-1.json`), avec ce qu'il sait de l'agent, et lit des éléments : ceux d'`agent/frames`, plus ceux d'une vue (`typed`, `spoke`, `wrote`, `waits`, `waitsOn`, `asks`, `runs`, `goes`, `viewTurn`, `viewEnd`, `viewUsage`, `viewError`, `row`, `bridged`, `linked`). Ce que seul le cœur sait part comme une **note**, une ligne `{"escouade": …}` : `end` (la fin d'un tour, avec le coût que le cœur calcule pour Claude Code), `hook` (Gemini CLI n'a que ses hooks ; le `permissionRequest` de Copilot), `frame` (un flux enregistré, pour les tests).

**Pourquoi synchrone, ligne à ligne.** Les vues du cœur lisaient chaque ligne sous le verrou de l'agent, chacune voyant ce que la précédente avait fait (le statut, le tour), et une fin de tour se fait sous ce verrou, depuis partout (`end_tui_turn`). Passer par le runtime aurait demandé un appel asynchrone sous verrou, ou tout le code des vues à refaire, et le fil du module sert aussi `fs/*`, qui peut vouloir ce verrou. **Retenu** : une **instance à part** par plugin (`plugins/view.rs`), du même module, sans `escouade.call`, sur un fil à elle (WASI bloque sur un runtime à lui, ce qu'un fil de tokio ne peut pas), appelée en attendant sa réponse, par le même tour que le reste (`Instance::turn`), bornée par le temps d'un tour du plugin. Une vue ne demande rien à la machine. Une instance en faute est remplacée à l'appel suivant : la vue repart avec une nouvelle clé, le cœur lui disant ce que la conversation contient (`known`). Un plugin arrêté perd son instance de vues.

**Chaque vue.**

| Vue | Où | Ce qui change |
| --- | --- | --- |
| Claude Code | `plugins/claude/src/view.rs` | les entrées du transcript, les commandes locales, le lien à claude.ai ; le coût reste calculé par le cœur |
| Codex | `plugins/codex/src/view.rs` | les `event_msg` du rollout, en notifications de l'app-server |
| Copilot CLI | `plugins/copilot/src/view.rs` | les événements du journal ; `permissionRequest` traduit par le plugin (`asks`) |
| Antigravity CLI | `plugins/antigravity/src/view.rs` | les étapes du transcript, en forme ACP |
| Cursor CLI, Mistral Vibe, Gemini CLI | `crates/escouade-acp/src/view.rs` | le client ACP (`Speaker`) à la place de `agent/cursor.rs` du cœur, retiré |
| OpenCode | rien à faire | `opencode attach` n'a pas de fichier à lire : ses événements passent déjà par le plugin |

**Une différence de comportement, voulue.** Plugin coupé ou désinstallé, le terminal marche, mais la conversation ne le suit plus et le dit une fois. Avant, la vue marchait plugin coupé, sa traduction étant liée.

**Un module qui ne répond plus.** Chaque essai peut durer tout le temps d'un tour, 10 s pour la plupart des harness, sous le verrou de l'agent. Le premier échec a donc droit à deux essais, l'instance étant refaite entre les deux, puis la conversation le dit, une fois, dans la langue de l'app : « Le plugin de Claude Code n'a pas lu ce que sa vue a écrit (il est coupé ou n'est pas installé). ». Les lignes passent ensuite sans lui pendant 30 s (`VIEW_PAUSE`), puis un seul essai, et ainsi de suite ; elles ne sont pas relues. Une fin de tour que le plugin ne peut pas lire se fait quand même, par le cœur, sans son coût. `agent/view_tests.rs` le joue sur un module qui boucle et sur un plugin coupé puis rallumé.

**Un écart corrigé en passant.** La fin d'un tour de « Vue Gemini » que le cœur fait à la fermeture de l'interface passait par la branche de Claude Code de `end_tui_turn`. Elle passe maintenant par son plugin.

**La parité.** `plugins_view_parity_tests` joue les 16 enregistrements des tests des vues, par les entrées et les hooks que reçoit le cœur, contre `tests/fixtures/plugins/parity/view.json`, écrit par la traduction liée avant son retrait (3931b674, `ESCOUADE_VIEW_PARITY=record`). À chaque étape : statut, messages, fin de tour, alerte, sortie du hook, lignes de stats, quota ; à la fin : la conversation entière, l'agent. Seuls les ids faits par le cœur et les heures sont normalisés. **Aucun écart.** Retirer un module fait échouer le test : il passe bien par lui.

**Les tests du cœur** n'ont pas changé d'assertion sur les vues : `codex_rollout`, `copilot_log`, `on_transcript`… passent par le module, et les flux enregistrés par la note `frame`. Les tests qui ne regardaient qu'une fonction pure du plugin sont partis avec elle (`tool_of`, `rollout_item`, `same_dir`, `safe`, `absolute`, les appels de Gemini). `agent/cursor_tests.rs` part avec `agent/cursor.rs` : le client ACP a ses propres tests et sa parité (§8.27). `Through`, qui jouait OpenCode dans le process, passe par `agent/frames` du module.

**Ce que le cœur garde de copié.** `terminal/launch` n'est toujours pas servi : le cœur lance encore lui-même les interfaces, et lit le quota et les modèles d'un compte. Il garde donc sa copie de ce qu'il lui faut, dans `engine/<harness>/{words,prices,client,rules}.rs` et `agent/{words,localcmd}.rs`. Une règle de lancement corrigée au store ne corrige donc pas encore celle de l'interface. Ce que seuls les tests du cœur lisent de ces copies est sous `cfg(test)` : à retirer quand ces tests iront dans les crates.

**La publication.** Le contrat a grandi, et une version ne se retire pas : l'utilisateur a publié `escouade-plugin-protocol` 0.1.1 et `escouade-acp` 0.1.2, `view.rs` n'étant pas dans la 0.1.1 sortie pendant ce ticket. Antigravity, Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI et Mistral ont leur `Cargo.lock` sur ces versions, et `AWAITING_PUBLICATION` a disparu pour de bon. ACP et les plugins sans vue restent sur le protocole 0.1.0 : un plugin ne suit une crate que s'il en a besoin.

**Les empreintes**, compilées depuis crates.io par `escouade-plugin build --first-party`, sont au tableau du §8.29. Les 23 plugins : deux dossiers sous Linux (`rust:1.98.0-bookworm`, Podman, `canon` `4d3e106a…`) et une passe sous Windows, sans aucun écart. Les 8 qui ont changé : deux passes de plus sous Windows, dans un dossier de données neuf, sans cache. macOS n'a pas été essayé. **15 plugins n'ont pas bougé.** Aux 7 du protocole s'ajoute **OpenCode**, qui ne s'en sert pas : retirer `rlib` de sa crate fait passer son module de 743 065 à 756 021 o, et le commentaire retiré de `lib.rs` décale les lignes que ses panics citent, si bien que `rlib` remis seul rend la taille, pas l'empreinte (essayé sous Linux). Il reste sur le protocole 0.1.0, et son `Cargo.lock` n'a pas changé.

**Le banc** `escouade-plugin validate`, puis `test <court> --first-party --escouade 2.2.0 --wasm`, sur les modules de ces empreintes : 23 sur 23, aucun échec. La parité reste sans écart avec `ESCOUADE_PARITY=1`.

**INT-0.** `int0_view_bench` (`plugins_view_parity_tests.rs`, ignoré) lit chaque enregistrement ligne à ligne, 200 fois, ce qu'une ligne prend de son entrée à ses effets sur l'agent. En release sans LTO, poste chargé, le même test sur 3931b674 pour la traduction liée :

| Vue | Liée : 50e / 99e centile | Module : 50e / 99e centile | Lignes/s, liée puis module |
| --- | --- | --- | --- |
| Claude Code (transcript) | 0,4 / 451 µs | 39 / 438 µs | 30 408 → 13 830 |
| Codex | 0,1 / 405 µs | 64 / 462 µs | 33 919 → 10 101 |
| Copilot CLI | 0,4 / 416 µs | 46 / 461 µs | 39 834 → 11 916 |
| Cursor CLI (transcript) | 56 / 693 µs | 183 / 1 400 µs | 4 706 → 2 337 |
| Mistral Vibe (outil) | 0,4 / 387 µs | 64 / 542 µs | 17 912 → 6 952 |
| Antigravity CLI (commande) | 20 / 523 µs | 131 / 976 µs | 7 361 → 3 720 |
| Gemini CLI | 18 / 702 µs | 105 / 811 µs | 9 243 → 5 221 |

**Le seuil de latence est tenu** : au plus 0,7 ms de plus au 99e centile, pour 5 ms permises. Le débit compte le départ de l'agent et de sa vue à chaque rejeu, qui pèse sur les enregistrements de 2 à 7 lignes : le seuil de 2 200 lignes/s est manqué par deux d'entre eux (`cursor-subagent` 2 009, `vibe-interrupted` 1 215, liée 2 187). Sur les enregistrements longs, le module tient de 6 000 à 13 800 lignes/s, de 2 à 4 fois moins que la traduction liée. Une vue se lit par à-coups, très en dessous.

**L'essai réel.** `real-harnesses.spec.ts` a rejoué le vrai Claude Code 2.1.318, Copilot CLI 1.0.98 et Mistral Vibe 3.0.3 : un ticket jusqu'à sa fusion, puis sa vue terminal, un message tapé là et revu dans la conversation d'Escouade. **Pas rejouées ici, à refaire par l'utilisateur** : Codex (0.144.5 sur ce poste, il faut 0.163 ou plus), Antigravity (`agy.exe` absent), Cursor CLI (absent du PATH), Gemini CLI (pas de clé d'API). Leurs vues passent les tests sur leurs faux harness et la parité sur leurs enregistrements. OpenCode n'a pas changé de vue.

### 8.31 La confiance d'un harness décrit (ESC-361)

Un harness décrit tourne sur `dev.escouade.acp` : le cœur jugeait donc le plugin qui le sert, pas l'auteur de son manifeste. Ce qui sépare les deux est maintenant `ProcessEngine::described` et un seul test de première main, `builtin::first_hand` : un id `dev.escouade.*`, que le manifeste refuse à un tiers (`Hand::Third`) et qui ne s'installe que signé.

- **La clôture.** `described` efface `fenceOnPermissions` d'un manifeste tiers avant que la description parte au client. Un tiers en `"fs": false` retrouve le sort de tout `"fs": false` : le client refuse le lancement dans un projet privé, et le dit. `check_launch` continue de juger le plugin qui sert (`is_builtin(plugin)`) et non l'auteur : `fenceByClient` y est la parole du client ACP, qui tient vraiment les `fs/*`, y compris pour un tiers en `"fs": true`. Juger `owner` à cet endroit fermerait ces harness-là aux projets privés. La première main garde sa parole, Goose compris, qui n'est dans aucun registre : la règle ne peut pas être `is_builtin(owner)`.
- **Les variables.** `check_env` juge chaque variable d'un lancement (`engine/launch`, `terminal/launch`, `engine/oneShot`, la relance) par `procs::forbidden_var`, la liste de `proc/*`, pour tout plugin qui n'est pas de première main, harness à module compris. `spawn` juge de même `accountVar` et `keyVar`, que le cœur pose sur la parole du manifeste. L'agent ne démarre pas, et la variable est nommée.
- **Le programme.** `locate.names` était joint à chaque dossier du PATH : un chemin absolu ou un `..` y remplaçait le dossier, malgré « un plugin tiers ne donne jamais de chemin ». `described` ne garde pour un tiers que des noms nus (`bare_name`).
- **Ce que l'utilisateur voit.** `asks` donne une ligne `harness:` par nom du programme, avec les variables (`VAR=valeur programme arguments`), et une ligne de plus pour `versionArgs` quand ce n'est pas `--version`. Une mise à jour qui change l'une d'elles redemande donc l'accord, comme pour `permissions.commands`.
- **`versionArgs`.** Il ne tourne que pour les harness du registre (`registry::all`, par `Core::engines`, `check` et `selfupdate`), et un harness décrit n'y entre qu'installé et allumé, `dev.escouade.acp` allumé (`sync_described`) : ni un aperçu, ni un plugin coupé ne lancent rien. Une fois accordé, il tourne sans agent ni geste, à chaque liste des harness.

**Comparé à `permissions.commands`.** Les deux sont dits mot pour mot à l'installation et sont décidés par le plugin. `proc/*` tient de plus : pas de `.cmd` ni de `.bat`, les `ESCOUADE_*` de l'app retirés, le journal, 4 programmes à la fois. Un harness décrit reste lancé comme un harness : un lanceur npm (`.cmd`) y est permis parce que les CLI s'installent ainsi, et `registry::version` hérite de l'environnement de l'app et ne passe pas par le journal du plugin. La lecture de version n'a pas `acp.env`.

**Ce qui reste.** `forbidden_var` est une liste courte : `PYTHONPATH`, `RUBYOPT` ou `JAVA_TOOL_OPTIONS` passent, chez `proc/*` comme ici. Le refus d'une variable tombe au lancement : `crates/escouade-plugin-cli` ne le dit pas encore à `escouade-plugin check` ni au store. `readOnlyMode` reste cru d'un tiers en `"fs": false` : le client refuse les écritures qu'il voit, pas celles que l'agent fait seul.

### 8.32 Ce que Z2 a rempli (ESC-354)

- **`plugins/seed.json`** nomme **26 versions**, la dernière signée de chaque plugin de première main au catalogue du store du 2026-11-07 (`id`, `version`, empreinte de `version.json`) : les 2.2.2 des harness (Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode), de `dev.escouade.acp`, des sources et des hébergeurs (Trello, Linear, Jira, ClickUp, GitHub, GitLab, Bitbucket, Azure DevOps), de la voix (OpenAI, Groq, Mistral, ElevenLabs, Deepgram) ; Gitea 2.2.1 ; Goose 1.0.1 ; Discord 1.1.1, Contraste élevé 1.1.1, Slack 2.0.1, Règles de l'équipe 2.0.1. **Pas dedans** : Antigravity CLI (2.2.1 non signée, la 2.2.2 d'ESC-376 pas publiée) et isola (pas au store). Ils restent donnés par l'app, origine `builtin`, jusqu'à leur signature : une ligne de `seed.json` chacun.
- **`builtin::seeds`** prend aussi les plugins de première main que l'app n'a jamais donnés, et que le tri connaît déjà (`sort::seeds_beyond_the_registry`) : Goose et ceux sur demande. La vue les nomme sans leur manifeste (`sort::name_on_request`, Goose compris).
- **Un build de développement sans les fichiers de l'amorce** (ni `seed-dev.sh`, ni `seed.ts fetch`) : `Seed::of_app` écarte les entrées de la liste embarquée dont le dossier manque, et le plugin tourne du registre comme avant (`scripts/plugins-wasm.sh`). Sans cela, la liste remplie retirait à `npm run tauri dev` et aux e2e tous leurs harness. Une release les garde, et la vue dit ce qui manque.
- **Les temps dans le journal** : `installed from the seed in N ms` (au lancement et pour un geste), et `plugin <id>: <comment son module est venu> in N ms` à chaque activation.

**La release, essayée en local** contre le vrai store, en lecture seule (aucun Release lancé). `npx tsx store/scripts/seed.ts fetch`, le script du workflow, **échoue** : le store n'a pas encore de liste de révocation (404 sur `/api/v1/revocations.json` et sur la copie d'escouade.dev), et l'amorce ne part pas sans elle. **Il reste à l'utilisateur de signer et publier la liste nº 1** (vide), avant la première release de la 2.2. Le reste a été joué par les mêmes fonctions (`fetchSeed`), la liste seule venant du faux store : les 26 signatures tiennent avec les clés de `store/keys/` (`store.pub`, `backup.pub`), ainsi que les commentaires, les empreintes et les modules, en 3,2 s. Avec la version 2.1.4 de `main`, le script refuse aussitôt (`asks for Escouade 2.2.0`) : la release de la 2.2 passe la version avant.

**L'essai hors ligne.** Une build release (`npx tauri build`, la version passée à 2.2.0 le temps de l'essai, l'amorce réelle dans `src-tauri/seed/`), jouée par `tests/e2e/real-seed.spec.ts` (`ESCOUADE_E2E_REAL_SEED=1`, `ESCOUADE_E2E_EXE` vers cette build), sur des dossiers de données neufs, toutes les requêtes de l'app vers un port fermé (Réglages › Réseau et `HTTPS_PROXY`). Le store n'a jamais été atteint (pas de `store/catalog.json`) ; seule la vérification de mise à jour de l'app contourne un proxy en échec, comme prévu (`updates.rs`).

| | Ce qui a tourné | Temps |
| --- | --- | --- |
| Dossier vierge | rien d'installé ; « Ajouter un compte… » sur Claude Code à l'accueil installe `dev.escouade.claude-code` 2.2.2 depuis l'amorce (origine `store`, `seed`, `verified`) | 249 et 270 ms pour l'installation, 372 et 393 ms jusqu'à la ligne « Plugin Claude Code installé » |
| Une 2.1 configurée (comptes Claude Code et Codex, un compte Trello ; pas de `plugins.json`) | Claude Code, Codex et Trello installés au lancement, rien d'autre ; isola et Antigravity CLI restent `builtin` | 161 à 299 ms par plugin |
| Un agent Claude Code | il répond, sans téléchargement ; son module se précompile à la première activation | 102 et 116 ms de précompilation |
| La vue Plugins | les trois dans « Ajoutés », désinstallables ; Copilot CLI, GitHub, Linear, Deepgram, Discord, Goose… sous « Proposés par Escouade », « Dans l'app, sans téléchargement » ; Trello désinstallé n'est pas remis au lancement suivant | |

Les deux dossiers sont de l'essai : celui « d'une 2.1 » a les fichiers qu'une 2.1 garde (`settings.json`, `integrations.json`, pas de `plugins.json`), pas les données de l'utilisateur, pour qu'aucun de ses projets ne s'ouvre sous une build d'essai.

**Le poids**, mesuré sur ces deux builds (l'installeur NSIS, `createUpdaterArtifacts` coupé faute de clé ; la seule différence est `src-tauri/seed/`) :

| | En clair | Dans l'installeur |
| --- | --- | --- |
| L'amorce (26 plugins, dont 16,3 Mo de modules) | 17 414 737 o | **+2 491 942 o** : 34 799 184 o sans, 37 291 126 o avec (2,45 Mo en xz, 7,0 Mo en gzip) |
| `escouade-plugins` en 2.2 avant le WASM (§8.7) | 34 479 104 o | — |
| `escouade-plugins` aujourd'hui (la démonstration seule, jusqu'à Z3) | 1 012 224 o | 0,30 Mo en xz |

L'amorce pèse donc la moitié de l'`escouade-plugins` prévu en clair, et 2,5 Mo dans l'installeur : les modules partagent leur code (le kit, la bibliothèque standard), que LZMA compresse ensemble. L'installeur de la 2.1.4 publié pèse 22 301 192 o ; celui-ci, sans l'amorce, 34 799 184 o. L'écart ne vient pas de l'amorce : `escouade-plugin.exe` (l'outil `escouade-plugin build`, 39 759 872 o en clair, 9,3 Mo en xz) est maintenant dans l'installeur, et `escouade.exe` porte wasmtime (97 942 016 o, 24,4 Mo en xz).
