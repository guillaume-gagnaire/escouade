---
title: "L'API distante"
subject: "a-distance"
sources: ["docs/REMOTE.md, « La fondation commune » (points 2, 3, 6), « La fondation 2a », « Le classement des commandes »","src-tauri/src/remoteapi (list.rs, server.rs, proto.rs, devices.rs)","crates/escouade-command (la liste unique des commandes)","crates/escouade-policy/src/list.rs (servers.remoteApi)"]
tickets: ["ESC-478"]
decisions: [189]
compared: null
---

# L'API distante

Le cœur d'Escouade (agents, Git, Kanban, terminaux) peut être piloté par un client qui tourne ailleurs : le poste qui vise un serveur, puis le téléphone. Il le fait par l'**API distante** : les mêmes commandes que la fenêtre, servies sur une WebSocket. C'est la fondation commune du mode serveur et du téléphone (`docs/REMOTE.md`) ; rien ne la montre encore dans la fenêtre.

## Éteinte par défaut, jamais ouverte sur Internet

- L'API est **éteinte** tant qu'on ne l'allume pas. Aujourd'hui, seuls les essais l'allument : un build de développement par `ESCOUADE_REMOTE_API` (le port), le cœur sans fenêtre quand il le demande. Le mode serveur aura son réglage.
- Elle n'écoute que sur **127.0.0.1** : aucune autre interface, ni IPv6. On l'atteint par un tunnel (SSH, puis WireGuard : les tickets suivants). Pas de TLS ici : c'est le tunnel qui chiffre.
- Avant même d'ouvrir la WebSocket, le cœur refuse (HTTP 403) une requête dont le `Host` n'est pas le bouclage (`127.0.0.1`, `localhost`, `[::1]`, avec n'importe quel port, celui d'un tunnel compris), ou qui porte l'`Origin` d'une page web : **une page web du poste ne peut pas s'y connecter**. Seule passe l'`Origin` de la fenêtre d'Escouade elle-même (`tauri://localhost`, `http://tauri.localhost`), pour que la fenêtre puisse un jour passer par l'API.
- Un message de plus de 16 Mo ferme la connexion.
- La stratégie du poste **`servers.remoteApi`** à `false` interdit l'API : elle ne démarre pas, et chaque nouvelle connexion est refusée (« L'API distante est interdite par la stratégie de ce poste (servers.remoteApi). »).

## Le jeton d'appareil

- Chaque appareil qui peut se connecter a **son jeton**, révocable un par un. Révoquer un appareil ferme aussitôt ses connexions ; son jeton est ensuite refusé.
- Les jetons sont gardés comme les autres secrets d'Escouade : au trousseau du système, ou, s'il le refuse (Linux), dans le fichier des appareils, lisible du seul propriétaire. La liste des appareils (nom, date de création, dernière connexion) ne contient pas de jeton quand le trousseau les garde.
- Un jeton présenté est comparé **en temps constant** à tous ceux des appareils, à chaque connexion.
- L'appairage (donner un jeton à un nouvel appareil) vient avec le mode serveur et le téléphone.

## Ce que vaut un jeton

Un client connecté peut **tout ce que peut la fenêtre** sur ce cœur : créer des projets, lancer des agents et leur parler, répondre à leurs autorisations, ouvrir des terminaux et y taper, lancer des commandes de test, lire et écrire les fichiers des dépôts. Un jeton vaut donc l'accès à la machine du cœur avec les droits de son compte, et se garde comme une clé SSH. C'est pourquoi l'API reste derrière un tunnel, et le jeton n'est qu'une défense de plus si le tunnel fuit.

Ne sont refusées à distance que les commandes du poste, et celles qui changent de machine sans marcher encore telles quelles (ci-dessous).

## La connexion

1. Le client ouvre `ws://127.0.0.1:<port>/remote`.
2. **La poignée de main** : son premier message, dans les 10 secondes, donne sa version et son jeton (`{"t":"hello","version":"2.3.0","token":"…"}`).
   - Un jeton absent, inconnu ou révoqué : refusé (`token`).
   - Une **autre version** que celle du serveur : refusée (`version`), avec la version du serveur et le côté à mettre à jour, le plus ancien, pour que le client propose « Mettre à jour le serveur » (ou le client). L'API suit la version de l'app : il n'y a pas de compatibilité entre versions.
   - Accepté : `{"t":"welcome","version","client","device"}`.
3. **Les appels** : `{"t":"call","id":7,"cmd":"create_agent","args":{…}}`, avec les arguments de l'IPC, par les mêmes noms. La réponse reprend l'id : `{"t":"reply","id":7,"ok":…}`, ou `{"t":"reply","id":7,"error":…}` avec l'erreur **telle que l'IPC la donne** (le texte de la commande, ou « command X missing required key y »), et un `code` quand c'est l'API qui refuse (`desk-only`, `not-served-yet`, `unknown-command`, `unavailable`).
4. **Les canaux** : un canal se donne comme l'IPC le donne (`"__CHANNEL__:<id>"`, l'id choisi par le client). Ce qu'il reçoit revient sur la même connexion : en JSON (`{"t":"channel","channel":1,"data":…}` : les événements de l'interface, par `subscribe`), ou en **binaire** pour les octets (la sortie d'un terminal) : un octet de sorte, l'id sur 8 octets, puis les octets. Une réponse en octets (un fichier lu) arrive de même.
5. **Ce que montrerait le bureau** sur un cœur sans fenêtre (un carillon, une notification, le compte de ce qui attend, un lien à ouvrir) est poussé au client (`{"t":"host","event":…}`), qui le rend chez lui.

Les appels d'une connexion passent dans l'ordre de leur commande, comme dans la fenêtre : les frappes d'un terminal gardent leur ordre.

## Plusieurs clients à la fois

- La fenêtre est un client comme les autres : chaque client reçoit tous les événements du cœur.
- **L'état de l'interface** (projet actif, vue, agent choisi, disposition) est gardé par client. Celui de la fenêtre est enregistré comme avant ; celui d'un client distant vit le temps de sa connexion. Ce qu'un client choisit (l'agent qu'il vient de créer, le projet qu'il ajoute) est à lui ; ce qui disparaît (un agent supprimé, un projet fermé) est oublié par tous.
- **Une réponse donnée par un client se ferme chez les autres** : une autorisation, une question ou un plan répondus depuis le téléphone disparaissent de la fenêtre, comme une réponse venue de claude.ai.

## Ce qui est refusé à distance

Chaque commande du cœur a sa classe, dans une **liste unique** d'où l'IPC de la fenêtre et l'API sont faits tous deux (le détail, commande par commande, dans `docs/REMOTE.md`, « Le classement des commandes ») :

- **servies telles quelles** : presque toutes ;
- **du poste seulement**, refusées avec l'erreur `desk-only` : la fenêtre et ses pages de lancement, ses liens et sa fin, les dialogues (enregistrer une image), la voix (micro, haut-parleurs, modèles), le raccourci global de l'Overseer, l'updater du client ;
- **qui changent de machine** en mode serveur : servies quand elles marchent telles quelles (un chemin de la machine du cœur, une image collée envoyée en données, la phrase dite d'une conversation vocale) ; sinon refusées avec `not-served-yet` jusqu'au mode serveur : ouvrir ou montrer un fichier sur la machine du cœur, lire son presse-papiers, les fichiers joints et les écritures d'octets bruts, les serveurs de langage de l'éditeur.

## Le journal

`app.log` dit chaque connexion et sa fin avec le **nom de l'appareil**, chaque refus et sa raison (jeton, version, `Origin`, `Host`, stratégie, commande du poste demandée), et chaque appareil révoqué. Jamais un jeton, un chemin du disque ni les arguments d'un appel.

## Pas encore

Le tampon de sortie des terminaux et la reprise d'un client après une coupure, la fenêtre sur l'API (fondation 2b) ; `escouade-server`, l'appairage, le tunnel et « Se connecter à un serveur » (mode serveur).
