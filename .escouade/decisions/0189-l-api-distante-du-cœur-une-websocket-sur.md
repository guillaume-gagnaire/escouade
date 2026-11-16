---
number: 189
id: "l-api-distante-du-cœur-une-websocket-sur"
title: "L'API distante du cœur : une WebSocket sur 127.0.0.1, une seule liste des commandes, un jeton par appareil"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-478"
pages: ["a-distance/api-distante","fenetre/reglages-imposes"]
tickets: ["ESC-478"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":39,"removed":0},{"page":"fenetre/reglages-imposes","added":3,"removed":0}]
accepted_at: "2026-11-16T14:15:26+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Escouade à distance (le mode serveur, le téléphone) demande qu'un client piloté d'ailleurs parle au cœur. Le cœur ne parlait qu'à sa fenêtre, par l'IPC de Tauri, avec 405 commandes et un seul abonné aux événements. Il fallait choisir le transport et le format des messages, comment l'IPC et l'API restent les mêmes commandes, ce qui protège l'accès, et ce qu'on refuse à distance.

## Décision

- Transport : une WebSocket servie par le cœur, ws://127.0.0.1:<port>/remote, sur 127.0.0.1 seulement, éteinte par défaut (allumée par ESCOUADE_REMOTE_API dans un build de développement, par Headless::serve_remote sans fenêtre ; le mode serveur aura son réglage). Pas de TLS : le tunnel (SSH, WireGuard) chiffre.
- Format : JSON en texte. Premier message, la poignée de main {t: hello, version, token} ; puis {t: call, id, cmd, args} avec les arguments de l'IPC par les mêmes noms, et {t: reply, id, ok | error, code?} avec l'erreur telle que l'IPC la donne. Les canaux se donnent comme dans l'IPC (__CHANNEL__:<id>) et reviennent en {t: channel} ou en binaire (un octet de sorte, l'id sur 8 octets, les octets : la sortie d'un terminal). Ce que l'hôte sans fenêtre publie arrive en {t: host}. 16 Mo au plus par message.
- Versions : le client et le serveur doivent être de la même version de l'app ; un écart est refusé (code version) avec la version du serveur et le côté à mettre à jour.
- Une seule liste : chaque commande s'écrit une fois avec #[tauri::command] (qui désigne désormais escouade_command::command dans commands.rs) et se range dans une classe de remoteapi/list.rs ; l'IPC et l'API sont faits de cette liste par une macro, et un test vérifie que toute commande écrite y est.
- Jeton : un par appareil, révocable un par un (ses connexions fermées), gardé au trousseau (ou dans un fichier lisible du seul propriétaire sous Linux), comparé en temps constant ; avant l'ouverture, un Host du bouclage et pas d'Origin sauf celle de la fenêtre d'Escouade.
- Refusé à distance : les commandes du poste (fenêtre et pages, dialogues, voix, raccourci global, updater du client : desk-only) et, jusqu'au mode serveur, celles qui changent de machine et ne marchent pas telles quelles (not-served-yet). Un client connecté peut tout le reste, comme la fenêtre.
- Plusieurs clients : un abonné du hub par client, un état d'interface par client, une réponse d'un client fermée chez les autres.
- La stratégie servers.remoteApi à false interdit l'API, à son démarrage et à chaque connexion.

## Conséquences

- Rien ne change pour l'app de bureau tant que l'API est éteinte.
- Ajouter une commande : l'écrire comme avant, puis mettre son nom dans sa classe de remoteapi/list.rs et dans docs/REMOTE.md ; un oubli fait échouer remoteapi_tests.
- Les commandes prennent le State de remoteapi au lieu de celui de Tauri (même usage).
- Un jeton vaut l'accès à la machine du cœur avec les droits de son compte : il se garde comme une clé SSH.
- Restent pour la fondation 2b : le tampon des terminaux et la reprise après coupure, ipc.ts sur deux transports, les corps bruts (fichiers joints) ; puis escouade-server, l'appairage, le tunnel.
