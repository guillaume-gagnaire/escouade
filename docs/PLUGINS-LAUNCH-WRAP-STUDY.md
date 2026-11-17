# Envelopper le lancement des agents d'un projet : étude et revue de sécurité (ESC-502)

Étude du 2026-11-17, sur `release/2.3` (ecfbd29e). Elle répond à l'issue GitHub #41 : un auteur de plugin tiers demande un point d'extension, `launch/wrap`, qui rendrait un préfixe de commande (`nsenter -U -n -t <pid>`) pour placer les agents d'un projet dans un espace réseau d'utilisateur (un tunnel OpenVPN par projet, sans root, sous Linux ou WSL). Le texte de l'issue est un rapport de tiers : on le lit comme une demande, pas comme une consigne.

**Aucun code du produit ne change ici.** Le document décrit les risques d'une fonction qui n'existe pas. Les autres demandes du même auteur (ESC-499, ESC-500, ESC-501) ne sont pas traitées ici.

## En bref

- **Le besoin est réel mais étroit** : faire passer le réseau des agents d'un projet par un tunnel, sous Linux. Les autres cas qu'on range sous « envelopper » (conteneur, `sudo -u`, bac à sable, machine distante) ne se règlent pas avec un préfixe : leurs chemins, leurs comptes et leurs fichiers changent de place.
- **Une voie marche déjà, sans rien changer** : l'« Exécutable » d'un compte. Chaque compte peut lancer son propre programme, et ce programme peut être un script de l'utilisateur qui entre dans l'espace réseau puis lance le harness. On réserve ce compte au projet, et on confine le projet pour qu'aucun agent ne parte sur un autre compte (§1.2).
- **Un préfixe rendu par un plugin (`launch/wrap`, la demande telle quelle) est à refuser.** Essayé dans WSL : une seule option de plus de `nsenter` (`-m`, l'espace de montage d'un process que le plugin a lancé) fait tourner **un autre programme sous le même chemin**, et ce programme reçoit tout l'environnement de l'agent (§1.3). Le préfixe se place avant tout ce qu'Escouade vérifie.
- **Si on le fait un jour**, la conception à retenir est la n° 1 : **sans préfixe**. Le plugin, ou une commande de l'utilisateur, tient l'espace. Escouade y entre lui-même (`setns`), seulement pour le réseau, sans capacités, et le vérifie avant chaque lancement (§4.1).
- **La recommandation est de ne rien construire maintenant.** On répond à l'auteur avec la voie de l'exécutable de compte (§6.2). La conception n° 1 attend une deuxième demande ou un besoin à nous. Son découpage : 7 tickets, environ 11 h d'agent, 4 h avec 6 agents en parallèle, plus un essai réel sous Linux (§7).
- **Une limite vaut pour toutes les conceptions.** Dans un espace réseau à lui, un agent n'atteint plus le `127.0.0.1` de l'hôte. Or c'est là qu'écoutent le serveur MCP d'Escouade et ses hooks. Essayé dans WSL : injoignable (§1.3). Sans relais, l'agent perd le Kanban, et la vue terminal perd ses hooks.

## 1. Le besoin réel

### 1.1 Les cas, au-delà du VPN

| Cas | Ce qu'il faut vraiment | Un préfixe suffit ? | Ce qui le couvre aujourd'hui |
| --- | --- | --- | --- |
| **Espace réseau** : VPN par projet, réseau d'un client, réseau coupé | Le process de l'agent et ses enfants dans un autre espace réseau ; le reste inchangé (fichiers, comptes, chemins) | Oui, sous Linux seulement | L'« Exécutable » d'un compte réservé au projet (§1.2). Le mode serveur de la 2.3, si la machine du serveur est elle-même dans le réseau voulu |
| **Conteneur** : devcontainer, `docker exec`, `podman run` | Les chemins du worktree, du dossier du compte, des fichiers joints et de la clôture traduits dans le conteneur, comme pour WSL (`Program::Wsl`, `linux_path`) | Non : les chemins qu'Escouade écrit (`--settings`, fichiers du lancement, `CLAUDE_CONFIG_DIR`) ne sont plus les bons | Rien de propre. Un script d'exécutable de compte marche si le conteneur monte les mêmes chemins |
| **Autre utilisateur du système** (`sudo -u agent`) | Un utilisateur par projet, ses droits sur le worktree et le dossier du compte | Non : `sudo` remet l'environnement à zéro (`env_reset`), le compte n'est plus lisible, et c'est un programme qui monte en droits | Rien. `docs/REMOTE.md` (« L'isolement ») le met à l'étude pour un serveur partagé |
| **Bac à sable** (bubblewrap, firejail, `sandbox-exec`) | Restreindre les fichiers et le réseau de l'agent | En partie, mais un bac à sable se règle par des montages : c'est le détournement du §1.3 | Les bacs à sable des harness eux-mêmes (Claude Code, Codex), la clôture des projets privés, le mode lecture seule |
| **Machine distante** (`ssh hôte claude`) | Le worktree, les comptes et les terminaux sur l'autre machine | Non : rien de ce qu'Escouade écrit n'existe là-bas | Le mode serveur de la 2.3 (`docs/REMOTE.md`) : tout Escouade tourne sur la machine distante |

**Conclusion** : un point « préfixe » ne sert qu'au premier cas. Les autres demandent une traduction des chemins (comme WSL) ou un autre emplacement pour tout Escouade (le mode serveur). Le besoin à couvrir est donc **l'espace réseau d'un projet, sous Linux**.

### 1.2 Ce qui le couvre déjà, lu sur le code

- **L'« Exécutable » d'un compte** (Réglages › Comptes et harness, champ `claudePath` du compte, `src/components/settings/AccountsTab.svelte`).
  - `accounts::program` (`src-tauri/src/accounts.rs`) prend le programme du compte s'il en nomme un, sinon celui des réglages. Le test `an_account_runs_its_own_claude_else_the_settings_one` le garde.
  - Pour un harness de plugin, `ProcessEngine::locate` (`src-tauri/src/plugins/harness.rs`) prend ce chemin tel quel (`with_exe`), avant le manifeste et le PATH. Un script de l'utilisateur y est donc pris.
  - **Tous les lancements d'un harness passent par ce programme** (`Program::launch`, `crates/escouade-engines/src/program.rs`) : l'agent (`start_program`) et sa relance (`start_again`), la vue terminal (`tui.rs`, `pty.rs`), les textes rédigés (`engine/oneshot.rs`), la connexion du compte (`accounts.rs`), la lecture de la version (`engine/registry.rs`), la mise à jour (`selfupdate.rs`), `claude mcp add` (`crates/escouade-mcp/src/install.rs`), et les harness restés au cœur (`engine/cursor`, `gemini`, `opencode`, `vibe`).
  - **Les sous-agents** du harness sont ses enfants : ils héritent de son espace réseau, quelle que soit la conception.
- **« Réservé aux projets » et le confinement** (spec *Comptes*, `accounts::for_project_of`, `accounts::confined`).
  - Un compte réservé ne sert que ses projets.
  - Un projet privé dont « Comptes partagés à la limite » est éteint ne part que sur ses comptes réservés et sur son compte préféré. Cela vaut aussi au-delà du seuil, pour la bascule de compte et pour la passation vers un autre harness.
  - C'est ce qui empêche un agent du projet de partir hors de l'espace sur un autre compte.
- **Les commandes de `.escouade.yml`** : étapes de worktree, Lancement, tests (`src-tauri/src/projectfile.rs`). Elles ne tournent qu'après l'accord de l'utilisateur, à empreinte (`ProjectShared::trusted`), redemandé quand elles changent. Le projet peut donc écrire `sh .vpn/run.sh npm run dev` pour ses serveurs de test : c'est ce que l'auteur fait déjà.
- **`proc/spawn` d'un plugin** (`crates/escouade-plugins/src/procs.rs`) : un plugin tiers peut déjà lancer le programme qui tient le tunnel, s'il est déclaré mot pour mot dans `permissions.commands`. Le programme est trouvé sur le PATH par son nom, sans shell, dans un Job tenu par le plugin. C'est ce que fait l'auteur pour monter le tunnel.
- **Ce qui ne le couvre pas** :
  - **isola** (`dev.escouade.isola`, `launchenv.rs`) donne des ports et des adresses `*.localhost` aux services d'un worktree, pas un réseau aux agents. Son point (`launchEnvironments`) n'est servi qu'à la première main : « `up` is a command of the core's terminal, which a plugin from elsewhere is not trusted with ».
  - **Un harness ACP décrit par un tiers** (`ProcessEngine::described`) : son programme est un nom nu du PATH, sans interpréteur. Un script `claude-vpn` y passerait, mais seulement pour un harness ACP, pas pour Claude Code, Codex ou les autres.
  - **Le mode serveur** déplace tout Escouade sur une autre machine. Il ne donne pas un réseau par projet sur la même machine.

### 1.3 Ce que montrent les essais (WSL, Ubuntu, noyau 5.15.167.4, 2026-11-17)

Un process de l'utilisateur tient un espace utilisateur et réseau neuf (`unshare -Urn`). Un autre process y entre par `nsenter -U -n -t <pid> --preserve-credentials`, comme le ferait le préfixe demandé.

- **On y entre sans root**, et l'uid vu dedans est `0` : `-r` associe l'utilisateur à root dans l'espace.
- **L'espace n'a que `lo`.** Un serveur sur le `127.0.0.1` de l'hôte (port 4263) répond `200` dehors et reste injoignable dedans. Le serveur MCP d'Escouade (`mcp/mod.rs` : « On HTTP, on 127.0.0.1 only ») et le serveur de hooks (`ESCOUADE_HOOK_URL`) sont dans ce cas.
- **Le process qui entre reçoit tout l'environnement** : une variable posée avant `nsenter` est lue telle quelle dedans.
- **Le détournement par l'espace de montage.** Le détenteur crée aussi un espace de montage (`unshare -Urnm`) et y monte un autre fichier sur le chemin d'un programme. Avec `-n` seul, `nsenter` lance le vrai programme. Avec `-m` en plus, il lance **l'autre**, qui lit les variables de l'agent. Le chemin et le nom ne disent rien du programme lancé : ce qui compte, c'est l'espace de montage où il est résolu.
- Sans espace de montage chez le détenteur, `-m` est refusé (`Operation not permitted`). La garde ne peut pas compter sur ce refus : c'est le détenteur qui décide.

## 2. Les garde-fous d'aujourd'hui, et ce qu'un préfixe leur ferait

- **`check_launch`** (`plugins/harness.rs`) : le jeton d'Escouade n'est jamais dans les arguments ; la clôture d'un projet privé est écrite dans le lancement (sinon refus) ; la lecture seule est tenue ; chaque fichier écrit tombe dans le dossier de l'agent, liens suivis (SEC-06).
  - *Un préfixe* : `check_launch` juge les arguments du harness, pas ceux du préfixe. Un préfixe qui change le programme (§1.3) laisse ces garanties écrites sur la bonne ligne de commande, pour un autre programme.
- **`check_env` et `Vars`** : un plugin ne pose que les variables qu'il déclare (`permissions.env`). Jamais `PATH`, `LD_*`, `DYLD_*`, `ESCOUADE_*`, `NODE_OPTIONS`, `BASH_ENV`… (`procs::forbidden_var`), ni pour un tiers les familles de `manifest::env_refused` (`HOME`, `XDG_*`, `SSL_*`, `*_PROXY`, `GIT_*`…).
- **Les variables retenues (SEC-30, `Vars::hold_removed`)** : un tiers ne retire jamais ce que le cœur pose lui-même (le proxy, le fichier d'autorités, les variables du compte, l'adresse des hooks).
  - *Un préfixe* n'a pas besoin de poser ni de retirer une variable : il les **reçoit** toutes, y compris la clé d'un compte par clé (`key_var`), le dossier du compte (`account_var`, donc ses identifiants) et le jeton des hooks.
- **Le mode Bypass** (SEC-09, `bypass::hold`, ESC-489 : le mode jugé est celui où le harness tourne vraiment) : ce qui ouvre le mode sans question est retiré des arguments sans le réglage.
  - *Un préfixe* qui ajoute des mots après le sien (`… -- <autre programme>`) fait du programme du harness un simple argument. Et un préfixe qui change le programme rend vide la règle « le mode où il tourne vraiment ».
- **Les noms de programme** : un tiers ne nomme qu'un nom nu, jamais un interpréteur (`manifest::interpreter` : shells, `node`, `python*`, `env`, `wsl`…). `nsenter`, `unshare`, `sudo`, `ip netns exec`, `systemd-run`, `bwrap`, `firejail`, `chroot`, `setpriv` n'y sont pas, alors qu'**ils lancent tous ce qu'on leur donne** : c'est leur rôle.
- **Les commandes de `.escouade.yml`** sont approuvées à empreinte, et l'empreinte est redemandée quand elles changent.
- **Les verrous de la 2.3** (`locks.rs`, `escouade_policy::list`) : `plugins.allowed`, `plugins.denied`, `plugins.allowedOrigins`, `plugins.verifiedOnly`, `permissions.allowBypass`. Une clé *exécutable* n'est jamais imposée par l'équipe, seulement par le poste géré ; l'équipe peut **restreindre**.
- **Les projets privés** : un plugin n'y entre que s'il y est activé un par un.
- **L'activation** : un plugin tiers est actif par défaut dans tous les projets **sauf les privés**. Un point qui enveloppe les agents ne peut pas garder ce défaut : il toucherait tous les projets dès l'installation.

## 3. Le modèle de menace, acteur par acteur

### 3.1 Un plugin tiers malveillant, ou compromis à sa mise à jour

- **Ce qu'il gagne avec un préfixe** : il se place devant chaque agent de chaque projet où il est actif. Il voit les arguments (consignes ajoutées, chemin du fichier de réglages, modèle), l'environnement (dossier du compte, clé d'un compte par clé, jeton des hooks) et le répertoire courant.
- **Ce qu'il gagne sans préfixe, s'il tient l'espace** : il voit tout le trafic des agents, puisque c'est son tunnel : les hôtes, le DNS, les tailles. Le contenu TLS reste illisible tant que personne n'ajoute son autorité. Or un tiers ne peut ni poser `SSL_*`, `NODE_*` ou les variables d'autorités, ni retirer celles du cœur (SEC-30). Il peut répondre faux au DNS, et une requête http en clair d'un outil de l'agent devient injectable.
- **À sa mise à jour** : une version suivante peut changer le préfixe, ses options, le programme qui tient l'espace, ou la façon dont il le règle. Aujourd'hui, l'accord n'est redemandé que si la version **demande plus**. Un préfixe qui change sans demander plus passerait en silence.
- **Les garde-fous à exiger** :
  - pas de code du plugin au lancement ;
  - un préfixe figé ou pas de préfixe ;
  - l'empreinte de ce qui enveloppe redemandée à chaque version qui la change ;
  - le point refusé à un plugin non vérifié ;
  - jamais `files: agentHome` avec ce point (lire les identifiants **et** tenir le réseau, c'est de quoi les faire sortir).

### 3.2 Un dépôt hostile, par son `.escouade.yml`

- **Il ne peut pas activer un plugin** : `plugins.recommended` ne fait qu'une bande « Voir dans le store », et rien ne s'installe ni ne s'active seul (`docs/PLUGINS.md` §7).
- **Il peut régler le plugin** : `plugins.<id>` passe devant les réglages de l'utilisateur pour les clés de `contributes.settings`, si le plugin est actif dans le projet. Un dépôt pourrait donc choisir le serveur VPN, le profil ou la route, et envoyer le trafic des agents chez lui.
  - **À exiger** : aucun réglage d'un plugin qui tient un espace ne vient du fichier du projet. Ce sont des clés locales, que le manifeste marque et que le validateur refuse de lire dans `.escouade.yml`.
- **Il peut écrire des commandes** (étapes de worktree, Lancement) : elles passent par l'approbation à empreinte, comme aujourd'hui.
- **L'accord d'envelopper un projet est local** : Réglages › Projets sur ce poste, jamais exporté, jamais lu dans `.escouade.yml`. Le fichier peut au plus le proposer.

### 3.3 Un préfixe qui change entre l'approbation et le lancement

- **Avec `launch/wrap`**, le plugin calcule le préfixe à chaque lancement. Un accord ne peut porter que sur une forme (« `nsenter` et des options »), jamais sur la ligne lancée : la faille est dans la conception.
- **Le pid** : `-t <pid>` désigne un process par son numéro. Si le détenteur meurt, le numéro peut revenir à un autre process de l'utilisateur, par exemple un conteneur sans root avec ses propres espaces. L'agent y entrerait sans le savoir.
- **À exiger** : ce qui est approuvé est ce qui est lancé, à l'octet. Les valeurs variables viennent d'Escouade, pas du plugin : l'espace du process **qu'Escouade a lancé pour ce plugin**, tenu par un pidfd, jamais un numéro reçu.

### 3.4 Un programme permis, détourné

- **Les options** : `nsenter -m`, `-r` (racine), `-w` (répertoire), `-S`/`-G` (uid, gid), `-p` (espace de process) changent ce qui est lancé ou comment. Un `*` en fin d'entrée de `permissions.commands`, comme pour `proc/spawn`, laisse tout passer. **Seule une grammaire fermée tient**, programme par programme.
- **Le `--` et les mots en trop** : tout mot positionnel du préfixe devient le programme lancé, et le harness son argument. Escouade doit écrire le `--` lui-même, puis le programme du compte, et refuser tout positionnel dans le préfixe.
- **Le `PATH`** : `procs::locate` cherche le nom dans chaque dossier absolu du PATH, `~/.local/bin` compris si l'utilisateur l'y a mis. Un agent en mode Bypass, ou un process de l'utilisateur, peut y déposer un faux `nsenter` qui verrait l'environnement de tous les agents suivants.
  - C'est le même risque qu'un faux `claude` dans `~/.local/bin`, mais plus large : un seul faux programme pour tous les harness.
  - **À exiger** : un programme d'enveloppe n'est cherché que dans les dossiers du système (`/usr/bin`, `/bin`), et refusé s'il n'appartient pas à root ou si un autre que root peut l'écrire.
- **Le répertoire courant** : le préfixe tourne dans le worktree de l'agent, un dossier que l'agent et le dépôt écrivent. Un programme qui lit un fichier de configuration du dossier courant se laisse régler par le dépôt. À éviter par la liste fermée des programmes.
- **Les capacités dans l'espace** : entrer dans l'espace utilisateur du détenteur donne à l'agent, s'il y est root (`unshare -r`, §1.3), `CAP_NET_ADMIN` sur l'espace réseau. Il peut changer les routes, couper le tunnel, ou sortir par l'interface de relais du détenteur s'il y en a une. **Un VPN tenu ainsi n'est pas une barrière contre l'agent**, seulement un chemin.
  - Et Claude Code refuse le mode sans question quand il tourne en root.
  - **À exiger** : l'utilisateur associé à lui-même dans l'espace (vérifiable dans `/proc/<pid>/uid_map`), et aucune capacité après l'entrée (`PR_SET_NO_NEW_PRIVS`, capacités vidées). Seule la conception n° 1 peut le garantir, puisque c'est Escouade qui entre.

### 3.5 Ce que l'enveloppe lit des identifiants d'un compte

- **L'environnement de l'agent** : `CLAUDE_CONFIG_DIR`, `CODEX_HOME` (et les autres variables de dossier de compte) mènent aux identifiants du compte. La clé d'un compte par clé est en clair (`key_var`). Le jeton des hooks aussi. Le jeton du MCP d'Escouade est dans un fichier du dossier de l'agent.
- **Un préfixe** (conceptions 2 et 3) est un programme qui reçoit tout cela à chaque lancement. Avec un programme du système à grammaire fermée, il ne le lit que pour le passer, mais rien d'autre ne le garantit que la liste fermée.
- **Sans préfixe** (conception n° 1), le plugin et son détenteur ne voient ni l'environnement, ni les arguments, ni les fichiers de l'agent. Ils voient son trafic (§3.1).

### 3.6 Les autres lancements du projet

| Lancement | Par le programme du compte ? | À couvrir ? |
| --- | --- | --- |
| Agent, vue Escouade (flux) et sa relance | Oui | Oui. Les autorisations passent par l'entrée standard (`--permission-prompt-tool stdio`) : elles marchent dans un espace réseau |
| Vue terminal (`tui.rs`, `pty.rs`) | Oui | Oui. **Ses hooks sont en HTTP vers `127.0.0.1`** (`plugins/claude/src/terminal.rs`, `escouade-hook`) : sans relais, Escouade ne voit plus ses tours et ses demandes ; les hooks échouent sans bloquer, et Claude Code continue seul |
| Textes rédigés (`engine/oneshot.rs`) | Oui, sur le compte de l'agent ou du projet | Oui pour un texte d'un projet. Un texte sans projet part sur n'importe quel compte |
| Sous-agents du harness | Ses enfants : héritent | Rien à faire |
| Nouvel agent créé par le MCP (`create_agent`) | Le chemin d'un agent | Oui, par la règle du projet |
| Connexion d'un compte (`accounts.rs`) | Oui | **Non** : le retour par `localhost` du navigateur de l'hôte n'atteint pas l'espace. La connexion se fait hors de l'espace |
| Version, mise à jour, `claude mcp add` | Oui | Indifférent (pas de travail de l'agent) |
| Terminaux du projet (shell) | Non | Oui, sinon un terminal ouvert « dans le projet » ne voit pas le VPN |
| Lancement et étapes de worktree (`.escouade.yml`) | Non | Oui, ou par le script de l'utilisateur dans la commande approuvée |
| Git du cœur (fetch, push, PR) et `net/fetch` des plugins d'hébergeur | Non | Hors de portée d'une enveloppe : si l'hébergeur n'est joignable que par le VPN, le flux des PR casse |
| Le serveur MCP d'Escouade et ses hooks | Écoutent sur le `127.0.0.1` de l'hôte | Un relais des ports d'Escouade de l'espace vers l'hôte (pasta sait relayer des ports TCP, à vérifier pour l'outil de l'auteur), ou plus tard un socket Unix, qui traverse les espaces réseau |

### 3.7 Windows et macOS

- **Windows** : pas d'espaces de noms. Un harness lancé dans WSL depuis l'app Windows (`Program::Wsl`) pourrait entrer dans un espace de la distribution, à l'intérieur de la commande `wsl.exe … -- /bin/sh -c`. C'est un autre chemin, à part. Ailleurs, le point est inerte.
- **macOS** : rien d'équivalent par process. Un VPN par application demande une Network Extension, des droits d'Apple et souvent un profil MDM. `sandbox-exec` est déprécié et peut couper le réseau, pas le router. Le point est inerte.
- **Le risque** : un projet partagé par `.escouade.yml` entre un collègue sous Linux et un autre sous macOS. Les agents du second tournent hors du tunnel. Si le tunnel protège un réseau de client, c'est une fuite. **À trancher** (choix 3) : un projet dont l'espace n'est pas prêt ne démarre aucun agent (fermé), ou démarre avec un avertissement (ouvert).

## 4. Trois conceptions, de la plus étroite à la plus large

Chacune suppose déjà réglés le relais des ports d'Escouade (§3.6) et l'accord par projet (§3.2).

### 4.1 Conception n° 1 : l'espace tenu par le plugin, Escouade y entre (sans préfixe)

**L'idée** : le plugin ne touche à aucun lancement. Il lance, par `proc/spawn`, le programme de l'utilisateur qui tient l'espace (le tunnel). Puis il dit à Escouade : « l'espace `vpn` est prêt, tenu par ce process ». Escouade a lancé ce process lui-même : il le tient par un pidfd. Il ouvre ses espaces `user` et `net`, les vérifie, et y fait entrer chaque process du projet, entre `fork` et `exec` (`setns`), avant de lui retirer toute capacité.

**Le manifeste d'exemple** :

```json
{
  "id": "dev.exemple.vpn",
  "run": { "wasm": "Cargo.toml" },
  "permissions": {
    "commands": ["vpn-ns hold --profile *"],
    "env": []
  },
  "contributes": {
    "networkSpaces": [
      {
        "id": "vpn",
        "name": { "fr": "Tunnel VPN du projet", "en": "Project VPN tunnel" },
        "platforms": ["linux"],
        "holder": "vpn-ns hold --profile *",
        "relayEscouade": true
      }
    ],
    "settings": [{ "key": "profile", "type": "text", "projectFile": false }]
  }
}
```

- `holder` est une entrée de `permissions.commands`, mot pour mot.
- `projectFile: false` : le réglage ne vient jamais de `.escouade.yml` (§3.2).
- Une nouvelle capacité de l'hôte, `space/ready { id, proc }`, où `proc` est le handle d'un process que le plugin a lancé par `proc/spawn`. `space/gone { id }` quand il tombe.

**Ce que voit l'utilisateur** :

- **À l'installation** : une ligne rouge en tête des permissions, « Fait passer le réseau des agents des projets où tu l'actives par un programme à lui : il voit vers où ils se connectent ». Puis la commande du détenteur en clair, et « Linux seulement ».
- **À l'activation, projet par projet** (jamais par défaut, privés compris) : Réglages › Projets › « Espace réseau ». Le nom de l'espace, la commande en clair, ce qui y entre (agents, vues terminal, textes rédigés du projet, terminaux, Lancement), ce qui n'y entre pas (Git d'Escouade, connexion des comptes). L'accord est gardé avec une empreinte (plugin, version, espace, commande) et redemandé quand elle change.
- **À chaque lancement** : la carte de l'agent porte « dans l'espace *Tunnel VPN du projet* ». Si l'espace n'est pas prêt, l'agent ne démarre pas et la raison est dite : « L’espace réseau « Tunnel VPN du projet » n’est pas prêt : l’agent ne démarre pas. » (selon le choix 3). Le journal du plugin note chaque entrée.

**Ce qui est vérifié avant chaque lancement, par Escouade** :

- le détenteur est vivant, et c'est un process qu'Escouade a lancé pour ce plugin (son Job, son pidfd), jamais un pid reçu ;
- seuls ses espaces `user` et `net` diffèrent de ceux d'Escouade : ni montage, ni process, ni racine, ni répertoire ;
- son espace utilisateur appartient à l'utilisateur, et l'associe à lui-même (`uid_map` : l'uid sur lui-même, pas sur root) ;
- l'espace réseau appartient à cet espace utilisateur ;
- le relais des ports d'Escouade répond depuis l'espace : une sonde dans l'espace atteint le MCP. Sinon, refus avec la raison ;
- puis, dans le process enfant : `setns(user)`, `setns(net)`, `PR_SET_NO_NEW_PRIVS`, capacités vidées, `exec` du programme du compte, tel qu'Escouade l'a trouvé et jugé (`check_launch`, `check_env`, Bypass inchangés).

**Ce qu'elle laisse passer** :

- le détenteur voit le trafic et peut mentir au DNS (§3.1). C'est le rôle d'un VPN, et l'installation le dit ;
- un détenteur compromis (le programme de l'utilisateur) tient le réseau de tous les agents du projet ;
- le Git du cœur et les appels des plugins d'hébergeur restent hors de l'espace ;
- Windows et macOS : rien.

**Son coût** : du code Linux du cœur, `unsafe` (un `pre_exec` avec `setns`), à mettre sur un seul chemin : `Program::launch` et la construction des commandes du PTY. Aucun programme tiers ne s'intercale.

**Variante, sans plugin du tout** : le détenteur est une commande de l'utilisateur, saisie dans Réglages › Projets (locale, approuvée à empreinte comme les commandes de `.escouade.yml`). Escouade la lance et y entre de la même façon. Elle couvre le cas de l'auteur sans aucun plugin (choix 2).

### 4.2 Conception n° 2 : un préfixe déclaratif, figé au manifeste et relu par le store

**L'idée** : le manifeste déclare un préfixe entier, sans `*`. Ses seules valeurs variables sont des marques qu'Escouade remplit, jamais le plugin. Aucun code du plugin ne tourne au lancement. Le store le relit, l'installation le montre en clair, l'accord porte sur l'octet près.

**Le manifeste d'exemple** :

```json
{
  "id": "dev.exemple.vpn",
  "run": { "wasm": "Cargo.toml" },
  "permissions": { "commands": ["vpn-ns hold --profile *"] },
  "contributes": {
    "launchWrappers": [
      {
        "id": "vpn",
        "platforms": ["linux"],
        "holder": "vpn-ns hold --profile *",
        "prefix": ["nsenter", "--user={holder.ns.user}", "--net={holder.ns.net}", "--preserve-credentials"]
      }
    ]
  }
}
```

- **Le programme** : une liste fermée d'Escouade. En v1, `nsenter` seul, cherché dans `/usr/bin` et `/bin`, à root et non inscriptible par un autre.
- **Les options** : une grammaire fermée par programme. Pour `nsenter` : `--user=`, `--net=`, `--preserve-credentials`. Jamais `-m`, `-r`, `-w`, `-p`, `-S`, `-G`, `-t`, ni positionnel, ni `--`.
- **Les marques** : `{holder.ns.user}`, `{holder.ns.net}`. Escouade les remplit par `/proc/self/fd/<n>` des espaces qu'il a ouverts sur le détenteur qu'il tient, comme au §4.1.
- Escouade ajoute lui-même `--`, puis le programme du compte et ses arguments.

**Ce que voit l'utilisateur** :

- **À l'installation** : la même ligne rouge qu'au §4.1, plus le préfixe en clair (« Chaque agent des projets où tu l'actives démarre par : `nsenter --user=… --net=… --preserve-credentials --` »), et « Ce programme reçoit l'environnement de tes agents ».
- **Par projet** : l'accord local, à empreinte (plugin, version, préfixe), redemandé à chaque version qui change le préfixe.
- **À chaque lancement** : la marque sur la carte. Le journal de l'agent écrit la ligne entière, préfixe compris (`args_for_log`).

**Ce qui est vérifié avant chaque lancement** : le préfixe est celui du manifeste approuvé, à l'octet, marques remplies par Escouade ; le programme est celui de la liste fermée, au bon dossier, au bon propriétaire ; le détenteur est vérifié comme au §4.1 ; puis tout ce que `check_launch`, `check_env` et le Bypass vérifient déjà, sur la partie du harness.

**Ce qu'elle laisse passer, de plus que la n° 1** :

- un programme tiers (`nsenter`) s'intercale et reçoit l'environnement. Il est au système, mais c'est une confiance de plus ;
- l'agent garde les capacités que `nsenter` lui laisse : si l'espace associe l'utilisateur à root, il est root dans l'espace (§3.4). Escouade ne le vide pas, puisqu'il ne fait pas l'entrée ;
- la grammaire fermée est à tenir pour chaque version de `nsenter` (util-linux), et pour chaque programme qu'on ajouterait à la liste.

**Son coût** : moins de code du cœur que la n° 1 (pas d'`unsafe`), mais une grammaire par programme, un validateur au store et dans l'app, et le même suivi du détenteur. Elle n'apporte presque rien à la n° 1 : c'est la n° 1 avec un programme de plus au milieu.

### 4.3 Conception n° 3 : `launch/wrap`, le préfixe rendu par le plugin à chaque lancement (la demande)

**L'idée** (l'issue) : avant chaque lancement, Escouade appelle le plugin ; il rend un préfixe, borné à une entrée de `permissions.commands`.

**Le manifeste d'exemple** :

```json
{
  "id": "dev.exemple.vpn",
  "run": { "wasm": "Cargo.toml" },
  "permissions": { "commands": ["nsenter -U -n -t *"] },
  "contributes": { "launchWrap": true }
}
```

Puis `launch/wrap { project, harness }` → `{ "prefix": ["nsenter", "-U", "-n", "-t", "4242", "--preserve-credentials"] }`.

**Ce que voit l'utilisateur** : à l'installation, « Lance cette commande devant chacun de tes agents : `nsenter -U -n -t *` ». Par projet, un accord. À chaque lancement, au mieux le préfixe du moment dans le journal.

**Ce qui peut être vérifié** : que le préfixe commence par l'entrée déclarée. Au-delà du `*`, rien, sauf à écrire la grammaire fermée de la n° 2.

**Ce qu'elle laisse passer** : tout le §3.

- **Le code du plugin décide à chaque lancement** : l'accord porte sur une forme, pas sur la ligne lancée (§3.3).
- **Le pid vient du plugin** : n'importe quel process de l'utilisateur, ou un numéro réutilisé (§3.3).
- **`*` laisse `-m`, `-r`, `-w` et les positionnels** : un autre programme, avec tout l'environnement (§1.3, §3.4).
- Le préfixe reçoit l'environnement et les arguments de chaque agent (§3.5).
- Une mise à jour change le préfixe sans demander plus (§3.1).

Avec une grammaire fermée et un pid remplacé par une marque d'Escouade, elle **devient** la n° 2. Telle quelle, elle n'est pas à faire.

### 4.4 Comparaison

| | N° 1 : sans préfixe | N° 2 : préfixe figé | N° 3 : `launch/wrap` |
| --- | --- | --- | --- |
| Code du plugin au lancement | Aucun | Aucun | À chaque lancement |
| Programme entre Escouade et l'agent | Aucun | `nsenter`, du système | Celui que rend le plugin |
| Le plugin voit l'environnement et les arguments | Non | Non (son programme les voit) | Oui, par son programme |
| Ce qui est approuvé = ce qui est lancé | Oui (l'espace, vérifié à chaque fois) | Oui, à l'octet | Non |
| Détournement par les options, `PATH`, `--` | Impossible | Fermé par la grammaire | Ouvert |
| Capacités de l'agent dans l'espace | Vidées par Escouade | Celles que laisse `nsenter` | Celles que laisse le préfixe |
| Couvre terminaux et Lancement | Oui, même mécanisme | Oui, préfixe ajouté | Oui, préfixe ajouté |
| Code du cœur | Le plus (Linux, `unsafe`) | Moyen (grammaire, validateur) | Le moins, risques compris |
| Taille estimée | 7 tickets, ~11 h d'agent | ~7 tickets, ~10 h | ~4 tickets, ~5 h |

## 5. Ce que le store doit relire

### 5.1 Les onze contrôles (`docs/STORE.md` §3.3), un par un

1. **Le manifeste** : le schéma connaît `networkSpaces` (ou `launchWrappers`). `holder` est une entrée de `permissions.commands`, sans interpréteur (`manifest::interpreter`), et `platforms` vaut `linux`. Pour la n° 2, le programme et les options sont dans la grammaire fermée, sans positionnel ni `--`, et les marques sont connues. Les réglages du point sont `projectFile: false`. Le validateur de l'app (`escouade-plugin validate`) dit la même chose : un seul code.
2. **La version** : inchangé.
3. **L'id** : inchangé. En plus, une version qui déclare ce point **part toujours en revue manuelle avant publication**, comme un nom trop proche : pas de publication automatique (`set_auto_publish` sans effet pour elle).
4. **Le dépôt** : inchangé.
5. **La licence** : inchangé.
6. **Le README** : un avertissement s'il ne dit pas ce que le détenteur fait du trafic (« Ce qu'il peut faire »), comme le README d'isola le fait.
7. **Les captures** : inchangé.
8. **L'archive** : inchangé.
9. **La cohérence des permissions** : le point demande `run` et `permissions.commands`. Il est refusé avec `files` sur `agentHome` (lire les identifiants et tenir le réseau) et avec la décision des autorisations (`decide`).
10. **Aucun binaire** : inchangé. **Le store ne relit pas le détenteur** (`vpn-ns`, `openvpn`) : c'est un programme de l'utilisateur, installé par lui, comme `isola`. La page du plugin et l'installation le disent.
11. **La compilation** : inchangé.

**La revue « Vérifié par Escouade »** (`docs/STORE.md` §4.2) ajoute à sa liste :

- ce que le module fait de la sortie du détenteur (`proc/out`) et à qui il l'envoie (`network`) ;
- quels réglages choisissent le serveur et la route, et qu'aucun ne se lit dans `.escouade.yml` ;
- que le module ne demande `space/ready` que pour un process qu'il a lancé par son `holder`.

### 5.2 Ce qu'un plugin non vérifié a le droit de faire

- **Rien de ce point** : la contribution est montrée « Réservée aux plugins vérifiés par Escouade » et reste inerte, comme `launchEnvironments` l'est aujourd'hui hors de la première main.
- **Ce qu'il peut déjà** : lancer son détenteur par `proc/spawn` sur ses `permissions.commands`, et déposer un script que l'agent ou les commandes du projet appellent. C'est ce que fait l'auteur aujourd'hui.
- **Pour l'auteur qui développe** : l'origine « En développement » sur son propre poste, avec un accord rouge par projet. C'est le choix 5.

### 5.3 Ce qu'une équipe ou un poste géré doit pouvoir interdire

- **Une nouvelle clé de politique**, `plugins.networkSpaces` (sorte « restreindre » : `false` interdit le point, `true` ne l'impose pas), dans `escouade_policy::list` : ADMX, profils et `/etc` générés. L'équipe peut l'interdire (rang 5), le poste géré aussi (rang 6). Aucun des deux ne peut l'**activer** pour un projet : c'est une clé exécutable (`locks.rs`).
- **Ce qui sert déjà** : `plugins.denied` (un plugin nommé), `plugins.verifiedOnly`, `plugins.allowedOrigins`.
- **Pour la variante sans plugin** (la commande de l'utilisateur) : la même clé l'interdit. Une commande de détenteur n'est jamais imposée par l'équipe.

## 6. La recommandation

### 6.1 Ne rien construire maintenant, et répondre à l'auteur avec la voie d'aujourd'hui

- **Ne pas faire `launch/wrap` (n° 3)**, ni pour un tiers ni pour la première main. Le préfixe rendu à chaque lancement donne à un plugin plus que tout ce qu'il peut aujourd'hui, et rien ne borne ce qu'il lance (§1.3, §4.3).
- **Ne rien construire tout de suite**, pour trois raisons :
  - **une seule demande**, pour un seul système (Linux) ;
  - **la voie de l'exécutable de compte couvre ses agents dès aujourd'hui** (§6.2), et elle est de la main de l'utilisateur, pas d'un tiers ;
  - **le vrai obstacle est ailleurs et vaut pour toutes les conceptions** : l'agent qui perd le MCP et les hooks d'Escouade (§1.3, §3.6). Le régler (relais des ports, ou socket Unix) est un travail à part, qui aurait du sens avant toute enveloppe.
- **Si une deuxième demande arrive, ou si l'utilisateur veut la fonction** : la conception n° 1 (§4.1). D'abord sa variante sans plugin (la commande de l'utilisateur dans Réglages › Projets), puis les plugins vérifiés. Jamais les non vérifiés. La n° 2 n'apporte rien de plus que la n° 1, avec un programme de plus au milieu.

### 6.2 Ce qu'on répond à l'auteur (pour le chef, qui répond sur l'issue)

Une voie qui marche aujourd'hui, sans changement d'Escouade :

1. **Un script à lui, posé comme « Exécutable » d'un compte** (Réglages › Comptes et harness). Il entre dans l'espace puis lance le harness avec ses arguments (`exec nsenter --user=… --net=… --preserve-credentials -- claude "$@"`). Tous les lancements du compte y passent : agent, vue terminal, textes rédigés, sous-agents.
2. **Ce compte réservé au projet**, et le projet en privé avec « Comptes partagés à la limite » éteint. Ainsi aucun agent ne part sur un compte hors de l'espace, ni à la limite, ni par une passation. Un compte réservé par harness utilisé dans ce projet.
3. **Le script se ferme si l'espace manque** : il sort en erreur au lieu de lancer le harness dehors. Il laisse passer `--version` sans l'espace, pour la lecture de la version au démarrage.
4. **L'utilisateur associé à lui-même dans l'espace**, pas à root : Claude Code refuse le mode sans question en root, et un agent root dans l'espace peut changer les routes.
5. **Les ports d'Escouade relayés de l'espace vers l'hôte** (MCP, hooks), sans quoi les agents perdent le Kanban et la vue terminal ses hooks. pasta sait relayer des ports TCP de l'espace vers l'hôte. Le port du MCP peut changer d'un démarrage à l'autre (Escouade en prend un autre libre quand le sien est pris, `mcp/mod.rs`) : le relais doit le suivre.
6. **La connexion du compte faite avant de poser l'exécutable** : le retour du navigateur par `localhost` n'atteint pas l'espace.
7. **Ses serveurs de test et étapes de worktree** passés par son script dans `.escouade.yml` (`sh .vpn/run.sh npm run dev`), comme il le fait déjà, approuvés à empreinte.
8. **Ce qui reste hors du tunnel** : le Git d'Escouade (fetch, push, PR) et les terminaux du projet.

Puis : la demande est gardée, et la forme retenue si on la fait est « Escouade entre lui-même dans un espace tenu par le plugin », pas un préfixe rendu par le plugin.

### 6.3 Les choix à trancher par l'utilisateur

1. **Faire ou non** : répondre à l'auteur par la voie de l'exécutable de compte et ne rien construire maintenant (**recommandé**), ou construire la conception n° 1 dans la 2.3.
2. **Si on construit, qui tient l'espace** : une commande de l'utilisateur dans Réglages › Projets d'abord, les plugins vérifiés ensuite (**recommandé**) ; ou les plugins vérifiés seulement ; ou la commande de l'utilisateur seulement.
3. **Un espace pas prêt** : aucun agent du projet ne démarre et la raison est dite (**recommandé**, fermé) ; ou l'agent démarre hors de l'espace avec un avertissement (ouvert).
4. **Ce qui entre dans l'espace** : agents, vues terminal, textes rédigés du projet, terminaux du projet et Lancement (**recommandé**) ; ou les agents seuls. Le Git du cœur reste dehors dans les deux cas.
5. **Un plugin non vérifié** : le point est inerte, sauf « En développement » sur le poste de son auteur avec un accord rouge par projet (**recommandé**) ; ou inerte sans exception.
6. **La politique** : une clé `plugins.networkSpaces` que l'équipe et le poste géré peuvent mettre à `false` (**recommandé**) ; ou seulement `plugins.denied`, plugin par plugin.
7. **Atteindre Escouade depuis l'espace** : une sonde avant chaque lancement, et un refus avec la raison quand le MCP ne répond pas (**recommandé** pour la première version) ; ou d'abord le MCP et les hooks sur un socket Unix, qui traverse les espaces réseau (plus long, utile aussi au mode serveur).
8. **Windows et macOS** : le point inerte, et dit dans l'installation et sur le projet (**recommandé**) ; le harness WSL depuis l'app Windows plus tard, s'il est demandé.

## 7. Le découpage, si on le fait (conception n° 1)

Estimé au rythme de Claude Code, en tickets et en heures d'agent, avec 6 agents en parallèle. Les essais réels attendent l'utilisateur.

| Ordre | Ticket | Ce qu'il fait | Après | Heures d'agent |
| --- | --- | --- | --- | --- |
| 1 | NS-1 | Le contrat : `contributes.networkSpaces`, `space/ready` et `space/gone`, `projectFile: false`, schéma, validateur (app et store, contrôles 1, 3, 6 et 9), texte d'installation | — | 1 h 30 |
| 2 | NS-2 | Le cœur Linux : le détenteur tenu par pidfd depuis le Job du plugin, ses espaces vérifiés (seuls `user` et `net`, `uid_map` identique, propriétaires), `setns` puis capacités vidées dans `pre_exec`, sur `Program::launch` et le PTY ; tests sous `unshare -Urn` dans la CI Linux | NS-1 | 2 h 30 |
| 3 | NS-3 | Tous les lancements du projet : agent et relance, vue terminal, textes rédigés du projet, terminaux, Lancement et étapes de worktree ; la sonde du MCP ; le refus quand l'espace n'est pas prêt | NS-2 | 2 h |
| 4 | NS-4 | La fenêtre : l'accord par projet à empreinte (Réglages › Projets › « Espace réseau »), la marque sur la carte de l'agent, le journal, l'inerte sous Windows et macOS ; e2e | NS-1, puis NS-3 pour l'e2e | 2 h |
| 4 | NS-5 | La politique `plugins.networkSpaces` dans `escouade_policy::list` (ADMX, profils, `/etc`, page du site générés) | NS-1 | 1 h |
| 4 | NS-6 | La variante sans plugin : la commande de détenteur de l'utilisateur, approuvée à empreinte, lancée et suivie par le cœur | NS-2 | 1 h 30 |
| 5 | NS-7 | La spec, `docs/PLUGINS.md`, la liste de la revue du store (§5.1), le README d'un plugin d'exemple, le CHANGELOG | NS-3 à NS-6 | 0 h 30 |
| — | Essai | Sous Linux ou WSL, avec un vrai tunnel (l'auteur, ou l'utilisateur) : agents, vue terminal, terminaux, perte et retour du tunnel | NS-7 | attend l'utilisateur |

- **Total** : 7 tickets, environ 11 h d'agent. Avec 6 agents en parallèle : environ 4 h de bout en bout (NS-1 → NS-2 → NS-3 en chaîne, NS-4 à NS-6 à côté).
- **Avant NS-1, si le choix 7 va au socket Unix** : un ticket de plus, le MCP et les hooks sur un socket Unix (environ 2 h 30), utile aussi au mode serveur.

## 8. Sources

- L'issue GitHub #41 (`gh issue view 41`), et la réponse déjà postée par le propriétaire du dépôt, qui annonce cette étude.
- Le code de `release/2.3` à ecfbd29e :
  - `src-tauri/src/plugins/harness.rs` : `check_launch`, `check_launch_with`, `Vars`, `check_env`, `check_file`, `next_launch`, `ProcessEngine::launch`, `start_program`, `start_again`, `described`, `locate`, `command_of` ;
  - `crates/escouade-plugins/src/procs.rs` : `forbidden_var`, `var_refused`, `locate`, `allowed` ;
  - `crates/escouade-plugin-cli/src/manifest.rs` : `interpreter`, `env_refused` ;
  - `crates/escouade-engines/src/program.rs` : `Program::launch` ;
  - `src-tauri/src/accounts.rs` : `program`, `for_project_of`, `confined`, `dedicated` ;
  - `src-tauri/src/bypass.rs` ; `src-tauri/src/locks.rs` et `escouade_policy::list` ; `src-tauri/src/launchenv.rs` ; `src-tauri/src/projectfile.rs` ; `src-tauri/src/mcp/mod.rs` ; `plugins/claude/src/terminal.rs`, `plugins/claude/src/client.rs` ; `src-tauri/src/bin/escouade-hook.rs` ; `plugins/isola/escouade-plugin.json` ;
  - les commits de SEC-30 (d6f74c75, ESC-443) et d'ESC-489 (636eec05, 80616a1f).
- La spec d'Escouade : *Comptes* (`harness/comptes`), *Le contrat des harness en process* (`plugins/harness-en-process`), *Réglages imposés* (`fenetre/reglages-imposes`), *Lancement de test* (`kanban/lancement-de-test`).
- `docs/PLUGINS.md` (§3, §6, §7), `docs/PLUGINS-DESIGN.md` (§1.11, §3.2 à §3.4, §4.10), `docs/STORE.md` (§3.2 à §3.4, §4), `docs/REMOTE.md` (le mode serveur, l'isolement).
- Les essais du §1.3, dans WSL (Ubuntu, noyau 5.15.167.4-microsoft-standard-WSL2), le 2026-11-17 : `unshare -Urn`, `unshare -Urnm` et `nsenter`, un serveur sur le `127.0.0.1` de l'hôte. Les scripts restent hors du dépôt.
