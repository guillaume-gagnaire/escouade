# Étude : le contrat des réglages, des secrets et des permissions d'un plugin (ESC-501)

Issues #38, #39, #40 et #42, d'un même auteur de plugin tiers : un plugin « VPN » en Rust/WASM (`run.wasm`) qui monte un tunnel OpenVPN par projet, sans droits d'administrateur, dans un espace réseau d'utilisateur, essayé avec la 2.2.x sous WSL. Leur texte est un rapport d'un tiers, lu comme une donnée.

- **Ce que c'est** : une étude. Aucun code du produit n'a changé ; la maquette est dans `design/plugins-contract-2-3/` (rien n'y est branché).
- **Écrite sur** `release/2.3` au 2026-11-17 (`ecfbd29e`). ESC-499 (les secrets sans trousseau, issue #37) est sur `main` (`f9498c6f`, la 2.2.3 en vue) et pas encore reversé dans `release/2.3` : quand cette étude cite son fichier, c'est le code de `main`.
- **Pas ici** : le préfixe de la commande de lancement d'un agent (#41, une étude à part avec sa revue de sécurité) ; les correctifs de #37 et #43 (ESC-499, ESC-500).
- **Pas de changement de la spec** : rien du comportement du produit ne change avec cette étude. Chaque ticket du découpage (§9) tiendra la spec et laissera sa décision.

## 0. En bref

- **#38** (secrets et réglages par projet) : la demande est juste. Recommandé : des **profils** déclarés par le plugin (un serveur, ses identifiants, sa configuration), que l'utilisateur crée autant qu'il veut et que **chaque projet choisit** ; plus une portée `"scope": "project"` pour les réglages simples. Un secret « par projet » nu (`secrets/get` avec `project`) est la direction A, plus petite, décrite et chiffrée, mais elle oblige à ressaisir les mêmes identifiants pour deux projets du même client.
- **#39** (une section du plugin dans les réglages d'un projet) : juste. Recommandé : une page **Plugins** dans Réglages › Projets › *projet*, avec les marques « Défini par .escouade.yml » et « Réglage local » qu'ont déjà les autres réglages d'un projet. `menus`, `statusBar` et `badges` ne valent rien aujourd'hui, et la fenêtre les **annonce** pourtant : à corriger tout de suite, à concevoir plus tard.
- **#40** (groupes, ordre, description, multi-ligne, fichier) : juste, sauf deux points. La description d'un réglage existe déjà (rendue, pas documentée), et l'ordre est déjà celui du manifeste. Recommandé : `contributes.groups` (qui range réglages et secrets ensemble, dans l'ordre), `description` sur les secrets, `"multiline": true`, et un bouton « Importer un fichier… » qui garde le **contenu**, jamais le chemin.
- **#42** (jokers typés, limite de programmes) : juste. Recommandé : des jokers d'un mot entier, `{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}`, à la grammaire fermée (§5.3) ; **pas** de chemin dans le projet. La limite se déclare (`limits.processes`, 16 au plus), se montre à l'installation et se justifie à la revue.
- **Un piège trouvé en route** : le trousseau de Windows refuse un secret de plus de **1 280 caractères** (2 560 octets en UTF-16, `keyring` 3.6.3, `src/windows.rs`, `CRED_MAX_CREDENTIAL_BLOB_SIZE`). Un fichier `.ovpn` avec ses certificats (5 Ko) n'y entre pas : le secret multi-ligne a besoin d'un découpage (§4.4).
- **Découpage** : 9 tickets, environ **23 h d'agent**, en trois vagues avec 6 agents en parallèle (§9). **11 choix** à trancher (§10).

## 1. Ce que fait le code aujourd'hui

Vérifié dans le code de `release/2.3` (`main` pour ESC-499), fichier et fonction.

### 1.1 Les secrets d'un plugin

| Où | Fonction | Ce qu'elle fait |
| --- | --- | --- |
| `crates/escouade-plugins/src/runtime.rs` | `Host::request`, branche `"secrets/get"` | Ne lit que `params.key` ; toute autre clé des paramètres est ignorée. Sans `key` : `-32602`. |
| même fichier | `Host::secret` | La clé doit être couverte par `permissions.secrets` (`matches` : égale, ou un préfixe suivi de `*`) ; sinon `-32001` (`notYours`) et une ligne au journal. Puis le secret prêté d'un compte (`Runtime::lent_secret`), sinon le coffre sous `secret_name(id, key)`, soit `plugin:<id>:<clé>`. La valeur est masquée dans le journal (`journal.mask`). |
| même fichier | `secret_name`, `matches` | `plugin:<id>:<clé>` ; un motif `préfixe*` couvre toute clé qui commence par ce préfixe. |
| `crates/escouade-plugins/src/lib.rs` | `secret_keys` | Les secrets que la page du plugin montre : les clés de `contributes.secrets`, puis celles de `permissions.secrets` qui ne finissent pas par `*`. |
| `src-tauri/src/commands.rs` | `plugin_secrets`, `plugin_set_secret` | La page du plugin lit si chaque secret est gardé, et garde ou oublie une clé ; **une clé hors de `secret_keys` est refusée**. |
| `store/server/utils/checks.ts` | `checkPermissions` (contrôle n° 9) | Une entrée de `permissions.secrets` qui n'est ni une clé de `contributes.secrets` ni `account:<genre>` / `account:*` refuse la version. |
| `crates/escouade-forge/src/integrations/secrets.rs` | `Keychain::set` | `keyring` 3.6 (`WinCredential`, `MacCredential`) ; sous Windows, 1 280 caractères au plus par secret (§4.4). |
| `main` : `crates/escouade-forge/src/integrations/kept.rs` | `Kept` | Sans trousseau (Linux, WSL), le fichier `plugin-secrets.json` choisi par l'utilisateur, lu avant le trousseau, sous les mêmes noms `plugin:<id>:<clé>`. |

### 1.2 Les réglages, et ce qu'un projet en dit

| Où | Fonction | Ce qu'elle fait |
| --- | --- | --- |
| `crates/escouade-plugins/src/runtime.rs` | `Host::request`, branche `"settings/get"` | Sans projet : les réglages de l'utilisateur. Avec un `project` (une chaîne) : `Runtime::settings_of`. |
| même fichier | `Runtime::settings_of` | Les choix de l'utilisateur (`choices.settings`, dans la liste des plugins installés), puis, si le plugin est actif dans ce projet (`active_in`, la règle des projets privés), ceux du `.escouade.yml` par-dessus. |
| `crates/escouade-plugins/src/lib.rs` | `project_settings` | Du fichier, ne garde que les clés de `contributes.settings`, jamais un secret, d'une valeur de leur type (`select` : une de ses options). |
| `src-tauri/src/core.rs` | `project_plugin` (le trait `Place` de `events.rs`) | Lit `plugins.<id>` du fichier du projet (`projectfile::plugin_settings`) et dit si le projet est privé. |
| `src-tauri/src/projectfile.rs` | `file_only`, `plugin_settings` | `plugins.*` est « au fichier seul » : jamais appliqué au projet, jamais marqué dans la fenêtre, jamais écrit par l'export. |

Il n'y a **aucune valeur d'un réglage de plugin propre à un projet sur la machine** : un projet n'a que ce que dit son fichier, que seul un éditeur de texte change.

### 1.3 L'écran

| Où | Ce qu'il montre |
| --- | --- |
| `src/components/plugins/PluginSettings.svelte` | Deux sections, « Réglages » puis « Secrets », chacune dans l'ordre du manifeste. Un réglage : `string` et `number` en `<input>` d'une ligne, `boolean` en interrupteur, `select` en liste ; **son `description` est déjà rendu** sous son titre, et `default`, `options` servent (type `ManifestSetting` de `src/lib/plugins.ts`). Un secret : son titre seul, un champ mot de passe d'une ligne, « Garder », « Oublier ». |
| `src/components/plugins/PluginProjects.svelte` | L'onglet « Projets » : un interrupteur par projet, rien d'autre. |
| `src/components/settings/ProjectPage.svelte` | Réglages › Projets › *projet* › Projet : `ProposedPlugins` (la bande des plugins proposés et absents), aucun réglage de plugin. |
| `schemas/plugin-1.json`, `website/content/docs/*/plugins/manifest.md` | `settings[]` : `key`, `type`, `title` ; ni `description`, ni `default`, ni `options` documentés. `secrets[]` : `key`, `title`. |

### 1.4 Les points réservés

| Où | Ce qui s'y passe |
| --- | --- |
| `crates/escouade-plugin-cli/src/manifest.rs`, `OPEN_POINTS` ; `schemas/plugin-manifest.mjs`, `OPEN_POINTS` | `menus`, `statusBar`, `badges`, `projectTemplates` : une liste d'objets, rien d'autre n'est vérifié. `escouadeYml` : un objet. |
| `src/`, `src-tauri/src/`, `crates/` | Rien ne les lit pour en faire quelque chose. |
| `src/lib/plugins.ts`, `contributionsOf` et `categoriesOf` ; `src/lib/plugins-settings.ts`, `SHOWN` | **La fenêtre les annonce** : « Un élément de la barre d'état » dans ce qu'apporte le plugin, et la catégorie « Outils ». Un auteur qui en déclare croit qu'ils marchent ; l'utilisateur aussi. |

### 1.5 `permissions.commands` et `proc/spawn`

| Où | Fonction | Ce qu'elle fait |
| --- | --- | --- |
| `crates/escouade-plugin-cli/src/manifest.rs` | `command_holds` (et `commandHolds` de `schemas/plugin-manifest.mjs`) | Le premier mot est un nom de programme (ni `/`, `\`, `:`, `*`, `~`, ni un `.` en tête) ; un `*` n'est permis qu'en dernier mot. Un mot `{int}` passe aujourd'hui : c'est un mot littéral. |
| `crates/escouade-plugins/src/procs.rs` | `Procs::new` | Découpe chaque entrée en mots (espaces). |
| même fichier | `allowed` | La commande et ses arguments égalent une entrée mot pour mot ; une entrée qui finit par `*` admet **n'importe quels mots** après sa tête, et jamais un `*` seul. |
| même fichier | `Procs::spawn` | `locate` (un programme du PATH, jamais un chemin, pas de `.cmd` ni `.bat`), `folder` (`dir` : `agent`, `project` ou son dossier temporaire, la clôture d'un projet privé), `var_refused` (`permissions.env`), puis la limite, puis le Job object. |
| même fichier | `AT_ONCE = 4`, `Procs::live` | **4 programmes à la fois** par instance, et une instance par plugin : 4 par plugin, tous projets confondus. Au-delà : `-32003` « 4 programs at once at most ». |
| `crates/escouade-plugins/src/wasm.rs` | `Limits::asked`, `MEMORY_MAX`, `TURN_MAX` | `limits` du manifeste ne connaît que `fetchSeconds`, `fetchBytes`, `fileBytes`, `memoryBytes`, `turnSeconds`. |
| `src/lib/plugins.ts` | `askOf` (`'commands'`) | La fenêtre d'installation : « Lance cette commande sur ton ordinateur », chaque entrée en texte. |
| `docs/PLUGINS.md` §3.3 | — | « Le store refuse `*` sur un programme généraliste pour un tiers, sauf justification relue » : c'est la **revue**, pas un des onze contrôles (`checkPermissions` ne lit pas `commands`, sauf pour un plugin sans programme). |

### 1.6 Ce que les issues disent de juste ou de faux

- **#38** — juste : `secrets/get` ne prend que `key` ; on ne peut garder « les identifiants du projet X ». Juste aussi : `settings/get` a un `project`. À préciser : un motif `login_*` serait accepté par l'hôte (`matches`), mais la page ne laisse saisir que les clés déclarées une à une (`plugin_set_secret`) et le store refuse le motif (contrôle n° 9) ; les « emplacements numérotés » sont donc bien le seul contournement.
- **#39** — juste : aucun écran, `plugins.<id>` s'édite à la main ; `menus`, `statusBar`, `badges` réservés. À ajouter : la fenêtre les annonce quand même (§1.4).
- **#40** — juste : liste plate, pas de multi-ligne, pas de titre qui reprenne un nom saisi. **À moitié faux** : un réglage a déjà sa `description` (rendue, pas documentée) ; un secret non. L'ordre est déjà celui du manifeste ; ce qui manque, c'est de **mêler** un réglage et un secret dans un même groupe (deux sections séparées).
- **#42** — juste : `*` en dernier mot seulement, 4 programmes par plugin. Un point que l'issue ne dit pas : le contournement `unshare -Urn openvpn *` est **plus large qu'il n'en a l'air**. Après `openvpn`, `*` admet `--up <commande> --script-security 2`, c'est-à-dire n'importe quel programme lancé par OpenVPN : l'entrée vaut presque « tout ». C'est un argument de plus pour les jokers typés.

## 2. #38 : des secrets et des réglages par projet

### 2.1 Direction A : une portée « projet »

Ce que l'issue demande, tel quel.

```json
"permissions": { "secrets": ["password", "ovpn"] },
"contributes": {
  "settings": [
    { "key": "server", "type": "string", "title": "Serveur", "scope": "project" },
    { "key": "autoStart", "type": "boolean", "title": "Tunnel au lancement d'un agent", "default": true }
  ],
  "secrets": [
    { "key": "password", "title": "Mot de passe", "scope": "project" },
    { "key": "ovpn", "title": "Configuration OpenVPN", "scope": "project", "multiline": true, "import": [".ovpn", ".conf"] }
  ]
}
```

- `scope` : `app` (d'office, ce qui existe) ou `project`. Un champ `project` n'est pas sur la page du plugin : il est dans les réglages de chaque projet (§3).
- `secrets/get { key, project }` : obligatoire pour une clé `project`, refusé (`-32602`) pour une clé `app`, pour qu'une confusion ne rende pas en silence le secret d'un autre périmètre.
- `settings/get { project }` : comme aujourd'hui, plus la valeur locale du projet.
- **Ce que voit l'utilisateur** : `captures/a-plugin.png` (la page du plugin : ce qui vaut pour tous les projets), `captures/a-projet.png` (Réglages › Projets › atlas-web › Plugins : le serveur du fichier, le port réglé sur la machine, le mot de passe gardé pour ce projet, la configuration collée ou importée). En clair : `-clair.png`.
- **Ce que voit l'auteur** : un champ de plus par déclaration, un paramètre de plus à `secrets/get`. Le SDK : `escouade.secret('password', { project })` ; le kit : `ctx.secret_of(project, "password")`.
- **Sa limite** : deux projets du même client, ce sont deux saisies du même mot de passe et de la même configuration, et un changement de mot de passe à refaire partout.

### 2.2 Direction B : des profils, choisis par projet (recommandée)

Le contournement de l'auteur, des emplacements `login_1`…`login_6` que chaque projet désigne, est exactement un **profil** : un ensemble de champs que l'utilisateur crée autant de fois qu'il veut, et qu'un projet choisit. Escouade a déjà le geste pour les comptes (plusieurs comptes par genre, ESC-397) ; un profil est un compte sans hôte ni connexion.

```json
"permissions": { "secrets": ["profile:vpn"] },
"contributes": {
  "profiles": [{
    "id": "vpn",
    "title": { "fr": "Profil VPN", "en": "VPN profile" },
    "nameField": "name",
    "fields": [
      { "key": "name", "type": "string", "title": "Nom du profil" },
      { "key": "server", "type": "string", "title": "Serveur" },
      { "key": "proto", "type": "select", "title": "Protocole", "options": ["udp", "tcp"], "default": "udp" },
      { "key": "login", "type": "string", "title": "Identifiant" },
      { "key": "password", "secret": true, "title": "Mot de passe" },
      { "key": "ovpn", "secret": true, "multiline": true, "import": [".ovpn", ".conf"], "title": "Configuration OpenVPN" }
    ]
  }],
  "settings": [
    { "key": "profile", "type": "profile", "of": "vpn", "scope": "project", "title": "Profil VPN" },
    { "key": "routes", "type": "string", "scope": "project", "title": "Routes par le tunnel" }
  ]
}
```

- **L'utilisateur** crée ses profils sur la page du plugin (« Ajouter un profil… »), les modifie dans une fenêtre aux champs groupés, et chaque projet choisit le sien dans ses réglages. Captures : `b-plugin.png` (la liste, avec les projets qui s'en servent), `b-profil.png` (un profil ouvert), `b-projet.png` (le choix du projet, et le fichier qui en propose un).
- **L'auteur** : `settings/get { project }` rend `profile: "<id du profil>"` ; `profiles/get { kind: "vpn", profile }` rend ses champs simples ; `secrets/get { key: "password", profile }` rend un secret du profil. `profiles/list { kind }` rend les profils (ids et noms) pour un plugin qui veut les montrer dans son journal.
- Le titre « dynamique » de #40 vient de lui-même : un profil s'appelle par son `nameField`.
- **Un projet sans profil** : le réglage vaut `null` ; le plugin dit ce qui manque, la page du projet aussi (« Aucun profil : pas de tunnel »).
- **Un profil supprimé** : ses secrets sont effacés, les projets qui le prenaient retombent à `null`, la fenêtre de suppression les nomme (« Utilisé par atlas-web »).
- Pourquoi la recommander : elle couvre la demande (un projet, ses identifiants), le cas que A rate (deux projets d'un même client), et la moitié de #40 (des groupes de champs qui se répètent, un titre qui reprend le nom saisi), sans un champ par projet à ressaisir.

### 2.3 La sécurité

- **Où va un secret** : au trousseau, sous un nom qui dit son périmètre et que le plugin ne choisit pas en entier : `plugin:<id>:project:<projet>:<clé>` (A), `plugin:<id>:profile:<genre>:<profil>:<clé>` (B). `<projet>` est l'identifiant interne d'Escouade (aléatoire, à la machine), `<profil>` un identifiant tiré par Escouade à la création ; ni l'un ni l'autre ne vient du dépôt. Sans trousseau, le fichier choisi d'ESC-499 sous les mêmes noms.
- **Qui le lit** : le plugin seul (le préfixe `plugin:<id>:`, `-32001` sinon) ; en A, pour un projet où il est **actif** (`active_in` : un projet privé seulement s'il y a été allumé), le même contrôle que `Runtime::settings_of` et `Runtime::project_folder`. En B, un profil se lit tant que le plugin est installé et activé ; le choix du profil d'un projet ne passe que pour un projet où il est actif.
- **Ce que l'hôte ne peut pas promettre** : une instance sert tous les projets du plugin. L'hôte sait qu'un projet est actif ; il ne sait pas si la lecture sert « un agent de ce projet ». Que le plugin n'utilise le secret de atlas-web que pour atlas-web, c'est son code, que la revue lit. Une instance par projet l'imposerait, au prix de N instances et d'un état coupé en morceaux (§8).
- **Le journal** : « was given its secret "password" of project atlas-web », jamais la valeur (masquée comme aujourd'hui).
- **Un `.escouade.yml` hostile** (un dépôt cloné de n'importe qui) :
  - ne porte **jamais** un secret (comme aujourd'hui : `project_settings` saute `secret_keys`) ;
  - ne **choisit pas** un profil : un dépôt qui choisirait « le VPN de l'entreprise » ferait passer ses agents par le réseau de l'entreprise. Il peut en **proposer** un par son nom ; la page du projet le dit dans une bande et l'utilisateur choisit (capture `b-projet.png`) ;
  - ne nomme ni un projet ni un identifiant de profil, ne rallume pas un plugin dans un projet privé ;
  - peut donner un réglage simple de portée `project` que l'auteur laisse au fichier (`"repo": true`, choix n° 2) : une route, un port. Une valeur locale de la machine passe devant (rang 4, comme les autres réglages d'un projet).
- **À la fermeture et à la désinstallation** : un projet dont l'historique est effacé efface ses secrets (A) ; désinstaller le plugin efface `plugin:<id>:*`. Le trousseau ne sait pas lister par préfixe (`keyring` n'énumère pas) : Escouade tient la **liste des noms** gardés par plugin dans la liste des plugins installés (les noms, jamais les valeurs).

### 2.4 La compatibilité

- **Un plugin d'aujourd'hui** ne change rien et marche pareil : `scope` absent vaut `app`, `secrets/get { key }` répond comme avant.
- **Un plugin nouveau sur une app ancienne** : `secrets/get { key, project }` y rendrait **en silence** le secret de l'app (le `project` est ignoré, §1.1). Il doit donc dire `escouade.min` à la version qui le sert (hors de sa plage, le plugin est chargé coupé, la raison dite) ; `initialize` annonce `capabilities.projectSecrets` (A) ou `capabilities.profiles` (B), que le SDK et le kit vérifient avant l'appel.
- **Le protocole** reste en version 1 (`x-protocolVersion`), par ajouts : `SecretsGetParams.project`, `SecretsGetParams.profile`, `profiles/get`, `profiles/list`. `escouade-plugin-protocol` et `escouade-plugin-kit` 0.1.3 → 0.1.4, le SDK `@escouade-dev/plugin` 0.3.0 → 0.4.0.
- **Le schéma** reste `plugin-1.json`, par ajouts. Un `type: "profile"` serait refusé par le lecteur d'une app ancienne (`SETTING_TYPES`) : sans conséquence, le plugin étant hors de sa plage.
- **Le store** : contrôle n° 1 (`validate` : `scope`, `profiles`, `type: "profile"` qui nomme un genre déclaré) et n° 9 (`profile:<genre>` cohérent avec un genre déclaré qui a des champs secrets, comme `account:<genre>` aujourd'hui).

## 3. #39 : une section du plugin dans les réglages d'un projet

### 3.1 Direction 1 : une page « Plugins » du projet (recommandée)

- Réglages › Projets › *projet* gagne une page **Plugins**, entre « Kanban » et « Équipe », qui n'apparaît que si un plugin **installé et actif** dans ce projet déclare un champ de portée `project` (la règle du 2026-11-11 : les réglages ne listent que ce qu'apportent les plugins installés ; la bande « Ce projet propose N plugins » reste la seule offre, sur la page Projet).
- Un groupe par plugin : sa tuile, son nom, son identifiant, « Page du plugin ». Dedans, ses champs `project`, avec les marques des autres réglages d'un projet (`projets/escouade-yml`, « Dans la fenêtre ») : « Défini par .escouade.yml », « Réglage local de cette machine : il passe devant .escouade.yml », « Écrire dans .escouade.yml », « Reprendre la valeur du fichier ».
- Un secret ou un profil dit « Gardé sur cette machine, jamais dans .escouade.yml » ; il n'a pas « Écrire dans .escouade.yml ».
- Un plugin coupé pour ce projet n'y est pas ; un projet privé où il n'est pas allumé non plus (l'onglet « Projets » du plugin l'allume).
- Captures : `a-projet.png` (direction A), `b-projet.png` (direction B), clair et sombre.
- **Écrire dans le fichier** : aujourd'hui `plugins.*` n'est jamais écrit (`projectfile`, « The plugins' keys stay the file's »). « Écrire dans .escouade.yml » écrirait `plugins.<id>.<clé>` **clé par clé**, en gardant les commentaires (le même écrivain que les autres clés), jamais un secret ni un profil ; l'export de tout le projet continue de ne pas les écrire (choix n° 4).

### 3.2 Direction 2 : dans l'onglet « Projets » de la page du plugin

- Chaque ligne de projet se déplie et montre les champs du plugin pour ce projet (capture `onglet.png`).
- Plus simple à construire (un seul composant, le plugin connu), mais l'utilisateur qui règle **un projet** doit ouvrir chaque plugin, et les réglages d'un projet ne sont plus tous dans Réglages › Projets.
- Recommandé : la direction 1, et dans l'onglet « Projets » un lien « Réglages du projet » par ligne qui mène à la page Plugins du projet.

### 3.3 `menus`, `statusBar`, `badges` : ce qu'ils valent

- **Rien aujourd'hui** : lus comme des listes d'objets, jamais servis (§1.4).
- **La fenêtre les annonce** : `contributionsOf` les compte dans « ce qu'il apporte » et `categoriesOf` range le plugin dans « Outils ». À corriger tout de suite (T7) : ne plus les montrer, et `escouade-plugin validate` les dit « réservés, sans effet dans cette version » (un avertissement, pas un refus, pour ne pas casser un manifeste publié).
- **`escouadeYml`** (« ses clés de .escouade.yml ») devient inutile avec `scope: "project"` : les champs de portée projet **sont** ses clés. À garder lu et ignoré, et à retirer de la documentation.
- **Les concevoir** n'est pas demandé ici. `statusBar` serait le plus utile au plugin VPN (« Tunnel : atlas-web connecté ») ; il mérite sa propre étude (un texte et une icône déclarés, un état poussé par le plugin, son rythme, sa place à côté des quotas), pas un ajout en passant (choix n° 9).

### 3.4 La sécurité et la compatibilité

- Rien de neuf pour un dépôt : il ne gagne que ce que le fichier pouvait déjà (les clés déclarées, de leur type), moins ce que l'auteur lui retire (`"repo": false`) ; un réglage local passe devant.
- Un plugin d'aujourd'hui : rien ne change. Ses réglages `app` restent sur sa page ; son `plugins.<id>` dans un fichier marche comme avant.
- Le store : contrôle n° 1 seulement (`scope`, `repo`).

## 4. #40 : grouper, ordonner, décrire, saisir plusieurs lignes, importer un fichier

### 4.1 La conception

```json
"contributes": {
  "groups": [
    { "id": "connection", "title": { "fr": "Connexion", "en": "Connection" },
      "description": { "fr": "Pour tous les projets, sauf ceux qui donnent les leurs." },
      "fields": ["server", "proto", "autoStart"] },
    { "id": "credentials", "title": { "fr": "Identifiants", "en": "Credentials" },
      "fields": ["login", "password", "ovpn"] },
    { "id": "advanced", "title": { "fr": "Avancé", "en": "Advanced" }, "folded": true,
      "fields": ["mtu", "verbosity", "keepalive"] }
  ],
  "settings": [
    { "key": "server", "type": "string", "title": "Serveur", "description": "Le nom ou l'adresse du serveur OpenVPN." },
    { "key": "notes", "type": "string", "multiline": true, "lines": 6, "title": "Notes" }
  ],
  "secrets": [
    { "key": "ovpn", "title": "Configuration OpenVPN", "multiline": true, "import": [".ovpn", ".conf"],
      "description": "Le fichier .ovpn de ton fournisseur, certificats compris." }
  ]
}
```

- **`groups`** : un titre, une description, `folded` (replié d'office, comme « Avancé »), et `fields`, les clés **réglages et secrets mêlés**, dans l'ordre où les montrer. L'ordre des groupes est celui de la liste. Un champ hors de tout groupe vient après, dans l'ordre du manifeste, sous « Réglages » puis « Secrets » comme aujourd'hui. Une clé dans deux groupes, ou qui n'existe pas : le manifeste est refusé à ce champ.
- **Pas de `order`** numérique : la liste `fields` dit l'ordre, sans trous ni ex æquo à arbitrer (§8).
- **`description`** sur un secret (nouveau) et sur un réglage (existant, enfin documenté avec `default` et `options`).
- **`multiline: true`** sur un réglage `string` ou un secret : une zone de texte en police à chasse fixe, numérotée, `lines` lignes visibles (3 à 20, 6 d'office). 64 Kio au plus.
- **Un secret multi-ligne ne se relit pas** : gardé, il dit « Gardé · 84 lignes · 5,1 Ko » et propose « Remplacer… » et « Oublier » (captures `a-plugin.png`, `b-profil.png`). Pendant la saisie, le texte se voit (on colle une configuration, on veut voir qu'elle est entière).
- **`import`** : un bouton « Importer un fichier… » (la boîte de dialogue du système, filtrée sur ces extensions) lit le fichier **une fois**, en texte UTF-8, 64 Kio au plus, et remplit le champ. Le chemin n'est jamais gardé, le plugin ne gagne aucun droit sur les fichiers.
- **Le titre dynamique** vient des profils (§2.2, `nameField`). Sans profils, pas de gabarit de titre (§8).

### 4.2 Une autre direction pour le fichier : un chemin et un droit de lecture

- `type: "file"` garderait un chemin, et l'hôte ouvrirait au plugin la lecture de ce seul fichier (une racine `chosen` de `fs/*`), suivie quand il change.
- Contre : un droit de plus à montrer et à relire, un chemin hors de toute racine connue (un `~/.ssh/config` choisi par mégarde), le fichier qui change sous le plugin sans que l'utilisateur le voie, un chemin qui n'existe pas sur l'autre poste de l'équipe. Pour un fichier de configuration qu'on importe une fois par profil, l'import suffit. Non recommandée (choix n° 6).

### 4.3 Le rendu

- Captures : `a-plugin.png` (groupes Connexion, Identifiants, Avancé replié ; la configuration gardée), `a-projet.png` (une configuration en cours de saisie), `b-profil.png` (les groupes d'un profil). Chacune en clair (`-clair.png`).
- Les règles de `DESIGN.md` : groupes sans boîte, un titre et des rangées séparées par un filet ; champs en puits ; « Avancé » replié comme ailleurs dans les réglages.

### 4.4 La sécurité : la taille d'un secret

- **Windows** : `keyring` 3.6.3 refuse un mot de passe dont l'UTF-16 dépasse 2 560 octets (`CRED_MAX_CREDENTIAL_BLOB_SIZE`), soit **1 280 caractères**. Une configuration OpenVPN avec ses certificats en ligne fait 4 à 8 Ko. **macOS** : pas de limite utile. **Sans trousseau** : le fichier d'ESC-499, sans limite.
- Recommandé (choix n° 5) : un secret long est découpé en morceaux de 1 200 caractères sous `…<clé>#1`, `#2`…, et l'entrée `<clé>` garde l'en-tête (`v1`, le nombre de morceaux, l'empreinte SHA-256 du tout). La lecture recolle et vérifie l'empreinte ; un morceau manquant rend une erreur `-32002` (trousseau), jamais un secret tronqué. « Oublier » efface tous les morceaux. 64 Kio au plus.
- L'autre voie, un fichier chiffré dont la clé seule est au trousseau, ajoute un format de fichier, sa sauvegarde et sa rotation : plus lourde pour le même résultat.
- Le journal masque aujourd'hui une valeur entière ; proposé : pour un secret multi-ligne, il masque aussi chaque ligne de plus de 8 caractères (un plugin qui écrit une ligne de la configuration dans son journal ne la montre pas).

### 4.5 La compatibilité

- `multiline` est un **drapeau** sur `string`, pas un type nouveau : une app ancienne lit le manifeste (le champ inconnu est ignoré) et montre un champ d'une ligne. Un type `text` ferait refuser le manifeste par `SETTING_TYPES` d'une app ancienne.
- `groups` inconnu d'une app ancienne : ignoré, la liste reste plate. `description` d'un secret : ignorée. `import` : ignoré.
- Le protocole ne change pas pour #40. Le store : contrôle n° 1 seulement.

## 5. #42 : des jokers typés, et la limite de programmes

### 5.1 Ce que l'auteur veut écrire

```json
"permissions": {
  "commands": [
    "unshare -Urn openvpn --config {path:temp} --script-security 1",
    "slirp4netns --configure --mtu=65520 {pid} tap0",
    "nsenter --preserve-credentials -U -n -t {pid} ip addr"
  ]
},
"limits": { "processes": 8 }
```

- La configuration est un secret de l'utilisateur, que le plugin écrit dans **son** dossier temporaire (`fs/write` sur la racine `temp`) avant de lancer OpenVPN sur ce fichier.
- `--script-security 1` après `--config` : OpenVPN prend la dernière valeur d'une option donnée deux fois, ce qui interdit les scripts que la configuration demanderait (`up`, `down`). **À vérifier par l'auteur** sur sa version d'OpenVPN ; la revue le lit.
- `{pid}` : le pid d'un programme que **ce plugin** a lancé et qui tourne encore (le `unshare` du tunnel), jamais celui d'un autre process de l'utilisateur.

### 5.2 Ce que voit l'utilisateur

- La fenêtre d'installation (capture `install.png`, clair et sombre) : chaque joker en pastille de mots, « un fichier de son dossier temporaire », « un programme qu'il a lancé », « un nombre », « un port », « un nom » ; puis la phrase « Chaque pastille est un seul mot, que l'hôte vérifie : jamais une option, jamais un chemin hors de ce qu'elle dit. »
- « Jusqu'à 8 programmes à la fois », avec « 4 sans le demander » et la raison que l'auteur donne (`limits.reason`).
- Le journal dit chaque lancement avec les mots réels (`proc/spawn: ran slirp4netns --configure --mtu=65520 41823 tap0 in temp`).

### 5.3 La grammaire exacte

Une entrée de `permissions.commands` :

- `entrée = mot (" " mot)*` ; les mots se séparent par des espaces, comme aujourd'hui.
- Le **premier mot** est un nom de programme, comme aujourd'hui (`command_holds`) ; **jamais un joker**.
- Un **mot littéral** ne contient ni `{` ni `}` ; il se compare octet pour octet.
- Un **joker** est un mot **entier**, l'un de :

| Joker | Admet | Refuse |
| --- | --- | --- |
| `{int}` | `^(0\|[1-9][0-9]{0,9})$`, au plus 4 294 967 295 | un signe (`-1`, `+1`), un zéro en tête, l'hexadécimal, un espace, tout le reste |
| `{port}` | un `{int}` de 1 à 65 535 | 0, au-delà |
| `{pid}` | un `{int}` qui est le pid d'un programme vivant lancé par `proc/spawn` de **cette** instance (`proc/spawn` rend désormais `{ proc, pid }`) | tout autre pid, un programme fini |
| `{name}` | `^[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}$`, ni `.` ni `..`, ni un nom réservé de Windows (`CON`, `NUL`, `COM1`…) | un `-` en tête (une option), `/`, `\`, `:`, `=`, `@`, `%`, `$`, `` ` ``, `"`, `'`, `^`, `&`, `\|`, `<`, `>`, `*`, `?`, `~`, un espace, un caractère de contrôle |
| `{path:temp}` | un chemin **relatif** (des composants `[A-Za-z0-9_.-]+`, ni `.` ni `..`, pas de `-` en tête) sous le dossier temporaire du plugin | un chemin absolu, `~`, une lettre de lecteur, `\\?\`, un chemin UNC, `..` |
| `{path:plugin}` | de même, sous le dossier du plugin (ses fichiers, en lecture) | de même |

- **Un chemin est remplacé par l'hôte** : il joint le chemin relatif à sa racine, le résout (liens et jonctions suivis, `fs/canon`), vérifie qu'il reste sous la racine et que le fichier existe, puis passe **le chemin absolu résolu** au programme. Le programme lit donc exactement ce qui a été vérifié, et un chemin absolu ne commence jamais par `-`.
- `*` reste ce qu'il est : seul, en **dernier** mot, jamais en premier. Une entrée peut mêler jokers et `*` final (`nsenter -t {pid} *` reste possible, et la revue le juge comme un `*` aujourd'hui).
- **Un joker ne correspond qu'à un mot** : l'entrée a une longueur fixe (sauf `*` final).
- **`--`** peut être un mot littéral d'une entrée ; un joker ne correspond jamais à `--` ni à un mot qui commence par `-`.
- **Aucune substitution** : il n'y a pas de shell (`locate` refuse `.cmd` et `.bat`, le programme est lancé directement), donc `$VAR`, `$(…)`, `` `…` ``, `%VAR%`, `~` et les motifs ne sont jamais développés ; `{name}` en refuse quand même les caractères, pour qu'un programme qui relit ses arguments (le CRT de Windows, un outil qui fait son propre développement) ne puisse pas couper un mot en deux.
- **Refusé au manifeste** (code `command`, par `command_holds` et `commandHolds`, donc par `escouade-plugin validate` et le contrôle n° 1 du store) : un joker inconnu (`{float}`), un joker dans un mot (`--pid={int}`), un joker en premier mot, `{path:project}`, `{path:agent}`, une accolade dans un mot littéral.

### 5.4 Pourquoi c'est plus étroit que `*`

- `*` admet **n'importe combien de mots, de n'importe quelle forme**, options comprises : après `openvpn`, `--up /bin/sh --script-security 2` ; après `nsenter … -t`, un pid puis n'importe quel programme.
- Un joker admet **un seul mot, d'une forme fermée**, qui ne peut être ni une option (jamais de `-` en tête) ni un chemin hors de sa racine ; l'entrée garde sa longueur. Une entrée à jokers se lit en entier à l'installation et à la revue : ce qui varie est dit.

### 5.5 Pourquoi pas de chemin dans le projet

- `{path:project}` serait « un fichier que **le dépôt** choisit » : un dépôt cloné de n'importe qui y met une configuration OpenVPN avec `up` et `script-security 2`, un `.gitconfig`, un `ssh_config` ; un programme qui exécute ce que dit sa configuration devient une exécution de code venue du dépôt. La racine `temp` est au plugin seul ; `plugin` est en lecture et revu.
- Un besoin réel d'un fichier du projet passera par le plugin qui le **lit** (`fs/read`, une racine qu'il déclare), le juge, puis l'écrit dans son dossier temporaire.

### 5.6 La limite de programmes

- **Ce qu'elle protège** : le module est isolé, **le programme qu'il lance ne l'est pas** ; il a les droits de l'utilisateur. Borner leur nombre borne ce qu'un plugin qui boucle ou qui est malveillant peut tirer de la machine (processeur, mémoire, descripteurs, fenêtres de console) et ce que tient l'hôte pour chacun : deux tuyaux, trois tâches, un crédit de 256 Kio non consommés et jusqu'à 1 Mio en attente d'entrée (`procs.rs` : `CREDIT`, `WAITING_INPUT`), soit à peu près 1,3 Mio de l'app par programme au pire, plus le programme lui-même.
- **Direction 1 (recommandée)** : `limits.processes`, de 1 à **16** (le plafond de l'hôte, comme `MEMORY_MAX`), et `limits.reason`, une phrase. Au-delà de 4, la fenêtre d'installation le dit et une mise à jour qui l'augmente redemande l'accord ; la revue juge la raison. Coût : 16 programmes, au pire une vingtaine de Mio de l'app et ce que les programmes prennent, rien d'autre.
- **Direction 2 : compter par projet**. `proc/spawn` ne dit son projet que s'il tourne dans `dir: "project"` ou `"agent"` ; le tunnel tourne dans le dossier temporaire, sans projet : il faudrait un `project` obligatoire, et la limite grandirait avec le nombre de projets (10 projets, 40 programmes) sans que personne l'ait accordé. Non recommandée.
- **La compatibilité** : `limits.processes` est ignoré par une app ancienne (elle garde 4) ; un `{pid}` d'une app ancienne est un mot littéral qui ne correspond à rien : **l'entrée est refusée, jamais élargie**. Le plugin dit `escouade.min`. `proc/spawn` qui rend `pid` en plus est un ajout.
- **Le store** : contrôle n° 1 (la grammaire, le plafond), rien d'autre d'automatique ; la revue lit chaque entrée et la raison de la limite.

## 6. La compatibilité d'ensemble

- **Un plugin d'aujourd'hui marche sans changement** : chaque ajout est facultatif, ses valeurs absentes valent le comportement d'aujourd'hui (`scope: app`, pas de groupe, une ligne, 4 programmes, pas de joker).
- **Le protocole** : version 1, par ajouts (`secrets/get` : `project`, `profile` ; `profiles/get`, `profiles/list` ; `proc/spawn` rend `pid` ; `initialize` : `capabilities.projectSecrets`, `profiles`, `typedCommands`). Crates `escouade-plugin-protocol` et `escouade-plugin-kit` 0.1.4, SDK 0.4.0.
- **Le schéma** : `plugin-1.json`, par ajouts ; le lecteur Rust, celui du store (`plugin-manifest.mjs`) et leur corpus partagé (`tests/fixtures/plugins/manifests.json`) changent ensemble.
- **Les plugins de la première main** : aucun n'a de `contributes.settings`, et `isola` (le seul qui a `permissions.commands` : `isola ls --json`, `isola down`, `isola destroy`) n'a ni accolade ni joker. Aucun n'est touché.
- **Les onze contrôles du store** :

| Contrôle | Change ? | Ce qui change |
| --- | --- | --- |
| 1. Manifeste et `validate` | **oui** | `scope`, `repo`, `groups`, `description` d'un secret, `multiline`, `lines`, `import`, `profiles`, `type: "profile"`, `limits.processes` et `reason`, la grammaire des jokers, l'avertissement des points réservés |
| 2. Version et étiquette | non | |
| 3. Identifiant | non | |
| 4. Dépôt | non | |
| 5. Licence | non | |
| 6. README | non | |
| 7. Captures | non | |
| 8. Archive | non | |
| 9. Permissions | **oui** | `profile:<genre>` cohérent avec un genre de `contributes.profiles` qui a des champs secrets ; un secret de portée `project` compte comme déclaré |
| 10. Aucun binaire | non | |
| 11. Compilation | non | |

- **La revue** (`/review-plugin`) : une ligne de plus à lire pour les jokers (ce qui varie, et pourquoi), la limite relevée et sa raison, et l'usage des secrets d'un projet (le plugin ne s'en sert que pour ce projet).

## 7. Le lien avec ce qui existe ou arrive

- **Le `.escouade.yml` d'équipe (TS-13)** : aujourd'hui, un réglage de plugin **ne peut pas** être recommandé ni imposé. `plugins.<id>.<clé>` n'est ni une stratégie ni une clé de `projectfile::KEYS` : `CoreKeys::sort` (`src-tauri/src/team_settings.rs`) ne lui donne pas de sorte, la clé est gardée, dite, et ne s'applique pas. Proposé (choix n° 10) : une clé `plugins.<id>.<clé>` d'un réglage **simple** (`string`, `number`, `boolean`, `select`, `multiline` compris) a la sorte **valeur** ; `defaults` la recommande (rang 2), `enforced` l'impose sur les projets de l'équipe (rang 5, « Imposé par l'équipe Acme », grisé dans la page Plugins du projet). Jamais un secret, un profil ou une valeur importée ; jugée contre le manifeste du plugin installé (une clé qu'il ne déclare pas, ou d'un mauvais type, est dite et ignorée).
- **Les réglages ne listent que ce qu'apportent les plugins installés** (2026-11-11) : la page Plugins d'un projet ne montre que des plugins installés et actifs dans ce projet, et n'existe que s'il y en a un ; aucune offre « Non installé ». La bande des plugins proposés reste sur la page Projet.
- **Le repli des secrets sans trousseau (ESC-499)** : les nouveaux noms (`…:project:…`, `…:profile:…`, les morceaux `#n`) passent par le même `Kept`, lu avant le trousseau ; « Déplacer vers le trousseau » les déplace comme les autres. Le découpage (§4.4) ne sert qu'au trousseau de Windows : le fichier garde un secret d'un seul tenant. **ESC-499 doit être reversé dans `release/2.3` avant** le ticket des secrets (T4).
- **Les comptes partagés (ESC-397)** : les profils reprennent leur geste (plusieurs par genre, une fenêtre pour en ajouter un) sans leurs hôtes ni leur connexion ; un compte reste ce qui sert une source ou un hébergeur.
- **#41** (le préfixe de lancement d'un agent) : faire passer les agents d'un projet par le tunnel est le vrai but du plugin VPN ; c'est #41, pas ici. Les jokers et les profils de cette étude ne donnent aucun droit sur le lancement d'un agent.

## 8. Ce qu'on recommande de ne pas faire

- **Un `order` numérique** : la liste `fields` d'un groupe dit l'ordre ; des numéros font des trous, des ex æquo, et un ordre qui dépend d'un tri.
- **Un gabarit de titre** (`"title": "Profil {name}"`) hors des profils : un mini-langage de plus pour un cas que les profils couvrent.
- **Un type `file` qui garde un chemin** et ouvre sa lecture au plugin (§4.2).
- **Un type `text`** pour le multi-ligne : il casserait le manifeste sur une app ancienne (§4.5).
- **Des jokers sur un chemin du projet ou de l'agent**, et **un joker dans un mot** (`--pid={int}`) : la première est une exécution de code venue du dépôt (§5.5) ; la seconde rouvre l'analyse d'un mot que la grammaire évite. On l'ajoutera quand un plugin en aura besoin, avec sa propre grammaire.
- **Un `*` au milieu** d'une entrée : c'est le contraire de ce que demande l'issue.
- **Compter la limite par projet** (§5.6).
- **Une instance par projet** pour isoler les secrets d'un projet : N instances, un état coupé, et les plugins de harness qui tiennent leurs agents dans une seule.
- **Laisser `.escouade.yml` choisir un profil**, porter un secret, une valeur importée ou un chemin.
- **Relire un secret** dans la fenêtre : un secret se remplace, il ne se montre pas.
- **Concevoir `menus`, `statusBar`, `badges` dans ce lot** parce qu'ils sont réservés : seul `statusBar` a un besoin, et il mérite sa propre étude.
- **Garder `escouadeYml`** comme point documenté : la portée `project` le remplace.

## 9. Le découpage en tickets

Au rythme de Claude Code, avec 6 agents en parallèle ; heures d'agent, chaque ticket avec ses tests (Rust, Vitest, un e2e quand il touche la fenêtre), sa page de spec et sa décision. Dans l'ordre :

| # | Ticket | Ce qu'il livre | Dépend de | Heures d'agent |
| --- | --- | --- | --- | --- |
| T7 | Les points réservés ne s'annoncent plus | `contributionsOf`, `categoriesOf`, `SHOWN` sans `menus`, `statusBar`, `badges`, `projectTemplates` ; l'avertissement de `validate` ; `description`, `default` et `options` d'un réglage documentés (schéma, site fr et en) | — | 0 h 45 |
| T1 | Le contrat, lu et refusé | Le lecteur Rust, celui du store et leur corpus : `scope`, `repo`, `groups`, `description` d'un secret, `multiline`, `lines`, `import`, `profiles`, `type: "profile"`, `limits.processes` et `reason`, la grammaire des jokers ; `plugin-1.json`, `plugin-manifest.d.ts` | — | 2 h 30 |
| T2 | #40 dans la fenêtre | `PluginSettings.svelte` : groupes, repli, description des secrets, zone multi-ligne, « Importer un fichier… », « Gardé · N lignes » ; e2e et captures | T1 | 3 h |
| T3 | Le découpage des longs secrets | `Keychain` : morceaux de 1 200 caractères, en-tête et empreinte, effacement entier ; `Kept` inchangé ; essai réel sous Windows | ESC-499 reversé | 1 h 30 |
| T5 | Les jokers typés | `procs::allowed` sur la grammaire, `{pid}` (et `pid` rendu par `proc/spawn`), `{path:*}` résolu et remplacé, `askOf` et la fenêtre d'installation, le journal ; un corpus de cas hostiles (options, `..`, liens, pids d'autrui, Windows) | T1 | 3 h |
| T6 | La limite déclarée | `limits.processes` (plafond 16) dans `Procs`, la ligne de la fenêtre d'installation, l'accord redemandé à la hausse | T1 | 1 h 30 |
| T4 | Les profils et la portée projet | Les valeurs locales d'un projet (la liste des plugins installés), `settings_of` sur les six rangs, `contributes.profiles` (liste, fenêtre, suppression), `secrets/get` avec `profile`, `profiles/get`, `profiles/list`, la liste des noms gardés, la page Plugins d'un projet, « Écrire dans .escouade.yml » clé par clé, la bande d'un profil proposé ; e2e | T1, T2, T3 | 5 h 30 |
| T8 | TS-13 : un réglage de plugin recommandé ou imposé | `CoreKeys::sort` pour `plugins.<id>.<clé>`, le verrou dans la page Plugins d'un projet de l'équipe, l'e2e de `team-settings` | T4 | 2 h |
| T9 | Le SDK, le kit, le banc et la documentation | Protocole et kit 0.1.4, SDK 0.4.0 (`secret(key, { profile })`, `profiles`, le faux Escouade), le banc de conformité, les pages du site (manifeste, API, capacités, permissions) en français et en anglais, `docs/PLUGINS.md` | T4, T5, T6 | 3 h |

- **Vagues** : T7 et T1 tout de suite (T7 peut aller dans une 2.2.x) ; puis T2, T3, T5, T6 en parallèle ; puis T4 ; puis T8 et T9. Environ **23 h d'agent** pour la direction B ; avec 6 agents, **trois vagues**, de l'ordre d'une journée de travail du chef, revues comprises.
- **Avec la direction A** à la place de B : T4 tombe à 3 h 30 (pas de profils, `secrets/get` avec `project`), le reste est le même : environ **21 h d'agent**.
- **Ce qui attend l'utilisateur ou un essai réel** : les choix du §10 avant T1 ; l'essai de T3 sous Windows (un vrai trousseau, une configuration de 6 Ko) ; un essai du plugin VPN de l'auteur sous WSL, si l'auteur le partage (le chef lui répond sur les issues) ; la publication du SDK et des crates (le chef).

## 10. Les choix à trancher

1. **#38 : profils (B) ou portée projet des secrets (A) ?** Recommandé : **B**, avec `scope: "project"` pour les réglages simples. Il couvre la demande, deux projets d'un même client, et le titre dynamique de #40, pour 2 h d'agent de plus. A reste la voie la plus courte si l'on veut répondre vite à l'auteur.
2. **Un réglage `project` peut-il venir de `.escouade.yml` ?** Recommandé : **oui pour un réglage simple, d'office**, comme aujourd'hui ; l'auteur l'interdit par `"repo": false` ; **jamais** un secret, un profil ou une valeur importée. Garder le comportement d'aujourd'hui évite de casser les fichiers existants.
3. **#39 : où la section du plugin ?** Recommandé : **la page Plugins de Réglages › Projets › projet** (direction 1), avec un lien depuis l'onglet « Projets » du plugin : les réglages d'un projet restent au même endroit.
4. **« Écrire dans .escouade.yml » pour `plugins.<id>.<clé>` ?** Recommandé : **oui, clé par clé**, comme pour les autres réglages d'un projet ; l'export de tout le projet continue de ne pas les écrire. Cela change une règle de la spec (`projets/escouade-yml` : « jamais écrites par l'export ») : décision à prendre avec T4.
5. **Un secret long sous Windows : morceaux ou fichier chiffré ?** Recommandé : **des morceaux** au trousseau (64 Kio au plus), sans nouveau format de fichier.
6. **Un champ fichier : import du contenu ou chemin avec un droit de lecture ?** Recommandé : **l'import du contenu**, rien d'autre.
7. **Les jokers de la première version** : `{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}`, d'un mot entier, sans `{path:project}` ni `{path:agent}`. Recommandé : **cette liste**, et rien d'autre tant qu'un plugin n'en montre pas le besoin.
8. **La limite de programmes** : déclarée (`limits.processes`, plafond 16) ou comptée par projet ? Recommandé : **déclarée, plafond 16**, dite à l'installation au-delà de 4 et justifiée à la revue.
9. **`menus`, `statusBar`, `badges`** : ne plus les annoncer et les laisser réservés, ou concevoir `statusBar` maintenant ? Recommandé : **ne plus les annoncer tout de suite (T7)** ; une étude de `statusBar` plus tard, si l'auteur ou un autre plugin la demande.
10. **TS-13 : une équipe peut-elle recommander ou imposer un réglage de plugin sur ses projets ?** Recommandé : **oui, une valeur simple seulement**, jamais un secret ni un profil (T8).
11. **Quelle ligne ?** Recommandé : **T7 dans une 2.2.x** (un correctif d'affichage et de documentation, sans changer le contrat) ; **le reste dans la 2.3**, sur `release/2.3`, une fois ESC-499 reversé.

## Annexe : la maquette

- `design/plugins-contract-2-3/maquette.html` : une page statique, les jetons, polices et icônes de l'app ; plugin, projets et valeurs fictifs (« Tunnel », `dev.exemple.tunnel`). `#a-plugin`, `#a-projet`, `#b-plugin`, `#b-profil`, `#b-projet`, `#onglet`, `#install`, et `?theme=light`.
- `node design/plugins-contract-2-3/render.mjs` écrit les 14 captures de `design/plugins-contract-2-3/captures/` (1440 × 900, sombre puis `-clair`) et dit tout texte qui déborde, toute boîte coupée et ce qui sort de la fenêtre (aucun au 2026-11-17).
