---
title: "Le mode serveur"
subject: "a-distance"
sources: ["docs/REMOTE.md, « Le mode serveur », « Le mode serveur, MS-1 (ESC-514) », « Le mode serveur, MS-3 (ESC-515) », « Tâches › Mode serveur »","docs/CRATES.md, § 14 (le cœur sans Tauri)","src-tauri/src/server (mod.rs, secrets.rs, logs.rs, status.rs, update.rs), src-tauri/src/bin/escouade-server.rs","src-tauri/src/remoteapi/pairing.rs, devices.rs, server.rs, proto.rs","crates/escouade-tauri-shim","deploy/ (Dockerfile, docker-compose.yml, coolify/docker-compose.yml, install.sh, README.md)","scripts/server-libs.sh, server-trial.sh, server-package.sh, server-release.sh, server-image-test.sh, server-image-client.mjs, server-install-test.sh, minisign-verify.mjs, image-tags.mjs",".github/workflows/ci.yml (Server (Linux)), .github/workflows/release.yml (server, server-image), .github/dependabot.yml","website/content/docs/fr/server.md, website/content/docs/en/server.md","tests/e2e/server.spec.ts, tests/e2e/server.ts","Demande de l'utilisateur du 2026-11-16 : « faudra penser à proposer une image docker prête à l'emploi pour déployer un serveur escouade »","Revue de sécurité de la 2.3 (ESC-624), rapport privé SR-1, constats SR1-07, SR1-12, SR1-13 (ESC-640)"]
tickets: ["ESC-514","ESC-515","ESC-537","ESC-538","ESC-539","ESC-567","ESC-620","ESC-654","ESC-631","ESC-640"]
decisions: [202,216,281,289,328,331]
compared: null
---

# Le mode serveur

Escouade tourne sur une machine sans bureau (un VPS, un conteneur), H24, et le client de bureau la pilote à distance par l'API distante (voir *L'API distante*), à travers un tunnel. Le cœur (agents, harness, worktrees, Kanban, terminaux, serveur MCP) tourne sur le serveur ; le client n'en est qu'une vue : le fermer, éteindre le poste ou couper le réseau ne touche à aucun agent.

Ce qui est livré : le programme du serveur, `escouade-server` (MS-1) ; le client qui s'y connecte, par un tunnel SSH qu'il ouvre lui-même, décrit dans *Se connecter à un serveur* (ESC-537) ; et la distribution du serveur (ESC-515) : l'image Docker, un Compose d'exemple et une recette Coolify, le script d'installation systemd, la mise à jour signée, la release ; et ce qui change de machine (ESC-538 : les dossiers du serveur, les fichiers envoyés et reçus, les ports relayés, les notifications, la voix, les comptes, les secrets), décrit dans *Ce qui change de machine*. Le guide de l'utilisateur est la page « Mode serveur » de la documentation du site.

## `escouade-server`

- Un binaire pour **Linux x64 et arm64**, qui ne lie **ni WebKit ni GTK** (ni D-Bus, ni le son, ni X11 ou Wayland) : il démarre et tourne sans aucun affichage, dans un conteneur Debian nu. La CI le construit pour les deux architectures, vérifie ses bibliothèques et le démarre dans un tel conteneur. La release le construit sur Ubuntu 22.04 : il tourne sur Debian 12, Ubuntu 22.04 et plus récents.
- Il se construit aussi sous Windows et macOS, pour les essais ; seul Linux est livré.
- Il prend le **dossier de données** (`~/.escouade`, ou `--data-dir`) et le tient : un second Escouade sur le même dossier est refusé, avec un message clair (« Escouade tourne déjà sur ce dossier de données (…) : un seul à la fois. »).
- **Le dossier de données est au seul utilisateur du serveur** : sous Unix, chaque commande le crée en 0700, et le remet en 0700 s'il était plus ouvert, quel que soit l'`umask` (ESC-640, SR1-13 : un serveur lancé à la main, sans le `UMask=0077` de systemd ni le `/data` de l'image, laissait `~/.escouade` en 0755).
- Il charge le cœur comme l'app de bureau (projets, agents, Kanban, Git, plugins, comptes), sans fenêtre : ce que le bureau montrerait (une notification, un carillon, le compte de ce qui attend) est poussé aux clients, qui le rendent chez eux ; le clic d'une notification revient au serveur, qui garde l'action de ses 64 dernières. Ce qu'il demanderait d'ouvrir (un lien, un de ses fichiers) n'est jamais ouvert par un client sans geste.
- Il sert l'API distante sur **127.0.0.1** (port 46480, ou `--port`). On l'atteint par un tunnel (`ssh -L <port local>:127.0.0.1:46480 <hôte>`).
- Il s'arrête proprement à **SIGTERM** (systemd, `docker stop`) ou Ctrl+C : l'API se ferme, les tours en cours des agents sont coupés (chacun reprend sa session au démarrage suivant, comme quand on quitte l'app), l'état est écrit, le dossier est lâché.
- Il dit ce qu'il fait dans `server-status.json`, dans son dossier : son processus, sa version, l'adresse de son API, ce qui le tient occupé (un tour, une question, une tâche de fond, un lancement, un terminal servi depuis moins de 5 minutes : ce qui retient aussi le redémarrage automatique de l'app), réécrit toutes les 5 secondes et aussitôt que cela change, retiré à l'arrêt propre. `health` et `update` le lisent. Il ne contient aucun secret.
- **Au premier démarrage**, tant qu'aucun appareil n'est appairé, le journal dit comment en appairer un (« lance « escouade-server pair » sur ce serveur », « docker exec <conteneur> escouade-server pair » dans l'image), jamais le code.

## Ses commandes

| Commande | Ce qu'elle fait |
| --- | --- |
| `escouade-server` ou `escouade-server run` | Le serveur, jusqu'à SIGTERM |
| `escouade-server pair` | Un code d'appairage pour un nouvel appareil |
| `escouade-server devices` | Les appareils appairés : id, nom, date d'appairage, dernière connexion |
| `escouade-server devices revoke <id>` | Révoque un appareil : son jeton ne vaut plus ; un serveur qui tourne ferme ses connexions en quelques secondes |
| `escouade-server health` | Code 0 si le serveur du dossier tourne et écrit son état depuis moins de 30 s, 1 sinon, avec la raison : le `HEALTHCHECK` de l'image |
| `escouade-server update` | Met le serveur à une version signée (voir *La mise à jour*) : `--to X.Y.Z` (avec `--downgrade` pour une plus ancienne), `--beta`, `--now`, `--check` |
| `--data-dir <dossier>`, `--port <n>`, `--listen <adresse>`, `--allow-host <nom>`, `--trust-proxy <adresse>`, `--debug`, `--version`, `--help` | Les options |

Chaque option a sa variable d'environnement, pour un conteneur ; l'option donnée l'emporte, une variable vide compte pour absente : `ESCOUADE_SERVER_DATA`, `ESCOUADE_SERVER_PORT`, `ESCOUADE_SERVER_LISTEN`, `ESCOUADE_SERVER_ALLOW_HOSTS` (des noms séparés par des virgules), `ESCOUADE_SERVER_TRUST_PROXIES` (des adresses IP séparées par des virgules). C'est ce qui fait que `docker exec <conteneur> escouade-server pair` vise le dossier du serveur du conteneur.

`--help` n'exécute jamais une commande à sa place (`pair --help`, `update --help` montrent l'aide, sans faire de code ni rien télécharger). `--to`, `--beta`, `--now`, `--check` et `--downgrade` ne vont qu'avec `update`, `--downgrade` qu'avec `--to` (« --downgrade va avec --to X.Y.Z : la version plus ancienne se nomme »), et `--to` veut une version (`X.Y.Z`, `X.Y.Z-beta.N`). Les commandes parlent la langue des réglages du dossier de données, sinon celle du système. `pair` et `devices` marchent que le serveur tourne ou non.

## L'appairage d'un appareil

1. Sur le serveur : `escouade-server pair` affiche un code, par exemple `8NUC-W6HS`, valable **une fois** et **10 minutes**. Un nouveau `pair` remplace le code précédent. Le client peut aussi le lancer lui-même par SSH (« le lancer d'ici par SSH »).
2. Dans le client, l'utilisateur donne ce code et un nom pour l'appareil (« Portable du bureau »).
3. Le client l'envoie à l'API **à la place de la poignée de main**. Le serveur répond le **jeton d'appareil**, donné une seule fois, que le client garde et redonne à chaque connexion.

Les refus, chacun avec sa raison pour le client (`none`, `expired`, `wrong`, `wait`) :

- **expiré** : « Ce code d'appairage a expiré : lance « escouade-server pair » de nouveau. » ;
- **déjà utilisé, ou jamais demandé** (un code redonné) : « Aucun code d'appairage n'attend… » ;
- **faux** : « Code d'appairage faux. » ; **le code qui attend vaut toujours** : un code faux ne l'annule jamais (ESC-654 : cinq codes faux de n'importe qui le finissaient, et bloquaient l'appairage à volonté) ;
- **trop d'essais dans la minute** : au-delà de 20 poignées de main ratées d'une même source (son adresse, le pair du tunnel du téléphone, ou le client d'un proxy déclaré), ou de 120 codes faux de toutes les sources venues d'ailleurs que la machine, dans la minute, le code de cette source (ou venu d'ailleurs) n'est pas lu (« Trop d'essais faux en une minute : le code d'appairage vaut toujours, réessaie dans une minute. »). Les codes tapés **sur la machine** (`escouade-server pair` puis la boucle locale, un client par SSH) ont leur propre plafond : des codes faux venus du réseau ne les refusent jamais, et la fenêtre dit quand ce plafond-là est atteint (ESC-631). C'est ce qui tient la force brute : au plus 2 400 codes lus pendant les 10 minutes d'un code, sur 2⁴⁰, une chance sur 450 millions environ.

Le code est comparé **en temps constant**. Un code refusé compte comme une poignée de main ratée de sa source ; un bon jeton, lui, passe toujours (voir *L'API distante*, « Les bornes »). Le fichier qui garde le code en attente ne contient que son empreinte, lisible du seul propriétaire. **Ni le code ni le jeton ne sont jamais écrits dans le journal** ; `pair` affiche le code sur sa seule sortie, pour l'utilisateur.

## Les appareils révoqués

Un appareil révoqué (par `escouade-server devices revoke`, ou à sa propre demande quand il retire le serveur) laisse une trace : son nom, la date, qui l'a révoqué et l'empreinte de son jeton, jamais le jeton ; les 32 derniers sont gardés. Un client qui présente encore ce jeton est refusé avec ces informations, pour dire « Ce poste n'est plus appairé », quand et depuis où.

## Les secrets sur un serveur

Un VPS n'a pas de trousseau (pas de Secret Service, pas de session). Les secrets du serveur sont donc gardés dans **`secrets.json`, dans son dossier de données, lisible du seul propriétaire** (droits 0600 ; un fichier rendu lisible par d'autres, une sauvegarde remise par exemple, est remis à 0600 au démarrage, et le journal le dit). On y trouve :

- les jetons des appareils ;
- les clés des intégrations ;
- les clés de Cursor et de Mistral Vibe ;
- les secrets des plugins.

Le serveur le dit à chaque client : le message d'accueil de l'API porte `secretsInFile`, que le client affiche là où un secret se donne (« Gardée dans un fichier sur Travail », voir *Ce qui change de machine*). Il porte aussi la machine du serveur (son nom et le compte du service), que le client affiche. Les clés des voix en ligne, elles, restent au trousseau du poste : la voix est celle du poste. Les harness, eux, se connectent sur le serveur et y gardent leurs propres jetons : **la sécurité des comptes devient celle du serveur**, son disque, son accès SSH, ses sauvegardes. La documentation, le script d'installation et la FAQ « Où vont mes données ? » le disent.

## L'image Docker

`ghcr.io/guillaume-gagnaire/escouade/escouade-server`, pour linux/amd64 et linux/arm64, étiquetée à la version, à sa version mineure (`2.3`, qui suit les correctifs stables de la 2.3) et `latest` ; une bêta prend sa version et `beta`, jamais `latest` ni la mineure.

- **Ce qu'elle contient** : Debian 13 mince ; le programme et l'amorce de ses plugins ; `git`, `openssh-client`, `curl`, `ca-certificates` ; `tini` en PID 1 (il recueille les processus orphelins des harness et passe SIGTERM au serveur) ; Node.js et npm, pour les harness qui s'installent par `npm install -g`. Aucun harness, aucun compte. Elle pèse 427 Mo décompressée, environ 150 Mo à télécharger.
- **Ses bases épinglées** (ESC-640, SR1-12) : Debian (`debian:trixie-slim`) et Node (`node:22-trixie-slim`, d'où viennent `node` et `npm`) sont nommées par l'empreinte de leur index multi-architecture (`@sha256:…`), l'étiquette à côté pour dire laquelle : chaque construction part des mêmes octets, quoi que l'étiquette du registre désigne plus tard. Dependabot (écosystème `docker`, `deploy/`) propose chaque mois l'empreinte nouvelle ; un test refuse une base sans empreinte.
- **Sans privilège** : elle tourne en `escouade` (uid 10001), sans root ni sudo. Elle tourne avec un système de fichiers en lecture seule (`--read-only`, `/tmp` en mémoire), sans aucune capacité (`--cap-drop ALL`) et `no-new-privileges` : les exemples le font.
- **Tout dans `/data`**, le volume : `HOME` est `/data` ; les données d'Escouade (`/data/.escouade`), les projets et leurs worktrees, les harness (l'installeur de Claude Code les pose dans `~/.local/bin`, `npm install -g` dans `/data/.npm-global`) et leurs connexions. Rien n'est écrit ailleurs. Les harness survivent donc à une nouvelle image ; Node, lui, est dans l'image, mis à jour avec elle.
- **Le réseau** : sans `ESCOUADE_SERVER_LISTEN`, l'API n'écoute que sur 127.0.0.1 **du conteneur** : rien n'est joignable du dehors, même un port publié. L'image n'expose aucun port. `ESCOUADE_SERVER_LISTEN=0.0.0.0` l'ouvre au réseau du conteneur, pour un port publié sur 127.0.0.1 de l'hôte seulement (le tunnel SSH vers l'hôte) ou un proxy TLS devant. **L'API y est alors en clair** (HTTP et WebSocket sans TLS) : tout autre conteneur de ce réseau (sur Coolify, les autres ressources du même réseau) l'atteint et peut tenter des codes d'appairage et des jetons, dans les bornes de *L'appairage*. Le TLS est celui du proxy ou du tunnel : le Dockerfile, les deux exemples et la documentation le disent, et conseillent le proxy TLS ou le tunnel, jamais un port publié sur une adresse publique.
- **Sa santé** : `HEALTHCHECK` lance `escouade-server health` (toutes les 30 s, 120 s laissées au démarrage).
- **Son arrêt** : `docker stop` (SIGTERM) arrête proprement ; les exemples donnent 60 s.
- **Sa mise à jour** est une nouvelle image : `escouade-server update` refuse de tourner dans l'image (il le sait par `ESCOUADE_SERVER_IMAGE`) et dit quoi faire.
- **Les limites conseillées**, d'après une mesure sur l'image x64 : au repos, les plugins chargés, environ 235 Mo et 106 tâches (processus et threads) ; un agent du faux `claude` des tests en plein tour en ajoute environ 18 Mo et 7. Un vrai harness pèse plus. Les exemples fixent 4 Go de mémoire et 2048 tâches, pour quelques agents à la fois.

## Compose et Coolify

- `deploy/docker-compose.yml` : l'image **sur sa version mineure** (`:2.3`, jamais `latest` : un `docker compose pull` prend les correctifs de la 2.3, jamais la 2.4 sans qu'on change la ligne ; ESC-640, SR1-12), `ESCOUADE_SERVER_LISTEN=0.0.0.0` et ce qu'il expose, le port publié sur **127.0.0.1 de l'hôte seulement** (`127.0.0.1:46480:46480`), le volume `escouade-data`, la lecture seule, aucune capacité, `no-new-privileges`, 60 s pour s'arrêter, les limites. L'appairage : `docker compose exec escouade escouade-server pair`.
- `deploy/coolify/docker-compose.yml` : le même, tel que Coolify le prend (une ressource « Docker Compose »). Tel quel, rien ne va sur Internet. En commentaire, l'accès par un domaine derrière le proxy de Coolify (Traefik, en TLS) : `SERVICE_FQDN_ESCOUADE_46480`, `ESCOUADE_SERVER_ALLOW_HOSTS=${SERVICE_FQDN_ESCOUADE}` et `ESCOUADE_SERVER_TRUST_PROXIES` (l'adresse du proxy de Coolify sur ce réseau), avec ses risques.

Aucun exemple n'expose l'API sans TLS sur Internet.

## Les deux accès, et leurs risques

- **Le tunnel SSH** (conseillé) : `ssh -N -L 46480:127.0.0.1:46480 <serveur>`. Rien de nouveau n'est ouvert sur le serveur. Risque : qui a un compte SSH sur le serveur, ou un processus de l'hôte, atteint le port et peut tenter de s'appairer ; l'accès vaut ce que vaut le SSH.
- **Un proxy TLS devant** (Traefik de Coolify, Caddy, nginx) : le serveur écoute sur le réseau du conteneur et accepte le nom du domaine dans `Host` (`--allow-host`, `ESCOUADE_SERVER_ALLOW_HOSTS`), tout autre nom restant refusé. **Le proxy se déclare** (`--trust-proxy <son adresse>`, `ESCOUADE_SERVER_TRUST_PROXIES`, ESC-631) : chaque client qu'il fait suivre compte alors ses échecs à part, d'après la dernière adresse de `X-Forwarded-For` (celle que le proxy écrit) ; sans cela, toutes ses connexions comptent comme une seule source. L'en-tête n'est jamais lu d'une adresse qui n'est pas déclarée. Risque : l'API est sur Internet ; qui atteint le domaine peut tenter un code (40 bits, 10 minutes, une fois, 120 essais lus par minute au plus, sans jamais bloquer un code tapé sur le serveur ou par SSH) ou un jeton (256 bits), chaque essai journalisé et borné ; une faille du serveur serait exposée à tous. À réserver à un domaine filtré par adresse IP ou à un réseau privé, et jamais sans TLS. Le client se connecte aujourd'hui par le tunnel SSH qu'il ouvre (voir *Se connecter à un serveur*) ; une adresse `wss://` d'un domaine viendra plus tard.

## Le service systemd

`install.sh` (dans `deploy/`, et à chaque release), pour Linux x64 ou arm64 avec systemd :

- il télécharge l'archive de la release (la dernière, ou `--version`), vérifie sa **SHA-256** et sa **signature** (la clé de l'updater de l'app ; avec `minisign`, sinon `openssl` 3) : une archive refusée ne change rien ;
- par défaut, un **service de l'utilisateur** (`systemctl --user`, `loginctl enable-linger` pour qu'il tourne sans session) ; avec `--system`, un **service du système** lancé par l'utilisateur `escouade` (créé s'il manque ; `--user` en choisit un autre), à qui le programme appartient pour qu'il se mette à jour lui-même ;
- le service : `Restart=always` (après une panne et après `update`), `KillMode=mixed` (SIGTERM au seul serveur, qui arrête ses agents lui-même), 90 s pour s'arrêter, `UMask=0077`, `NoNewPrivileges` ;
- **idempotent** : relancé, il ne change que ce qui diffère, et ne redémarre pas un serveur à la même version ;
- **désinstallable** : `--uninstall` retire le service et le programme et garde les données ; `--uninstall --purge` retire aussi les données (avec `--system`, l'utilisateur et tout son dossier) ;
- à la fin, il dit comment appairer, le tunnel, la mise à jour, et que les connexions des comptes vivent sur ce serveur.

## La mise à jour

`escouade-server update` :

1. la version : `--to X.Y.Z` (celle que propose le client), sinon celle de `latest.json` de la dernière release ; avec `--beta`, **la plus haute** de `latest.json` et de `beta.json` (comme l'app avec « Recevoir les versions bêta » : une stable sortie après la dernière bêta l'emporte ; sans `beta.json`, celle de `latest.json`) ; `--check` la dit sans rien faire ;
2. **jamais une version plus ancienne que la sienne** (ESC-640, SR1-07 : un miroir de `ESCOUADE_SERVER_UPDATE_URL`, ou qui peut changer `latest.json`, servirait une ancienne version signée et vulnérable) : « la version publiée (1.0.0) est plus ancienne que ce programme (2.3.0) : rien n'est changé ». Une bêta compte avant sa version (2.3.0-beta.4 est plus ancienne que 2.3.0) : un serveur sur une bêta y reste jusqu'à la stable qui la suit. Revenir en arrière se fait en la nommant, `--to X.Y.Z --downgrade` ; `--to` seul vers une plus ancienne est refusé et dit comment faire ; la même version ne télécharge rien ;
3. l'archive de cette version pour l'architecture et sa signature, **vérifiée avec la clé de l'updater de l'app** avant de rien toucher, et **liée à son nom** : le commentaire de confiance de la signature (signé avec elle, `timestamp:…<tab>file:<nom>` tel que l'écrit le signataire de Tauri) doit nommer l'archive attendue, `file:escouade-server-<version>-linux-<arch>.tar.gz`. L'archive signée d'une autre version servie sous ce nom est refusée, même si ses octets sont bien signés ; une signature qui ne tient pas : « signature refusée … rien n'est changé » ;
4. chaque fichier de l'archive (le programme, l'amorce) posé à la place du sien, l'ancien gardé en `.old` ;
5. le serveur qui tourne est arrêté **une fois au repos** (aucun tour, aucune question en attente, aucune tâche de fond, aucun lancement, aucun terminal servi depuis 5 minutes) ; dans un terminal, il propose de redémarrer tout de suite en coupant les tours ; `--now` le fait sans attendre ; systemd le relance sur la nouvelle version, et `update` le dit. Sans superviseur, il dit de le relancer ;
6. dans l'image Docker, il refuse : la mise à jour est une nouvelle image.

Quand la poignée de main refuse la version du client parce que le serveur est plus ancien, elle dit comment mettre le serveur à la version du client (voir *L'API distante*) ; l'écran « Mettre à jour le serveur » vient avec le client. Cette commande ne vise jamais qu'une version plus récente que celle du serveur : seul le côté le plus ancien est mis à jour.

L'app de bureau a la même faiblesse de principe, bornée autrement : voir *Livraison et mises à jour*, « Ce que couvre la signature ».

## La release

Le workflow de release publie, pour chaque version : les archives `escouade-server-<version>-linux-x64.tar.gz` et `-linux-arm64.tar.gz` (le programme, l'amorce des plugins, la licence), leurs SHA-256, leurs signatures (la clé de l'updater, comme l'app ; leur commentaire de confiance nomme l'archive), `install.sh`, et l'image sur ghcr.io. Chaque architecture se construit sur un runner à elle ; la signature est vérifiée aussitôt faite ; l'image de chaque architecture est poussée par son empreinte seule, puis, une fois la release publiée, les deux sont réunies sous la version, sa mineure et `latest` (une bêta : sa version et `beta`). Les archives et `install.sh` ont une attestation de provenance comme les installeurs de l'app ; l'image a la sienne, gardée au registre (`gh attestation verify oci://…`).

## La sécurité

- L'API n'écoute que sur 127.0.0.1. `--listen <adresse>` la met sur une autre adresse. Cette option ne sert qu'à Docker, derrière un tunnel, un réseau privé ou un proxy. Elle avertit dans le journal et sur la sortie d'erreur : « ATTENTION : l'API distante écoute sur … pas seulement sur 127.0.0.1 … jamais ouvert sur Internet ». Rien n'est ouvert par défaut.
- `--allow-host` n'ajoute qu'un nom au contrôle de `Host` ; il n'ouvre aucune interface. `--trust-proxy` ne change que la source à qui sont comptés les échecs ; il n'ouvre rien non plus, et le journal dit au démarrage quels proxys sont déclarés.
- Les stratégies imposées valent pour le serveur : `servers.remoteApi` à `false` l'empêche de servir l'API, et refuse aussi l'appairage.
- Tout le reste de *L'API distante* vaut ici : le `Host` du bouclage, pas d'`Origin` de page web, les bornes des connexions, des appels et des files.

## Le journal

Chaque ligne va sur la **sortie standard**, pour le journal de systemd et les logs de Docker. Elle va aussi dans **`server.log`**, dans le dossier de données, qui tourne à 5 Mo (`server.1.log` à `server.3.log`, le plus ancien supprimé). Sous Unix, ces fichiers sont **lisibles de leur seul propriétaire** (0600) quel que soit l'`umask`, et ceux qu'une version d'avant avait laissés plus ouverts y sont remis à l'ouverture du journal (ESC-640, SR1-13 : il nomme des appareils et des projets). Le journal dit :

- le démarrage, l'adresse de l'API, le dossier de données et celui des secrets, les noms pris en plus dans `Host` et les proxys déclarés ; sans appareil appairé, comment en appairer un ;
- chaque appareil appairé, chaque connexion ;
- chaque refus et sa raison ;
- l'arrêt et ce qui l'a demandé.

Il ne dit jamais de jeton, de code, de secret ni ce qu'écrit un terminal. `--debug` ajoute le détail d'Escouade.

## Pas encore

- L'écran « Mettre à jour le serveur » qui lance la commande ou dit l'image (le fait est exposé : `serverUpdate`).
- Une adresse `wss://` d'un domaine dans le client.
- Les serveurs de langage de l'éditeur, encore refusés à distance.

## Le tunnel du téléphone sur un serveur (ESC-567)

Un serveur sert aussi le téléphone par le tunnel WireGuard intégré (voir *Le tunnel du téléphone*), sur la même API. Il s'allume depuis Réglages de l'espace › Téléphone dans un client connecté, ou dans son terminal :

| Commande | Ce qu'elle fait |
| --- | --- |
| `escouade-server phone` (`status`) | Le tunnel allumé ou non, son port UDP, sa clé publique, les téléphones |
| `escouade-server phone on [PORT]` / `off` | Allume (51820 par défaut) ou éteint le tunnel |
| `escouade-server phone add NOM [--endpoint HÔTE]` | Ajoute un téléphone et affiche sa config WireGuard et son QR code, pour l'app WireGuard officielle ; sa clé privée n'est montrée qu'une fois et gardée nulle part |
| `escouade-server phone remove ID` | Retire un téléphone, ses connexions coupées |

- Le serveur qui tourne relit ce que ces commandes changent dans les deux secondes.
- Il n'y a pas de box : le serveur ne demande rien à une box, et dit d'ouvrir le port UDP dans son pare-feu (et dans le `ports` de Docker ou de Coolify : `51820:51820/udp`).
- `servers.phone` à `false` (ou `servers.remoteApi`) posée sur le serveur l'interdit.
- La pile TCP/IP du tunnel côté serveur est smoltcp 0.14, la même que celle du téléphone (ESC-640, SR1-14 : la 0.12 tirait heapless 0.8, signalé « unsound »).

## Les tickets en parallèle sur un serveur (ESC-620)

Un serveur tourne H24 sur une machine qu'on dimensionne pour lui : son Kanban prend **jusqu'à 64 tickets à la fois** (16 sur un poste), voir *Kanban*, « Les tickets en parallèle ».

- La borne est celle du cœur qui fait tourner le Kanban : un client qui règle l'espace d'un serveur la reçoit du serveur (`board_capacity`, servie par l'API distante), avec la machine du serveur (ses cœurs, sa mémoire) pour l'avertissement au-delà de 6.
- Une stratégie posée sur le serveur (`projects.maxParallel`) ou l'équipe d'un projet l'abaisse, comme sur un poste.
- Les limites conseillées de l'image (4 Go, 2048 tâches) valent pour quelques agents ; pour beaucoup d'agents, compter la place d'un agent que dit l'avertissement (environ 6 Go et 2 cœurs pour un agent qui compile et lance des tests, jusqu'à 17 Go au pic d'une compilation).
