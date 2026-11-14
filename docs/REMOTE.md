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
  - Une seule redirection suffit : le reste (terminaux, serveurs de test) passe dans la connexion de l'API.
- **Plus tard.** WireGuard intégré (celui du téléphone, plus bas) et Tailscale s'il est installé, pour qui n'a pas SSH, ou pour un téléphone.

### Ce qui change de machine

| Quoi | Aujourd'hui | En mode serveur |
| --- | --- | --- |
| Choisir un dossier (assistant « Nouveau projet ») | Sélecteur du système | Un navigateur des dossiers du serveur, servi par le cœur ; cloner un dépôt devient le cas courant |
| Logo d'un projet | Chemin d'une image | L'image envoyée au serveur |
| Fichiers joints | Chemin d'origine (Windows) ou copie | Toujours une copie envoyée |
| Coller une image | Presse-papiers lu par le cœur sous Linux | Lu par le client, envoyé |
| Fichiers envoyés par un agent | « Ouvrir » et « Afficher dans le dossier » sur la machine du cœur | Téléchargés sur le poste ; « Ouvrir » ouvre la copie (jamais un exécutable, comme ESC-171) ; « Afficher » les montre dans le panneau Fichiers |
| « Afficher dans l'Explorateur » | Explorateur du système | Panneau Fichiers, ou caché |
| Serveur de test d'un agent (Lancement) | 127.0.0.1 du poste | Un port local du client, relayé dans la connexion vers le 127.0.0.1 du serveur (WebSocket du rechargement à chaud compris) |
| Notifications, son, clignotement, tray | Par le cœur | Un événement, rendu par le client ; le cœur ne notifie plus rien lui-même |
| Voix (micro, Whisper, Piper, voix en ligne) | Micro et haut-parleurs de la machine du cœur | Sur le poste : le module voix (sans aucune dépendance à Tauri) tourne dans le client et envoie le texte ; la clé d'une voix en ligne reste au trousseau du poste |
| Connexion d'un compte | CLI lancé dans un PTY, navigateur ouvert sur la même machine | L'adresse de connexion s'ouvre sur le poste ; le retour par `localhost` est à essayer harness par harness (coller un code, code d'appareil) |
| Secrets (clés de Cursor, des intégrations, des voix) | Trousseau (Windows, macOS) | Un fichier lisible du seul propriétaire, dans les données du serveur (pas de Secret Service sur un VPS), et l'interface le dit |
| Mise à jour | L'app se met à jour | Le client et le serveur, chacun le sien ; le serveur avec la même signature que l'updater ; le client propose de mettre le serveur à sa version |
| Serveur MCP | 127.0.0.1 du poste | 127.0.0.1 du serveur : les agents du serveur l'atteignent ; un Claude Code du poste qui pilote Escouade passera par le tunnel (plus tard) |
| WSL | Windows | Sans objet |

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

## Le téléphone

Il repose sur la même fondation : l'API distante du cœur, servie dans un tunnel. Il vise le PC ou un serveur. Le tout se fait sans service tiers ni abonnement, avec une configuration réduite à une redirection de port dans la box et un QR code à scanner.

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
- [ ] Essais : le cœur sans fenêtre sous Linux sans affichage ; `ssh -L` lancé par l'app sur les trois systèmes.
- [ ] Le cœur sans Tauri : le trait `Host` à la place de l'`AppHandle` ; un verrou par fichier à la place de `single-instance` hors bureau.
- [ ] L'API distante : une seule liste des commandes (macro) pour l'IPC et la WebSocket ; les flux binaires des terminaux multiplexés ; la poignée de main des versions.
- [ ] `hub.rs` à plusieurs abonnés ; `UiState` par client ; une réponse fermée chez les autres clients.
- [ ] Un tampon de sortie pour chaque terminal et chaque lancement ; la reprise d'un client après une coupure.
- [ ] `ipc.ts` sur deux transports ; l'app de bureau testée sur l'API distante en local (e2e).

**Mode serveur**
- [ ] Essais : la connexion de Claude Code, Codex, Copilot CLI, Cursor CLI et OpenCode sur un VPS, le navigateur sur une autre machine.
- [ ] `escouade-server` pour Linux x64 et arm64, sans WebKit ni GTK ; le service systemd ; `escouade-server pair` ; ses journaux.
- [ ] Les secrets dans un fichier lisible du seul propriétaire, sur un serveur ; dit dans l'interface.
- [ ] La mise à jour du serveur (même signature que l'updater), proposée par le client à sa version.
- [ ] Dans le client : « Se connecter à un serveur » (hôte SSH, appairage), la machine dans la barre de titre, le retour en local.
- [ ] La table « Ce qui change de machine » :
  - le navigateur des dossiers du serveur ;
  - l'envoi des logos et des fichiers joints ;
  - le téléchargement des fichiers des agents ;
  - les ports des serveurs de test relayés ;
  - les notifications rendues par le client ;
  - la voix sur le poste.
- [ ] Un e2e `escouade-server` + client par une redirection locale.

**Téléphone**
- [ ] Essais : `boringtun` + `smoltcp` dans un binaire de test, joint depuis l'app WireGuard d'un téléphone en 4G, sous Windows, macOS et Linux.
- [ ] Module `remote` (backend) :
  - le pair WireGuard et la pile réseau ;
  - l'API distante dans le tunnel ;
  - les pairs et les jetons gardés (clés privées dans le trousseau comme les jetons des intégrations, en fichier sur un serveur).
- [ ] Ouverture du port par UPnP et NAT-PMP, détection du CGNAT et de l'adresse publique, suivi de son changement.
- [ ] Réglages › Téléphone :
  - l'activation et le port ;
  - un QR code par appareil ;
  - la liste des appareils et leur révocation ;
  - l'état : port ouvert ou non, adresse publique, dernière connexion.
- [ ] La mise en page mobile des écrans retenus (option A).
- [ ] Détection de Tailscale et de son adresse.

**Docs**
- [ ] README et site : le mode serveur (installation sur un VPS, connexion par SSH) et l'accès depuis le téléphone.
- [ ] La FAQ « Où vont mes données ? » : elle doit mentionner ces deux accès, et les identifiants des comptes sur le serveur.
