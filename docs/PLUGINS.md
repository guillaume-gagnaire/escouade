# Les plugins d'Escouade : la référence

Ce document dit **ce qu'est un plugin aujourd'hui**, et où c'est dans le code. Il décrit l'état de `main` après le chantier « tout en WASM » (ESC-330 à ESC-355, décision 0144) : **tout plugin qui a un programme tourne en WASM dans Escouade, la première main comprise**. Il n'y a plus de process de plugin, plus de binaire `escouade-plugins`, plus d'origine `builtin`.

- §1 : ce qu'est un plugin (les pièces, les trois genres, les points d'extension) ;
- §2 : le contrat JSON-RPC, point d'extension par point d'extension ;
- §3 : l'hôte WASM, ses capacités et ses limites ;
- §4 : écrire un plugin (le SDK TypeScript, le kit Rust) ;
- §5 : la première main (`dev.escouade.*`), l'amorce et son tri ;
- §6 : le store, la signature et la confiance ;
- §7 : l'activation, les projets privés, les secrets ;
- §8 : ce qui reste au cœur, et ce qui n'est pas un point d'extension ;
- Annexe : l'historique (l'étude, les étapes), court.

Pour l'auteur : la documentation du site (`website/content/docs/*/plugins/`) est la page à lire en premier ; ce document est celle du contributeur d'Escouade. La place de marché est dans `docs/STORE.md`, l'interface des harness dans `docs/ENGINES.md`.

## 1. Ce qu'est un plugin

**Un plugin est un dossier avec un manifeste** (`escouade-plugin.json`). Il apporte des **contributions déclarées** (un thème, un webhook, un harness ACP décrit, un compte, une source du Kanban, un hébergeur…), qu'Escouade lit et montre avec ses propres composants, et, s'il le faut, un **programme** qui parle à Escouade en JSON-RPC 2.0.

### 1.1 Les trois genres

| Genre | Le manifeste dit | Ce qui s'exécute | Exemples |
| --- | --- | --- | --- |
| **Déclaratif** | pas de `run` | rien, ou seulement le code d'Escouade (un thème, un webhook, un harness ACP décrit) | `high-contrast`, `discord`, `goose` |
| **Programme en JS/TS** | `run.js` : un bundle (`dist/plugin.mjs`) | le store le compile en un module WASM (Javy, `escouade-js`) ; l'app le fait tourner par wasmtime, dans son process, sans Node | `slack`, `team-rules` |
| **Programme en Rust** | `run.wasm` : le `Cargo.toml` du plugin | le store le compile pour `wasm32-wasip1` ; l'app le fait tourner de même | les 23 autres plugins de la première main |

**Il n'y a pas de quatrième genre.** `run.command` (un exécutable natif, avec `windows`, `macos`, `linux` et `args`) se lit encore, pour que le manifeste soit refusé proprement : le plugin se charge **coupé**, la fiche, la fenêtre d'installation et le banc disent pourquoi (`manifest::command_refused`), `runtime` le refuse si jamais il arrivait jusque-là, et le store le refuse (contrôle n° 10). L'origine `builtin` (un plugin embarqué dans l'app, lancé comme un programme `{escouade}/…`) n'existe plus.

### 1.2 Les pièces

| Pièce | Où | Ce qu'elle fait |
| --- | --- | --- |
| Le manifeste | `crates/escouade-plugin-cli/src/manifest.rs`, `schemas/plugin-manifest.mjs` (deux lecteurs, un corpus : `tests/fixtures/plugins/manifests.json`), `schemas/plugin-1.json` | Lit et refuse, champ par champ, en français et en anglais |
| Le chargement et l'installation | `src-tauri/src/plugins/mod.rs` | Installe (dossier, dépôt git, store, amorce, en développement), fixe l'empreinte, tient `<data>/plugins/` et les choix de l'utilisateur |
| L'amorce | `src-tauri/src/plugins/seed.rs`, `sort.rs`, `builtin.rs` (la liste des identifiants et leurs manifestes), `plugins/seed.json` | Les plugins de la première main que l'installeur porte, et le tri de ceux qu'on installe |
| Le runtime | `src-tauri/src/plugins/runtime.rs` | Une instance par plugin, démarrée à la demande, délais, relances, fermeture |
| L'hôte WASM | `src-tauri/src/plugins/wasm.rs`, `procs.rs`, `files.rs`, `socket.rs` | wasmtime, les capacités (`net/*`, `ws/*`, `fs/*`, `proc/*`, `timer/*`), les limites, le cache des modules précompilés |
| La compilation | `crates/escouade-plugin-cli/src/wasm.rs` et `wasm_prelude.js`, `src-tauri/escouade-js` | Javy, le prélude JS, la réactorisation ; dans le crate sans Tauri que l'image du store construit |
| Le protocole | `schemas/plugin-protocol-1.json` (le socle, dont le TS du SDK est généré), `crates/escouade-plugin-protocol` (les types partagés avec le cœur : comptes, sources, hébergeurs, harness, voix) | Un seul protocole |
| Les faits | `src-tauri/src/facts.rs`, `src-tauri/src/plugins/events.rs` | Le bus de faits typés, les webhooks et les commandes déclarés |
| Les autorisations | `src-tauri/src/plugins/decide.rs` | Les plugins qui décident, un refus qui l'emporte, le délai |
| Les adaptateurs | `adapters.rs` (sources, hébergeurs), `hosted.rs` (un client par compte), `harness.rs` et `shared.rs` (harness), `reader.rs` (workflows), `accounts.rs` (comptes), `src-tauri/src/launchenv.rs`, `src-tauri/src/voice/service.rs` | Ce que le cœur appelle, sans savoir qui répond |
| Le journal | `journal.rs` | Démarrages, plantages, appels, décisions ; `<data>/plugin-logs/<id>.log`, secrets masqués |
| Le store dans l'app | `store.rs`, `store_keys.rs` | Catalogue, liste de révocation, téléchargement et vérifications, sceau revérifié hors ligne |
| Le banc | `src-tauri/src/plugins/bench/`, la CLI `escouade-plugin` (`test`, `dev`, `validate`, `build`) | La conformité d'un plugin, livrée avec l'app |
| Le kit Rust | `crates/escouade-plugin-kit`, `crates/escouade-acp` (sur crates.io) | La boucle des plugins Rust ; le client ACP et son service |
| Le SDK | `sdk/plugin` (`@escouade-dev/plugin`, MIT), `sdk/create-escouade-plugin` | TypeScript, faux Escouade, gabarit |
| La fenêtre | `src/components/plugins/`, `src/lib/plugins*.ts`, `src/lib/plugin-theme*.ts`, `src/lib/proposed-plugins.ts` | La vue Plugins, le store, les thèmes, les plugins proposés |

### 1.3 Le manifeste

Un fichier `escouade-plugin.json` à la racine ; `schemas/plugin-1.json` en est le schéma (`https://escouade.dev/schemas/plugin-1.json`), et la page `plugins/manifest` du site le dit champ par champ. Celui de `dev.escouade.slack` :

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.slack",
  "name": "Slack",
  "version": "2.0.1",
  "description": { "fr": "…", "en": "…" },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
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

- `id` : unique, en nom de domaine inversé, en minuscules. `dev.escouade.*` est réservé à la première main (§5).
- `escouade.api` : la version **majeure** du protocole (1) ; `min` et `max` : les versions d'Escouade où il a été essayé. Un plugin hors de sa plage est chargé désactivé, la raison dite.
- `icon` : un PNG carré de 256 px au moins, 1 Mo au plus, relatif au dossier ; le store le réencode sans métadonnées.
- `run` : facultatif (sans lui, le plugin est déclaratif). `js` ou `wasm`, jamais les deux ; `command` est refusé (§1.1).
- `activation` : ce qui démarre l'instance (un fait, une commande, un compte) ; sans activation, rien ne tourne.
- `permissions` : `events`, `network`, `secrets`, `commands`, `env`, `files`, `decide`. Une permission inconnue n'est pas donnée.
- `permissions.env` (ESC-424) : les variables d'environnement que le plugin pose sur ce qu'il lance (`proc/spawn`, les lancements de ses harness : `engine/launch`, `terminal/launch`, `engine/oneShot`, `account/read`), un nom chacune, montrées à l'installation (« Pose cette variable d'environnement sur ce qu'il lance ») ; toute autre est refusée, la raison dite. Le `acp.env` d'un harness décrit compte comme déclaré (l'installation le montre déjà), et le cœur pose lui-même les siennes (`accountVar`, `keyVar`, `ESCOUADE_HOOK_URL`). **Refusées à un tiers même déclarées**, par le lecteur du manifeste (code `variable`, `escouade-plugin validate` et le store) comme par l'hôte : celles d'Escouade (`ESCOUADE_*`), `LD_*`, `DYLD_*`, `GIT_*` sauf `GIT_AUTHOR_*` et `GIT_COMMITTER_*`, `PYTHON*`, `RUBY*`, `PERL*`, `NODE_*`, `JAVA_TOOL_OPTIONS`, `_JAVA_OPTIONS`, `JDK_JAVA_OPTIONS`, `npm_config_*`, `*_PROXY`, `SSL_*`, `CURL_CA_BUNDLE`, `REQUESTS_CA_BUNDLE`, `CODEX_CA_CERTIFICATE`, `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `XDG_*`, `PATH`, `PATHEXT`, `COMSPEC`, `SHELL`, `BASH_ENV`, `ENV`, `ZDOTDIR`, `IFS`, `PROMPT_COMMAND`, `EDITOR`, `VISUAL`, `PAGER`, `BROWSER` (`manifest::env_refused`, `envRefused` du store). La première main est tenue à ce qu'elle déclare, sans la liste des familles (Antigravity CLI pose `HOME` du dossier de son agent) ; une version de la première main d'avant le champ garde ses lancements comme avant.
- `limits` : `fetchSeconds`, `fetchBytes`, `fileBytes`, `memoryBytes`, `turnSeconds`, jusqu'aux plafonds de l'hôte (§3.5).
- `contributes` : les points d'extension (§1.4). `store` : ce que sa page du store en dit.

Un champ inconnu est ignoré ; un manifeste invalide ne charge rien et dit pourquoi, champ par champ.

### 1.4 Les points d'extension

Un point est **servi** quand le cœur s'en sert ; le schéma en réserve davantage. Un plugin peut tenir plusieurs points sur **un compte** (`contributes.accounts`) : le plugin GitHub donne les Issues, les PR, la CI et les déploiements sur une seule connexion.

| Point | Contribution | État | Qui s'en sert |
| --- | --- | --- | --- |
| Événements | `permissions.events`, `webhooks`, `commands` | servi | Discord, Slack |
| Autorisations décidées | `permissions.decide` | servi | Règles de l'équipe |
| Thèmes | `themes` | servi | Contraste élevé |
| Harness ACP décrit | `harnesses[].acp` | servi | Goose |
| Harness à module | `harnesses` + `engine/*`, `agent/*` | servi à la première main ; tiers : un harness ACP décrit | Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, Mistral Vibe, OpenCode, Antigravity CLI |
| Comptes | `accounts` | servi | ceux des sources et des hébergeurs |
| Sources du Kanban | `ticketSources` | servi | Jira, Linear, Trello, ClickUp, GitHub, GitLab, Azure Boards |
| Hébergeurs | `forges` | servi | GitHub, GitLab, Bitbucket, Gitea et Forgejo, Azure DevOps |
| Services de voix | `voice/*` | servi | OpenAI, Groq, Mistral, ElevenLabs, Deepgram |
| Environnements de lancement | `launchEnvironments` | servi | isola |
| Lecteurs de workflow | `workflowReaders` | contrat seul, rien ne l'appelle | — |
| Réglages et secrets | `settings`, `secrets` | servi | tous |
| Menus, barre d'état, badges | `menus`, `statusBar`, `badges` | réservé | — |
| Modèles de projet, clés de `.escouade.yml` | `projectTemplates`, `escouadeYml` | réservé (`plugins.<id>` est lu) | — |
| Outils MCP, textes rédigés, langues, statistiques, Ctrl+K, canaux de notification, vues libres | — | pas servi | — |

**Pas de HTML libre dans la fenêtre** : ce qu'un plugin montre est déclaré et rendu par Escouade, dans les deux langues, aux règles de `DESIGN.md`. L'iframe isolée reste à essayer sous WKWebView et WebKitGTK avant d'offrir une vue libre.

## 2. Le contrat JSON-RPC

Le protocole est `schemas/plugin-protocol-1.json` (version 1, délai par défaut 5 s) ; les types de chaque point sont dans `crates/escouade-plugin-protocol`. Les messages sont du JSON-RPC 2.0, une ligne par message, avec des lots dans les deux sens, sur l'entrée et la sortie du module WASM ; le SDK et le kit les écrivent pour l'auteur.

### 2.1 Le socle

| Méthode | Sens | Délai | Ce qu'elle fait |
| --- | --- | --- | --- |
| `initialize` | vers le plugin | 10 s | La poignée de main : versions, capacités, `lang` (la langue de l'app) |
| `shutdown` | vers le plugin | 1 s | Escouade ferme |
| `events/fact` | vers le plugin, notification | — | Un fait de `permissions.events`, d'un projet où le plugin est actif |
| `permissions/decide` | vers le plugin | 500 ms | Une demande d'un agent, avant l'utilisateur |
| `view/translate` | vers le plugin | délai d'un tour | Pour un harness à module qui a une vue terminal : Escouade lit le fichier que l'interface du harness écrit et en passe les lignes (et les notes `end`, `hook`, `frame`) ; le plugin rend ce qu'elles veulent dire, en éléments de conversation. Sans rien demander à l'hôte |
| `secrets/get` | vers Escouade | 5 s | Un de ses secrets (`plugin:<id>:<clé>`) |
| `settings/get` | vers Escouade | 5 s | Ses réglages ; avec un projet, ceux de son `.escouade.yml` par-dessus |
| `files/read` | vers Escouade | 5 s | Un fichier de son dossier (256 Kio, texte, jamais le manifeste ni un fichier caché) |
| `log` | vers Escouade, notification | — | Une ligne de son journal |
| `net/*`, `ws/*`, `timer/*`, `fs/*`, `proc/*` | vers Escouade (et leurs notifications en retour) | voir §3 | Les capacités de l'hôte |

Erreurs (`x-errors`) : celles de JSON-RPC, puis `notYours` -32001, `keychain` -32002, `refused` -32003, `failed` -32004, `notFound` -32005.

### 2.2 Les harness

Trois façons, de la plus simple à la plus complète :

1. **Décrit par ACP** (`contributes.harnesses[].acp`) : un manifeste, sans code. Le plugin `dev.escouade.acp` (`crates/escouade-acp`) lance l'agent, lui parle et montre sa conversation, ses permissions, ses diffs. `args`, `env`, `removeEnv`, `fs`, `modes`, `readOnlyMode`, `modelOption`, `questionsInText`, `tools`, `subagentTools`, `extensions`. Exemple : `plugins/goose/`. Un tiers peut publier un tel harness ; le cœur efface `fenceOnPermissions` de son manifeste et n'y garde que des noms de programme nus (`ProcessEngine::described`).
2. **Un plugin ACP avec ses extensions** (`dev.escouade.cursor-cli`, `dev.escouade.mistral`, `dev.escouade.gemini-cli`, et Antigravity CLI qui met ses événements en forme d'ACP) : un module `run.wasm` qui sert le client ACP avec ses extensions.
3. **Un client à lui** (`dev.escouade.claude-code`, `codex`, `copilot-cli`, `opencode`) : un module qui tient le client du harness et sa traduction, par le contrat `engine/*` et `agent/*` (`crates/escouade-plugin-protocol/src/engine.rs`, adaptateur `src-tauri/src/plugins/harness.rs`, application des éléments `src-tauri/src/agent/items.rs`). Un harness reste une machine à états sans attente : le kit lui donne `Plugin::handle` tel quel.

**Ce que garde le cœur** : le lancement (sur la description du plugin), le PTY, les comptes et leur jauge, l'état de l'agent. `engine/launch` rend une **description** de lancement (arguments, variables, fichiers à écrire, clôture appliquée, lecture seule) que le cœur vérifie (`check_launch` : le jeton du serveur MCP jamais dans les arguments, la clôture d'un projet privé, les fichiers dans le dossier de l'agent) avant de lancer le programme qu'il a trouvé lui-même, dans son Job object, sous WSL au besoin. Un harness partagé par des agents (OpenCode) est lancé par le cœur une seule fois (`Launch::shared`) et survit à un plantage du plugin.

**Le dialogue** : le cœur passe les lignes du harness par lots (`agent/frames`) ; le plugin rend des **éléments** normalisés (`start`, `text`, `thinking`, `tool`, `toolDone`, `permission`, `question`, `turn`, `notice`, `write`…) que le cœur applique à l'agent (`apply_items`) ; `agent/sendMessage`, `interrupt`, `setModel`, `setEffort`, `setMode`, `answerPermission`, `answerQuestion` rendent des `writes` à jouer dans l'ordre ; `agent/readUsage` rend le quota ; `agent/attach` redonne l'agent au plugin après un redémarrage ; `engine/oneShot` décrit un texte rédigé ; `terminal/launch` et `terminal/hook` décrivent la vue terminal ; `account/read` et `account/lines` lisent ce qu'un compte dit par un process à lui. Les extensions du contrat sont toutes optionnelles : un plugin écrit avant reste tel quel.

**La vue terminal** (ESC-359) : chaque plugin de harness sert `terminal/launch` (Claude Code, Codex, Copilot CLI, Antigravity CLI ; Cursor CLI, Mistral Vibe et Gemini CLI par `escouade-acp`). Le cœur lui passe l'agent (`TerminalParams` : son modèle, son effort, sa session, le fichier privé de l'interface, ses hooks, le serveur d'Escouade, sa clôture, les noms des variables de son compte) et vérifie la description comme celle d'un agent (`check_launch`) ; les fichiers d'une interface vont sous le dossier de l'agent ou à côté de son fichier privé. Ce que le cœur écrit pour les agents d'un harness (`BuiltinHarness::prepare`), il l'écrit d'abord pour son interface ; la clôture de Claude Code reste le fichier de réglages du cœur. Une lecture à faire avant le lancement (`before` : la confiance des hooks de Codex, un modèle que le compte de Copilot ne liste pas) passe par `account/read`. OpenCode n'a pas de lancement à lui : son interface rejoint le serveur de l'agent (`agent/open`). Plugin coupé ou désinstallé, l'interface ne s'ouvre pas, et la fenêtre dit pourquoi ; une vue ouverte garde son terminal, et la conversation dit qu'elle ne le suit plus.

**Un compte, par un process à lui** (`account/read`, `account/lines`) : ses modèles (`models`), son quota (`quota`) et ce que le plugin nomme (`hooks`, `listed`, `fork`). Un dialogue sans état : le plugin décrit le process (ses arguments, ses variables, ses fichiers sous le dossier du compte), le cœur le lance comme un agent et lui passe ce qu'il écrit, l'état du plugin revenant avec chaque lot, jusqu'à la réponse. Claude Code, Codex, Copilot CLI, OpenCode et Antigravity CLI le servent ; le menu des modèles de Cursor CLI, Mistral Vibe et Gemini CLI reste fait par le cœur, à partir de ce que dit leur client ACP, pour leurs agents comme pour un compte sans agent.

**Un plugin qui plante** : le harness continue (le cœur le lance), les lots restés sans réponse sont redonnés après la relance, et après le dernier plantage permis l'agent s'arrête en le disant, sa session gardée. Un plugin coupé : l'agent le dit au lancement.

**La confiance d'un harness décrit** (ESC-361, ESC-424) : `check_env` juge chaque variable d'un lancement par `harness::Vars` (`ProcessEngine::vars`) : déclarée (`permissions.env`, `acp.env`) ou posée par le cœur, et jamais, pour un tiers, une de `procs::forbidden_var` ni des familles de `manifest::env_refused`. Un harness d'un tiers ne nomme comme programme ni un shell ni un interpréteur (`manifest::interpreter` : `sh`, `bash`, `zsh`, `cmd`, `powershell`, `pwsh`, `python*`, `node`, `ruby`, `perl`, `osascript`, `wscript`, `cscript`, `env`, `wsl`…) : le manifeste est refusé (code `interpreter`) et `ProcessEngine::described` retire ces noms. Son `versionArgs` ne tourne qu'après un geste de l'utilisateur sur ce harness (un agent ou un terminal lancé, `registry::gesture`), ni à l'installation ni au démarrage (`Engine::version_waits_gesture`). L'installation dit chaque programme et chaque variable (`harness:`, `env:`), et une mise à jour qui en change redemande l'accord.

**Ce qu'un harness dit sans le lancer** (SEC-30, ESC-443) : son `acp.removeEnv` est jugé comme son `acp.env` : pour un tiers, une variable des familles refusées (le proxy, les autorités, `ESCOUADE_*`…) est refusée au manifeste (code `variable`, à `acp.removeEnv[i]`). L'hôte tient la même règle sur chaque lancement d'un tiers (`Vars::hold_removed`, après `check_env` : `engine/launch`, la relance, `terminal/launch`, `engine/oneShot`, `account/read`) : il ne retire jamais ce qu'il pose lui-même (le proxy et le fichier d'autorités des réglages, `accountVar`, `keyVar`, l'adresse des hooks) et garde la variable, une ligne au journal ; la première main retire ce qu'elle dit (Copilot CLI, ses jetons). La fenêtre d'installation et de mise à jour (et celle de l'accord d'un projet privé) montre, avec ce que le harness lance, ses commandes d'installation (`harnessInstall:`, une fois si elle est la même partout) et de mise à jour (`harnessUpdate:`), « écrites par le plugin », et chaque variable retirée (`harnessUnset:`) ; une mise à jour qui en change redemande l'accord. Pour un tiers, l'accueil (« Aucun harness trouvé »), Réglages › Comptes et harness et le refus d'une version trop ancienne disent que la commande est donnée par le plugin, par son identifiant, pas par Escouade (`Engine::commands_from`, `EngineDecl::install_from`) ; la première main (`dev.escouade.*`) garde l'affichage d'avant. `fenceByClient` est la parole du client ACP, qui tient les `fs/*`.

### 2.3 Les sources de tickets

`contributes.ticketSources`, sur un compte déclaré : `tickets/containers`, `tickets/states`, `tickets/issues` (filtre, page), `tickets/setState`, `tickets/comment`. Le cœur appelle `TicketSource` (`src-tauri/src/integrations/`) ; `ProcessTicketSource` (`adapters.rs`) parle au plugin. La synchro des colonnes reste au cœur. Une source peut donner une vue propre des états (`tickets/states` → `{ states, defaults }`, Azure Boards, ClickUp). Une source tierce (YouTrack, Asana…) suit le même contrat.

### 2.4 Les hébergeurs de code

`contributes.forges`, sur un compte déclaré : `account/me` et `forge/*` (`createPr`, `pullRequest`, `pullRequestsOfBranch`, `pullRequests`, `mergeOptions`, `merge`, `autoMerge`, `checks`, `branchChecks`, `jobLog`, `rerunFailed`, `reviews`, `comments`, `comment`, `review`, `reply`, `deployments`, `environments`). Chaque méthode est refusée par défaut (`unsupported`, -32601), selon les `capabilities` déclarées. `ProcessForge` répond au trait `Forge` (`src-tauri/src/forge/`) ; chaque réponse est `{ value, rateLimit? }`.

**Les comptes** (`contributes.accounts`, `AccountKind`, `plugins/accounts.rs`) : les hôtes (le public, les instances à soi et comment les reconnaître), les façons de se connecter (jeton, CLI), les champs, les droits, la page du jeton. Un compte sert tous les points de son plugin. Le jeton va au coffre sous `plugin:<id>:account:<compte>` et **le cœur le prête** au plugin avant chaque appel (`Runtime::lend`) : `secrets/get` donne le prêté d'abord, jamais le secret d'un autre plugin. Les erreurs sont typées : `-(32000 + statut)` pour un statut HTTP (`-32401` : reconnecter le compte), `-32010` pour un refus durable, `-32601` pour un appel non servi.

### 2.5 Les services de voix

`voice/*`, le trait `VoiceService` (`src-tauri/src/voice/service.rs`). OpenAI, Groq, ElevenLabs, Deepgram et Mistral sont des plugins. La clé reste au trousseau (`dictation-<service>`) et passe par `secrets/get`. Le flux de la dictée utilise `ws/*` ; un corps binaire passe en `bodyBase64`. Whisper et Piper, hors ligne, restent au cœur ; un plugin de voix qui ne démarre pas laisse le service du cœur répondre.

### 2.6 Les événements et les autorisations

Le bus est `src-tauri/src/facts.rs` : un `Body` typé par fait, **trente-neuf faits** en version 1 (agents, tours, demandes, tickets, projets et worktrees, lancements et tests, app, docs et mémoire et décisions, PR et comptes, workflow). Un plugin les reçoit de trois façons : `events/fact` dans son programme (`permissions.events`, globs permis), un **webhook** déclaré (`contributes.webhooks` : une URL et un gabarit par fait, vers un hôte de `permissions.network`), une **commande** déclarée (`contributes.commands`, `permissions.commands`, approuvée telle quelle par l'utilisateur). Les faits d'un projet ne vont qu'aux plugins actifs dans ce projet ; ceux d'un projet privé, qu'à ceux qui y sont activés un par un.

**`decide`** (`permissions.decide`, `decide.rs`) : le plugin reçoit `permissions/decide` avant l'utilisateur et répond autoriser, refuser avec un message, ou laisser. Plusieurs plugins répondent en parallèle, **un refus l'emporte** ; passé 500 ms, la demande va à l'utilisateur. La réponse s'inscrit au journal. Ni `ExitPlanMode` ni les questions ne passent par là.

### 2.7 Les autres points

- **Environnement de lancement** (`contributes.launchEnvironments`, `launchenv.rs`) : le cœur lit `detect`, décide qu'un worktree est celui de l'environnement et lance `up` ; le plugin répond `launch/ls`, `launch/down`, `launch/destroy` (par `proc/spawn` sur `permissions.commands`, le seul usage de `proc/*` de la première main : isola). Seule la première main sert ce point, `up` étant une commande du terminal du cœur.
- **Lecteur de workflow** (`contributes.workflowReaders`, `reader.rs`) : `workflow/read` sur les fichiers que le manifeste déclare ; contrat et tests seulement, rien dans l'app ne l'appelle.
- **Réglages, secrets, thèmes** : rendus par les composants d'Escouade (`contributes.settings`, `secrets`, `themes`) ; un thème donne des jetons de `THEME_TOKENS`, filtrés.

## 3. L'hôte WASM

### 3.1 Le modèle d'exécution

**Une instance par plugin, sur un fil, réveillée par des événements.** Le module est un **réacteur** (`crate-type = ["cdylib"]`, pas de `_start`, l'export `escouade-turn`, le même nom pour `run.js` et `run.wasm`) qui garde son état entre deux tours. Le fil attend sur un seul canal ce qui peut arriver (une ligne du cœur, la fin d'un `net/fetch`, un morceau de flux, un message de WebSocket, la sortie d'un programme, un minuteur), l'écrit sur l'entrée du module, lance **un** tour, puis sert ce que le module a écrit. Rien n'attend dans un tour : une tâche Rust qui attend rend la main, et le kit a un exécuteur d'une centaine de lignes, sans tokio (`serve/tasks.rs`). Un appel synchrone (`escouade.call`) sert les fichiers.

### 3.2 Ce qui est appliqué

**Ce que le manifeste ne déclare pas, le module ne peut pas l'atteindre** : il n'a ni socket, ni descripteur de fichier, ni process, ni environnement. Une capacité non déclarée répond `-32003`. L'installation dit chaque capacité en mots, la plus forte d'abord, et le journal garde chaque appel et chaque refus.

### 3.3 Les capacités

| Capacité | Permission | Ce que l'hôte applique |
| --- | --- | --- |
| `net/fetch` | `network` : des hôtes (un hôte couvre ses sous-domaines) | https seulement ; jamais une IP écrite dans l'URL ; pas de redirection suivie ; 10 s et 1 Mio par défaut (`limits` : jusqu'à 120 s et 32 Mio) ; jamais une adresse de la machine ou du réseau local (jugée après la résolution) ; le proxy et le TLS de Réglages › Réseau ; `bodyBase64` dans les deux sens ; 16 à la fois |
| `{account}` | `network: ["{account}"]` | l'hôte de l'adresse que l'utilisateur a saisie pour un compte, et lui seul, http et réseau local permis pour lui (instances auto-hébergées) |
| `local:shared` | `network: ["local:shared"]` | http vers `127.0.0.1:<port>`, seulement le port que le cœur a lu dans la sortie d'un process partagé qu'il a lancé pour ce plugin, tant qu'il vit (OpenCode) |
| `net/stream`, `net/chunk`, `net/end`, `net/credit`, `net/cancel` | les mêmes hôtes | un crédit : l'hôte ne lit plus la socket au-delà de 256 Kio non consommés ; 8 flux par plugin ; 120 s sans octet ferment un flux, sauf `"sse": true` |
| `ws/open`, `ws/send`, `ws/close`, `ws/message`, `ws/closed` | les mêmes hôtes, `wss` seulement | 4 sockets, 1 Mio par message, file bornée à 256 Kio, en-têtes hors du journal |
| `timer/set`, `timer/cancel`, `timer/fired` | aucune (un droit de base) | 10 ms au moins, 64 minuteurs |
| `fs/read`, `fs/write`, `fs/stat`, `fs/list`, `fs/canon` | `files: [{ root, access }]` | les racines sont une liste fermée : `plugin`, `agent`, `agentRepo`, `agentHome`, `temp` ; le chemin résolu, liens compris, doit être sous une racine de cet agent-là ; la clôture d'un projet privé est appliquée **ici, par l'hôte** ; 256 Kio en lecture par défaut (`limits.fileBytes`, 4 Mio au plus) |
| `fs/gitProtected`, `fs/fileOnly` | `files` | pour un harness : si git ne peut rien lancer de l'agent dans son dossier, et si l'écriture d'un chemin n'atteint qu'un fichier |
| `proc/spawn`, `proc/write`, `proc/kill`, `proc/out`, `proc/exit` | `commands`, `env` | la commande et ses arguments commencent par une entrée déclarée, mot pour mot (`*` en fin d'entrée) ; `dir` est une racine, jamais un chemin libre ; l'environnement d'un agent sans les secrets d'Escouade, plus les variables de `permissions.env` seules, jamais une des familles refusées (§1.3, `procs::var_refused`) ; 4 à la fois ; pas de shell, pas de `.cmd` ni de `.bat` ; le programme meurt avec l'instance |

`proc/*` est la surface la plus forte : le module est isolé, le programme qu'il lance ne l'est pas. Le store refuse `*` sur un programme généraliste pour un tiers, sauf justification relue. `files` demande aussi une justification à la revue (`agentHome` contient les identifiants d'un harness).

### 3.4 Le texte de la fenêtre d'installation

Un plugin sans programme : ses permissions, sans programme. Un module : « Tourne dans Escouade, isolé », puis une ligne par capacité (« Se connecte à », « Lit les fichiers de tes agents », « Écrit dans les dossiers de tes agents » en orange, « Lance cette commande sur ton ordinateur » en rouge, « Pose cette variable d'environnement sur ce qu'il lance », « Décide des autorisations de tes agents » en rouge, « Reçoit ces événements », « Garde un secret »). L'utilisateur peut retirer chaque permission.

### 3.5 Les limites

Par défaut **64 Mio de mémoire, 2 s par tour, 500 ms pour une décision**. Le manifeste (`limits`) en demande plus, jusqu'aux plafonds de l'hôte : 256 Mio (`MEMORY_MAX`), 10 s par tour (`TURN_MAX`). Le temps d'attente du réseau n'est pas dans un tour. Une instance qui dépasse, boucle sans fin ou panique **échoue comme un plantage** : trois relances en cinq minutes (une file de 256 messages attend la relance), puis le plugin est arrêté, « Relancer » le remet. Ce que tenaient les agents attachés est redonné par `agent/attach`. Les modules sont précompilés à la première activation (50 à 100 ms), gardés dans un cache local, jamais reçus du réseau.

## 4. Écrire un plugin

- **TypeScript** : `sdk/plugin` (`@escouade-dev/plugin`, MIT). Le programme (`run.js`) est un bundle qui n'importe que `node:fs` et `node:readline` et n'a pas d'`export` ; le store le compile. `sdk/create-escouade-plugin` écrit le gabarit ; le SDK donne un faux Escouade en mémoire (`@escouade-dev/plugin/testing`) pour tester sans l'app.
- **Rust** : `crates/escouade-plugin-kit` (avec `escouade-plugin-protocol` et `escouade-acp`, sur crates.io ; un `[patch.crates-io]` de `src-tauri/Cargo.toml` les ramène sur les chemins locaux dans le dépôt). Le plugin est un `cdylib` pour `wasm32-wasip1` (la chaîne d'outils épinglée de `plugins/rust-toolchain.toml`) ; la même source compile aussi en natif pour `cargo test`. `serve::run` donne la boucle ; un hébergeur donne un `ForgeKind` et un `SourceKind` à `Hosts::new` ; un harness écrit son `Plugin::handle` ; le kit a `http` (le client, qui suit les redirections), `time` (`sleep`, `timeout`), `host` (les fichiers, les programmes). Pas de thread, `std::process` échoue.
- **Ajouter un plugin de première main** : un dossier `plugins/<id-court>/` (manifeste, README, LICENSE, CHANGELOG, tests, un `Cargo.toml` membre de `plugins/Cargo.toml`), son identifiant dans `src-tauri/src/plugins/builtin.rs` (`REGISTRY`) pour que l'amorce le connaisse, et, côté cœur, ce qu'il faut pour que le point d'extension le trouve (`Provider::plugin()`, `TicketSourceKind::plugin()`, `harness::BUILTIN_HARNESSES`). Ses tests de cœur jouent contre son module, construit par `scripts/plugins-wasm.sh`.
- **L'essayer** : `escouade-plugin test` (le banc de conformité), `dev` (recharge), `validate`, `build` (compile un `run.js` ou un `run.wasm` comme le store). La vue Plugins › « En développement » charge un dossier en place. Dans le dépôt, `plugins/ci.mjs` fait tout ce qu'un auteur fait avant une étiquette (LICENSE, CHANGELOG, tests, `validate`, banc `--first-party`).

La documentation à jour, avec des exemples compilés (`sdk/doc-examples/{ts,rust}`), est dans les pages du site `plugins/*`.

## 5. La première main

**Les 28 plugins `dev.escouade.*` vivent dans `plugins/<id-court>/`**, chacun lisible comme le dépôt d'un plugin tiers, avec son identifiant, sa version et son `CHANGELOG.md`. Ils sont publiés, vérifiés et signés au store comme n'importe quel plugin, et c'est le store qui les compile.

| Genre | Plugins |
| --- | --- |
| Harness | `claude` (`dev.escouade.claude-code`), `codex`, `copilot` (`copilot-cli`), `cursor` (`cursor-cli`), `gemini` (`gemini-cli`), `antigravity` (`antigravity-cli`), `opencode`, `mistral` (aussi la voix) |
| Harness décrit | `acp` (le client qui les sert), `goose` (déclaratif) |
| Sources de tickets | `trello`, `linear`, `jira`, `clickup` ; `github`, `gitlab`, `azure-devops` en ont aussi |
| Hébergeurs | `github`, `gitlab`, `bitbucket`, `gitea`, `azure-devops` |
| Voix | `openai`, `groq`, `elevenlabs`, `deepgram`, `mistral` |
| Environnement de lancement | `isola` |
| Notifications, thème, règles | `discord` (déclaratif), `slack` (TypeScript), `high-contrast` (déclaratif), `team-rules` (TypeScript) |

Le nom du dossier n'est pas toujours le dernier mot de l'identifiant : le store relie un dossier à un id par son manifeste. Le faux plugin des tests est un module `run.js` (`tests/fixtures/plugins/fake/plugin.mjs`) ; la démonstration `dev.escouade.minimal` est un module commité (`tests/fixtures/plugins/minimal/`), qui n'est pas un produit.

### 5.1 L'amorce

**L'app marche dès son installation et hors ligne, et un harness n'attend aucun téléchargement.** L'installeur porte une **amorce** : pour chaque plugin de première main, ce que le store a publié pour sa dernière version (`version.json`, sa signature, l'archive, `plugin.wasm`), les mêmes octets, la même signature. `plugins/seed.json` nomme les 28 versions (`id`, `version`, empreinte de `version.json`) ; les octets sont dans `src-tauri/seed/`, copiés dans les ressources de Tauri ; `release.yml` télécharge l'amorce nommée (`store/scripts/seed.ts`) et vérifie chaque signature avant de construire. **Le chef publie d'abord, la release ensuite.** Un build de développement écarte les entrées dont les fichiers manquent (`Seed::of_app`) et lance les modules de `scripts/plugins-wasm.sh` sous l'origine `workspace` (jamais en release).

**L'installation depuis l'amorce** passe par le chemin d'une installation du store, sans réseau : mêmes vérifications, mêmes dossiers, origine `store`, avec une marque `seed`. **Le tri** (`plugins/sort.rs`, ESC-373) n'installe que ce que la configuration utilise, lue dans les réglages, les projets, les agents et les comptes, jamais sur le réseau :

- un harness : un compte de ce harness, un agent sur lui, le harness par défaut d'un projet ou celui des textes rédigés ;
- une source ou un hébergeur : un projet qui l'a pour source ou dont un remote est chez lui, un compte déjà enregistré ;
- un service de voix : choisi « En ligne », ou une clé gardée ;
- isola : un projet avec un `.isola.toml` ;
- Discord, Slack, Contraste élevé, Règles de l'équipe : seulement s'ils sont déjà installés.

Ce qui ne se lit pas sûrement s'installe : une mise à jour ne fait jamais perdre une fonction de la 2.1. Une entrée `builtin` d'un `plugins.json` de 2.1 n'est plus un plugin : ses choix sont gardés (`.kept/`), et le tri l'installe avec eux s'il la désigne. Une première installation n'installe rien d'office ; « Ajouter un compte… » sur un harness, et tout geste qui demande un plugin absent, l'installe depuis l'amorce (`Core::need_plugin`) et le dit. La vue Plugins range les autres sous « Proposés par Escouade », installables d'un clic. Un plugin désinstallé n'est pas remis au lancement.

**Ensuite, le store.** Les plugins de première main se mettent à jour par lui, comme tout plugin : une nouvelle version est proposée, jamais installée seule ; on ne redescend jamais de version ; sans réseau ou sans store, tout marche à la version de l'amorce. Les plugins de première main **se coupent et se désinstallent** comme les autres.

## 6. Le store, la signature et la confiance

- **Les origines** : un dossier local, un dépôt git à une étiquette, le store (l'amorce en est une), « En développement » (un dossier chargé en place et relu à chaque changement).
- **L'intégrité** : chaque version installée est fixée par son empreinte (le commit, le SHA-256 du dossier, ou `store:<SHA-256 de version.json>`) ; une copie dont les fichiers ont changé est chargée désactivée.
- **La signature** : une version revue en local (`/review-plugin`, `npm run plugin-review`) est signée par le chef d'escouade avec une clé minisign gardée sur son poste, jamais sur le serveur ; l'app la vérifie avec les clés publiques embarquées (`store_keys.rs`) : « Vérifié par Escouade », version par version, retirable par une liste de révocation signée. **Avant de lancer un module**, qu'il vienne du store ou de l'amorce : la signature de `version.json` et son commentaire de confiance (`escouade-verified-v1 <id>@<version>`), l'empreinte de `plugin.wasm` et de l'archive contre `version.json`, la liste de révocation (celle du store, sinon celle de l'amorce), `escouade.min` et `max`, et, pour `dev.escouade.*`, **la signature, sinon refus**. Un `dev.escouade.*` révoqué sans remplaçant laisse le plugin coupé, la raison dite.
- **`dev.escouade.*` est réservé** : le store ne l'accepte que d'une soumission du dépôt de l'app (`STORE_FIRST_PARTY_REPO`, comparé par son id GitHub) ; l’app ne le prend que d’une version signée du store ou de l’amorce (`Hand::First`), jamais d’un dossier en développement, d’un dossier copié ni d’un dépôt cloné, qui sont de la main de n’importe qui quel que soit leur id (audit SEC-27) ; le banc et `escouade-plugin` le prennent avec `--first-party`.
- **La compilation est au store** : il compile un `run.js` (Javy) et un `run.wasm` (cargo, `wasm32-wasip1`, chaîne d'outils épinglée) dans un conteneur sans réseau, et compare ce que la revue locale recompile. Les onze contrôles de la soumission refusent `run.command` (n° 10) ; `docs/STORE.md` les dit.
- **Publier un plugin de première main** : l'étiquette `plugins/<dossier>/vX.Y.Z`, `submit_version`, les contrôles, `publish_version` (non vérifié, en demandant la revue), la revue locale, la signature. `npm run plugins:publish -- --changed` enchaîne tout cela ; **le chef seul le lance**, un agent de ticket jamais.
- **N'installer que des versions vérifiées** (ESC-424) : allumé pour une nouvelle installation (`Settings::default`) ; une installation d'avant garde son choix, éteint si elle ne l'avait jamais touché.
- **La mise à jour** : jamais en silence ; l'accord est redemandé si le plugin demande plus ; une version refusée laisse la précédente.
- **La télémétrie** : l'événement `plugin_installed` part avec `from_store` oui ou non, sans le nom du plugin, si l'utilisateur a accepté les statistiques. Un plugin n'a pas accès à la télémétrie d'Escouade.

## 7. L'activation, les projets privés, les secrets

- **Installé** n'est pas **activé**. Un plugin s'active pour toute l'app, ou projet par projet ; par défaut pour tous les projets **sauf les projets privés**, où il n'entre que s'il y est activé un par un (ESC-140) : ses faits, ses tickets et ses textes ne vont pas aux autres plugins, et un harness n'y lance son premier agent qu'une fois activé.
- **Un projet qui propose un plugin** (`.escouade.yml` : `plugins.recommended` et `plugins.<id>`) ne l'installe ni ne l'active : une bande le propose. `settings/get` avec un `project` donne les réglages du fichier par-dessus ceux de l'utilisateur (les clés de `contributes.settings`, jamais un secret).
- **Les secrets** vont au coffre (`integrations/secrets.rs`), sous `plugin:<id>:<clé>`, saisis dans les champs que le plugin déclare ; il n'obtient que les siens (`notYours`, -32001 sinon), jamais ceux d'un compte, d'un harness ou d'un autre plugin ; le journal les masque.
- **L'utilisateur voit** avant l'installation ce que le plugin fera, permission par permission, et retire après : désactiver, retirer une permission (l'instance repart sans elle), désinstaller (dossier, réglages, secrets ; une case garde les données), lire le journal de chaque plugin.

## 8. Ce qui reste au cœur

Restent au cœur, hors de tout plugin : les agents et leur état (`agent.rs`, `conv.rs`), le lancement des process des harness (`proc.rs`, `job.rs`, `plugins/shared.rs`), les terminaux et le PTY, le Kanban et le pilote auto, la **synchro** d'une source (`integrations/sync.rs`) et le flux des PR d'un ticket (le plugin donne les appels, le cœur décide), git local et les worktrees, le serveur MCP d'Escouade (`mcp/`), la clôture des projets privés (`fence.rs`), le coffre, la jauge des comptes et le changement au seuil, les textes rédigés d'Escouade (celui d'Antigravity CLI passe par son plugin, `engine/oneShot` dans un dossier que le cœur fait et retire), le coût d'un tour et sa grille de prix (`pricing.rs`, OpenAI compris), Whisper et Piper, **les mises à jour d'Escouade, jamais par un plugin**. Un plugin n'ajoute pas d'outils au serveur MCP.

**Ce que le cœur ne copie plus** (ESC-359). Il gardait des copies des mots des plugins de harness pour lancer leurs interfaces et lire le quota et les modèles d'un compte (`engine/<harness>/{words,prices,client,rules}.rs`, `agent/{words,localcmd}.rs`) : elles sont parties, les plugins servant `terminal/launch` et `account/read`. Ce qu'il garde d'un harness est à lui : les variables de ses comptes, le menu de ses modèles tant qu'un compte n'en a pas dit, les fichiers qu'il lit (le journal d'une vue) ou retire (le serveur d'Escouade d'un dossier d'agent, son process fini), la règle d'effort du menu. Les commandes locales de Claude Code lui arrivent lues par le plugin (`Item::Replayed::local`).

**Les commandes d’installation** (ESC-441). Le cœur ne garde plus de copie de la commande qui installe un harness (`const INSTALL` des huit façades `engine/<harness>/`) : il la lit dans le manifeste de son plugin (`contributes.harnesses[].install`), celui du harness branché qui fait tourner ses agents, sinon le manifeste que l’app livre de ce plugin (`plugins::harness::install_of`). Elle sert à « Aucun harness trouvé » de Réglages › Comptes et harness, à la ligne « Claude Code est introuvable » d’une conversation et à celle d’OpenCode 2. Un manifeste qui n’en déclare pas donne une commande vide : « introuvable » ne nomme alors que `npm install -g @anthropic-ai/claude-code`, qui répare une installation par npm coupée en route (ESC-133), et OpenCode 2 dit « Installe OpenCode 1. » sans commande. Restent au cœur les commandes de mise à jour (`update_command`) et les noms des paquets npm, qui ne sont pas des adresses.

**Le quota de Claude Code** (ESC-441). Le cœur ne parle plus à `api.anthropic.com` : le quota d'un compte sans agent vient de `account/read` de `quota`, servi par `dev.escouade.claude-code` (un `claude` du compte, `initialize` puis `get_usage`, sans tour ni session), comme celui de Codex et de Copilot CLI. Le cœur garde ce qui est à lui : le rythme (un compte au plus toutes les 5 minutes), la jauge et le changement de compte au seuil, et le savoir de la connexion (présente, absente ou expirée), lue sans en garder le jeton. Aucun jeton ne passe par un plugin. L'autre voie, le plugin appelant l'endpoint avec un jeton que le cœur lui aurait passé (`permissions.network` vers `api.anthropic.com`), est écartée : elle aurait remis à un module un jeton qui vaut l'abonnement de l'utilisateur sur toute l'API, pour gagner 0,8 s sur une lecture qui a lieu au plus toutes les 5 minutes (docs/PROTOCOL.md, « Le quota d'un compte sans agent »).

### 8.1 Écarts connus entre le code et ce que la doc voudrait (à ne pas corriger ici)

- `readOnlyMode` d'un harness décrit reste cru d'un tiers en `"fs": false` : le client refuse les écritures qu'il voit, pas celles que l'agent fait seul. Un programme de première main peut lire d'autres variables de l'environnement d'un agent que celles qu'il pose : `permissions.env` borne ce qu'un plugin ajoute, pas ce qu'il hérite.
- Dans le code, des commentaires renvoient à l'ancienne numérotation de ce document (« §8.x », « §4.x », « §5.x »), qui n'existe plus ici : ils se lisent dans le texte de l'étude (voir l'annexe).

## Annexe : l'historique

Ce document était l'étude d'ESC-170 (2026-10-25), devenue référence avec ESC-184 (2026-11-04), puis le plan du chantier « tout en WASM ». Le texte complet de cette étude, avec ses mesures, ses plans d'étapes et le détail de chaque portage, est gardé tel quel dans `docs/PLUGINS-DESIGN.md`. C'est sa numérotation que citent les renvois « `PLUGINS-DESIGN.md` §… » du code, de la spec et de la CI, et les « `PLUGINS.md` §… » des décisions et des commits d'avant le 2026-11-11.

| Date | Étape | Ce qu'elle a changé |
| --- | --- | --- |
| 2026-10-25 | ESC-170 | L'étude : VS Code, Zed, Obsidian, Raycast, JetBrains, MCP ; trois expériences (WASM, iframe, process) ; le protocole JSON-RPC, le manifeste, le modèle de confiance |
| 2026-10-26 | ESC-203 | « Tout ce qui est externe est un plugin », en process : **abandonné** le 2026-11-04 |
| 2026-10-29 à 11-01 | PLG, INT, STORE | Le manifeste, le runtime, le banc, le SDK, les événements, les thèmes, les harness ACP décrits, les comptes, les adaptateurs, la première main en process (`escouade-plugins`, l'origine `builtin`), la place de marché |
| 2026-11-01 | ESC-319 | Le prototype WASM des plugins JS/TS sans Node : Javy, wasmtime, mesures ; pas de webview |
| 2026-11-04 | ESC-330 | Décision de l'utilisateur : tous les plugins en WASM, la première main comprise ; étude de l'hôte, des capacités, de la compilation Rust, de l'amorce |
| 2026-11-05 à 11-08 | A1–A9, P1–P8, K1, Z1–Z2 | Le tour réacteur, `net/stream`, `ws/*`, `fs/*`, `proc/*`, le kit, le portage de chaque plugin, le store qui compile, l'amorce et son tri |
| 2026-11-11 | ESC-355 (Z3), décision 0144 | Le natif est parti : plus de transport par process, de `escouade-plugins` ni d'origine `builtin` ; `run.command` ne démarre plus |
| 2026-11-11 | ESC-227 | Cette référence |
| 2026-11-12 | ESC-359 | `terminal/launch` servi par les plugins, `account/read` pour un compte ; le cœur rend ses copies (étude §8.34) |
