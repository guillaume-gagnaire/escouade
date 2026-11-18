---
title: "Le mode serveur"
subject: "a-distance"
sources: ["docs/REMOTE.md, « Le mode serveur », « Le mode serveur, MS-1 (ESC-514) », « Tâches › Mode serveur »","docs/CRATES.md, § 14 (le cœur sans Tauri)","src-tauri/src/server (mod.rs, secrets.rs, logs.rs), src-tauri/src/bin/escouade-server.rs","src-tauri/src/remoteapi/pairing.rs, devices.rs, server.rs, proto.rs","crates/escouade-tauri-shim","scripts/server-libs.sh, scripts/server-trial.sh, .github/workflows/ci.yml (Server (Linux))","tests/e2e/server.spec.ts, tests/e2e/server.ts"]
tickets: ["ESC-514","ESC-537"]
decisions: [202]
compared: null
---

# Le mode serveur

Escouade tourne sur une machine sans bureau (un VPS, un conteneur), H24, et le client de bureau la pilote à distance par l'API distante (voir *L'API distante*), à travers un tunnel. Le cœur (agents, harness, worktrees, Kanban, terminaux, serveur MCP) tourne sur le serveur ; le client n'en est qu'une vue : le fermer, éteindre le poste ou couper le réseau ne touche à aucun agent.

Ce que livre la première étape (MS-1) : le programme du serveur, `escouade-server`. Le client qui s'y connecte, par un tunnel SSH qu'il ouvre lui-même, est décrit dans *Se connecter à un serveur* (MS-3). Le service systemd, l'image Docker et la mise à jour signée du serveur viennent ensuite.

## `escouade-server`

- Un binaire pour **Linux x64 et arm64**, qui ne lie **ni WebKit ni GTK** (ni D-Bus, ni le son, ni X11 ou Wayland) : il démarre et tourne sans aucun affichage, dans un conteneur Debian nu. La CI le construit pour les deux architectures, vérifie ses bibliothèques et le démarre dans un tel conteneur.
- Il se construit aussi sous Windows et macOS, pour les essais ; seul Linux est livré.
- Il prend le **dossier de données** (`~/.escouade`, ou `--data-dir`) et le tient : un second Escouade sur le même dossier est refusé, avec un message clair (« Escouade tourne déjà sur ce dossier de données (…) : un seul à la fois. »).
- Il charge le cœur comme l'app de bureau (projets, agents, Kanban, Git, plugins, comptes), sans fenêtre : ce que le bureau montrerait (une notification, un carillon, le compte de ce qui attend, un lien à ouvrir) est poussé aux clients, qui le rendent chez eux.
- Il sert l'API distante sur **127.0.0.1** (port 46480, ou `--port`). On l'atteint par un tunnel (`ssh -L <port local>:127.0.0.1:46480 <hôte>`).
- Il s'arrête proprement à **SIGTERM** (systemd, `docker stop`) ou Ctrl+C : l'API se ferme, les tours en cours des agents sont coupés (chacun reprend sa session au démarrage suivant, comme quand on quitte l'app), l'état est écrit, le dossier est lâché.

## Ses commandes

| Commande | Ce qu'elle fait |
| --- | --- |
| `escouade-server` ou `escouade-server run` | Le serveur, jusqu'à SIGTERM |
| `escouade-server pair` | Un code d'appairage pour un nouvel appareil |
| `escouade-server devices` | Les appareils appairés : id, nom, date d'appairage, dernière connexion |
| `escouade-server devices revoke <id>` | Révoque un appareil : son jeton ne vaut plus ; un serveur qui tourne ferme ses connexions en quelques secondes |
| `--data-dir <dossier>`, `--port <n>`, `--debug`, `--version`, `--help` | Les options |

`--help` n'exécute jamais une commande à sa place (`pair --help` montre l'aide, sans faire de code). Les commandes parlent la langue des réglages du dossier de données, sinon celle du système. `pair` et `devices` marchent que le serveur tourne ou non.

## L'appairage d'un appareil

1. Sur le serveur : `escouade-server pair` affiche un code, par exemple `8NUC-W6HS`, valable **une fois** et **10 minutes**. Un nouveau `pair` remplace le code précédent. Le client peut aussi le lancer lui-même par SSH (« le lancer d'ici par SSH »).
2. Dans le client, l'utilisateur donne ce code et un nom pour l'appareil (« Portable du bureau »).
3. Le client l'envoie à l'API **à la place de la poignée de main**. Le serveur répond le **jeton d'appareil**, donné une seule fois, que le client garde et redonne à chaque connexion.

Les refus, chacun avec sa raison pour le client (`none`, `expired`, `wrong`, `tries`) :

- **expiré** : « Ce code d'appairage a expiré : lance « escouade-server pair » de nouveau. » ;
- **déjà utilisé, ou jamais demandé** (un code redonné) : « Aucun code d'appairage n'attend… » ;
- **faux** : « Code d'appairage faux (4 essais restants). » ;
- **trop d'essais** : le cinquième code faux met fin au code (« Trop d'essais faux : ce code d'appairage ne vaut plus… »).

Le code est comparé **en temps constant**. Un code refusé compte comme une poignée de main ratée (après 20 dans la minute, les connexions sont fermées dès qu'acceptées). Le fichier qui garde le code en attente ne contient que son empreinte, lisible du seul propriétaire. **Ni le code ni le jeton ne sont jamais écrits dans le journal** ; `pair` affiche le code sur sa seule sortie, pour l'utilisateur.

## Les appareils révoqués

Un appareil révoqué (par `escouade-server devices revoke`, ou à sa propre demande quand il retire le serveur) laisse une trace : son nom, la date, qui l'a révoqué et l'empreinte de son jeton, jamais le jeton ; les 32 derniers sont gardés. Un client qui présente encore ce jeton est refusé avec ces informations, pour dire « Ce poste n'est plus appairé », quand et depuis où.

## Les secrets sur un serveur

Un VPS n'a pas de trousseau (pas de Secret Service, pas de session). Les secrets du serveur sont donc gardés dans **`secrets.json`, dans son dossier de données, lisible du seul propriétaire** (droits 0600 ; un fichier rendu lisible par d'autres, une sauvegarde remise par exemple, est remis à 0600 au démarrage, et le journal le dit). On y trouve :

- les jetons des appareils ;
- les clés des intégrations ;
- les clés de Cursor et des voix en ligne ;
- les secrets des plugins.

Le serveur le dit à chaque client : le message d'accueil de l'API porte `secretsInFile`, que le client lit pour l'afficher là où un secret se donne. Il porte aussi la machine du serveur (son nom et le compte du service), que le client affiche. La sécurité des comptes devient celle du serveur : son disque, son accès SSH, ses sauvegardes.

## La sécurité

- L'API n'écoute que sur 127.0.0.1. `--listen <adresse>` la met sur une autre adresse. Cette option ne sert qu'à Docker, derrière un tunnel, un réseau privé ou un proxy. Elle avertit dans le journal et sur la sortie d'erreur : « ATTENTION : l'API distante écoute sur … pas seulement sur 127.0.0.1 … jamais ouvert sur Internet ». Rien n'est ouvert par défaut.
- Les stratégies imposées valent pour le serveur : `servers.remoteApi` à `false` l'empêche de servir l'API, et refuse aussi l'appairage.
- Tout le reste de *L'API distante* vaut ici : le `Host` du bouclage, pas d'`Origin` de page web, les bornes des connexions, des appels et des files.

## Le journal

Chaque ligne va sur la **sortie standard**, pour le journal de systemd et les logs de Docker. Elle va aussi dans **`server.log`**, dans le dossier de données, qui tourne à 5 Mo (`server.1.log` à `server.3.log`, le plus ancien supprimé). Le journal dit :

- le démarrage, l'adresse de l'API, le dossier de données et celui des secrets ;
- chaque appareil appairé, chaque connexion ;
- chaque refus et sa raison ;
- l'arrêt et ce qui l'a demandé.

Il ne dit jamais de jeton, de code, de secret ni ce qu'écrit un terminal. `--debug` ajoute le détail d'Escouade.

## Pas encore

- Plusieurs serveurs dans un même client, leurs espaces et leurs piles.
- Le service systemd et son installation.
- L'image Docker.
- La mise à jour signée du serveur, que le client propose à sa version.
- Ce qui change de machine : le navigateur des dossiers du serveur, les fichiers des agents téléchargés, les ports des serveurs de test relayés, les notifications rendues par le client, la voix sur le poste.
