# Escouade à distance : le mode serveur et le téléphone (chantier)

Deux demandes, une même fondation :

- **Le téléphone** : suivre et piloter Escouade depuis son téléphone, hors de chez soi. Voir les agents qui attendent, répondre à une autorisation ou une question, envoyer un message, regarder le Kanban, ouvrir le serveur de dev d'un agent pour le tester.
- **Le mode serveur** (demandé par un utilisateur, relayé le 2026-10-25) : « lancer Escouade sur un serveur VPS et utiliser le client ici pour piloter à distance (et laisser tourner H24) ».

Dans les deux cas, le cœur (agents, Git, Kanban, terminaux, serveur MCP) tourne sur une machine, et l'interface sur une autre. Ce document décrit d'abord la fondation commune, puis le mode serveur, puis le téléphone.

Le remote control (claude.ai et l'app Claude) existe déjà pour la conversation d'un agent. Ce chantier couvre le reste d'Escouade.

## En bref

C'est faisable, et c'est la même fondation que le téléphone. Rien n'est prêt aujourd'hui. Le cœur et l'interface se parlent par l'IPC de Tauri, dans un seul process. Le cœur suppose aussi qu'il tourne sur le poste de l'utilisateur, celui qui a le micro, les notifications, l'Explorateur, les sélecteurs de dossier, le trousseau et le navigateur des connexions.

**Ce qui marche dès aujourd'hui, sans rien développer** (non essayé par nous) :
- Installer Escouade pour Linux (AppImage ou deb) sur un VPS qui a un bureau, et s'y connecter par un bureau à distance (xrdp, VNC).
- Fermer la fenêtre la réduit dans le tray, et les agents continuent.
- Le remote control de claude.ai suit une conversation depuis le téléphone.
- Limites :
  - le bureau à distance est à installer ;
  - sous Linux, il n'y a pas de trousseau, donc les clés d'API de Cursor et des voix en ligne ne s'enregistrent pas ;
  - il n'y a pas de notification système sous Linux ;
  - la voix utilise le micro du serveur.

## L'état du code (main, 2026-10-25)

- **Front → cœur.**
  - Un seul module, `src/lib/ipc.ts` : l'objet `api` et ses quelque 200 appels `invoke`.
  - Aucun `listen` : tous les événements passent par un `Channel<UiEvent>` ouvert une fois par `api.subscribe`, avec 29 sortes d'événements.
  - Changer de transport, côté front, touche ce module seul.
- **Cœur.**
  - 199 commandes Tauri (`commands.rs`), qui prennent `State<Arc<Core>>` et des `Channel`.
  - `hub.rs` ne garde qu'un abonné : le dernier qui s'abonne reçoit tout.
- **Terminaux.**
  - Un `Channel<ArrayBuffer>` par terminal.
  - Seules les interfaces des agents (vues TUI) gardent leur sortie (4 Mo) pour la rejouer à l'ouverture. Un terminal simple ou un lancement n'a pas de tampon.
- **Lien du cœur à Tauri.**
  - `Core<R: Runtime>` tient un `AppHandle`, pour peu de choses : la version, les notifications, le clignotement et l'affichage de la fenêtre, le tray, la mise à jour et le redémarrage.
  - Les tests font déjà tourner le cœur entier sans WebView (`tauri::test::mock_app`, `core_tests.rs`), mais en test seulement.
- **Réseau.** Tout écoute sur 127.0.0.1 :
  - le serveur MCP (ports 47000 à 47999, jeton Bearer, contrôle de `Host`, refus de toute `Origin`) ;
  - le serveur des hooks ;
  - `opencode serve`.

  Rien n'est exposé.
- **Cycle de vie.**
  - Fermer la fenêtre garde les agents.
  - Quitter les arrête : le tour en cours est perdu, et chacun reprend sa session au lancement suivant.
- **Trousseau.**
  - Windows et macOS seulement.
  - Sous Linux, les secrets des intégrations et le jeton MCP vont dans un fichier lisible du seul propriétaire.
  - Les clés d'API de Cursor et des voix en ligne ne s'enregistrent pas du tout.

## La fondation commune

1. **Le cœur sans Tauri.**
   - Un trait `Host` (version, notifier, attirer l'attention, tray, se mettre à jour, redémarrer) remplace l'`AppHandle`.
   - L'app de bureau l'implémente avec Tauri ; le serveur l'implémente sans.
   - Le cœur devient une bibliothèque qui tourne dans un process sans fenêtre.
2. **Une API distante du cœur.**
   - Les mêmes commandes que l'IPC, servies en WebSocket :
     - requêtes et réponses en JSON ;
     - événements poussés ;
     - flux binaires des terminaux multiplexés sur la même connexion.
   - Une seule liste des commandes, par une macro qui enregistre chacune pour Tauri et pour l'API, pour que les deux ne divergent jamais.
   - Ce n'est pas l'API des plugins (`PLUGINS.md`), qui est publique, stable et réduite. L'API distante suit la version de l'app. À la connexion, le client et le serveur se disent leur version et refusent un écart, avec « Mettre à jour le serveur ».
3. **Plusieurs clients à la fois.**
   - `hub.rs` accepte plusieurs abonnés.
   - L'état de l'interface (`UiState` : projet actif, vue, agent choisi, disposition) est gardé par client.
   - Une réponse donnée par un client (autorisation, question) se ferme chez les autres, comme une réponse venue de claude.ai.
4. **Reprendre après une coupure.**
   - À la reconnexion, le client reçoit l'état entier (`InitialState`), puis les événements.
   - Chaque terminal et chaque lancement garde un tampon de sortie, comme les interfaces d'agent, pour retrouver l'écran.
5. **Le front sur deux transports.** `ipc.ts` choisit l'IPC de Tauri (en local) ou la WebSocket (à distance), sans toucher aux composants.
6. **Jamais un port ouvert sur Internet.**
   - L'API écoute sur 127.0.0.1 (ou dans le tunnel WireGuard), et on l'atteint par un tunnel.
   - Un jeton d'appareil s'ajoute, révocable un par un, en défense en profondeur si le tunnel fuit.
   - Le raisonnement est celui du téléphone, plus bas : un serveur qui lance des agents, des terminaux et des commandes ne s'expose pas en HTTP.

### L'inventaire des usages de Tauri (ESC-451, 2026-11-14)

Relevé sur `release/2.3` : chaque usage de Tauri dans le crate de l'app (`src-tauri/src`), et ce qu'il devient. Trois classes :
- **Bureau** : il reste dans l'app de bureau, et le client distant le garde chez lui ; le cœur n'en a pas besoin.
- **Événement** : le cœur ne le fait plus lui-même en mode serveur ; il le publie, et le client le rend.
- **Change de machine** : la table « Ce qui change de machine », plus bas, dit ce qu'il devient.

**Dans le cœur (`Core`), par l'`AppHandle`.** Il passe désormais par le trait `Host` (`host.rs`), dans l'ordre de ses méthodes :

| Usage | Où, avant | Méthode de `Host` | Classe |
| --- | --- | --- | --- |
| La version de l'app (`package_info`) | `Core::load` (télémétrie), `commands::initial_state` | `version` | Reste au cœur : le serveur a la sienne, que le client compare à la sienne |
| Le dossier des ressources (graine des plugins, ESC-338) | `Core::load` | `resource_dir` | Reste au cœur : à côté du binaire du serveur |
| L'origine des fichiers déposés (`FileOrigins`, Windows) | `Core::load`, par `try_state` | `file_origins` | Change de machine : les fichiers joints sont toujours une copie envoyée |
| L'utilisateur regarde la fenêtre (visible, au premier plan) | `Core::alert` (`notify::window_attended`) | `attended` | Événement : le client dit s'il est regardé |
| La fenêtre à l'écran | `updates::snapshot` (redémarrage automatique) | `window_visible` | Événement, de même |
| Le carillon | `Core::alert` (`notify::play_chime`) | `chime` | Événement |
| Le clignotement de la barre des tâches, le rebond du Dock | `Core::alert` (`notify::flash`) | `flash` | Événement |
| La notification système (toast WinRT, `tauri-plugin-notification` sous macOS) ; un clic montre la fenêtre et l'agent | `Core::alert` (`notify::toast`, `notify::show_main`) | `notify` | Événement |
| Le compte des agents qui attendent, dans le tray (icône, info-bulle) | `Core::update_tray` (`tray_by_id`) | `show_waiting` | Événement |
| Les menus natifs dans la langue de l'interface | `Core::relabel_menus` (`menus::relabel`) | `relabel` | Bureau |
| Ouvrir un lien, un fichier, montrer un fichier dans son dossier (opener) | `sentfiles` (fichiers envoyés par un agent, images locales), commandes `open_url`, `open_path` | `open` | Change de machine (fichiers des agents téléchargés, « Afficher » dans le panneau Fichiers) |
| L'image du presse-papiers (GTK, sous Linux) | commande `clipboard_image` | `clipboard_image` | Change de machine : lue par le client, envoyée |
| Lâcher le micro en quittant (`Voice`, état de Tauri) | `Core::shutdown` (`try_state`) | `release_devices` | Change de machine : la voix tourne sur le poste |

**Dans le cœur, sans l'`AppHandle`.**
- `tauri::async_runtime` (`spawn`, `spawn_blocking`, `block_on`), partout : c'est Tokio, sans fenêtre ; il marche tel quel dans un process sans fenêtre. Il deviendra `tokio` quand le cœur sortira du crate de l'app.
- `tauri::ipc::Channel` dans `hub.rs` : le hub prend désormais n'importe quelle fonction pour abonné (`Hub::subscribe`) ; le `Channel` de la WebView n'en est qu'un (`Hub::set_channel`). Événement : c'est le chemin de l'API distante (fondation 2).
- `tauri_plugin_opener` en fonctions libres (`outside.rs`, `fsedit::reveal`) : appelées par `TauriHost::open`, et par la commande « Afficher dans l'Explorateur » (Bureau, puis panneau Fichiers à distance).
- `tauri::webview_version` dans la télémétrie : Bureau (un serveur n'a pas de WebView).
- Les chemins des données (`paths`, dans `escouade-model`) ne doivent rien à Tauri : `~/.escouade` ou `ESCOUADE_DATA_DIR`, sur toutes les machines.

**Autour du cœur : l'app de bureau** (`lib.rs`, `commands.rs` et les modules de la fenêtre). Rien n'en passe dans le cœur :

| Usage | Où | Classe |
| --- | --- | --- |
| Les 347 commandes (`#[tauri::command]`, `State<Arc<Core>>`), leurs `Channel` (événements, terminaux) | `commands.rs`, `windowthread.rs` (leurs voies, hors du fil de la fenêtre) | Restent, et servent aussi l'API distante par une seule liste (fondation 2) |
| `single-instance` (D-Bus sous Linux) | `lib.rs` | Bureau ; hors bureau, un verrou par fichier (`headless::InstanceLock`, ci-dessous) |
| `window-state`, la fenêtre et sa navigation, le zoom, le thème | `lib.rs`, `navigation.rs` | Bureau |
| Le tray et ses menus, le menu de l'app sous macOS | `lib.rs`, `menus.rs` | Bureau ; le compte de ce qui attend lui arrive par événement |
| Les notifications de mise à jour (`toast_with_button`), `show_main` | `notify.rs`, `updates.rs`, `deeplink.rs` | Bureau |
| Les liens `escouade://` (`deep-link`) | `deeplink.rs` | Bureau (le client les reçoit) |
| Les dialogues (`dialog` : enregistrer une image, recharger avec des fichiers non enregistrés ; le sélecteur de dossier est appelé par le front) | `commands.rs`, `navigation.rs` | Change de machine (navigateur des dossiers du serveur) ou reste au client |
| L'updater et le redémarrage (`updater`, `process`) | `updates.rs` | Change de machine : le client et le serveur, chacun le sien |
| Les pages des lancements (WebViews à part, ESC-311), la fin du process de la page | `devpage.rs`, `pagecrash.rs` | Change de machine (port relayé vers le serveur de test) ; Bureau pour la page |
| L'origine des fichiers déposés | `fileorigin.rs` | Change de machine |
| Les serveurs de langage de l'éditeur (ESC-446) : leur gestionnaire tenu par l'app (`lsp::for_app`, état de Tauri), leurs messages à la fenêtre par `emit` (`TauriSink`) | `lsp/mod.rs`, `lib.rs` | Change de machine : ils tournent sur le serveur, à côté des dépôts ; leurs messages deviennent des événements pour le client (fondation 2) |
| Le raccourci global de l'Overseer (ESC-468, `tauri-plugin-global-shortcut`) : le cœur tient le raccourci choisi (`hotkey::Hotkey`) et l'inscrit par un `Arc<dyn Keys>` que l'app lui donne (`PluginKeys`, sur l'`AppHandle`) ; une touche pressée ouvre l'écran de l'Overseer par son lien (`escouade://overseer`, `deeplink`) | `hotkey.rs`, `lib.rs`, `overseer_core.rs` | Bureau : le raccourci est celui du poste ; sans fenêtre, aucune touche n'est inscrite ; à distance, le client l'inscrit chez lui et ouvre l'écran de l'espace |

### La fondation 1 (ESC-451, 2026-11-14) : le cœur sans `AppHandle`

- **`Core` n'est plus générique** sur le runtime de Tauri (`Core<R: Runtime>` disparaît, avec `ToolCtx`, `Tools`, `McpServer` du serveur MCP) : il tient un `Arc<dyn Host>` à la place de l'`AppHandle`. `Core::load` prend un `AppHandle` (l'app, et les tests sur `tauri::test::mock_app`) ou un hôte à lui (`IntoHost`).
- **Deux hôtes** (`host.rs`) :
  - `TauriHost`, celui du bureau : il fait tout comme avant, par les mêmes fonctions (`notify`, `menus`, `outside`, `clipboard`). Rien de visible ne change.
  - `HeadlessHost`, sans fenêtre : il publie ce que le bureau montrerait (`HostEvent` : carillon, clignotement, notification, compte de ce qui attend, ce qu'il faudrait ouvrir) à chacun de ses abonnés ; personne ne regarde de fenêtre, il n'a ni presse-papiers ni micro.
- **Le hub** prend une fonction pour abonné : un process sans fenêtre reçoit les `UiEvent` sans WebView. Il garde un seul abonné ; plusieurs viendront avec la fondation 2.
- **Le verrou du dossier de données** (`headless::InstanceLock`) : un fichier `instance.lock` verrouillé par le système (`flock` sous Unix, ouvert sans partage sous Windows), lâché avec le process quelle que soit sa fin. Le process sans fenêtre le prend avant de charger le cœur, et refuse un dossier déjà tenu ; l'app de bureau le prend aussi, sans s'arrêter si elle ne l'a pas (`single-instance` l'en garde déjà), pour qu'un process sans fenêtre ne démarre pas sur ses données.
- **Le cœur sans fenêtre** (`headless::Headless::start`) : le verrou, l'hôte sans fenêtre, `Core::load`, puis `Core::start` comme l'app (Kanban, Git, enregistrements). C'est ce que lancera `escouade-server`.
- **L'essai court, en test** (`headless_tests.rs`, dans `cargo nextest` sur les trois systèmes, la CI Linux comprise) : sous Linux, sans `DISPLAY` ni `WAYLAND_DISPLAY`, le cœur sans fenêtre crée un projet, lance un agent sur le faux `claude` et le mène à la fin de son tour ; le hub dit l'agent fini, et l'hôte publie le carillon, le clignotement et la notification de fin. Essayé aussi sous WSL, WSLg écarté.
- **Ce qui restait lié à Tauri** : le crate de l'app liait toujours WebKitGTK et GTK, même sans fenêtre. Fait par ESC-514 sans sortir le cœur dans un crate à lui : une feature `desktop` du crate de l'app, et `escouade-tauri-shim` à la place de Tauri sans elle (`docs/CRATES.md`, § 14 ; « Le mode serveur, MS-1 » ci-dessous).

### La fondation 2a (ESC-478, 2026-11-16) : l'API distante du cœur

La spec la décrit (« Escouade à distance › L'API distante »). Rien ne change pour l'app de bureau tant que l'API est éteinte, ce qu'elle est par défaut.

- **Une seule liste des commandes** (`src-tauri/src/remoteapi/list.rs`) :
  - le crate de macro `crates/escouade-command` fournit `#[command]`, qui donne chaque fonction telle quelle à `#[tauri::command]` et écrit à côté `__remote_<nom>`, son appel par l'API distante (ses arguments lus par les mêmes noms que l'IPC, ses états et ses canaux tirés de l'appel, son résultat rendu comme l'IPC le rend) ;
  - dans `commands.rs`, un module `tauri` fait que `#[tauri::command]` désigne cette macro : aucune ligne de commande n'a changé, et une commande s'y écrit comme avant ;
  - `escouade_command::commands!` prend la liste classée et en fait le `generate_handler!` de l'IPC et la table de l'API : les deux ne peuvent pas diverger ; un test vérifie en plus que chaque `#[tauri::command]` du crate est dans la liste, et que le classement ci-dessous dit la même chose ;
  - `State` de Tauri ne se construit que dans Tauri : les commandes prennent celui de `remoteapi` (même `Deref`, même `inner()`), lu dans les états de Tauri pour l'IPC, dans les `Services` de l'appel pour l'API ; `Maybe` pour un état que le process peut ne pas avoir (la voix et l'updater, absents sans fenêtre : `subscribe` et `save_settings` s'en passent).
- **La WebSocket** (`remoteapi::server`, `remoteapi::proto`) :
  - `ws://127.0.0.1:<port>/remote`, sur 127.0.0.1 seulement, éteinte par défaut : l'app de bureau l'allume quand `ESCOUADE_REMOTE_API` donne un port, le cœur sans fenêtre par `Headless::serve_remote` ;
  - avant l'ouverture : le chemin, un `Host` du bouclage (tout port, celui d'un tunnel compris) et pas d'`Origin` sauf celle de la fenêtre d'Escouade (`tauri://localhost`, `http://tauri.localhost`) : une page web du poste est refusée en 403 ;
  - la poignée de main : le premier message, dans les 10 s, donne la version du client et son jeton d'appareil ; le serveur refuse un jeton absent, inconnu ou révoqué (`token`), puis une autre version que la sienne (`version`, avec la sienne et le côté à mettre à jour) ;
  - puis des appels `{"t":"call","id","cmd","args"}` et leurs réponses `{"t":"reply","id","ok"|"error","code"?}`, les messages des canaux (`__CHANNEL__:<id>`, comme l'IPC) en JSON ou en binaire (un octet de sorte, l'id sur 8 octets, les octets : la sortie d'un terminal), et ce que publie l'hôte sans fenêtre (`{"t":"host","event"}`) ; un message de plus de 16 Mo ferme la connexion ;
  - chaque appel passe sur la voie de sa commande (`windowthread::lane_of`, comme l'IPC : les frappes d'un terminal gardent leur ordre).
- **Les jetons d'appareil** (`remoteapi::devices`) : la liste dans `remote-devices.json` (nom, dates), chaque jeton au trousseau (`remote-device-<id>`) ou, s'il le refuse (Linux), dans ce fichier lisible du seul propriétaire, comme le jeton MCP ; comparés en temps constant à tous les jetons à chaque connexion ; révoquer un appareil ferme ses connexions. En build de debug seulement, `ESCOUADE_REMOTE_API_TOKEN` donne un appareil « test » gardé en mémoire (les e2e de la fondation 2b).
- **Plusieurs clients** : le hub garde un abonné par client (`remoteapi::Client` : la fenêtre, ou une connexion) ; `subscribe` d'un client ne remplace que le sien. L'état d'interface de la fenêtre reste `core.ui` (enregistré) ; chaque connexion a le sien (`core.client_ui`, oublié à sa fin) ; ce que le cœur y écrit (l'agent créé, le projet ajouté) va au client de l'appel, ce qui disparaît (un agent, un projet) est oublié de tous. Une réponse d'un client se ferme chez les autres : le hub leur pousse le même événement.
- **La stratégie** `servers.remoteApi` (liste unique d'`escouade-policy`) : à `false`, l'API ne démarre pas et refuse chaque nouvelle connexion (`policy`).
- **Pas encore** (2a) : le tampon de sortie des terminaux et la reprise après une coupure, `ipc.ts` sur deux transports et l'app de bureau testée sur l'API, les corps bruts des requêtes : c'est la fondation 2b, ci-dessous ; `escouade-server`, l'appairage, le tunnel, « Se connecter à un serveur ».

### La fondation 2b (ESC-495, 2026-11-16) : la fenêtre sur deux transports, la reprise, les bornes

La spec la décrit (« Escouade à distance › L'API distante »). Rien ne change pour l'utilisateur tant que l'API est éteinte : la fenêtre reste sur l'IPC de Tauri, qui reçoit les mêmes octets qu'avant.

- **`ipc.ts` sur deux transports** (`src/lib/transport.ts`) :
  - `invoke` et `Channel` aux signatures de Tauri ; `ipc.ts` (ses fonctions et leurs signatures inchangées) et les modules qui appelaient le cœur (`opener`, `plugins`, `store`, `teams`, `team-projects`, `team-settings`, `teamboard`) les prennent là ; un test (`transport-only.test.ts`) vérifie qu'aucun autre module n'importe `@tauri-apps/api/core` ;
  - choisi au démarrage : la WebSocket quand la page en a reçu l'adresse (`window.__ESCOUADE_REMOTE__`), l'IPC sinon ; en debug, `ESCOUADE_REMOTE_API` et `ESCOUADE_REMOTE_API_TOKEN` donnés tous deux mettent la fenêtre sur l'API (`remoteapi::window` : la socket liée avant la fenêtre, l'adresse, le jeton et la version donnés par un script d'initialisation de la fenêtre principale, réservé à l'origine de la page d'Escouade ; la CSP ouverte alors au seul `ws://127.0.0.1:*`) ; pas de réglage visible ;
  - sur l'API : mêmes commandes, arguments, résultats et erreurs ; canaux JSON et binaires ; corps bruts et réponses en octets en trames binaires ; les codes de l'API (`desk-only`, `not-served-yet`, `policy`, `token`, `version`, `too-many-connections`, `busy`, `too-large`) rendus en texte de la langue de la fenêtre (zone `remote`) ; les commandes du poste, que le `welcome` nomme, faites par l'IPC de la fenêtre.
- **Ce qui reste propre à la fenêtre** (`@tauri-apps/…` hors de `transport.ts`, la liste que vérifie `transport-only.test.ts`) :
  - les dialogues natifs (`@tauri-apps/plugin-dialog` : le sélecteur de dossier de « Nouveau projet », des réglages avancés, du clone d'un projet d'équipe, d'un plugin local, du logo d'un projet) ;
  - le navigateur du système pour un lien de la page d'un plugin ou du store (`@tauri-apps/plugin-opener`) ;
  - la fenêtre et sa WebView (`@tauri-apps/api/window`, `@tauri-apps/api/webview` : la barre de titre, le thème, le zoom) ;
  - les événements des fenêtres des pages de lancement (`@tauri-apps/api/event`, `PagesWindow`) ;
  - côté Rust, ce que l'app de bureau fait de sa fenêtre : ses menus et son tray, le glisser-déposer natif (l'origine des fichiers déposés, `fileorigin` : `attach_origin` et `file_origin` restent refusées à distance, `not-served-yet`) ;
  - par les commandes du poste (`desk-only`, faites par l'IPC de la fenêtre) : les pages des lancements, la voix, le raccourci global, l'updater, les liens `escouade://`, la fin de l'app ;
  - les e2e appellent encore l'IPC de Tauri directement (`window.__TAURI_INTERNALS__`) pour leurs sondes.
- **Le tampon de sortie** (`termlog.rs`) : chaque terminal, lancement, étape d'un lancement de test et connexion de compte garde ses derniers **1 Mo**, numérotés par leur place dans toute la sortie (`TermLog`) ; l'interface d'un agent en TUI garde ses **4 Mo** dans le même anneau (`tui::TuiOutput`, qui rejouait déjà l'écran à l'ouverture d'une vue : pas de second tampon). La fenêtre sur l'IPC reçoit les octets seuls ; un client de l'API les reçoit numérotés (trame `Frame::Term` : la fin de ses octets).
- **La reprise** :
  - le `welcome` donne une **session** (128 bits au hasard), que le client redonne au `hello` suivant ; une session est celle d'un appareil (`remoteapi::sessions`) : celle d'un autre, ou oubliée, n'est pas trouvée, et le client n'y reprend rien ; gardée 15 min après sa dernière connexion, 8 par appareil, 64 en tout, 256 terminaux chacune ;
  - le client rouvre la WebSocket (250 ms, 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s), redonne sa session, reprend chaque terminal (`{"t":"resume","channels":[{"channel","after"}]}` : la fin des derniers octets reçus) et relit l'état entier (`subscribe`, `state.resync` : les choix de la fenêtre gardés et redonnés au cœur, les terminaux finis entre-temps dits finis par `InitialState.exitedTerminals`) ;
  - le cœur envoie la suite (`Frame::Term`), ou, si le client est trop en retard, tout ce qui reste et le compte de ce qui est perdu (`lost`, que la fenêtre écrit dans le terminal) ; pour une interface d'agent trop en retard, l'écran rejoué (`Frame::Channel`) puis la numérotation (`mark`) ; la fin de la reprise est dite (`resumed`) ;
  - le client ne montre aucun octet deux fois (une fin déjà reçue est sautée) et rien au-delà d'un trou (il redemande la suite) ;
  - un appel resté sans réponse à la coupure est rendu en erreur, jamais rejoué ; un appel fait pendant la coupure échoue aussitôt ; la bande « Connexion au cœur perdue · nouvel essai… » le dit ; un jeton refusé, une autre version ou la stratégie arrêtent les essais, et la bande dit pourquoi.
- **Les bornes** (`remoteapi::server::Limits`, `remoteapi::outbox`) :
  - la file d'envoi d'une connexion : 2048 messages et 8 Mo ; au-delà, les événements de l'état se fondent en une relecture (`resync` : le dernier état gagne), la sortie d'un terminal n'est pas mise en file et le trou est dit (`gap`) quand il y a de la place, pour une reprise par le numéro ; les réponses et les rejeux ne sont jamais jetés, mais au-delà de 96 Mo en attente, ou quand rien n'est lu pendant 30 s alors que quelque chose attend, la connexion est fermée (code 4002, `slow-client`, dans le journal avec ce qui attendait) ;
  - 4 connexions par appareil, 16 en tout (`too-many-connections`) ; 8 poignées de main en cours ; au-delà de 20 poignées de main ratées dans la minute, les connexions sont fermées dès qu'acceptées jusqu'à la fin de la minute ;
  - 256 appels en attente de réponse par connexion (`busy`) ;
  - ce que donne le client est borné et vérifié : le numéro de reprise (au-delà de la fin : la fin), 1024 terminaux par reprise, une session de 64 caractères au plus, 32 en-têtes de 64 et 8 Ko, un corps brut de 64 Mo au plus et de la longueur que dit son appel (`too-large`), 8 corps en attente ;
  - le journal ne dit ni jeton, ni session, ni contenu d'un terminal, ni argument : un texte du client n'y paraît que coupé à 80 caractères imprimables (le chemin refusé, la version).
- **Les corps bruts** : `fs_write_bytes` et `attach_chunk` prennent `remoteapi::Body` (la requête de Tauri pour l'IPC, les trames `Frame::Body` pour l'API, chacune de 16 Mo au plus) ; une réponse en octets de plus de 16 Mo arrive en parties (`Frame::ReplyPart`) ; `fs_write_bytes` et les fichiers joints (`attach_open`, `attach_chunk`, `attach_finish`, `attach_cancel`, `attach_discard`) sont servis.
- **Les essais** : Rust (`termlog_tests`, `remoteapi_tests` : la coupure pendant qu'un terminal écrit, la session d'un autre appareil, le client muet, la borne dure, les connexions, les poignées de main ratées, les appels en attente, les corps bruts, la file et les sessions seules ; `remoteapi::window`) ; Vitest (`transport.test.ts`, contre un faux serveur WebSocket) ; e2e : `ESCOUADE_E2E_REMOTE=1` lance la fenêtre sur l'API, la part « remote API » de la CI rejoue `app`, `conversation`, `terminal-height`, `kanban-pr`, `mcp` et `remote-cut` (la WebSocket fermée de force pendant qu'un terminal écrit).

### Le classement des commandes (ESC-478)

Chaque commande du cœur est écrite une fois (`#[tauri::command]` dans `commands.rs`), puis rangée dans une des quatre classes de la liste unique, `src-tauri/src/remoteapi/list.rs` : l'IPC de Tauri et l'API distante sont faits tous deux de cette liste. Un test (`remoteapi_tests`) vérifie que toute commande écrite y est, et que cette section dit la même classe que la liste.

**Ajouter une commande** : l'écrire dans `commands.rs` avec `#[tauri::command]`, comme avant, puis mettre son nom dans sa classe de `remoteapi/list.rs`, et ici : une puce par commande, à sa place dans l'ordre alphabétique de sa classe (deux tickets qui ajoutent chacun une commande ne touchent pas la même ligne).

#### Servies telles quelles

Elles agissent sur le cœur, quelle que soit la machine du client :

- `account_add_machine`
- `account_create`
- `account_key`
- `account_machine`
- `account_models_refresh`
- `account_remove`
- `account_reorder`
- `account_shareable`
- `account_sign_in_ended`
- `account_status`
- `account_update`
- `agent_branch_switch`
- `agent_folder_create`
- `agent_folder_delete`
- `agent_folder_fold`
- `agent_folder_move`
- `agent_folder_principal`
- `agent_folder_rename`
- `agent_git_sync`
- `agent_local_bytes`
- `agent_local_image`
- `agent_logout`
- `agent_merge_check`
- `agent_no_pr`
- `agent_pr_detail`
- `agent_pr_fix`
- `agent_pr_log`
- `agent_pr_open`
- `agent_pr_propose`
- `agent_pr_rerun`
- `agent_prepare_launch`
- `agent_prs_refresh`
- `agent_signed_in`
- `agent_terminal`
- `answer_permission`
- `answer_question`
- `archive_agent`
- `autopilot_resume`
- `back_to_previous_account`
- `back_up_now`
- `backups_view`
- `board_capacity`
- `board_set`
- `branch_check`
- `branch_create`
- `branch_delete`
- `branch_list`
- `branch_switch`
- `branches_merged`
- `cancel_restore`
- `cancel_resume`
- `ci_detail`
- `ci_job_log`
- `ci_log_to_agent`
- `ci_refresh`
- `ci_rerun`
- `claude_found`
- `claude_memory_offer`
- `claude_memory_take`
- `claude_update_status`
- `clean_worktrees`
- `clone_repo`
- `closed_projects`
- `code_search`
- `commit_direct`
- `commit_preview`
- `commit_propose`
- `compare_backup`
- `continue_on_engine`
- `create_agent`
- `create_agent_on_branch`
- `create_agent_on_pr`
- `data_dir`
- `delete_agent`
- `delete_loss` : ce que perdrait la suppression qu’un agent demande (PR 44, ESC-511), lu par git sur la machine du cœur
- `discover_act`
- `discover_view`
- `discussion_drop`
- `discussion_pour`
- `discussion_start`
- `dismiss_recovery`
- `docs_compare`
- `docs_decision_accept`
- `docs_decision_propose`
- `docs_decision_replace`
- `docs_decisions`
- `docs_detect_spec`
- `docs_document_check`
- `docs_document_move`
- `docs_document_text`
- `docs_documents`
- `docs_documents_search`
- `docs_drive`
- `docs_folder_add`
- `docs_import`
- `docs_import_plan`
- `docs_pending`
- `docs_plan`
- `docs_plan_cancel`
- `docs_pull` : « Synchroniser » sans rien à partager (ESC-534) : la récupération et la fusion dans le dossier du projet, celui du cœur
- `docs_recent`
- `docs_share`
- `docs_share_cancel`
- `docs_share_resolve`
- `docs_spec_page`
- `docs_spec_search`
- `docs_spec_tree`
- `docs_spec_write`
- `duplicate_agent`
- `effective_workflows`
- `engine_order_set`
- `engines`
- `engines_declared`
- `engines_found_now`
- `engines_off_now`
- `erase_closed_project`
- `favorite_model_set`
- `feature_offer` : la proposition d’une fonctionnalité coupée à montrer maintenant (F-3, ESC-558), celle du cœur ; jamais pendant le tour d’un agent que le client montre (`visible`)
- `feature_offer_act`
- `feature_offers`
- `features_tour_act`
- `features_tour_view`
- `file_suggestions`
- `forge_check`
- `forge_connect`
- `forge_disconnect`
- `forge_guess`
- `forge_token_page`
- `forges`
- `forget_repo`
- `fs_base`
- `fs_browse`
- `fs_create`
- `fs_delete`
- `fs_mkdir`
- `fs_read`
- `fs_read_bytes`
- `fs_rename`
- `fs_stat`
- `fs_tree`
- `fs_write`
- `fs_write_bytes`
- `get_commands`
- `get_conversation`
- `get_workflow_tracks`
- `get_workflows`
- `git_branches`
- `git_diff`
- `git_diff_refs`
- `git_discard`
- `git_fetch`
- `git_files`
- `git_log`
- `git_project_diff`
- `git_pull`
- `git_push`
- `git_show`
- `git_user_name`
- `harness_enable_in`
- `harness_plugin_of`
- `harness_scope`
- `host_notification_click`
- `http_ready`
- `inbox_answer` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur ; les fichiers de la réponse arrivent par `attach_*`, en octets bruts
- `inbox_closing` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_delegate` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_erase_closed_files` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur : les fichiers effacés sont ceux du cœur
- `inbox_file` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur : une part d’un fichier en base64, pour la fenêtre
- `inbox_file_bytes` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur : un fichier entier en octets bruts (trames binaires, en parties au-delà de 16 Mo, ESC-495)
- `inbox_get` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_list` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_reopen` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_seen` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_snooze` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `inbox_track` : la boîte « À toi » (ESC-510) : ce que la fenêtre seule voit de la boîte (ouverte, « Comparer en grand », un fichier regardé), compté sur le cœur dont c’est la boîte
- `inbox_view_act` : la boîte « À toi » (ESC-513) : le geste d’une vue est celui de sa source (permission, ticket, décision, docs), sur l’état du cœur
- `inbox_view_snooze` : la boîte « À toi » (ESC-513) : « Plus tard » sur une vue, gardé dans l’état durable du cœur
- `inbox_views` : la boîte « À toi » (ESC-513) : ce qu’Escouade y verse de lui-même, lu dans l’état du cœur à chaque lecture
- `inbox_weight` : la boîte « À toi » (ESC-509) : l’écran du téléphone, servi sur le cœur
- `integrate_base`
- `integration_connect`
- `integration_containers`
- `integration_disconnect`
- `integration_import`
- `integration_issues`
- `integration_resync`
- `integration_states`
- `interrupt`
- `isola_approve`
- `isola_config`
- `isola_down`
- `isola_services`
- `mcp_activity`
- `mcp_clear_activity`
- `mcp_declare_status`
- `mcp_manual_command`
- `mcp_set_enabled`
- `mcp_status`
- `memory_forget`
- `memory_list`
- `merge_agent`
- `merge_agent_repos`
- `message_subagent`
- `onboarding_act`
- `onboarding_need`
- `onboarding_offers`
- `onboarding_view`
- `overseer_revoke`
- `overseer_send`
- `overseer_undo`
- `overseer_view`
- `overview_prs`
- `phone_check` : Réglages › Téléphone (ESC-567) : le réseau revu (la box, l'adresse publique, Tailscale)
- `phone_remove` : Réglages › Téléphone (ESC-567) : un téléphone retiré du tunnel, ses connexions coupées
- `phone_rotate_key` : Réglages › Téléphone (ESC-567) : la clé du tunnel changée, chaque téléphone à appairer de nouveau
- `phone_set` : Réglages › Téléphone (ESC-567) : l'accès allumé ou éteint, son port UDP, Tailscale ; refusé par la stratégie
- `phone_status` : Réglages › Téléphone (ESC-567) : l'état du tunnel, du port, de la box, de l'adresse publique, de Tailscale
- `plugin_approve_commands`
- `plugin_discard`
- `plugin_icon`
- `plugin_inspect_git`
- `plugin_inspect_seed`
- `plugin_inspect_store`
- `plugin_install`
- `plugin_install_seed`
- `plugin_need_engine`
- `plugin_profile_delete`
- `plugin_profile_save` : les profils d'un plugin (ESC-530), gardés par le cœur
- `plugin_profile_secret` : le secret d'un profil, au trousseau (ou dans le fichier) de la machine du cœur, comme `plugin_set_secret`
- `plugin_profile_secrets`
- `plugin_project_set` : les réglages d'un plugin par projet (ESC-530), gardés par le cœur
- `plugin_project_write` : écrit le `.escouade.yml` du projet, dans son dossier sur la machine du cœur
- `plugin_remove`
- `plugin_run`
- `plugin_secret_move` : les secrets des plugins gardés dans un fichier (ESC-499) : le fichier et le trousseau de la machine du cœur
- `plugin_secrets`
- `plugin_seed_offers`
- `plugin_set_enabled`
- `plugin_set_permission`
- `plugin_set_project`
- `plugin_set_secret`
- `plugin_set_setting`
- `plugin_start`
- `plugins_check_updates`
- `plugins_list`
- `pr_candidates`
- `pr_commit_files`
- `pr_contents`
- `pr_control`
- `pr_detail`
- `pr_log`
- `pr_merge`
- `pr_pending`
- `pr_pending_change`
- `pr_rerun`
- `pr_review_discard`
- `pr_review_note`
- `pr_review_send`
- `pr_review_start`
- `pr_review_submit`
- `pr_reviews`
- `pr_write`
- `project_code_host`
- `project_environments`
- `project_environments_read`
- `project_link_team_board`
- `project_logo`
- `project_prs`
- `project_prs_watch`
- `project_set_team`
- `project_team_hint_dismiss`
- `project_team_pour`
- `project_team_pour_preview`
- `refine_workflows`
- `refresh_repos`
- `refresh_usage`
- `refresh_workflows`
- `refuse_delete` : la suppression qu’un agent demande, refusée sur sa carte (ESC-511)
- `remote_device_revoke` : Réglages de l’espace › Appareils (ESC-539) : un appareil appairé au cœur retiré, ses connexions fermées
- `remote_devices` : Réglages de l’espace › Appareils (ESC-539) : les appareils appairés au cœur
- `remote_image`
- `remote_pair_code` : Réglages de l’espace › Appareils (ESC-539) : un code d’appairage de 10 minutes créé depuis un appareil déjà appairé (S5)
- `remove_project`
- `remove_workflow_file`
- `rename_agent`
- `reorder_projects`
- `restart_agent_process`
- `restore_backup`
- `resume_on_account`
- `run_start`
- `save_settings`
- `scheduled_create`
- `scheduled_delete`
- `scheduled_pause`
- `scheduled_preview`
- `scheduled_update`
- `search_conversations`
- `secrets_file` : les secrets des plugins gardés dans un fichier (ESC-499) : le fichier et le trousseau de la machine du cœur
- `secrets_file_remove` : les secrets des plugins gardés dans un fichier (ESC-499) : le fichier et le trousseau de la machine du cœur
- `send_message`
- `sent_file_bytes`
- `sent_files`
- `set_agent_account`
- `set_agent_options`
- `set_agent_view`
- `set_project_logo_bytes`
- `set_remote_control`
- `set_ui`
- `set_workflow_removed`
- `shared_resolve`
- `shared_trust`
- `shared_unkeep`
- `shared_write`
- `space_glance` : « Partout » et les piles (ESC-539) : ce qui attend sur le cœur, lu par la connexion de chaque espace
- `stats`
- `stop_bg_task`
- `stop_subagent`
- `store_refresh`
- `store_suggestions`
- `store_view`
- `subagent_transcript`
- `subscribe`
- `suggest_run_commands`
- `suggest_worktree_steps`
- `target_cis`
- `task_output`
- `team_confirm_words`
- `team_cost_notice`
- `team_costs`
- `team_costs_ack`
- `team_costs_send`
- `team_costs_untold`
- `team_create`
- `team_docs_choose`
- `team_docs_dismiss`
- `team_docs_remove_repo_files`
- `team_docs_settle`
- `team_docs_status`
- `team_escrow_new`
- `team_forget`
- `team_invite`
- `team_join`
- `team_join_preview`
- `team_list`
- `team_memory_decide`
- `team_memory_lift`
- `team_memory_of_ticket`
- `team_memory_quarantine`
- `team_memory_set_review`
- `team_memory_view`
- `team_new_recovery`
- `team_own_device`
- `team_pair_cancel`
- `team_pair_join`
- `team_pair_start`
- `team_pair_step`
- `team_plugin_dismiss`
- `team_plugin_inspect`
- `team_plugin_revoke`
- `team_plugin_sign`
- `team_plugins`
- `team_plugins_all`
- `team_plugins_retry`
- `team_project_accept_board`
- `team_project_put`
- `team_project_remove`
- `team_projects`
- `team_projects_retry`
- `team_recover`
- `team_settings`
- `team_settings_check`
- `team_settings_consent`
- `team_settings_cosign`
- `team_settings_propose`
- `team_settings_retry`
- `team_settings_seen`
- `team_settings_withdraw`
- `team_sync`
- `team_sync_status`
- `telemetry_set`
- `term_kill`
- `term_report`
- `term_resize`
- `term_spawn`
- `term_write`
- `test_recipe_approve`
- `test_run_start`
- `ticket_approve`
- `ticket_create`
- `ticket_delete`
- `ticket_dismiss`
- `ticket_finish_anyway`
- `ticket_hide`
- `ticket_images`
- `ticket_pr_back_to_todo`
- `ticket_pr_fix_ci`
- `ticket_pr_fix_reviews`
- `ticket_pr_merge`
- `ticket_pr_stop_waiting`
- `ticket_pr_update_branch`
- `ticket_prioritize`
- `ticket_prs_refresh`
- `ticket_reject`
- `ticket_resolve_conflict`
- `ticket_resume`
- `ticket_set_target`
- `ticket_share`
- `ticket_start`
- `ticket_team_assign`
- `ticket_team_handoff`
- `ticket_team_handoff_cancel`
- `ticket_team_handoff_engines`
- `ticket_team_handoff_plan`
- `ticket_team_handoff_take`
- `ticket_team_local`
- `ticket_team_resume`
- `ticket_team_settle`
- `ticket_update`
- `tui_asks_trust`
- `update_project`
- `vectors_model`
- `vectors_model_cancel`
- `vectors_model_delete`
- `vectors_model_download`
- `vsx_card`
- `vsx_file`
- `vsx_files`
- `vsx_image`
- `vsx_install`
- `vsx_installed`
- `vsx_remove`
- `vsx_search`
- `vsx_set_enabled`
- `vsx_set_theme`
- `vsx_updates`
- `wake_remote` : « Réveiller » (ESC-544) : l'agent démarre sur le cœur, comme un autre
- `warm_agent`
- `workflow_refine_plan`

#### Du poste seulement

Refusées à distance avec l'erreur `desk-only` : le client les fait chez lui.

- `conversation_end` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `conversation_hold` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `conversation_listen` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `conversation_say` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `conversation_start` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `conversation_stop` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `deep_links_take` : la fenêtre du poste : ses liens, son état, sa fin
- `devpage_close` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_detach` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_devtools` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_go` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_list` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_mode` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_open` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_place` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `devpage_show` : les pages des lancements, des WebViews du poste (le port du serveur de test sera relayé en mode serveur)
- `local_voice_get` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `local_voice_set` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `open_url` : la fenêtre du poste : ses liens, son état, sa fin
- `overseer_hotkey_press` : le raccourci global du poste
- `overseer_shortcut_set` : le raccourci global du poste
- `overseer_voice_start` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `play_chime` : la fenêtre du poste : ses liens, son état, sa fin
- `quit_app` : la fenêtre du poste : ses liens, son état, sa fin
- `received_save` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `received_show` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `restart_for_restore` : la fenêtre du poste : ses liens, son état, sa fin
- `save_image` : un dialogue du poste : le client enregistre l’image lui-même
- `server_alert` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `server_attempt` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_cancel` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_code_over_ssh` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_forget` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_leave` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_link` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_links` : les espaces (ESC-539) : le lien de chaque serveur connu, pour les connexions des piles et de « Partout »
- `server_note` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_open` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_pair` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_relay` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `server_relays` : ce qui change de machine, côté poste (ESC-538) : les fichiers reçus du serveur, ses ports relayés, ses notifications montrées, les réglages de la voix
- `server_rename` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `server_retry` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `servers_ssh_hosts` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `servers_view` : les serveurs que connaît ce poste, leur tunnel SSH et leur jeton (ESC-537) : toujours ceux du poste, quel que soit le cœur de la fenêtre
- `set_held` : la fenêtre du poste : ses liens, son état, sa fin
- `set_unsaved` : la fenêtre du poste : ses liens, son état, sa fin
- `space_notify` : les espaces (ESC-539) : la notification d’un autre serveur, nommée par son espace, rendue par ce poste
- `space_window` : les espaces (ESC-539) : un espace dans sa propre fenêtre, une au plus
- `speech_delete` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `speech_download` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `speech_download_cancel` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `speech_info` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `speech_preview` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `speech_preview_online` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `update_check` : l’updater du client : le client et le serveur ont chacun le leur
- `update_close` : l’updater du client : le client et le serveur ont chacun le leur
- `update_download` : l’updater du client : le client et le serveur ont chacun le leur
- `update_postpone` : l’updater du client : le client et le serveur ont chacun le leur
- `update_presence` : l’updater du client : le client et le serveur ont chacun le leur
- `update_restart` : l’updater du client : le client et le serveur ont chacun le leur
- `voice_cancel` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_delete_model` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_download` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_download_cancel` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_info` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_mistral_key` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_online_catalog` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_online_key_delete` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_online_key_set` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_online_keys` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_start` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `voice_stop` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste

#### Changent de machine, servies

Elles changent de machine en mode serveur (table « Ce qui change de machine »), mais marchent telles quelles : servies.

- `account_login` : la connexion d’un compte dans un terminal : sa sortie va au client, le navigateur s’ouvre sur la machine du cœur (à essayer harness par harness)
- `attach_cancel` : les fichiers joints : une copie envoyée en morceaux d’octets bruts (trames binaires, ESC-495), écrite sur la machine du cœur
- `attach_chunk` : les fichiers joints : une copie envoyée en morceaux d’octets bruts (trames binaires, ESC-495), écrite sur la machine du cœur
- `attach_discard` : les fichiers joints : une copie envoyée en morceaux d’octets bruts (trames binaires, ESC-495), écrite sur la machine du cœur
- `attach_finish` : les fichiers joints : une copie envoyée en morceaux d’octets bruts (trames binaires, ESC-495), écrite sur la machine du cœur
- `attach_open` : les fichiers joints : une copie envoyée en morceaux d’octets bruts (trames binaires, ESC-495), écrite sur la machine du cœur
- `check_authorities` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `claude_locate` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `conversation_send` : le micro, les haut-parleurs et leurs modèles : la voix tourne sur le poste
- `create_project` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `demo_create` : le projet de démonstration, écrit dans le dossier de données de la machine du cœur
- `demo_remove` : le projet de démonstration, écrit dans le dossier de données de la machine du cœur
- `docs_document_add` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `first_commit` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `inspect_folder` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `inspect_setup` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `onboarding_clone` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `onboarding_folder` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `paste_image` : l’image collée, envoyée en données par le client
- `plugin_inspect_dev` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `plugin_inspect_folder` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `shared_preview` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `suggest_run_commands_at` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `suggest_worktree_steps_at` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `team_plugin_key_create` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `team_plugin_key_import` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `team_plugin_submit` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `team_project_clone` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)
- `team_project_logo` : un chemin de la machine du cœur : il marche tel quel (le navigateur des dossiers du serveur viendra avec le mode serveur)

#### Changent de machine, refusées

Elles ne marchent pas encore telles quelles : refusées avec l'erreur `not-served-yet`, pour le ticket du mode serveur.

- `agent_local_open` : ouvert sur la machine du cœur : téléchargé sur le poste en mode serveur
- `attach_origin` : l’origine d’un fichier déposé sur le poste (Windows) : lue par le client et envoyée en mode serveur
- `clipboard_image` : lu sur la machine du cœur : lu par le client et envoyé en mode serveur
- `file_origin` : l’origine d’un fichier déposé sur le poste (Windows) : lue par le client et envoyée en mode serveur
- `fs_reveal` : ouvert sur la machine du cœur : téléchargé sur le poste en mode serveur
- `inbox_open_file` : ouvert sur la machine du cœur : téléchargé sur le poste en mode serveur (`inbox_file_bytes` le sert déjà)
- `inbox_save_file` : « Enregistrer sous… » d’un fichier de la boîte (ESC-510) : le dialogue du système et la copie sur la machine du cœur : téléchargé sur le poste et enregistré par le client en mode serveur (`inbox_file_bytes` le sert déjà)
- `lsp_log` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_release` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_release_view` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_restart` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_running` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_send` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_servers` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_start` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `lsp_trust` : les serveurs de langage de l’éditeur : leurs messages à la fenêtre deviendront des événements
- `open_path` : ouvert sur la machine du cœur : téléchargé sur le poste en mode serveur
- `read_logo` : lu sur la machine du cœur : lu par le client et envoyé en mode serveur
- `sent_file_open` : ouvert sur la machine du cœur : téléchargé sur le poste en mode serveur
- `set_project_logo` : lu sur la machine du cœur : lu par le client et envoyé en mode serveur

## Le mode serveur

### Ce que voit l'utilisateur

- **Sur le VPS.**
  - `escouade-server` : Linux x64 et arm64, sans WebKit ni GTK.
  - Une commande d'installation pose un service systemd : de l'utilisateur avec `loginctl enable-linger`, ou du système.
  - Les données vont dans `~/.escouade`, comme aujourd'hui.
  - `escouade-server pair` affiche un code d'appairage pour un nouvel appareil.
- **Sur le PC.**
  - Escouade › « Se connecter à un serveur » : l'hôte SSH (pris dans `~/.ssh/config` s'il y est), puis le code d'appairage la première fois.
  - La barre de titre dit sur quelle machine on est. On repasse en local d'un clic.
- **H24.** Fermer le client, éteindre le PC ou couper le réseau ne touche à aucun agent : ils tournent sur le serveur, et le client n'en est qu'une vue.

### Le tunnel : SSH d'abord

- **Pourquoi SSH.** Un VPS s'administre déjà par SSH, avec des clés.
- **Comment.** Le client lance le `ssh` du système (présent sous Windows 10 et plus, macOS et Linux) :

  ```
  ssh -N -L <port local>:127.0.0.1:<port du serveur> <hôte>
  ```

  - Il prend la config, l'agent et les `known_hosts` de l'utilisateur.
  - Rien de nouveau n'est exposé sur le VPS, et Escouade ne gère aucune clé SSH.
  - Une seule redirection suffit à l'API : les terminaux passent dans sa connexion ; un serveur de test ou le retour d'une connexion de compte prend une redirection de plus, à la demande (ESC-538).
- **Plus tard.** WireGuard intégré (celui du téléphone, plus bas) et Tailscale s'il est installé, pour qui n'a pas SSH, ou pour un téléphone.

### Ce qui change de machine

La dernière colonne dit ce que fait le client depuis MS-4 (ESC-538, « Ce qui change de machine, MS-4 » plus bas).

| Quoi | Aujourd'hui | En mode serveur | Côté client (ESC-538) |
| --- | --- | --- | --- |
| Choisir un dossier (assistant « Nouveau projet ») | Sélecteur du système | Un navigateur des dossiers du serveur, servi par le cœur ; cloner un dépôt devient le cas courant | Fait : « Cloner un dépôt » d'abord, « Un dossier du serveur » (`fs_browse`, `clone_repo`) |
| Logo d'un projet | Chemin d'une image | L'image envoyée au serveur | Fait : lue sur le poste, ses octets envoyés (`set_project_logo_bytes`) |
| Fichiers joints | Chemin d'origine (Windows) ou copie | Toujours une copie envoyée | Fait : toujours une copie (corps bruts, ESC-495) |
| Coller une image | Presse-papiers lu par le cœur sous Linux | Lu par le client, envoyé | Fait : `clipboard_image` du poste, puis l'envoi d'une image collée |
| Fichiers envoyés par un agent | « Ouvrir » et « Afficher dans le dossier » sur la machine du cœur | Téléchargés sur le poste ; « Ouvrir » ouvre la copie (jamais un exécutable, comme ESC-171) ; « Afficher » les montre dans le panneau Fichiers | Fait (S12) : `sent_file_bytes`, `agent_local_bytes`, puis `received_save` sur le poste ; « Voir sur le serveur » |
| « Afficher dans l'Explorateur » | Explorateur du système | Panneau Fichiers, ou caché | Fait : le panneau Fichiers pour un projet, caché pour les dossiers des données |
| Serveur de test d'un agent (Lancement) | 127.0.0.1 du poste | Un port local du client, relayé vers le 127.0.0.1 du serveur (WebSocket du rechargement à chaud compris) | Fait : une redirection SSH de plus par port (`server_relay`), pas dans la connexion de l'API (voir plus bas) |
| Notifications, son, clignotement, tray | Par le cœur | Un événement, rendu par le client ; le cœur ne notifie plus rien lui-même | Fait : `server_alert` sur le poste, le nom de l'espace en tête, le clic rendu au serveur (`host_notification_click`) |
| Voix (micro, Whisper, Piper, voix en ligne) | Micro et haut-parleurs de la machine du cœur | Sur le poste : le module voix tourne dans le client et envoie le texte ; la clé d'une voix en ligne reste au trousseau du poste | Fait : ses commandes étaient du poste (`desk`) ; ses réglages sont maintenant ceux du poste (`local_voice_get`, `local_voice_set`) |
| Connexion d'un compte | CLI lancé dans un PTY, navigateur ouvert sur la même machine | L'adresse de connexion s'ouvre sur le poste ; le retour par `localhost` relayé, sinon un code (S13) | Fait : la page ouverte au clic, le retour relayé sur le même port, sinon le code ; harness par harness plus bas |
| Secrets (clés de Cursor, des intégrations, des voix) | Trousseau (Windows, macOS) | Un fichier lisible du seul propriétaire, dans les données du serveur (pas de Secret Service sur un VPS), et l'interface le dit | Fait : « Gardée dans un fichier sur Travail » là où un secret se donne ; les clés de la voix restent au poste |
| Mise à jour | L'app se met à jour | Le client et le serveur, chacun le sien ; le serveur avec la même signature que l'updater ; le client propose de mettre le serveur à sa version | Pas ici : la mise à jour signée du serveur est d'ESC-515 (`escouade-server update`, `serverUpdate` dans le refus de version) ; l'écran « Mettre à jour le serveur » du client reste à faire |
| Serveur MCP | 127.0.0.1 du poste | 127.0.0.1 du serveur : les agents du serveur l'atteignent ; un Claude Code du poste qui pilote Escouade passera par le tunnel (plus tard) | Rien à faire : il marche tel quel sur le serveur ; le pilotage depuis le poste reste à faire |
| WSL | Windows | Sans objet | Fait : sur un serveur, Réglages › Avancé ne montre que bash |

### Ce qui marche tel quel

- Les agents et leurs harness, installés sur le serveur.
- Git et les worktrees.
- Le Kanban et l'autopilote.
- Les terminaux et les vues TUI : le PTY tourne sur le serveur, xterm rend sur le poste.
- L'éditeur et les recherches.
- Les comptes et les quotas.
- Le remote control de claude.ai, plus utile encore H24 : une session reste joignable depuis l'app Claude.

### Points durs

- **Les connexions sans navigateur.** Pour chaque harness, il faut voir ce que fait son retour par `localhost` quand le navigateur est sur une autre machine. C'est à essayer avant tout.
- **Les identifiants des comptes vivent sur le VPS.** Sa sécurité devient celle des comptes : disque, accès SSH, sauvegardes. À dire à l'installation.
- **Les droits des agents.**
  - Sur un VPS jetable, le mode sans permission est moins risqué que sur le PC.
  - Mais un VPS garde souvent des secrets d'hébergement (clés d'un cloud). Les fichiers privés (`fence.rs`) s'appliquent de la même façon.
- **Redémarrer le serveur coupe les tours en cours.** Une mise à jour attend la fin des tours, ou le propose.
- **Un serveur, un utilisateur.** Les comptes sont les siens. Plusieurs appareils du même utilisateur, oui ; deux personnes sur le même serveur, hors champ.
- **Un serveur sans affichage ni son.** Il faut vérifier que rien du cœur ne suppose un bureau :
  - `single-instance` passe par D-Bus, d'où un verrou par fichier à la place ;
  - cpal ;
  - GTK du presse-papiers.
- **Les tests.** Un e2e lance `escouade-server`, puis le client par une redirection locale : sans SSH réel en CI, et avec un `ssh` du système vers `localhost` en option.

### Le mode serveur, MS-1 (ESC-514, 2026-11-17) : `escouade-server`

La spec le décrit (« Escouade à distance › Le mode serveur »). Le client qui s'y connecte (« Se connecter à un serveur », le tunnel SSH), le service systemd, l'image Docker et la mise à jour signée du serveur sont d'autres tickets.

- **Le cœur sans Tauri** (`docs/CRATES.md`, § 14, et sa décision) : aucun fichier déplacé. Le crate de l'app a une feature `desktop` (par défaut) qui porte Tauri, ses plugins, GTK, WebKitGTK et le reste du bureau ; sans elle, `crates/escouade-tauri-shim` tient lieu de Tauri sous son nom (`async_runtime`, `ipc::Channel`, les corps de l'IPC, `http`, `Url`), et ce qui ne sert que la fenêtre n'est pas compilé (`#[cfg(feature = "desktop")]`). Les commandes refusées à distance par leur classe (`desk`, `moves_refused`) n'existent pas dans le serveur : sa table de l'API les refuse (`remoteapi::refused_here`).
- **Le binaire** (`src-tauri/src/bin/escouade-server.rs`, `src-tauri/src/server/`) : `cargo build -p escouade --bin escouade-server --no-default-features`. Il ne lie que la libc, libm, libgcc_s et libstdc++ (whisper.cpp et eSpeak NG, liés en statique) : ni WebKit, ni GTK, ni D-Bus, ni ALSA (le micro et les haut-parleurs, cpal, sont la feature `devices` de la voix, que seul le bureau prend), ni X11 ou Wayland. Vérifié par `scripts/server-libs.sh` (`ldd`) et par `scripts/server-trial.sh` dans un conteneur `debian:trixie-slim` nu, sans affichage : il démarre, sert l'API sur 127.0.0.1 seulement, fait un code d'appairage, refuse un second serveur sur son dossier et s'arrête à SIGTERM avec le code 0, son état écrit. Le job « Server (Linux x64 / arm64) » de la CI fait les deux (runners `ubuntu-24.04` et `ubuntu-24.04-arm`, sans WebKitGTK ni GTK installés). Sous Windows, il se construit avec `desktop` pour l'e2e. Construit sur Ubuntu 24.04, il demande la glibc 2.39 (Debian 13) : une release pour des systèmes plus anciens se construira sur une base plus ancienne.
- **Ce qu'il fait** (`server::serve`) : le verrou du dossier de données d'abord (`headless::InstanceLock` ; « Escouade tourne déjà sur ce dossier de données (…) : un seul à la fois. »), les groupes de process laissés par un arrêt brutal, le cœur sans fenêtre sur un hôte qui garde ses secrets dans un fichier (`Headless::start_locked`), l'API sur 127.0.0.1:46480 (`--port`), puis ce que l'app fait une fois son cœur levé (la migration des comptes, `claude` de WSL réchauffé, les modèles des comptes, la mise à jour de Claude Code). À SIGTERM ou SIGINT : l'API fermée, `Core::shutdown` (les tours coupés, les process et les plugins arrêtés, l'état écrit), le dossier lâché.
- **Ses commandes** : `run` (par défaut), `pair`, `devices`, `devices revoke <id>`, `--data-dir`, `--port`, `--listen <adresse>` (une autre interface que 127.0.0.1, pour Docker ou un proxy : il avertit, rien n'est ouvert par défaut), `--debug`, `--version`, `--help` (partout : `pair --help` ne fait pas de code). Elles parlent la langue des réglages du dossier, sinon celle du système.
- **L'appairage** (`remoteapi::pairing`) : `pair` écrit `remote-pairing.json` (0600, l'empreinte SHA-256 du code, sa fin, les essais restants), sous le verrou d'un fichier `.lock` à côté (`headless::with_file_lock` : le serveur et `pair` sont deux process) ; un code de 8 lettres sans 0, O, 1 ni I (40 bits), valable 10 minutes, une fois, 5 essais faux au plus. Le client l'envoie à la place de la poignée de main (`{"t":"pair","code","name"}`) ; le serveur répond `{"t":"paired","device","token"}` puis ferme, ou refuse (`pairing`, compté comme une poignée de main ratée). Comparé en temps constant ; ni le code ni le jeton dans le journal.
- **Les appareils entre deux process** (`remoteapi::devices`) : chaque changement de `remote-devices.json` (ajout, révocation, dernière connexion) relit le fichier sous son verrou, puis l'écrit ; le serveur le relit quand il a changé (à chaque poignée de main, et toutes les 2 s : un appareil révoqué par `escouade-server devices revoke` est déconnecté).
- **Les secrets** (`server::secrets::FileStore`) : le magasin en fichier d'ESC-499 n'est pas encore dans `release/2.3` (parti de `main`) ; ce ticket en fait un au plus simple, à réunir avec lui quand il sera reversé : `secrets.json` (0600, remis à 0600 au démarrage s'il ne l'est plus), relu quand un autre process l'a changé, écrit sous son verrou. Il garde tout ce que le cœur garde au trousseau sur un poste : les jetons des appareils, les clés des intégrations, de Cursor, des voix en ligne et des plugins. Le `welcome` porte `secretsInFile` (`Host::secrets_in_file`) ; la fenêtre le lit (`connection.secretsInFile`), l'interface qui le montre vient avec le client.
- **Le journal** (`server::logs`) : chaque ligne sur la sortie standard (systemd, Docker) et dans `server.log` du dossier de données, qui tourne à 5 Mo (trois anciens gardés).
- **La fenêtre sur un autre cœur** (`remoteapi::window`, build de debug) : `ESCOUADE_REMOTE_URL` (une WebSocket de 127.0.0.1) et `ESCOUADE_REMOTE_API_TOKEN` mettent la fenêtre sur l'API d'un `escouade-server` ; le cœur de l'app ne sert alors rien, les commandes du poste restent les siennes.
- **Les essais** : Rust (`server_tests` : la ligne de commande, l'appairage et ses refus (expiré, redonné, faux, trop d'essais, un nouveau code qui remplace l'ancien), les secrets en fichier et leurs droits, le verrou d'un fichier, l'appairage par la WebSocket puis la connexion par le jeton et la révocation par un autre process, le journal sans code ni jeton, l'arrêt propre et son état écrit, le refus d'un dossier tenu) ; Vitest (`secretsInFile`) ; e2e (`server.spec.ts`, `tests/e2e/server.ts` : `escouade-server` sur un dossier neuf, `pair`, une redirection locale en Node à la place de `ssh -L`, l'appairage, la fenêtre sur son API ; un projet, un agent du faux `claude` et sa réponse, tout sur le serveur ; le client fermé puis rouvert retrouve l'agent et lui parle encore).
- **Pas encore** : le service systemd et sa commande d'installation ; l'image Docker ; le tunnel et « Se connecter à un serveur » ; l'affichage de `secretsInFile` ; la release du binaire et sa mise à jour signée ; les équipes sur un serveur (`teams` n'est pas ouvert par `escouade-server`). Les quatre premiers points sauf le client : MS-3, ci-dessous.

### Le mode serveur, MS-3 (ESC-515, 2026-11-18) : la distribution

La page du site « Mode serveur » (`website/content/docs/*/server.md`) est le guide de l'utilisateur ; `deploy/README.md` dit les fichiers.

- **Les options en variables** (`server::parse_with`) : `ESCOUADE_SERVER_DATA`, `ESCOUADE_SERVER_PORT`, `ESCOUADE_SERVER_LISTEN`, `ESCOUADE_SERVER_ALLOW_HOSTS` ; une option donnée l'emporte. C'est ce qui fait que `docker exec <conteneur> escouade-server pair` trouve le dossier du serveur. `--allow-host <nom>` (`RemoteApi::allow_hosts`) ajoute un nom que le contrôle de `Host` accepte en plus du bouclage, pour un proxy TLS ; tout autre nom reste refusé.
- **Ce que le serveur dit de lui** (`server::status`) : `server-status.json` du dossier, réécrit toutes les 5 s et dès que change ce qui ferait attendre un redémarrage (`updates::busy`, comme le redémarrage automatique de l'app : un tour, une question, une tâche de fond, un lancement, un terminal servi depuis moins de 5 min…), retiré à l'arrêt propre ; son pid, sa version, l'adresse de l'API, son superviseur (`INVOCATION_ID` : systemd ; `ESCOUADE_SERVER_IMAGE` : l'image). `escouade-server health` (le `HEALTHCHECK`) vérifie qu'il est frais (30 s) et que le processus vit. Au démarrage sans appareil appairé, le journal dit comment appairer (`server::first_start_hint`), jamais le code.
- **La mise à jour** (`server::update`) : `escouade-server update [--to X.Y.Z] [--beta] [--now] [--check]`. La version de `latest.json` (ou `beta.json`), l'archive `escouade-server-<v>-linux-<x64|arm64>.tar.gz` et sa `.sig` (le format de l'updater de Tauri : un fichier minisign en base64), vérifiée avec la clé publique de l'updater (`UPDATER_PUBLIC_KEY`, la même que `tauri.conf.json` : un test le tient) avant de rien toucher ; chaque fichier de l'archive posé à la place du sien à côté du programme, l'ancien gardé en `.old` ; puis le serveur arrêté par SIGTERM une fois au repos (ou tout de suite avec `--now`, ou `m` à la question dans un terminal), et systemd (`Restart=always`) le relance. Dans l'image, il refuse : la mise à jour est une nouvelle image. Une autre adresse des releases par `ESCOUADE_SERVER_UPDATE_URL` (la signature est vérifiée pareil) ; une autre clé par `ESCOUADE_SERVER_UPDATE_PUBKEY` dans un build de développement seulement.
- **Le fait exposé au client** : le refus `version` porte `serverUpdate` quand c'est le serveur à mettre à jour (`proto::server_update` : `{how: "command", command: "escouade-server update --to <version du client>"}`, ou `{how: "image", command: "docker pull ghcr.io/…/escouade-server:<version> && docker compose up -d"}`) ; le client Rust d’ESC-537 le porte dans son `Problem::Version` (`serverUpdate`), et la fenêtre dans `connection.refusal.serverUpdate` (`transport.ts`, `serverUpdateOf`). L’écran « Mettre à jour le serveur » qui s’en sert reste à faire.
- **L'image** (`deploy/Dockerfile`) : Debian 13 mince, le programme et l'amorce des plugins dans `/opt/escouade-server`, `git`, `openssh-client`, `curl`, `ca-certificates`, `tini` (PID 1), Node.js 22 et npm (copiés de l'image officielle `node:22-trixie-slim`), l'utilisateur `escouade` (uid 10001), sans sudo. `HOME=/data` : les données (`/data/.escouade`), les projets, les harness (`~/.local/bin` pour l'installeur de Claude Code, `NPM_CONFIG_PREFIX=/data/.npm-global` pour `npm install -g`) et leurs connexions sont dans le volume ; elle tourne `--read-only --tmpfs /tmp --cap-drop ALL --security-opt no-new-privileges`. Node est dans l'image plutôt que dans le volume : les harness de npm (Codex, Copilot CLI, Gemini CLI, OpenCode) le veulent, une nouvelle image le met à jour, et le volume ne garde que ce qui est à l'utilisateur ; Claude Code, lui, s'installe par son installeur natif, sans Node. Elle pèse 427 Mo décompressée, ≈ 150 Mo compressée (Debian et ses paquets ≈ 195, Node et npm ≈ 138, le programme et l’amorce ≈ 95). Sans `ESCOUADE_SERVER_LISTEN`, l'API n'écoute que sur 127.0.0.1 du conteneur ; les exemples mettent `0.0.0.0` et publient le port sur 127.0.0.1 de l'hôte seulement.
- **Mesures** (2026-11-18, l'image x64 sous podman) : au repos, les plugins chargés, ≈ 235 Mo et 106 tâches (processus et threads, ce que compte `pids_limit`) ; un agent du faux `claude` en plein tour (« flood 40s ») : ≈ 252 Mo et 113 tâches. Le vrai Claude Code pèse plus (non mesuré ici : il faut un compte). Conseillé : `mem_limit: 4g`, `pids_limit: 2048`, à monter avec le nombre d'agents en parallèle.
- **`deploy/`** : `docker-compose.yml` (le port sur 127.0.0.1, lecture seule, aucune capacité, `stop_grace_period: 60s`, les limites), `coolify/docker-compose.yml` (le même ; le domaine derrière Traefik par `SERVICE_FQDN_ESCOUADE_46480` et `ESCOUADE_SERVER_ALLOW_HOSTS=${SERVICE_FQDN_ESCOUADE}`, en commentaire, avec ses risques ; pas essayé sur un vrai Coolify), `install.sh`.
- **`install.sh`** : l'archive de la release (ou `--version`), sa SHA-256, sa signature (avec `minisign`, sinon `openssl` 3 : Ed25519 sur le BLAKE2b-512 du fichier, puis la signature globale du commentaire de confiance) ; le programme dans `~/.local/share/escouade-server`, un lien dans `~/.local/bin` (ou `/usr/local/bin` avec `--system`) ; le service de l'utilisateur (`systemctl --user`, `loginctl enable-linger`) ou du système (`--system`, l'utilisateur `escouade` créé, le programme à lui pour que `update` lancé par lui le remplace) ; `KillMode=mixed`, `Restart=always`, `TimeoutStopSec=90`, `UMask=0077`, `NoNewPrivileges` (et `ProtectSystem=full`, `PrivateTmp` pour le système). Relancé, il ne change que ce qui diffère. `--uninstall` garde les données, `--purge` les retire (avec `--system`, l'utilisateur et son dossier).
- **La release** (`release.yml`) : le job `server` (x64 sur `ubuntu-22.04`, arm64 sur `ubuntu-22.04-arm`, la glibc 2.35) construit en release, vérifie les bibliothèques et l'essai dans Debian nu, puis `scripts/server-release.sh` : l'archive (`scripts/server-package.sh`, le programme, l'amorce, la licence, des octets stables), sa SHA-256, sa signature par le CLI de Tauri avec la clé de l'updater, vérifiée aussitôt (`scripts/minisign-verify.mjs`), et l'image de l'architecture poussée par son empreinte seule. Les fichiers vont au brouillon (avec `install.sh`) ; `publish` attend ce job. Après la publication, `server-image` met les deux empreintes sous un index étiqueté `<version>` et `latest` (`beta` pour une bêta), atteste sa provenance (`attest-build-provenance`, poussée au registre) et la vérifie (`gh attestation verify oci://…`). Le job `attest` atteste les archives et `install.sh` avec le reste.
- **L'essai à blanc** (aucune release ni image publiée) : `scripts/server-release.sh` lancé ici pour x64 avec une clé jetable (`RELEASE_PUBKEY`) et sans `PUSH` : l'archive, sa signature vérifiée, l'image construite (podman, au format Docker : celui d'OCI perd le `HEALTHCHECK`). arm64 : le binaire croisé dans `rust:1-bookworm`, son archive signée et vérifiée, et `server-trial.sh` déroulé sous `qemu-aarch64` ; l'image arm64 n'a pas été construite ici (il faut l'émulation de binfmt dans la machine podman) : le job de la CI sur le runner arm64 la construit et l'essaie. Le chemin de publication (poussée au registre, index, attestation) ne se prouve qu'au premier tag : il n'utilise que le `docker` et le `gh` du runner.
- **Les essais** : Rust (`server_tests` : les variables et les options, `health` et `update` et leurs refus, les adresses des fichiers, la clé de l'updater partout, une signature qui ne vaut que pour son fichier et sa clé, l'archive posée fichier par fichier, le refus d'une archive sans programme ou qui sort de son dossier, l'état écrit puis retiré et le journal d'un nouveau serveur, le `Host` d'un proxy accepté et les autres refusés, la commande de mise à jour donnée au client ; `remoteapi_tests` : `serverUpdate`) ; Vitest (`transport.test.ts` : `connection.refusal.serverUpdate` ; `scripts/server-deploy.test.ts` : les jobs de la release, l'image et les exemples qui n'ouvrent rien, la clé partout, aucun exemple qui publie l'API ailleurs que sur 127.0.0.1) ; `scripts/server-image-test.sh` (l'image sur un volume neuf, lecture seule, sans capacité : santé, uid, `/data`, l'API sur 127.0.0.1, le journal, l'appairage par `exec` et un client du protocole (`scripts/server-image-client.mjs`), `--harness` : Claude Code installé dans le volume, `--measure`, l'arrêt propre, le redémarrage sur le même volume, `update` refusé) ; `scripts/server-install-test.sh` (systemd : `--system`, idempotent, une archive changée refusée, `update` vérifié puis relancé par systemd, `openssl` seul, `--uninstall`, `--purge`, le service d'un utilisateur). Le job « Server (Linux x64 / arm64) » de la CI lance les deux scripts.

### Le client, MS-3 (ESC-537, 2026-11-18) : « Se connecter à un serveur »

La spec le décrit (« Escouade à distance › Se connecter à un serveur ») ; le parcours et les états viennent de [`SERVER-UX.md`](SERVER-UX.md) (§ 2, 3, 4), la direction est B (« les piles », S1), un serveur à la fois.

- **Les serveurs connus** (`src-tauri/src/servers/`, du bureau seulement) : `servers.json` dans les données de l'app (le nom de l'espace, l'hôte SSH, le port d'`escouade-server`, le nom de ce poste là-bas, la version et la machine vues, le dernier état), jamais de jeton. Le jeton : au trousseau du poste, une entrée par serveur (`server-token-<id>`), ou, sans trousseau (Linux), dans `servers-tokens.json` lisible du seul propriétaire ; jamais dans les réglages ni le journal.
- **`~/.ssh/config`** (`servers::sshconfig`) : les `Host` (plusieurs noms par ligne), `HostName`, `Port`, `User`, `ProxyJump`, `Include` (relatif à `~/.ssh`, avec `*` et `?`, 16 niveaux), la première valeur gagne, `Match` sauté ; sont proposés les noms sans motif qui ont un `HostName`. Rien d'autre n'est lu : ni clé, ni `known_hosts`.
- **Le tunnel** (`servers::tunnel`) : le `ssh` du système (sous Windows `System32\OpenSSH\ssh.exe`, sans console), `ssh -N -v -L 127.0.0.1:<port libre>:127.0.0.1:<port du serveur> <hôte>` avec `BatchMode=yes`, `StrictHostKeyChecking=yes` (sur la ligne de commande, il l'emporte sur la config : jamais une clé acceptée à la place de l'utilisateur), `ExitOnForwardFailure=yes`, `ServerAliveInterval=15`, `ControlMaster=no`, `ConnectTimeout=15`. Prêt quand `ssh` le dit (« Local forwarding listening on ») : rien n'est envoyé avant sur le port local. Un hôte tapé qui commencerait par `-` ou contiendrait un caractère de shell est refusé. Sa sortie est lue et classée : injoignable, clé inconnue ou changée, compte refusé, redirection interdite, pas d'Escouade (« open failed »), port local pris (un autre est pris), pas de `ssh`.
- **La surveillance** : le tunnel du serveur où est la fenêtre est relancé aux délais de la fenêtre (250 ms … 5 s), sur le même port local s'il est libre ; une fin que réessayer ne répare pas (une clé, un compte refusé) l'arrête et le dit. « Réessayer » relance aussitôt. Il s'arrête avec l'app.
- **La poignée de main** (`servers::client`) : sans jeton, pour savoir qu'Escouade répond et sa version (le refus `token` porte maintenant `serverVersion`) ; l'appairage par le code (tapé, ou « le lancer d'ici par SSH » : `escouade-server pair` lancé par le même hôte, cherché dans le `PATH`, `~/.local/bin`, `/usr/local/bin`, `/opt/escouade`, le code lu dans sa sortie) ; puis avec le jeton.
- **Ce que le serveur donne de plus** (§ 9) : `machine` dans le `welcome` (son nom et le compte du service), `ping`/`pong` (la latence de la barre d'état, et une connexion muette depuis 20 s dite coupée), la raison d'un code refusé (`reason` : `none`, `expired`, `wrong`, `tries`), une trace de chaque appareil révoqué (`remote-revoked.json` : son nom, quand, par qui, l'empreinte de son jeton, jamais le jeton ; les 32 derniers) rendue dans le refus `token` (`revoked`), et `unpair` : un appareil qui oublie le serveur se fait révoquer (S15).
- **La fenêtre** (`src/lib/transport.ts`) : au démarrage, elle demande au cœur du poste où se connecter (`server_link`) ; sur un serveur, elle parle à son API par le tunnel, et redemande le lien avant chaque essai. Les commandes des serveurs (`server_*`, `servers_*`) sont du poste (`desk`) : la fenêtre les fait toujours par son IPC. Passer d'une machine à l'autre recharge la fenêtre sur l'autre cœur ; l'app rouvre sur la dernière.
- **La CSP** de la fenêtre permet `ws://127.0.0.1:*`, et rien d'autre en WebSocket : le tunnel n'écoute que là.
- **Ce que voit l'utilisateur** : le dialogue (l'hôte, la connexion, l'appairage, l'espace), un état par échec, la machine dans la barre de titre (dès qu'un serveur est connu), la barre d'état (tunnel et latence, ou la reprise en ambre), le pied de la barre latérale, l'en-tête d'un terminal, « Nouveau projet » (sur un serveur, le sélecteur de dossier du poste disparaît : le chemin se tape, le navigateur des dossiers du serveur vient avec MS-4), l'envoi suspendu pendant une coupure (S10), l'écran d'un jeton révoqué ou d'une autre version, l'espace dans le rail (B), Réglages › Serveurs.
- **Les essais** : Rust (`servers_tests` : `~/.ssh/config` avec `Include` et `ProxyJump`, un hôte tapé, les options de `ssh`, la lecture de sa sortie, le port libre, chaque état contre un vrai `escouade-server` derrière le faux `ssh`, la coupure et la reprise sur le même port, l'appairage par code et par SSH, la révocation datée, l'oubli révoqué sur le serveur, le ping ; `locks_policies_tests` : `servers.allowed`) ; Vitest (`servers.test.ts`, `transport.test.ts`) ; e2e (`server-connect.spec.ts` : `escouade-server` derrière le faux `ssh` de `tests/fixtures/fake-ssh.mjs`, aux deux tailles).
- **Pas encore** : ce qui change de machine (MS-4 : le navigateur des dossiers du serveur…), la mise à jour du serveur depuis le client (le dialogue de version dit la commande), la palette de commandes (il n'y en a pas : l'entrée est dans le menu du « + » dès qu'un serveur est connu ou qu'un projet a été fermé, et dans Réglages › Serveurs), le menu du tray.

### Les espaces, MS-5 (ESC-539, 2026-11-18) : plusieurs serveurs dans un même Escouade

La spec le décrit (« Escouade à distance › Les espaces ») ; la conception vient de [`SERVER-UX.md`](SERVER-UX.md) (§ 5, 6, 7), la direction est B (« les piles », S1), avec « Partout » d'où l'on répond sans changer d'espace (S8 et S9 suivent B).

- **Un tunnel par serveur** (`servers::Servers`, `live` par serveur) : chaque serveur connu a le sien, ouvert au démarrage (`resume` → `ensure`, sauf s'il est refusé par la stratégie ou n'a plus de jeton), tenu avec ses propres essais ; aller sur ce poste ne le ferme pas, « Retirer » le ferme. `open` passe par le tunnel déjà tenu (une poignée de main avec le jeton), sinon en ouvre un. `link_of(id)`, `links()` ; `server_retry` d'un espace ne relance que son tunnel.
- **La fenêtre lit les autres espaces** (`src/lib/spaces.svelte.ts`) : une `RemoteTransport` par serveur, chacune son état (`connectionOf`) et sa reprise, ou l'IPC de ce poste quand la fenêtre est sur un serveur ; toutes les 3 s, `space_glance`.
- **`space_glance`** (servie, `src-tauri/src/glance.rs`) : ce qui attend (le nombre, les agents qui attendent avec leur première demande coupée à 400 caractères, ceux au travail, 40 au plus, les demandes de la boîte « À toi », 20 au plus, les projets nommés, les couleurs des trois premiers), jamais une conversation. `counts: true` (la stratégie de ce poste), ou `servers.aggregateView` à `false` sur le cœur lui-même : les nombres seuls. `contentless` dit `servers.contentlessNotifications` du cœur.
- **Les appareils** (servies) : `remote_devices`, `remote_device_revoke` (le nom de l'appareil qui le demande gardé dans la trace ; l'API ferme ses connexions à sa relecture, 2 s), `remote_pair_code` (S5 : un code de 10 minutes créé depuis un client appairé).
- **Un espace dans sa fenêtre** (`servers::window`, `space_window`, du bureau) : une `WebviewWindow` `space-<id>` (`space-here` pour ce poste) sur `index.html?space=<id>` ; son transport demande `server_link { space }`. Une par espace (S14) ; la capacité `default` couvre `space-*` ; la fermer la ferme (la principale va seule au tray).
- **Les notifications** : une demande nouvelle d'un autre serveur, vue par sa lecture, est notifiée par `space_notify` (du bureau : les réglages de ce poste, le son, le toast, un clic qui émet `space-focus` à la fenêtre principale) ; celles du cœur de ce poste disent « Ce poste · … » dès qu'un serveur est connu (`TauriHost::notify`).
- **Les stratégies** (`escouade-policy`, plus « à venir ») : `servers.personalSpace` à `false` refuse tout serveur hors de `servers.allowed` (aucun sans liste) ; `servers.aggregateView`, `servers.contentlessNotifications` s'appliquent aux serveurs de travail (de `servers.allowed`, ou tous sans liste). `servers_view` porte les verrous de ce poste, que la fenêtre lit même sur un serveur.
- **La mémoire de trois connexions** : mesurée par l'e2e `spaces.spec.ts` (le processus de l'app et le tas JS de la fenêtre, un espace contre trois), notée au bilan du ticket.
- **Les essais** : Rust (`servers_tests` : deux `escouade-server` derrière le faux `ssh`, chacun son tunnel, la coupure de l'un sans l'autre — `down-<hôte>` et `cut-<hôte>` du faux `ssh` —, sa reprise seule, l'oubli qui ne ferme que le sien ; `glance_tests` ; `locks_policies_tests` : `servers.personalSpace`) ; Vitest (`spaces.test.ts`) ; e2e (`spaces.spec.ts`, `test.use({ sshServer: 2 })`, aux deux tailles).

### Ce qui change de machine, MS-4 (ESC-538, 2026-11-18)

La spec le décrit (« Escouade à distance › Ce qui change de machine »), avec les pages Nouveau projet, Conversation (les fichiers envoyés), Comptes et Notifications. Les choix S12 et S13 sont ceux par défaut de [`SERVER-UX.md`](SERVER-UX.md) § 7. Sur ce poste, rien ne change.

- **Les dossiers du serveur** (`src-tauri/src/serverfolders.rs`, servi) : `fs_browse` liste un dossier (des dossiers seulement, les cachés laissés, 500 au plus et le reste compté, un chemin absolu) : dépôt et branche (lue dans `.git/HEAD`, un worktree compris), déjà un projet, la place du disque (`statvfs`, `GetDiskFreeSpaceExW`) ; il s'ouvre sur `~/projets` (ou `projects`, `dev`, `src`, `code`). `clone_repo` clone une adresse https ou ssh vérifiée comme celles d'une équipe (`escouade_team::catalog::address`) par le git du serveur (`team_projects::clone_one`). La fenêtre : `ServerFolders.svelte`, `CloneRepo.svelte` dans « Nouveau projet ».
- **Les fichiers envoyés** : le logo en octets (`set_project_logo_bytes`, vérifié par `projectlogo::checked`) ; les fichiers joints toujours copiés (`file-origin.ts` ne donne aucune origine sur un serveur) ; le presse-papiers du poste (`local('clipboard_image')`).
- **Les fichiers reçus** (S12) : `sent_file_bytes`, `agent_local_bytes` (servis, 512 Mo au plus) ; sur le poste `received_save` (`servers::received`) : `Téléchargements/Escouade/<espace>`, le type lu dans les octets (un exécutable PE, ELF ou Mach-O, un `#!` : un programme ; une image ou un PDF gardent leur extension), un nom de programme ou de script jamais ouvert, la copie jamais exécutable et marquée `Zone.Identifier` (zone 3) sous Windows, `received_show` ne rouvre qu'une copie de ce dossier.
- **Les ports relayés** : `server_relay` lance une redirection de plus (`ssh -N -L 127.0.0.1:<poste>:127.0.0.1:<serveur>`, les options sûres du tunnel), le même port s'il est libre (`same` l'exige, pour le retour d'une connexion), arrêtée en quittant le serveur. **Pas dans la connexion de l'API** comme l'écrivait « Le tunnel : SSH d'abord » : une redirection SSH de plus ne demande rien de nouveau au serveur et porte tout TCP (la WebSocket du rechargement à chaud comprise) ; un tunnel sans SSH (WireGuard, le téléphone) demandera ce relai dans l'API. La fenêtre réécrit l'adresse de bouclage du serveur (`onThisComputer`) dans `openUrl` et la page d'un lancement.
- **Les notifications** : `HostEvent::Notify` porte un `id` ; l'hôte sans fenêtre garde l'action de ses 64 dernières (`Host::clicked`, `host_notification_click`). La fenêtre rend chaque événement de l'hôte par `server_alert` sur le poste (pas de clignotement ni de notification si la fenêtre est regardée), le titre « <espace> · <titre> » ; un `open` n'est jamais suivi. Le `welcome` d'une session reprise porte `missed` (`Hub::emitted`, `Sessions::note_seen`) : « Reconnecté à Travail · 12 événements rattrapés ».
- **La voix** : ses commandes étaient déjà du poste ; ses réglages le sont maintenant aussi sur un serveur (`local_voice_get`, `local_voice_set`, `withPosteVoice`).
- **La connexion d'un compte** (S13, `ServerSignIn.svelte`) : l'adresse lue dans la sortie du terminal (`loginLink` : le `redirect_uri` qui vise `localhost`, le serveur local dit avant, un code d'appareil), ouverte au clic, le retour relayé sur le même port. D'après les flux de connexion de chaque harness, sans compte réel connecté :

  | Harness | Son retour | Sur un serveur |
  | --- | --- | --- |
  | Codex | `localhost:1455` (`redirect_uri`) | Relayé |
  | Gemini CLI, Antigravity CLI | `127.0.0.1:<port>` (`redirect_uri`) | Relayé |
  | Claude Code | `localhost` quand son navigateur s'ouvre, sinon une page de secours qui affiche un code | Le code à coller |
  | Copilot CLI | Code d'appareil de GitHub | Le code tapé sur la page |
  | Cursor CLI | La page, le CLI attend le lien du compte | La page suffit (ou la clé) |
  | OpenCode | Selon le fournisseur | Selon le fournisseur |
  | Mistral Vibe | Une clé | La clé, en fichier sur le serveur |

  L'essai réel (un harness connecté sur le VPS d'essai, le navigateur sur le poste) reste à faire avec l'utilisateur.
- **Les secrets** : `SecretsOnServer.svelte` (« Gardée dans un fichier sur Travail ») dans la fenêtre d'un compte à clé, les services connectés et les secrets des plugins.
- **« Afficher dans l'Explorateur »** : `openProjectFolder` ouvre le panneau Fichiers sur un serveur ; `openPath` refuse un chemin du serveur ; les boutons des dossiers des données sont cachés.
- **Les essais** : Rust (`server_moves_tests` : la liste des dossiers, ses bornes, le clone et ses refus, le logo en octets, les fichiers reçus et leur type, le clic d'une notification, les événements manqués ; `servers_tests` : un port relayé, le même port ou rien) ; Vitest (`server-moves.test.ts`, `servers/moves.test.ts`) ; e2e (`server-moves.spec.ts`, sur `escouade-server` derrière le faux `ssh`, aux deux tailles : un projet dans un dossier du serveur, un fichier joint, un fichier reçu et un script refusé, un port relayé, une notification nommée, la connexion d'un faux `codex login` relayée). Les crochets des e2e, en build de développement seulement : `ESCOUADE_BROWSE_HOME`, `ESCOUADE_DOWNLOADS_DIR`, `ESCOUADE_OPEN_LOG`, `ESCOUADE_ALERT_LOG`.
- **Pas encore** : l'écran « Mettre à jour le serveur » du client (la mise à jour signée est d'ESC-515) ; les serveurs de langage de l'éditeur à distance ; un Claude Code du poste qui pilote le serveur MCP du serveur ; les notifications sans contenu d'une stratégie (`servers.contentlessNotifications`, avec les espaces, MS-5).

## Le téléphone

Il repose sur la même fondation : l'API distante du cœur, servie dans un tunnel. Il vise le PC ou un serveur. Le tout se fait sans service tiers ni abonnement, avec une configuration réduite à une redirection de port dans la box et un QR code à scanner.

### Le tunnel côté cœur, MO-2 (ESC-567, 2026-11-21)

La spec le décrit (« Escouade à distance › Le tunnel du téléphone »). Le côté téléphone (l'app, sa clé, le QR code de l'appairage) est MO-3 et MO-4 (`MOBILE.md` § 8).

- **Le pair WireGuard** (`src-tauri/src/phone/stack.rs`) : `boringtun` (le protocole seul, sans sa feature `device` : ni TUN ni pilote) sur une socket UDP du process, et `smoltcp` (une pile TCP/IP à Escouade, `medium-ip`, IPv4) dans le tunnel. Une seule tâche tient la socket, les sessions et la pile. Le cœur est `10.77.0.1/24`, chaque téléphone une adresse de `10.77.0.2` à `.254` (son `AllowedIPs`, la seule d'où ses paquets sont pris). Une poignée de main d'une clé inconnue est jetée sans réponse et comptée ; un datagramme qui n'est pas WireGuard aussi ; sous charge, les cookies de WireGuard (le `RateLimiter` de boringtun, partagé). Les octets d'une connexion attendent dans deux canaux bornés (32 morceaux de 16 Ko) : un lecteur lent retient l'écrivain par la fenêtre de TCP. 64 connexions à la fois.
- **L'API dans le tunnel seulement** : la pile écoute `10.77.0.1:80` et remet chaque connexion à l'API (`RemoteApi::serve_stream`, `Via::Tunnel`). Sur le poste, l'API n'a alors aucune socket à elle (`Listen::Nothing`) : elle n'est sur aucune adresse de la machine. Dans le tunnel, le `Host` doit être l'adresse du tunnel (celui du bouclage y est refusé), l'`Origin` celle de la fenêtre ou de Capacitor (`capacitor://localhost`, `https://localhost`, `http://localhost`, refusées sur le bouclage), et le jeton d'appareil reste demandé. Un pair appairé comme un appareil (un `pair` passé par lui) ne prend plus que le jeton de cet appareil ; l'appareil révoqué emporte son pair.
- **Ce qui est gardé** (`phone.json` dans le dossier des données, lisible du seul propriétaire) : allumé ou non, le port (51820 par défaut), Tailscale ou non, les pairs (nom, clé publique, adresse, appareil), l'adresse publique vue et quand elle a changé. La clé privée du tunnel est au trousseau (`phone-tunnel-key`), dans le fichier des secrets sur un serveur, dans `phone.json` quand le trousseau la refuse.
- **La box** (`phone/nat.rs`) : l'adresse de la machine sur son réseau (la route, sans rien envoyer), une IPv6 globale ; UPnP (`igd-next`) puis NAT-PMP (écrit à la main, RFC 6886 : la passerelle UPnP, sinon le `.1` du réseau) ouvrent le port UDP pour une heure, renouvelé toutes les cinq minutes avec le regard sur le réseau, retiré à l'extinction ; l'adresse de la box sur Internet (UPnP, NAT-PMP) ; l'adresse vue de dehors par STUN (`stun.cloudflare.com`, puis `stun.l.google.com` : une requête de liaison, rien d'autre ne part). Un CGNAT (ou une double box) quand l'adresse de la box est partagée (100.64.0.0/10) ou privée, ou n'est pas l'adresse publique ; une machine sur Internet elle-même (un VPS) quand l'adresse publique est la sienne. Jamais de demande à la box sur un serveur (son pare-feu).
- **L'adresse publique relue** toutes les cinq minutes : un changement est gardé et dit (« Changée le … (avant : …) »), pour les téléphones appairés avant.
- **Tailscale** (`phone/tailscale.rs`) : sa commande (`tailscale ip -4`, `status --json` pour le nom MagicDNS), là où il s'installe ou dans le `PATH`, sinon la route vers 100.100.100.100. Trouvé, et « Servir aussi sur l'adresse Tailscale » allumé (par défaut), l'API est servie en TCP sur l'adresse Tailscale de la machine et le port du tunnel, pour une machine du tailnet seulement (100.64.0.0/10, `fd7a:115c:a1e0::/48`), le jeton d'appareil demandé.
- **La stratégie** : `servers.phone` à `false` (liste unique d'`escouade-policy` : ADMX/ADML, profil macOS, schéma Linux), ou `servers.remoteApi` à `false`, interdisent le tunnel : refusé à l'allumage, éteint aussitôt quand la stratégie arrive (`apply_locks`) et revu toutes les deux secondes, rallumé quand elle part.
- **Réglages › Téléphone** (`PhonePage.svelte`) : l'accès, le port, l'état du tunnel et l'adresse de l'API dedans, la box (ouvert par UPnP ou NAT-PMP, ou le geste à faire avec l'adresse de la machine, ce que la box a répondu), un CGNAT et ses issues, l'IPv6, l'adresse publique et son changement, Tailscale, les téléphones (leur dernière poignée de main, « Retirer »), et sous « Avancé » la clé publique et « Changer la clé du tunnel ». La même page dans un espace serveur agit sur le cœur du serveur (les commandes sont servies) et dit son pare-feu au lieu d'une box.
- **`escouade-server phone`** : `status`, `on [PORT]`, `off`, `add NOM [--endpoint HÔTE]` (la config WireGuard du téléphone et son QR code dans le terminal, pour l'app WireGuard officielle, en attendant l'appairage de MO-4 : sa clé privée n'est gardée nulle part), `remove ID`. Le serveur qui tourne relit `phone.json` dans les deux secondes. Une page ouverte dans le tunnel qui n'est pas une WebSocket est dite dans son journal.
- **Les essais** : Rust (`phone_tests` : la poignée de main et l'API jointe dans le tunnel par un pair WireGuard de test écrit avec la même pile, rien hors du tunnel ni d'autre `Host` ou `Origin` dedans, une clé inconnue sans réponse, un pair retiré, la clé tournée, un pair lié à son appareil et emporté par sa révocation, la box et l'adresse publique relue par une fausse box NAT-PMP et un faux STUN sur le bouclage, la stratégie qui coupe, Tailscale, un port pris, 2 Mo dans chaque sens, `escouade-server phone add` repris par le cœur qui tourne) ; Vitest (`PhonePage.test.ts`) ; e2e (`phone.spec.ts`, avec la fausse box). Aucun essai n'atteint la box de l'utilisateur : les e2e ont `ESCOUADE_PHONE_NET=off` (ou une fausse box, `fake;natpmp=…;stun=…;local=…`), lu en build de développement seulement.
- **Pas encore** : l'essai réel en 4G (« L'ordre proposé », 1 : un geste de l'utilisateur, déposé dans « À toi ») ; le QR code de l'appairage et la sorte « téléphone » des appareils (MO-4) ; le côté téléphone (MO-3) ; un nom DynDNS donné à la place de l'adresse publique ; le pare-feu IPv6 de la box ouvert par UPnP (pinhole) ; empêcher la veille du poste tant que l'accès est allumé.

### Architecture retenue

Escouade embarque un pair WireGuard, et son serveur n'est jamais exposé à Internet : seul le port UDP de WireGuard l'est.

```
Téléphone (app WireGuard + app ou navigateur)
   │  UDP, chiffré par WireGuard
   ▼
Box  ── redirection du port UDP X ──▶  PC ou serveur : Escouade
                                         ├─ WireGuard en espace utilisateur (port X)
                                         ├─ pile réseau intégrée (adresse du tunnel, ex. 10.77.0.1)
                                         └─ API d'Escouade, joignable dans le tunnel seulement
```

- **WireGuard dans l'app, sans interface système.**
  - `boringtun` (ou `defguard_wireguard_rs`) pour le protocole, `smoltcp` pour une pile TCP/IP à l'intérieur d'Escouade.
  - Pas d'interface `wg0`, donc aucun droit administrateur :
    - pas de pilote Wintun sous Windows ;
    - pas de Network Extension sous macOS ;
    - pas de capacité réseau sous Linux.
  - Seul le trafic d'Escouade passe dans le tunnel.
- **Côté téléphone.** L'app WireGuard officielle (iOS, Android), configurée en scannant un QR code. Plus tard, peut-être, une app Escouade qui embarque WireGuard elle-même.
- **Pourquoi pas un port HTTP ouvert directement.**
  - Escouade lance des agents qui exécutent des commandes, ouvre des terminaux et écrit dans les dépôts. Un serveur HTTP exposé serait scanné en quelques minutes, et la moindre faille deviendrait une prise de contrôle de la machine.
  - Avec WireGuard, un paquet non signé par une clé connue est ignoré sans réponse. Un scan ne voit rien, et la surface exposée se réduit à une implémentation petite et très auditée.

### Configuration, pour l'utilisateur

1. Réglages › Téléphone › « Activer l'accès depuis le téléphone » : Escouade choisit un port UDP (modifiable), puis génère sa paire de clés et celle du téléphone.
2. Escouade dit quoi faire dans la box : rediriger le port UDP X vers l'adresse locale du PC, avec l'adresse et le port à recopier. Il essaie d'abord de l'ouvrir lui-même par UPnP ou NAT-PMP, et dit si ça a marché.
3. Escouade affiche un QR code qui contient la config WireGuard du téléphone :
   - sa clé privée ;
   - la clé publique d'Escouade ;
   - l'adresse publique et le port ;
   - `AllowedIPs` limité à l'adresse du tunnel ;
   - `PersistentKeepalive = 25`.
4. Sur le téléphone : app WireGuard › scanner le QR code › activer. L'interface d'Escouade s'ouvre à l'adresse du tunnel.

Plusieurs téléphones : un pair et un QR code par appareil, révocable un par un.

### Avec un serveur

Sur un VPS, les points durs du téléphone tombent :
- l'adresse publique est fixe ;
- il n'y a pas de CGNAT ;
- il n'y a pas de veille ;
- il n'y a aucun port de box à rediriger : le port UDP de WireGuard s'ouvre dans le pare-feu du VPS.

Le téléphone et le client de bureau visent alors la même machine, qui tourne H24.

### Points durs (sur le PC)

- **L'adresse publique.** Sans IP fixe, elle change. Pistes :
  - la relire régulièrement (un service « quelle est mon IP ») et prévenir quand le QR code est périmé ;
  - accepter un nom DynDNS saisi par l'utilisateur (DuckDNS, celui de la box).
- **CGNAT.**
  - Derrière une IPv4 partagée par l'opérateur, la redirection ne sert à rien.
  - Escouade doit le détecter : l'adresse de la box côté WAN n'est pas l'adresse publique vue de l'extérieur.
  - Il le dit, avec les issues : demander une IPv4 dédiée à l'opérateur, passer par IPv6, ou un relais (un VPS, justement).
- **IPv6.** Souvent disponible même derrière un CGNAT. Il faut alors ouvrir le port dans le pare-feu IPv6 de la box plutôt qu'une redirection.
- **Pare-feu du PC.** Windows demande l'autorisation au premier lancement ; macOS et Linux selon leur config.
- **Le PC en veille.** Rien ne répond. À dire dans l'interface ; une option peut empêcher la veille tant que l'accès est actif.
- **Authentification dans le tunnel.** Même dans le tunnel, l'interface demande un jeton d'appareil : c'est une défense en profondeur si une clé de téléphone fuit. Révoquer un téléphone retire son pair et son jeton.

### Ce que voit le téléphone

À décider :

- **Option A, l'interface web d'Escouade servie dans le tunnel.**
  - La même interface Svelte, sur l'API distante de la fondation, avec une mise en page mobile.
  - C'est le moins de code à écrire.
- **Option B, une app mobile dédiée.**
  - Plus agréable : notifications push, WireGuard intégré, pas d'app WireGuard à part.
  - Mais c'est une app iOS et Android à maintenir et à publier.

Proposition :
- A d'abord, centrée sur ce qui sert loin du PC :
  - la vue d'ensemble ;
  - les réponses aux autorisations et aux questions ;
  - l'envoi d'un message ;
  - le Kanban en lecture ;
  - l'ouverture du serveur de test d'un agent.
- Puis B si l'usage le justifie.

### Alternative sans WireGuard dans l'app

Si Tailscale (ou Netbird, Headscale) est installé sur la machine d'Escouade et sur le téléphone, Escouade peut simplement servir son interface sur l'adresse Tailscale de la machine. Il n'y a aucun port à ouvrir, et le perçage de NAT et les relais sont fournis, CGNAT compris. À proposer en plus de l'accès intégré, quand Escouade détecte Tailscale.

À ne pas faire : refaire Tailscale (perçage de NAT des deux côtés, relais hébergés).

## Ajouts du 2026-11-14 : l'app mobile et les agents sur le serveur

Demandé par l'utilisateur pour la 2.3, quand il a choisi la feuille de route du site :
- intégrer une vraie app mobile à cette étude ;
- voir comment les agents tournent côté serveur.

### Les agents sur le serveur, en auto-hébergé

C'est déjà le principe du mode serveur plus haut : les agents, leurs harness et leurs worktrees tournent sur la machine d'`escouade-server`. Le client de bureau et le téléphone n'en sont que des vues. En auto-hébergé, c'est gratuit : l'utilisateur fournit la machine.

À préciser dans les essais :
- **Les ressources** : la mémoire d'un agent et de son harness au repos et en tour, pour dimensionner un VPS (par exemple combien d'agents tiennent sur 4 Go). Le disque des worktrees.
- **La connexion des harness sur le serveur**, sans navigateur sur place : un code d'appareil, ou une adresse ouverte sur le poste. C'est déjà dans les tâches du mode serveur.
- **Avec le serveur d'équipe** (`docs/TODO-2.3.md`) : une machine en mode serveur est un appareil de l'équipe comme un autre. Elle détient les clés de l'équipe pour lire le Kanban, la spec et la mémoire de ses projets, et l'équipe peut la retirer comme on retire un membre.
- **Un réseau privé** (ESC-551, 2026-11-20) : des agents du serveur qui joignent un git, un registre ou une API derrière le VPN d'une entreprise ; l'étude est [`PRIVATE-NETWORK.md`](PRIVATE-NETWORK.md) (le serveur dans l'entreprise, ou `escouade-connector` et une sortie bornée à une liste d'hôtes), ses choix R1 à R12 restent à trancher.
- **L'isolement** : les agents d'un serveur partagé par plusieurs personnes tournent avec les droits du compte du service. Un utilisateur système par membre, ou un conteneur par projet, est à étudier avant d'ouvrir un serveur à plusieurs.

### L'app mobile

L'option B de « Ce que voit le téléphone » passe en tête. L'option A, l'interface web dans le tunnel, reste la première étape et le recours.

- **La technique, à choisir par une étude avec maquettes** (demande de l'utilisateur, qui a lu que Tauri 2 n'est pas au point sur mobile). Dans tous les cas, l'app est un client sur l'API distante de la fondation, sans cœur ni agent sur le téléphone. Les candidats :
  - **Tauri 2** : il reprend l'interface Svelte et le Rust sans pont. Mais ses plugins mobiles sont jeunes : le push passe par des plugins communautaires, avant leur 1.0. Le travail en arrière-plan et les outils de build sont moins mûrs. Il rend dans la WebView du système.
  - **Capacitor** : il reprend aussi l'interface Svelte, dans la même WebView. Ses plugins mobiles sont mûrs (push, biométrie, appareil photo pour le QR code). Le Rust passe par un plugin natif (Swift et Kotlin, par UniFFI).
  - **React Native** (avec Expo) : des vues natives, et un outillage mûr (notifications par Expo, builds et publication par EAS). Mais l'interface est à réécrire en React, et le Rust passe par un pont natif (UniFFI, par uniffi-bindgen-react-native).
  - **Flutter** : un rendu à lui, très régulier, et un pont Rust mûr (flutter_rust_bridge). Mais l'interface est à réécrire en Dart, un langage de plus pour l'équipe.
  - **L'étude est faite** (ESC-445, 2026-11-14) : `docs/MOBILE-STUDY.md`, avec les quatre maquettes de `mobile-study/` (branche `study/esc-445-ios`) et leurs mesures sous Android et iOS. Elle recommande **Capacitor** (l'interface Svelte reprise, le push et le QR code par des plugins officiels) et écarte Tauri 2 pour le téléphone ; la décision 5 reste à prendre.
- **Ce que l'étude mesure**, sur une même maquette (la liste des agents qui attendent, la réponse à une autorisation, une notification push qui réveille l'app fermée, le tunnel WireGuard en Rust) :
  - la fluidité d'une longue conversation (Markdown, blocs de code, diff) sur un Android d'entrée de gamme ;
  - le poids de l'app et le démarrage ;
  - le push sur les deux systèmes ;
  - le pont vers le Rust ;
  - la part de code reprise d'Escouade ;
  - l'effort de maintenance à deux systèmes.
- **Ce qu'elle vise** : le PC, un serveur auto-hébergé, ou plus tard une machine hébergée, choisis comme dans le client de bureau, une machine par vue.
- **WireGuard dans l'app.**
  - Le pair WireGuard d'Escouade (`boringtun` et `smoltcp`, en Rust) se compile pour iOS et Android : il n'y a plus besoin de l'app WireGuard à part.
  - L'appairage se fait par le même QR code.
  - Comme sur le PC, seul le trafic de l'app passe dans le tunnel. Il n'y a donc pas de VPN système, ni de Network Extension sous iOS.
- **Les notifications push**, quand l'app est fermée : un agent qui attend une réponse, un ticket en revue, une CI rouge.
  - APNs et FCM ne passent que par un serveur qui détient les identifiants de l'app : il faut un relais de notifications tenu par l'éditeur, même pour l'auto-hébergé.
  - Le contenu est chiffré de bout en bout : le relais ne voit qu'un réveil, et le téléphone déchiffre lui-même le texte de la notification.
  - Sans relais, c'est-à-dire si l'utilisateur le refuse, l'app ne sait rien tant qu'elle est fermée.
- **En arrière-plan** : iOS coupe vite les connexions d'une app en arrière-plan. Le push sert de réveil, et l'app se reconnecte quand on l'ouvre.
- **Sur l'appareil** :
  - les clés du pair et le jeton d'appareil sont dans le trousseau du système (Secure Enclave sous iOS, Keystore sous Android) ;
  - un verrou biométrique est possible avant de répondre à une autorisation ;
  - l'appareil est révocable depuis le PC ou le serveur.
- **Ce qu'on fait depuis le téléphone** : la vue d'ensemble, la conversation d'un agent, les réponses aux autorisations et aux questions (avec la commande entière affichée), l'envoi d'un message, le Kanban, la dictée, et l'aperçu du serveur de test d'un agent.
- **La publication** :
  - le compte développeur Apple existe déjà (signature macOS) ;
  - il faut un compte Google Play ;
  - la revue d'Apple s'applique à chaque version.

### Plusieurs serveurs pilotés par un seul Escouade

Demandé le 2026-11-14 : « avoir plusieurs serveurs Escouade pilotés par un seul Escouade (comme ça j'ai Escouade pour le taf, et en perso) ». Cela tranche dans un sens la décision 1 plus bas (une fenêtre par machine), et il faut l'étendre.

- **Des espaces, comme dans Slack.** Une colonne d'icônes à gauche, une par machine : le poste lui-même, le serveur du travail, le serveur perso.
  - Chaque icône porte le nombre d'agents qui attendent.
  - On passe de l'un à l'autre d'un clic ou par Ctrl+1, Ctrl+2.
  - Un espace peut aussi s'ouvrir dans sa propre fenêtre.
  - Le poste est un espace comme les autres, son cœur étant dans l'app.
- **Chaque espace est cloisonné** :
  - ses projets, ses agents, ses réglages, ses comptes, ses plugins et son serveur MCP sont les siens ;
  - rien ne passe de l'un à l'autre : ni un agent, ni une mémoire, ni un plugin, ni un secret ;
  - le client garde une connexion, un tunnel (SSH ou WireGuard) et un jeton d'appareil par serveur, rangés à part dans le trousseau.
- **Les notifications** arrivent de tous les espaces, avec le nom de l'espace en tête. Une vue « tout ce qui attend, partout » est possible, mais il faut décider ce qu'elle montre d'un espace de travail (voir plus bas).
- **Les règles d'une entreprise ne valent que pour son espace.** Celles du serveur d'équipe (`docs/TODO-2.3.md`) s'appliquent à l'espace du travail, jamais à l'espace perso. Mais un poste géré par l'entreprise (stratégies Intune, profil MDM) peut imposer des règles à toute l'app :
  - les serveurs permis, par exemple « seulement ceux de l'entreprise » ;
  - l'interdiction de la vue agrégée ;
  - ou que les notifications de l'espace du travail ne montrent pas leur contenu.

  C'est une attente classique d'un RSSI pour un poste de l'entreprise.
- **L'app mobile** a la même liste d'espaces, appairés un par un.
- **À étudier** :
  - ce que la barre des projets et la vue d'ensemble deviennent avec plusieurs espaces ;
  - le coût mémoire de plusieurs connexions ouvertes ;
  - la reprise d'un espace après une coupure, sans gêner les autres.
- **Le design côté client** (ESC-516, 2026-11-17) : [`SERVER-UX.md`](SERVER-UX.md) dessine « Se connecter à un serveur », ses états, la machine partout où elle compte et trois directions pour les espaces (l'aiguillage, les piles, la colonne), avec les choix S1 à S16 et les tickets MS-3 à MS-5.

**Décisions à prendre (en plus des quatre plus bas)** :
5. L'app mobile dès la 2.3, ou l'interface web d'abord ; et sa technique (Tauri 2, Capacitor, React Native ou Flutter), après l'étude.
6. Le relais de notifications push, offert à l'auto-hébergé ou non.
7. Plusieurs espaces dans un même Escouade (demandé) : la colonne d'espaces à la Slack, et ce qu'une vue agrégée peut montrer d'un espace de travail.

## L'ordre proposé

1. **Essais courts.**
   - Le cœur dans un process sans fenêtre, sous Linux sans affichage (VPS ou conteneur).
   - La connexion de chaque harness sans navigateur, sur un VPS.
   - `ssh -L` lancé par l'app, sous Windows, macOS et Linux.
   - `boringtun` et `smoltcp`, joints depuis l'app WireGuard d'un téléphone en 4G.
2. **La fondation, commune.**
   - Le contenu : `Host`, la macro des commandes, l'API WebSocket, le hub à plusieurs abonnés, l'état d'interface par client, les tampons des terminaux, le front sur deux transports.
   - Elle ne change rien de visible. On la teste en faisant tourner l'app de bureau sur l'API distante, en local.
3. **Le mode serveur.**
   - Côté serveur : `escouade-server`, le service systemd, l'appairage, les secrets en fichier, la mise à jour.
   - Côté client : « Se connecter à un serveur », et tout ce que dit la table « Ce qui change de machine ».
4. **Le téléphone.** L'interface web servie dans le tunnel (option A), vers le PC ou le serveur, avec WireGuard intégré.

**Taille.** Un chantier de l'ordre du redesign. Il passe après les plugins de la 2.1, qui touchent aussi le cœur et ses événements : le bus de faits de `PLUGINS.md` sert aussi aux notifications d'un client distant. La version reste à décider.

## Décisions à prendre

1. **Une fenêtre par machine**, locale ou serveur, avec la bascule dans la barre de titre ? Ou des projets de plusieurs machines mêlés dans la barre des projets ? Proposition : une machine par fenêtre d'abord.
2. **Le tunnel par défaut du mode serveur** : SSH (proposé), WireGuard intégré ou Tailscale.
3. **La version** : après les plugins (2.2 ?), avec le téléphone.
4. **La distribution du serveur** : un binaire et systemd (proposé), puis une image Docker ?

## Tâches

**Fondation**
- [x] Essai : le cœur sans fenêtre sous Linux sans affichage (ESC-451, `headless_tests.rs`).
- [ ] Essai : `ssh -L` lancé par l'app sur les trois systèmes.
- [x] Le cœur sans Tauri : le trait `Host` à la place de l'`AppHandle` ; un verrou par fichier à la place de `single-instance` hors bureau (ESC-451).
- [x] L'API distante : une seule liste des commandes (macro) pour l'IPC et la WebSocket ; les flux binaires des terminaux multiplexés ; la poignée de main des versions (ESC-478, `remoteapi`).
- [x] `hub.rs` à plusieurs abonnés ; `UiState` par client ; une réponse fermée chez les autres clients (ESC-478).
- [x] Un tampon de sortie pour chaque terminal et chaque lancement ; la reprise d'un client après une coupure (ESC-495, `termlog`, `remoteapi::sessions`).
- [x] `ipc.ts` sur deux transports ; l'app de bureau testée sur l'API distante en local (e2e) (ESC-495, `src/lib/transport.ts`, la part « remote API »).
- [x] La file d'envoi bornée, les bornes des connexions et des poignées de main ratées ; les corps bruts en trames binaires (ESC-495).

**Mode serveur**
- [ ] Essais : la connexion de Claude Code, Codex, Copilot CLI, Cursor CLI et OpenCode sur un VPS, le navigateur sur une autre machine.
- [x] `escouade-server` pour Linux x64 et arm64, sans WebKit ni GTK ; `escouade-server pair` et `devices` ; ses journaux ; l'arrêt propre à SIGTERM (ESC-514, `src-tauri/src/server`, le job « Server (Linux) »).
- [ ] Le service systemd et sa commande d'installation ; l'image Docker.
- [x] Les secrets dans un fichier lisible du seul propriétaire, sur un serveur ; le fait dit au client (`welcome.secretsInFile`) (ESC-514).
- [x] L'interface dit que les secrets d'un serveur sont en fichier (ESC-538, `SecretsOnServer.svelte`).
- [ ] La mise à jour du serveur (même signature que l'updater), proposée par le client à sa version.
- [x] Dans le client : « Se connecter à un serveur » (hôte SSH, appairage), la machine dans la barre de titre, le retour en local (ESC-537).
- [x] La table « Ce qui change de machine » (ESC-538) :
  - le navigateur des dossiers du serveur ;
  - l'envoi des logos et des fichiers joints ;
  - le téléchargement des fichiers des agents ;
  - les ports des serveurs de test relayés ;
  - les notifications rendues par le client ;
  - la voix sur le poste ;
  - la connexion d'un compte relayée ou par code, les secrets en fichier dits, « Afficher dans l'Explorateur » vers le panneau Fichiers.
- [x] Un e2e `escouade-server` + client par une redirection locale (ESC-514, `tests/e2e/server.spec.ts`).

**Téléphone**
- [ ] Essais : `boringtun` + `smoltcp` dans un binaire de test, joint depuis l'app WireGuard d'un téléphone en 4G, sous Windows, macOS et Linux.
- [x] Module `phone` (backend) (ESC-567, `src-tauri/src/phone`) :
  - le pair WireGuard et la pile réseau ;
  - l'API distante dans le tunnel ;
  - les pairs et la clé gardés (la clé privée du tunnel dans le trousseau, en fichier sur un serveur).
- [x] Ouverture du port par UPnP et NAT-PMP, détection du CGNAT et de l'adresse publique, suivi de son changement (ESC-567, `phone::nat`).
- [ ] Réglages › Téléphone :
  - [x] l'activation et le port (ESC-567) ;
  - [ ] un QR code par appareil (MO-4) ;
  - [ ] la liste des appareils et leur révocation (les téléphones du tunnel et « Retirer » : ESC-567 ; le pair retiré avec le jeton : MO-4) ;
  - [x] l'état : port ouvert ou non, adresse publique, dernière connexion (ESC-567).
- [ ] La mise en page mobile des écrans retenus (option A).
- [x] Détection de Tailscale et de son adresse (ESC-567, `phone::tailscale`).

**Docs**
- [ ] README et site : le mode serveur (installation sur un VPS, connexion par SSH) et l'accès depuis le téléphone.
- [ ] La FAQ « Où vont mes données ? » : elle doit mentionner ces deux accès, et les identifiants des comptes sur le serveur.
