---
title: "Le mode serveur"
subject: "a-distance"
sources: ["docs/REMOTE.md, « Le mode serveur », « Le mode serveur, MS-1 (ESC-514) », « Le mode serveur, MS-3 (ESC-515) », « La mise à jour automatique du serveur (ESC-702) », « La socket Unix, BC-5 (ESC-706) », « Tâches › Mode serveur »","docs/BROWSER-CLIENT-STUDY.md, § 4 (ESC-655, choix B3)","docs/CRATES.md, § 14 (le cœur sans Tauri)","src-tauri/src/server (mod.rs, secrets.rs, logs.rs, status.rs, update.rs, autoupdate.rs, socket.rs), src-tauri/src/bin/escouade-server.rs","src-tauri/src/remoteapi/pairing.rs, devices.rs, server.rs, proto.rs","src/components/servers/UpdateServer.svelte, src/components/settings/ServerUpdates.svelte","crates/escouade-tauri-shim","deploy/ (Dockerfile, docker-compose.yml, coolify/docker-compose.yml, install.sh, README.md)","scripts/server-libs.sh, server-trial.sh, server-package.sh, server-release.sh, server-image-test.sh, server-image-client.mjs, server-install-test.sh, minisign-verify.mjs, image-tags.mjs",".github/workflows/ci.yml (Server (Linux)), .github/workflows/release.yml (server, server-image), .github/dependabot.yml","website/content/docs/fr/server.md, website/content/docs/en/server.md","tests/e2e/server.spec.ts, tests/e2e/server.ts, tests/e2e/server-update.spec.ts, tests/e2e/server-connect.spec.ts","src-tauri/src/server_socket_tests.rs","Demande de l'utilisateur du 2026-11-16 : « faudra penser à proposer une image docker prête à l'emploi pour déployer un serveur escouade »","Demande de l'utilisateur du 2026-11-27 : « Faudra gérer évidemment l'auto update du binaire serveur » (ESC-702)","Revue de sécurité de la 2.3 (ESC-624), rapport privé SR-1, constats SR1-07, SR1-12, SR1-13 (ESC-640)","docs/REMOTE.md, « Le mode serveur », « Le mode serveur, MS-1 (ESC-514) », « Le mode serveur, MS-3 (ESC-515) », « Le client navigateur, BC-1 (ESC-704) », « Tâches › Mode serveur »","docs/BROWSER-CLIENT-STUDY.md (ESC-655), §§ 1, 3, 4, 5 et 9","src-tauri/src/server (mod.rs, secrets.rs, logs.rs, status.rs, update.rs), src-tauri/src/bin/escouade-server.rs","src-tauri/src/remoteapi/pairing.rs, devices.rs, server.rs, page.rs, proto.rs","crates/escouade-tauri-shim, crates/escouade-web (la page embarquée)","vite.web.config.ts, src/web/tauri-refused.ts (la build web, les bouchons de Tauri)","tests/e2e/server.spec.ts, tests/e2e/server.ts","docs/REMOTE.md, « Le mode serveur », « Le mode serveur, MS-1 (ESC-514) », « Le mode serveur, MS-3 (ESC-515) », « Tâches › Mode serveur »","docs/REMOTE.md, « Le mode serveur », « Le mode serveur, MS-1 (ESC-514) », « Le mode serveur, MS-3 (ESC-515) », « La mise à jour automatique du serveur (ESC-702) », « Tâches › Mode serveur »","src-tauri/src/server (mod.rs, secrets.rs, logs.rs, status.rs, update.rs, autoupdate.rs), src-tauri/src/bin/escouade-server.rs","tests/e2e/server.spec.ts, tests/e2e/server.ts, tests/e2e/server-update.spec.ts"]
tickets: ["ESC-514","ESC-515","ESC-537","ESC-538","ESC-539","ESC-567","ESC-620","ESC-654","ESC-631","ESC-640","ESC-597","ESC-600","ESC-702","ESC-706","ESC-704"]
decisions: [202,216,281,289,328,331,355,363,362,379,357,358,378]
compared: null
---

# Le mode serveur

Escouade tourne sur une machine sans bureau (un VPS, un conteneur), H24, et le client de bureau la pilote à distance par l'API distante (voir *L'API distante*), à travers un tunnel. Le cœur (agents, harness, worktrees, Kanban, terminaux, serveur MCP) tourne sur le serveur ; le client n'en est qu'une vue : le fermer, éteindre le poste ou couper le réseau ne touche à aucun agent.

Ce qui est livré : le programme du serveur, `escouade-server` (MS-1) ; le client qui s'y connecte, par un tunnel SSH qu'il ouvre lui-même, décrit dans *Se connecter à un serveur* (ESC-537) ; et la distribution du serveur (ESC-515) : l'image Docker, un Compose d'exemple et une recette Coolify, le script d'installation systemd, la mise à jour signée, la release ; et ce qui change de machine (ESC-538 : les dossiers du serveur, les fichiers envoyés et reçus, les ports relayés, les notifications, la voix, les comptes, les secrets), décrit dans *Ce qui change de machine* ; et la mise à jour automatique du serveur avec l'écran « Mettre à jour le serveur » du client (ESC-702) ; et l'écoute sur une socket Unix, par défaut pour le service d'`install.sh` (ESC-706). Le guide de l'utilisateur est la page « Mode serveur » de la documentation du site.

## `escouade-server`

- Un binaire pour **Linux x64 et arm64**, qui ne lie **ni WebKit ni GTK** (ni D-Bus, ni le son, ni X11 ou Wayland) : il démarre et tourne sans aucun affichage, dans un conteneur Debian nu. La CI le construit pour les deux architectures, vérifie ses bibliothèques et le démarre dans un tel conteneur. La release le construit sur Ubuntu 22.04 : il tourne sur Debian 12, Ubuntu 22.04 et plus récents.
- Il se construit aussi sous Windows et macOS, pour les essais ; seul Linux est livré.
- Il prend le **dossier de données** (`~/.escouade`, ou `--data-dir`) et le tient : un second Escouade sur le même dossier est refusé, avec un message clair (« Escouade tourne déjà sur ce dossier de données (…) : un seul à la fois. »).
- **Le dossier de données est au seul utilisateur du serveur** : sous Unix, chaque commande le crée en 0700, et le remet en 0700 s'il était plus ouvert, quel que soit l'`umask` (ESC-640, SR1-13 : un serveur lancé à la main, sans le `UMask=0077` de systemd ni le `/data` de l'image, laissait `~/.escouade` en 0755).
- Il charge le cœur comme l'app de bureau (projets, agents, Kanban, Git, plugins, comptes), sans fenêtre : ce que le bureau montrerait (une notification, un carillon, le compte de ce qui attend) est poussé aux clients, qui le rendent chez eux ; le clic d'une notification revient au serveur, qui garde l'action de ses 64 dernières. Ce qu'il demanderait d'ouvrir (un lien, un de ses fichiers) n'est jamais ouvert par un client sans geste.
- Il sert l'API distante sur **127.0.0.1** (port 46480, ou `--port`), ou sur **une socket Unix** (`--socket`, voir *La socket Unix*), par défaut pour le service d'`install.sh`. On l'atteint par un tunnel SSH (`ssh -L <port local>:<socket> <hôte>`, ou `ssh -L <port local>:127.0.0.1:46480 <hôte>`).
- Il s'arrête proprement à **SIGTERM** (systemd, `docker stop`) ou Ctrl+C : l'API se ferme, les tours en cours des agents sont coupés (chacun reprend sa session au démarrage suivant, comme quand on quitte l'app), l'état est écrit, le dossier est lâché, sa socket retirée.
- Il dit ce qu'il fait dans `server-status.json`, dans son dossier : son processus, sa version, l'adresse de son API (et sa socket, `socket`, quand il écoute sur une), ce qui le tient occupé (un tour, une question, une tâche de fond, un lancement, un terminal servi depuis moins de 5 minutes : ce qui retient aussi le redémarrage automatique de l'app), et où en est sa mise à jour (`update`, voir *La mise à jour automatique*), réécrit toutes les 5 secondes et aussitôt que cela change, retiré à l'arrêt propre. `health`, `status` et `update` le lisent. Il ne contient aucun secret.
- **Au premier démarrage**, tant qu'aucun appareil n'est appairé, le journal dit comment en appairer un (« lance « escouade-server pair » sur ce serveur », « docker exec <conteneur> escouade-server pair » dans l'image), jamais le code.

## Ses commandes

| Commande | Ce qu'elle fait |
| --- | --- |
| `escouade-server` ou `escouade-server run` | Le serveur, jusqu'à SIGTERM |
| `escouade-server pair` | Un code d'appairage pour un nouvel appareil ; sur une socket, il dit aussi la socket et le tunnel à la main |
| `escouade-server devices` | Les appareils appairés : id, nom, date d'appairage, dernière connexion |
| `escouade-server devices revoke <id>` | Révoque un appareil : son jeton ne vaut plus ; un serveur qui tourne ferme ses connexions en quelques secondes |
| `escouade-server health` | Code 0 si le serveur du dossier tourne, écrit son état depuis moins de 30 s et, sur une socket, que sa socket est là ; 1 sinon, avec la raison : le `HEALTHCHECK` de l'image |
| `escouade-server status [--json]` | Ce que dit le serveur du dossier de lui-même : s'il tourne, sa version, où écoute son API (`api`, `socket`), rien d'autre ; ce que lit le client de bureau par SSH (ESC-706) |
| `escouade-server update` | Met le serveur à une version signée (voir *La mise à jour*) : `--to X.Y.Z` (avec `--downgrade` pour une plus ancienne), `--beta`, `--now`, `--check` |
| `escouade-server config` | La mise à jour automatique et son canal (voir *La mise à jour automatique*) : sans argument, ce qui est réglé ; `automatic on` ou `off`, `channel stable` ou `beta` |
| `--data-dir <dossier>`, `--port <n>`, `--listen <adresse>`, `--socket <chemin>`, `--allow-host <nom>`, `--trust-proxy <adresse>`, `--browser`, `--debug`, `--version`, `--help` | Les options (`--browser` : voir *Le client navigateur*) |

Chaque option a sa variable d'environnement, pour un conteneur ; l'option donnée l'emporte, une variable vide compte pour absente : `ESCOUADE_SERVER_DATA`, `ESCOUADE_SERVER_PORT`, `ESCOUADE_SERVER_LISTEN`, `ESCOUADE_SERVER_SOCKET`, `ESCOUADE_SERVER_ALLOW_HOSTS` (des noms séparés par des virgules), `ESCOUADE_SERVER_TRUST_PROXIES` (des adresses IP séparées par des virgules), `ESCOUADE_SERVER_BROWSER` (`1`, `true`, `on` ou `yes`). C'est ce qui fait que `docker exec <conteneur> escouade-server pair` vise le dossier du serveur du conteneur.

`--help` n'exécute jamais une commande à sa place (`pair --help`, `update --help` montrent l'aide, sans faire de code ni rien télécharger). `--to`, `--beta`, `--now`, `--check` et `--downgrade` ne vont qu'avec `update`, `--downgrade` qu'avec `--to` (« --downgrade va avec --to X.Y.Z : la version plus ancienne se nomme »), et `--to` veut une version (`X.Y.Z`, `X.Y.Z-beta.N`). `--json` ne va qu'avec `status`. Les commandes parlent la langue des réglages du dossier de données, sinon celle du système. `pair`, `devices`, `status` et `config` marchent que le serveur tourne ou non.

## La socket Unix (ESC-706)

Sur une machine où d'autres comptes existent, un port de la boucle locale est joignable par tous : ils peuvent tenter des codes, et surtout **prendre le port pendant que le serveur redémarre** (une mise à jour, une panne) ; le client de bureau, revenu par son tunnel, enverrait alors son jeton à l'usurpateur. Une socket dans un dossier qui n'est qu'à l'utilisateur ferme les deux.

- **`--socket <chemin>`** (`ESCOUADE_SERVER_SOCKET`) à la place du port. `--socket` et `--port` ou `--listen` s'excluent (« --socket et --port (ou --listen) s'excluent ») ; donnée sur la ligne de commande, l'une l'emporte sur la variable de l'autre. **Refusée sous Windows**, en le disant (« pas de socket Unix sous Windows ; le serveur y écoute sur un port (--port) »). Un chemin que `ssh -L` ne pourrait pas joindre est refusé : relatif, sans nom, avec un deux-points ou un caractère de contrôle, plus long que 103 octets.
- **Son dossier** est créé en `0700` ; un dossier déjà là est refusé s'il n'est pas à l'utilisateur, s'il est ouvert à d'autres (« chmod 700 ») ou s'il est un lien.
- **Chaque dossier au-dessus**, tel que le chemin le dit et tel qu'il est vraiment, appartient à l'utilisateur ou à root, et personne d'autre ne peut le modifier (aucun droit d'écriture du groupe ni des autres), sauf un dossier collant de root (`/tmp`). Un dossier refusé l'est avant que rien ne soit créé dessous.
- **Un verrou la tient** (`<socket>.lock`) : tenu par un autre, « un autre serveur écoute sur … : un seul à la fois ». Libre, une socket laissée là par un serveur tué (rien n'y répond) est remplacée ; une socket où quelque chose répond est gardée, avec le même refus. Ce qui n'est pas une socket à ce chemin n'est jamais retiré.
- **La socket est en `0600`**, liée avant que le cœur démarre (un refus ne démarre rien) et retirée à l'arrêt propre, si c'est toujours la sienne.
- **Ses connexions** sont celles de la machine, comme celles de la boucle locale : le contrôle de `Host` et d'`Origin`, l'appairage et ses bornes de *L'API distante* y valent tels quels.
- **Ce qui la suit** : `server-status.json` et `status --json` disent la socket ; `health` vérifie qu'elle est là (sans s'y connecter : une connexion vide compterait comme une poignée de main ratée) ; `pair` la dit, avec le tunnel à la main (`ssh -N -L 46480:<socket> <ce serveur>`) ; `devices`, `devices revoke`, `phone` et `update` passent par le dossier de données comme sur un port, et le service relancé reprend sa socket.
- L'**image Docker**, le Compose d'exemple et la recette Coolify **restent sur le port** : une socket ne traverse pas la frontière d'un conteneur sans monter un dossier de l'hôte, et un proxy parle TCP.

## L'appairage d'un appareil

1. Sur le serveur : `escouade-server pair` affiche un code, par exemple `8NUC-W6HS`, valable **une fois** et **10 minutes**. Un nouveau `pair` remplace le code précédent. Le client peut aussi le lancer lui-même par SSH (« le lancer d'ici par SSH »).
2. Dans le client, l'utilisateur donne ce code et un nom pour l'appareil (« Portable du bureau »).
3. Le client l'envoie à l'API **à la place de la poignée de main**. Le serveur répond le **jeton d'appareil**, donné une seule fois, que le client garde et redonne à chaque connexion.

Les refus, chacun avec sa raison pour le client (`none`, `expired`, `wrong`, `wait`) :

- **expiré** : « Ce code d'appairage a expiré : lance « escouade-server pair » de nouveau. » ;
- **déjà utilisé, ou jamais demandé** (un code redonné) : « Aucun code d'appairage n'attend… » ;
- **faux** : « Code d'appairage faux. » ; **le code qui attend vaut toujours** : un code faux ne l'annule jamais (ESC-654 : cinq codes faux de n'importe qui le finissaient, et bloquaient l'appairage à volonté) ;
- **trop d'essais dans la minute** : au-delà de 20 poignées de main ratées d'une même source (son adresse, le pair du tunnel du téléphone, ou le client d'un proxy déclaré), ou de 120 codes faux de toutes les sources venues d'ailleurs que la machine, dans la minute, le code de cette source (ou venu d'ailleurs) n'est pas lu (« Trop d'essais faux en une minute : le code d'appairage vaut toujours, réessaie dans une minute. »). Les codes tapés **sur la machine** (`escouade-server pair` puis la boucle locale ou la socket, un client par SSH) ont leur propre plafond : des codes faux venus du réseau ne les refusent jamais, et la fenêtre dit quand ce plafond-là est atteint (ESC-631). C'est ce qui tient la force brute : au plus 2 400 codes lus pendant les 10 minutes d'un code, sur 2⁴⁰, une chance sur 450 millions environ.

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
- **Le réseau** : elle écoute **sur le port**, jamais sur une socket (voir *La socket Unix*). Sans `ESCOUADE_SERVER_LISTEN`, l'API n'écoute que sur 127.0.0.1 **du conteneur** : rien n'est joignable du dehors, même un port publié. L'image n'expose aucun port. `ESCOUADE_SERVER_LISTEN=0.0.0.0` l'ouvre au réseau du conteneur, pour un port publié sur 127.0.0.1 de l'hôte seulement (le tunnel SSH vers l'hôte) ou un proxy TLS devant. **L'API y est alors en clair** (HTTP et WebSocket sans TLS) : tout autre conteneur de ce réseau (sur Coolify, les autres ressources du même réseau) l'atteint et peut tenter des codes d'appairage et des jetons, dans les bornes de *L'appairage*. Le TLS est celui du proxy ou du tunnel : le Dockerfile, les deux exemples et la documentation le disent, et conseillent le proxy TLS ou le tunnel, jamais un port publié sur une adresse publique.
- **Le client navigateur** y est éteint : `ESCOUADE_SERVER_BROWSER=1` l'allume (voir *Le client navigateur*). Le Dockerfile et les deux exemples le laissent éteint et le disent en commentaire.
- **Sa santé** : `HEALTHCHECK` lance `escouade-server health` (toutes les 30 s, 120 s laissées au démarrage).
- **Son arrêt** : `docker stop` (SIGTERM) arrête proprement ; les exemples donnent 60 s.
- **Sa mise à jour** est une nouvelle image : `escouade-server update` refuse de tourner dans l'image (il le sait par `ESCOUADE_SERVER_IMAGE`) et dit quoi faire ; la mise à jour automatique n'y remplace rien et dit seulement qu'une nouvelle image existe (voir *La mise à jour automatique*).
- **Les limites conseillées**, d'après une mesure sur l'image x64 : au repos, les plugins chargés, environ 235 Mo et 106 tâches (processus et threads) ; un agent du faux `claude` des tests en plein tour en ajoute environ 18 Mo et 7. Un vrai harness pèse plus. Les exemples fixent 4 Go de mémoire et 2048 tâches, pour quelques agents à la fois.

## Compose et Coolify

- `deploy/docker-compose.yml` : l'image **sur sa version mineure** (`:2.3`, jamais `latest` : un `docker compose pull` prend les correctifs de la 2.3, jamais la 2.4 sans qu'on change la ligne ; ESC-640, SR1-12), `ESCOUADE_SERVER_LISTEN=0.0.0.0` et ce qu'il expose, le port publié sur **127.0.0.1 de l'hôte seulement** (`127.0.0.1:46480:46480`), le volume `escouade-data`, la lecture seule, aucune capacité, `no-new-privileges`, 60 s pour s'arrêter, les limites. L'appairage : `docker compose exec escouade escouade-server pair`.
- `deploy/coolify/docker-compose.yml` : le même, tel que Coolify le prend (une ressource « Docker Compose »). Tel quel, rien ne va sur Internet. En commentaire, l'accès par un domaine derrière le proxy de Coolify (Traefik, en TLS) : `SERVICE_FQDN_ESCOUADE_46480`, `ESCOUADE_SERVER_ALLOW_HOSTS=${SERVICE_FQDN_ESCOUADE}` et `ESCOUADE_SERVER_TRUST_PROXIES` (l'adresse du proxy de Coolify sur ce réseau), avec ses risques.

Aucun exemple n'expose l'API sans TLS sur Internet.

## Les deux accès, et leurs risques

- **Le tunnel SSH** (conseillé), que le client ouvre lui-même (voir *Se connecter à un serveur*) : vers la socket du serveur (`ssh -N -L 46480:<socket> <serveur>`) ou vers son port (`ssh -N -L 46480:127.0.0.1:46480 <serveur>`). Rien de nouveau n'est ouvert sur le serveur. Risque, sur un port : qui a un compte sur le serveur, ou un processus de l'hôte, atteint le port et peut tenter de s'appairer, ou le prendre pendant un redémarrage ; sur la socket, ni l'un ni l'autre. L'accès vaut ce que vaut le SSH.
- **Un proxy TLS devant** (Traefik de Coolify, Caddy, nginx), sur le port : le serveur écoute sur le réseau du conteneur et accepte le nom du domaine dans `Host` (`--allow-host`, `ESCOUADE_SERVER_ALLOW_HOSTS`), tout autre nom restant refusé. **Le proxy se déclare** (`--trust-proxy <son adresse>`, `ESCOUADE_SERVER_TRUST_PROXIES`, ESC-631) : chaque client qu'il fait suivre compte alors ses échecs à part, d'après la dernière adresse de `X-Forwarded-For` (celle que le proxy écrit) ; sans cela, toutes ses connexions comptent comme une seule source. L'en-tête n'est jamais lu d'une adresse qui n'est pas déclarée. Risque : l'API est sur Internet ; qui atteint le domaine peut tenter un code (40 bits, 10 minutes, une fois, 120 essais lus par minute au plus, sans jamais bloquer un code tapé sur le serveur ou par SSH) ou un jeton (256 bits), chaque essai journalisé et borné ; une faille du serveur serait exposée à tous. À réserver à un domaine filtré par adresse IP ou à un réseau privé, et jamais sans TLS. Le client se connecte aujourd'hui par le tunnel SSH qu'il ouvre (voir *Se connecter à un serveur*) ; une adresse `wss://` d'un domaine viendra plus tard.

## Le service systemd

`install.sh` (dans `deploy/`, et à chaque release), pour Linux x64 ou arm64 avec systemd :

- il télécharge l'archive de la release (la dernière, ou `--version`), vérifie sa **SHA-256** et sa **signature** (la clé de l'updater de l'app ; avec `minisign`, sinon `openssl` 3) : une archive refusée ne change rien ;
- par défaut, un **service de l'utilisateur** (`systemctl --user`, `loginctl enable-linger` pour qu'il tourne sans session) ; avec `--system`, un **service du système** lancé par l'utilisateur `escouade` (créé s'il manque ; `--user` en choisit un autre), à qui le programme appartient pour qu'il se mette à jour lui-même ;
- **son écoute** (ESC-706) : par défaut **la socket**, `%t/escouade/server.sock` dans l'unité du service de l'utilisateur (`$XDG_RUNTIME_DIR/escouade/server.sock`), `/run/escouade-server/server.sock` pour celui du système (`RuntimeDirectory=escouade-server`, `RuntimeDirectoryMode=0700`, à l'utilisateur du service). `--port <n>` garde un port de 127.0.0.1, pour un proxy ou pour le repli quand le serveur SSH refuse la redirection vers une socket ; `--socket` y revient. **Relancé sur une installation existante sans `--socket` ni `--port`, il garde l'écoute de l'unité déjà là et le dit** (« Listening kept as the service there does: … ») : une installation d'avant reste sur son port, son unité inchangée ;
- le service : `Restart=always` (après une panne, après `update` et après la mise à jour automatique), `KillMode=mixed` (SIGTERM au seul serveur, qui arrête ses agents lui-même), 90 s pour s'arrêter, `UMask=0077`, `NoNewPrivileges` ;
- **une bêta installée** (`--version X.Y.Z-beta.N`) met le serveur sur le canal bêta (`escouade-server config channel beta`, une fois le service relancé), et le dit ;
- **idempotent** : relancé, il ne change que ce qui diffère, et ne redémarre pas un serveur à la même version ;
- **désinstallable** : `--uninstall` retire le service et le programme et garde les données ; `--uninstall --purge` retire aussi les données (avec `--system`, l'utilisateur et tout son dossier) ;
- à la fin, il dit où écoute le service, comment appairer, que le client ouvre lui-même le tunnel (et le tunnel à la main), la mise à jour (automatique, `config` pour la régler, `update` pour en faire une tout de suite), et que les connexions des comptes vivent sur ce serveur.

## La mise à jour

`escouade-server update` :

1. la version : `--to X.Y.Z` (celle que propose le client), sinon celle de `latest.json` de la dernière release ; sur le canal bêta (les réglages du serveur, `betaUpdates`, ou `--beta`, qui l'y met pour de bon et le dit), **la plus haute** de `latest.json` et de `beta.json` (comme l'app avec « Recevoir les versions bêta » : une stable sortie après la dernière bêta l'emporte ; sans `beta.json`, celle de `latest.json`) ; `--check` la dit sans rien faire ;
2. **jamais une version plus ancienne que la sienne** (ESC-640, SR1-07 : un miroir de `ESCOUADE_SERVER_UPDATE_URL`, ou qui peut changer `latest.json`, servirait une ancienne version signée et vulnérable) : « la version publiée (1.0.0) est plus ancienne que ce programme (2.3.0) : rien n'est changé ». Une bêta compte avant sa version (2.3.0-beta.4 est plus ancienne que 2.3.0) : un serveur sur une bêta y reste jusqu'à la stable qui la suit. Revenir en arrière se fait en la nommant, `--to X.Y.Z --downgrade` ; `--to` seul vers une plus ancienne est refusé et dit comment faire ; la même version ne télécharge rien ;
3. l'archive de cette version pour l'architecture et sa signature, **vérifiée avec la clé de l'updater de l'app** avant de rien toucher, et **liée à son nom** : le commentaire de confiance de la signature (signé avec elle, `timestamp:…<tab>file:<nom>` tel que l'écrit le signataire de Tauri) doit nommer l'archive attendue, `file:escouade-server-<version>-linux-<arch>.tar.gz`. L'archive signée d'une autre version servie sous ce nom est refusée, même si ses octets sont bien signés ; une signature qui ne tient pas : « signature refusée … rien n'est changé » ;
4. chaque fichier de l'archive (le programme, l'amorce) posé à la place du sien, l'ancien gardé en `.old` ;
5. le serveur qui tourne est arrêté **une fois au repos** (aucun tour, aucune question en attente, aucune tâche de fond, aucun lancement, aucun terminal servi depuis 5 minutes) ; dans un terminal, il propose de redémarrer tout de suite en coupant les tours ; `--now` le fait sans attendre ; systemd le relance sur la nouvelle version (sur sa socket ou son port), et `update` le dit. Sans superviseur, il dit de le relancer ;
6. dans l'image Docker, il refuse : la mise à jour est une nouvelle image.

L'app de bureau a la même faiblesse de principe, bornée autrement : voir *Livraison et mises à jour*, « Ce que couvre la signature ».

## La mise à jour automatique (ESC-702)

Le serveur se met à jour **tout seul**, par les fonctions mêmes de `escouade-server update` (la version visée, le téléchargement et sa signature liée au nom de l'archive, la pose avec les `.old`) : aucun chemin à part.

- **Quand** : 2 minutes après son démarrage (les agents repris, les plugins chargés d'abord), puis **toutes les 15 minutes**. L'app cherche toutes les 5 minutes, et le serveur refuse un client d'une autre version : un quart d'heure garde cet écart court, pour un JSON de quelques centaines d'octets ; l'archive ne se télécharge qu'une fois, quand elle est neuve. Il cherche aussi tout de suite quand un de ses deux réglages change, quand un client plus récent est refusé à la poignée de main (avec un jeton d'appareil valide seulement), et quand un administrateur le demande (ci-dessous). Un build de développement ne lit aucune vraie release de lui-même : seulement celle de `ESCOUADE_SERVER_UPDATE_URL` (comme l'app, voir *Livraison et mises à jour*).
- **Son canal et son interrupteur** sont deux réglages du serveur, ceux de l'app : **« Installer les mises à jour automatiquement » (`autoUpdate`), allumé par défaut comme dans l'app**, et « Recevoir les versions bêta » (`betaUpdates`), éteint par défaut. `install.sh --version X.Y.Z-beta.N` et `escouade-server update --beta` mettent le canal bêta ; `escouade-server config automatic on|off` et `config channel stable|beta` les changent (le serveur qui tourne les prend lui-même, dans les 2 secondes, sous ses stratégies ; arrêté, son fichier de réglages est écrit, le reste gardé tel quel) ; un client les change dans Réglages › Général de l'espace du serveur. **Les stratégies** du poste et d'une équipe qui couvrent le serveur les bornent comme pour l'app : `updates.automatic` à `false` éteint l'automatique (une version est dite, jamais posée seule), `updates.channel` à `stable` garde le canal stable.
- **Ce qu'il fait** d'une version de son canal plus récente que lui, sous systemd sous Linux et l'automatique allumé : il la télécharge, vérifie sa signature, la pose, puis attend **le repos** (aucun tour, aucune question, aucune tâche de fond, aucun lancement, aucun terminal servi depuis 5 minutes, ce qui retient le redémarrage de l'app) ; au repos, il note le redémarrage (`server-update.json`), s'arrête comme à SIGTERM (les agents reprennent leurs sessions) et systemd le relance. Au démarrage suivant, il dit qu'il a redémarré sur la nouvelle version ; s'il est encore à l'ancienne, il dit que la version n'a pas pris et ne la réessaie plus de lui-même (pas de boucle de redémarrages).
- **Jamais plus ancien** que lui : une version publiée plus ancienne vaut « rien de plus récent ». **Une archive dont la signature ne tient pas** ne change rien et n'est plus jamais retéléchargée d'elle-même.
- **L'automatique éteint, sans systemd, ou hors de Linux** : il dit la version trouvée et la commande (`escouade-server update --to X.Y.Z`), ne pose rien.
- **Dans l'image**, rien n'est remplacé : il dit qu'une nouvelle image existe, avec la commande `docker pull ghcr.io/…/escouade-server:<version> && docker compose up -d`, dans son journal, dans `server-status.json` et au client.
- **Le journal** dit chaque version trouvée, vérifiée, posée, l'attente du repos et ce qui le retient, l'arrêt pour la mise à jour et le redémarrage, et chaque refus avec sa raison (une version plus ancienne, une signature refusée, un téléchargement ou une pose ratés, une version qui n'a pas pris), une fois par version.

## « Mettre à jour le serveur » (ESC-702)

- **Sur le refus de version**, quand c'est le serveur à mettre à jour (voir *L'API distante*, `serverUpdate`), l'espace du serveur dit sa version et celle du poste, et offre **« Mettre à jour le serveur »**. Après une confirmation qui dit ce qui suit (le téléchargement, la signature, le redémarrage au repos, aucun agent coupé), le poste le demande au serveur par son jeton, à la place de la poignée de main ; le serveur se met à la version du poste, jamais plus ancienne que la sienne, par la mise à jour automatique (même s'il a l'automatique éteint), puis redémarre au repos. L'espace dit que c'est demandé et retente la connexion toutes les 30 secondes. La commande à lancer à la main sur le serveur reste affichée, à copier, avec « Réessayer ».
- **Réservée à un poste** : seul un appareil de la portée `computer` (l'administrateur du serveur, qui l'a appairé) le demande ; un téléphone est refusé (`scope`), un jeton inconnu aussi (`token`), sous les stratégies du serveur (`servers.remoteApi`).
- **Une erreur se dit une fois** (dans l'image, sans systemd, une version plus ancienne, un jeton refusé), avec le bouton pour réessayer à la main : jamais de nouvelle demande d'elle-même.
- **Dans l'image**, l'écran donne la commande `docker pull` à recopier sur le serveur, sans bouton qui changerait le serveur.
- **Connecté**, Réglages › Général d'un espace de serveur montre **les mises à jour du serveur** à la place de celles du poste : sa version, la version trouvée et où elle en est (posée, en attente du repos et ce qui le retient, en redémarrage, une nouvelle image et sa commande, une erreur), « Mettre à jour le serveur » avec la même confirmation quand une version attend et qu'il peut la poser, et ses deux réglages, sous ses stratégies.
- **Le téléphone** n'y a pas accès, et sa boîte « À toi » ne dit pas la version du serveur.

## La release

Le workflow de release construit d'abord la page du client navigateur (`npm run build:web`), puis le programme qui l'embarque ; l'archive d'un `escouade-server` sans la page est refusée. Il publie, pour chaque version : les archives `escouade-server-<version>-linux-x64.tar.gz` et `-linux-arm64.tar.gz` (le programme, l'amorce des plugins, la licence), leurs SHA-256, leurs signatures (la clé de l'updater, comme l'app ; leur commentaire de confiance nomme l'archive), `install.sh`, et l'image sur ghcr.io. Chaque architecture se construit sur un runner à elle ; la signature est vérifiée aussitôt faite ; l'image de chaque architecture est poussée par son empreinte seule, puis, une fois la release publiée, les deux sont réunies sous la version, sa mineure et `latest` (une bêta : sa version et `beta`). Les archives et `install.sh` ont une attestation de provenance comme les installeurs de l'app ; l'image a la sienne, gardée au registre (`gh attestation verify oci://…`).

## La sécurité

- L'API n'écoute que sur 127.0.0.1, ou sur une socket Unix dans un dossier qui n'est qu'à l'utilisateur (voir *La socket Unix*). `--listen <adresse>` la met sur une autre adresse. Cette option ne sert qu'à Docker, derrière un tunnel, un réseau privé ou un proxy. Elle avertit dans le journal et sur la sortie d'erreur : « ATTENTION : l'API distante écoute sur … pas seulement sur 127.0.0.1 … jamais ouvert sur Internet ». Rien n'est ouvert par défaut.
- `--allow-host` n'ajoute qu'un nom au contrôle de `Host` ; il n'ouvre aucune interface. `--trust-proxy` ne change que la source à qui sont comptés les échecs ; il n'ouvre rien non plus, et le journal dit au démarrage quels proxys sont déclarés.
- Les stratégies imposées valent pour le serveur : `servers.remoteApi` à `false` l'empêche de servir l'API, et refuse aussi l'appairage et la demande de mise à jour ; `updates.automatic` et `updates.channel` bornent sa mise à jour automatique.
- Tout le reste de *L'API distante* vaut ici : le `Host` du bouclage, pas d'`Origin` de page web (sauf celle de la page du client navigateur, quand `--browser` l'allume), les bornes des connexions, des appels et des files.

## Le journal

Chaque ligne va sur la **sortie standard**, pour le journal de systemd et les logs de Docker. Elle va aussi dans **`server.log`**, dans le dossier de données, qui tourne à 5 Mo (`server.1.log` à `server.3.log`, le plus ancien supprimé). Sous Unix, ces fichiers sont **lisibles de leur seul propriétaire** (0600) quel que soit l'`umask`, et ceux qu'une version d'avant avait laissés plus ouverts y sont remis à l'ouverture du journal (ESC-640, SR1-13 : il nomme des appareils et des projets). Le journal dit :

- le démarrage, l'adresse de l'API (ou sa socket), le dossier de données et celui des secrets, les noms pris en plus dans `Host` et les proxys déclarés, la page du client navigateur quand elle est servie ; sans appareil appairé, comment en appairer un ;
- chaque socket morte remplacée ;
- chaque appareil appairé, chaque connexion ;
- chaque étape de sa mise à jour automatique (voir plus haut) et chaque demande d'un administrateur ;
- chaque refus et sa raison ;
- l'arrêt et ce qui l'a demandé.

Il ne dit jamais de jeton, de code, de secret ni ce qu'écrit un terminal. `--debug` ajoute le détail d'Escouade.

## Le client navigateur (BC-1, ESC-704)

`escouade-server` peut servir l'interface d'Escouade à un navigateur, pour qui ne peut pas installer Escouade (un poste verrouillé, une tablette, un Chromebook). Ce ticket sert la page et tient sa surface ; se connecter depuis elle (l'appairage, l'identité de l'appareil) vient avec BC-2.

- **Éteint par défaut**, allumé par `--browser` ou `ESCOUADE_SERVER_BROWSER=1` (décision 357). **Éteint, rien ne change** : aucune page, aucune `Origin` web acceptée, aucune requête lue avant la WebSocket.
- **La page est dans le programme** : l'interface construite sans Tauri (`npm run build:web`, `dist-web/`), chaque fichier compressé, embarquée dans `escouade-server` seul (jamais dans l'app de bureau). Aucun chemin du disque n'est servi ; la page et le serveur ont toujours la même version, mise à jour d'un coup. Elle ajoute environ 12,7 Mo au programme (33 Mo de fichiers, 12,6 Mo compressés : les images de What's up ? et de la visite comptent le plus). Un `escouade-server` construit sans elle (un build de développement) refuse `--browser` et dit pourquoi.
- **La build sans Tauri** : chaque module `@tauri-apps/*` y est remplacé par un bouchon qui **refuse**, d'une erreur nommée (`TauriUnavailableError`), jamais un `null` qui ferait croire qu'un appel a marché ; un module ou un nom sans bouchon fait échouer la build, comme un paquet qui garderait `__TAURI_INTERNALS__` ou `ipc.localhost`. Ce que la fenêtre fait par Tauri, le navigateur le fera par ses moyens (BC-2, BC-3), ou dira qu'il ne le peut pas.
- **Comme la fenêtre** : la page a le même dossier public que la fenêtre, copié tel quel : What's up ? (la fenêtre des nouveautés d'une version, que charge `index.html`, et ses notes), les images de la visite des fonctionnalités, le logo. What's up ? y attend que l'interface soit prête, comme dans la fenêtre, et tourne sous la même politique de sécurité (son script vient de la page, son `<style>` prend le nonce).
- **Servie sur le port de l'API**, par sa socket seulement (jamais par le tunnel du téléphone ni Tailscale) : `GET /` (`index.html`) et `GET /<chemin>` de chaque autre fichier de la page (`assets/…`, `whats-up/…`, `features/…`, `logo.svg`), `HEAD` des mêmes ; tout autre chemin répond 404 (un chemin est cherché tel quel : ni `..`, ni encodage, ni segment caché, la casse comptée ; `/index.html` n'est pas une autre adresse de la page), une autre méthode 405. La connexion se ferme après chaque réponse. Une requête de WebSocket va à l'API comme avant.
- **Chaque réponse porte** la politique de sécurité de la fenêtre, sans les schémas de Tauri : `style-src 'self' 'nonce-…'` avec un nonce **tiré pour cette réponse** et posé sur les `<style>` de la page, `style-src-attr 'unsafe-inline'`, `script-src 'self' 'wasm-unsafe-eval'`, `connect-src 'self'` et la WebSocket de la page (`ws://<Host>` sur la boucle locale, `wss://<nom>` derrière un proxy), `object-src`, `base-uri`, `form-action` et `frame-ancestors` à `'none'` ; et `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin`, `Permissions-Policy` qui ferme le micro, la caméra et la géolocalisation. `assets/` est gardé un an (`immutable` : leurs noms changent avec leur contenu) ; le reste n'est jamais gardé sans revalidation (`no-cache`), avec un `ETag` qui lui évite d'être renvoyé s'il n'a pas changé (`304`), sauf `index.html` dont le nonce change à chaque réponse ; un refus n'est jamais gardé (`no-store`). Aucune réponse n'ouvre le CORS, aucune ne pose de cookie.
- **La marque de la page servie** : `index.html` porte `<meta name="escouade-served" content="escouade-server">`, que la page lira pour se savoir servie (BC-2) ; jamais de l'absence de Tauri.
- **Par où l'atteindre** : le tunnel SSH (`ssh -L 8765:127.0.0.1:46480 <serveur>`, puis `http://127.0.0.1:8765`), ou un proxy TLS déclaré par `--allow-host <nom>` (`https://<nom>` ; décision 358). **`http://` d'un nom est refusé**, sans repli : un proxy sans TLS ferait passer l'interface et la preuve de l'appareil en clair. Les origines acceptées et refusées sont dans *L'API distante*.

## Pas encore

- Une adresse `wss://` d'un domaine dans le client.
- Les serveurs de langage de l'éditeur, encore refusés à distance.
- Dans le client navigateur (BC-2 à BC-9) : se connecter et s'appairer depuis la page, ce qui change de machine, les plugins, `install.sh --browser`, l'e2e, la documentation du site, la revue de sécurité.

## Le tunnel du téléphone sur un serveur (ESC-567)

Un serveur sert aussi le téléphone par le tunnel WireGuard intégré (voir *Le tunnel du téléphone*), sur la même API, qu'il écoute sur un port ou sur sa socket. Il s'allume depuis Réglages de l'espace › Téléphone dans un client connecté, ou dans son terminal :

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

## Le réseau privé sur un serveur (RP-1, RP-4)

Un serveur peut faire joindre à ses agents le réseau privé d'une entreprise par ses connecteurs (voir *Atteindre un réseau privé*). Rien n'est ouvert par défaut.

| Commande ou option | Ce qu'elle fait |
| --- | --- |
| `--gate` (`ESCOUADE_SERVER_GATE`) | La porte seule : le seul proxy des agents, sur 127.0.0.1 |
| `--connector-listen <adresse:port>` (`ESCOUADE_SERVER_CONNECTOR_LISTEN`) | L'entrée des connecteurs, et la porte avec elle : un port à part, qui ne parle que le protocole des connecteurs, jamais l'API distante |
| `--connector-cert <fichier>`, `--connector-key <fichier>` (`ESCOUADE_SERVER_CONNECTOR_CERT`, `ESCOUADE_SERVER_CONNECTOR_KEY`) | Son certificat TLS, en PEM ; sans eux, elle se sert derrière un proxy TLS (Traefik, Caddy) |
| `--connector-url wss://…/connector` (`ESCOUADE_SERVER_CONNECTOR_URL`) | Son adresse telle que les connecteurs la joignent, que donne l'appairage |
| `escouade-server connector pair <nom>` | Le code d'appairage d'un connecteur : 8 lettres, 10 minutes, une fois ; avec l'adresse de l'entrée |
| `escouade-server connectors` (`list`) | Les connecteurs, leur état, leur version, leur plafond ; puis ceux retirés |
| `escouade-server connectors remove <nom ou id>` | Retire un connecteur : sa clé ne vaut plus ; un serveur qui tourne ferme son canal dans les deux secondes |

- Comme `pair` et `devices`, ces commandes marchent que le serveur tourne ou non.
- `secrets.json` garde aussi la clé du serveur pour les connecteurs, la clé publique de chacun et les codes qui attendent (leur clé prépartagée, jamais le code) ; `connectors.json`, à côté (0600), leur liste et la trace des retirés, sans aucune clé.
- Le journal dit l'entrée ouverte, son adresse et si elle a son propre TLS, chaque connecteur appairé, connecté ou coupé, chaque code faux et sa source ; jamais un code ni une clé.
- La stratégie `servers.privateNetwork` à `false` posée sur le serveur ferme l'entrée, ne laisse pas la porte s'ouvrir au démarrage et refuse l'appairage.
