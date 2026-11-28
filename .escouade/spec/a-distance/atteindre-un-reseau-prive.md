---
title: "Atteindre un réseau privé"
subject: "a-distance"
sources: ["docs/PRIVATE-NETWORK.md (ESC-551, l'étude RP-0 ; § 4.2 le protocole, § 4.3 la porte, § 4.4 les refus, § 4.5 l'appairage, § 4.8 la coupure, § 5 les choix R1 à R12, § 6 les tickets RP-1 à RP-9)","Réponse de l'utilisateur du 2026-11-21 (la demande a-22 de la boîte « À toi », ESC-587) : « A · Les recommandations : V0 guidée, V2 construite »","Réponse de l'utilisateur du 2026-11-27 (ESC-600, par l'overseer) : « Les deux » (5 essais par source sans finir le code, 20 par minute sur toute l'entrée), et dire la durée et la taille du code et ce qu'elles bornent","docs/REMOTE.md : « Le mode serveur », MS-1 (ESC-514, l'appairage), MS-3 (ESC-515), « Ce qui change de machine, MS-4 » (ESC-538), « Le téléphone », « Les agents sur le serveur, en auto-hébergé »","docs/SERVER-UX.md","docs/TEAM-SERVER.md § 13","docs/security/audit-2026-11/rapport.md, § 6.1 et SEC-02","src-tauri/src/fence.rs","crates/escouade-model/src/model.rs (proxy_env, network_env)","crates/escouade-plugins/src/procs.rs (variables refusées aux plugins)","crates/escouade-gate (la porte, RP-1 : lib.rs, http.rs, socks.rs, outside.rs, env.rs, via.rs, journal.rs, lists.rs ; tests/gate.rs, le faux connecteur)","crates/escouade-gate/src/entry (l'entrée, RP-4 : mod.rs, bounds.rs, registry.rs, store.rs ; tests/entry.rs, le faux connecteur de la crate du protocole)","src-tauri/src/privatenet.rs et privatenet_tests.rs (le cœur et la porte, RP-1)","src-tauri/src/privateentry.rs et privateentry_tests.rs (le cœur et l'entrée, RP-4)","src-tauri/src/server/mod.rs (--gate, ESCOUADE_SERVER_GATE, via ; --connector-listen, connector pair, connectors)","crates/escouade-model/src/git.rs (set_net_env, les git du cœur)","crates/escouade-connector-proto (ESC-598, RP-2 : le transport, Noise IK, les trames, les versions, les bornes ; ses vecteurs dans tests/vectors.rs ; pairing.rs, ESC-599)","fuzz/fuzz_targets/connector-frames.rs, fuzz/fuzz_targets/connector-handshake.rs (ESC-598)","Demande de l'utilisateur du 2026-11-20 (ESC-551) : des agents sur un serveur Escouade qui joignent un git et des services derrière le VPN d'une entreprise","docs/PRIVATE-NETWORK.md (ESC-551, l'étude RP-0 ; § 4.2 le protocole, § 4.3 la porte, § 4.4 les refus, § 4.5 l'appairage, § 4.8 la coupure, § 4.9 la fiche, § 5 les choix R1 à R12, § 6 les tickets RP-1 à RP-9)","src-tauri/src/server/mod.rs (--gate, ESCOUADE_SERVER_GATE, via)","crates/escouade-connector-proto (ESC-598, RP-2 : le transport, Noise IK, les trames, les versions, les bornes ; ses vecteurs dans tests/vectors.rs ; src/pairing.rs, l'appairage, ESC-599)","crates/escouade-connector (ESC-599, RP-3 : config.rs, ceiling.rs, guard.rs, run.rs, commands.rs, keys.rs, journal.rs, main.rs ; tests/checks.rs, service.rs, pair_and_run.rs, check_and_crate.rs)"]
tickets: ["ESC-551","ESC-587","ESC-597","ESC-598","ESC-599","ESC-600","ESC-601","ESC-602","ESC-603","ESC-604","ESC-605","ESC-654"]
decisions: ["reseau-prive-r1-le-serveur-dans-l-entrep","reseau-prive-r2-le-connecteur-parle-par","reseau-prive-r3-une-entree-des-connecteu","reseau-prive-r4-un-plafond-au-connecteur","reseau-prive-r5-la-liste-d-un-projet-est","reseau-prive-r6-l-agent-emprunte-la-port",336,337,338,345,363,359]
compared: null
---

> **Décidé ; la porte (RP-1), le protocole (RP-2), le connecteur (RP-3) et l'entrée des connecteurs (RP-4) construits, le reste à venir.** L'utilisateur a choisi le 2026-11-21 (ESC-587) les recommandations de l'étude RP-0 (`docs/PRIVATE-NETWORK.md`, ESC-551) : « V0 guidée, V2 construite ». Les choix R1 à R12 ci-dessous sont donc ceux de l'étude ; R1 à R6 ont chacun une décision proposée. La construction passe par les tickets RP-1 à RP-9 (ESC-597 à ESC-605), qui mettent cette page à jour à mesure : **RP-1 (ESC-597) a construit la porte** (*La porte, construite*, plus bas). **RP-2 (ESC-598) a construit le protocole** (la crate `escouade-connector-proto`, voir *Le protocole* plus bas). **RP-3 (ESC-599) a construit le programme `escouade-connector`** (*Le connecteur, construit*, plus bas) ; il n'est pas encore distribué (RP-6). **RP-4 (ESC-600) a construit l'entrée des connecteurs** (*L'entrée des connecteurs, construite*, plus bas) : l'appairage, l'état de chaque connecteur, les commandes servies à l'espace, la stratégie `servers.privateNetwork` ; la porte passe par ses canaux.

## Ce que c'est

Des agents qui tournent sur un serveur Escouade (voir *Le mode serveur*) joignent des services qui ne sont visibles que dans le réseau d'une entreprise, derrière son VPN : un git en SSH (`gitlab.acme.internal:22`), un registre de paquets en HTTPS (`nexus.acme.internal:443`), une API appelée par leurs tests (`api-recette.acme.internal:8443`). Ils ne joignent que les hôtes d'une liste, et l'entreprise tient cette liste.

## Les voies (R1)

- **V0, le serveur dans le réseau de l'entreprise** : guidée, rien à construire. `escouade-server` sur une machine de l'entreprise joint ce que son réseau permet ; le client s'y connecte par SSH à travers le VPN de l'utilisateur. Le guide le dit, avec le proxy de sortie de l'entreprise et son certificat racine réglés dans l'espace du serveur. La meilleure quand l'entreprise l'accepte.
- **V2, `escouade-connector`** : **construite**, pour un serveur hors de l'entreprise. Un programme que l'entreprise fait tourner chez elle ouvre une connexion **sortante** vers le serveur et ne laisse passer que les connexions des agents vers sa liste. Aucun port entrant chez l'entreprise.
- **V1, le serveur client du VPN** : non construite ; le guide donne une recette par un SOCKS en espace utilisateur, derrière la porte.
- **V3, le poste en relais** : non construite (elle perd le H24 et contourne la règle du VPN).

## Les choix décidés

- **R1, les voies** : V0 guidée, V2 construite, V1 en recette du guide, V3 non construite (décision proposée `reseau-prive-r1-le-serveur-dans-l-entrep`).
- **R2, le protocole** : une WebSocket sur TLS, sortante, avec à l'intérieur une poignée de main Noise IK dont les deux clés sont épinglées à l'appairage, puis des flux « ouvre `hôte:port` » à contrôle de débit ; une crate `escouade-connector-proto` partagée ; le serveur accepte la version du connecteur et les deux mineures d'avant (`reseau-prive-r2-le-connecteur-parle-par` ; sa forme exacte : `reseau-prive-rp-2-la-forme-du-protocole`).
- **R3, l'entrée du serveur** : un port à part (`--connector-listen`, `ESCOUADE_SERVER_CONNECTOR_LISTEN`), éteint par défaut, qui ne parle que le protocole des connecteurs, jamais l'API distante ; la stratégie `servers.privateNetwork` à `false` l'empêche (`reseau-prive-r3-une-entree-des-connecteu` ; sa forme construite : `reseau-prive-rp-4-l-entree-des-connecteu`).
- **R4, où se tient la liste** : un **plafond** au connecteur, écrit par l'entreprise, que le serveur ne peut pas élargir, et une **liste par projet** au serveur, dans ce plafond, vide par défaut (`reseau-prive-r4-un-plafond-au-connecteur`).
- **R5, qui change la liste d'un projet** : l'utilisateur, dans Réglages du projet › Réseau privé ; le `.escouade.yml` du dépôt la propose, accordée par son empreinte ; une stratégie ou le `.escouade.yml` d'équipe peuvent la fixer ; jamais un agent, aucun outil MCP (`reseau-prive-r5-la-liste-d-un-projet-est`).
- **R6, comment l'agent l'emprunte** : la porte devient le seul proxy des agents (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`), et `ssh` passe par `ProxyCommand="escouade-server via %h %p"` ; ensuite, dans l'image seulement, des noms en boucle locale pour ce qui ignore les proxys (`reseau-prive-r6-l-agent-emprunte-la-port`).
- **R7, le DNS** : résolu par le connecteur, des noms seulement ; l'adresse résolue vérifiée puis jointe telle quelle.
- **R8, le journal** : aux deux bouts, en JSON, chaque connexion permise ou refusée, jamais le contenu ; au serveur, les 1 000 dernières dans Réglages de l'espace › Réseau privé › Journal.
- **R9, les systèmes du connecteur** : Linux (x64, arm64), Windows (x64) et une image ; macOS se construit, sans service ni signature.
- **R10, la mise à jour du connecteur** : manuelle par défaut, automatique sur demande, signée.
- **R11, la sortie du serveur vers Internet** : hors de ce chantier, dite dans la fiche et dans l'interface.
- **R12, plusieurs connecteurs** : un serveur, plusieurs connecteurs, un par réseau, chacun son plafond ; la redondance plus tard.

## La porte, construite (RP-1, ESC-597)

Dans `escouade-server`, la crate `escouade-gate` (la porte) et le module `privatenet` du cœur (ce que le cœur en fait). La porte demande ses connecteurs à l'entrée réelle (RP-4, *L'entrée des connecteurs, construite*) par le trait `Connectors` : leur état, leur plafond, l'ouverture d'un flux.

### Quand elle tourne

- **Éteinte par défaut**, allumée par `escouade-server --gate` ou `ESCOUADE_SERVER_GATE=1` (aussi `true`, `on`, `yes`), ou par `--connector-listen` avec l'entrée des connecteurs (RP-4), avant le premier agent. Jamais sur le poste de bureau, dont les process gardent le proxy réglé comme avant (décision proposée `reseau-prive-rp-1-la-porte-s-allume-par`). La stratégie `servers.privateNetwork` à `false` l'empêche au démarrage.
- **Un port de 127.0.0.1**, choisi par le système, qui parle HTTP et SOCKS5, reconnus à leur premier octet. Une connexion d'ailleurs que la boucle locale est fermée ; 1 024 connexions à la fois au plus.

### Les jetons

- **Un jeton par agent**, dans l'adresse du proxy (`http://<jeton>@127.0.0.1:P`, `socks5h://<jeton>@127.0.0.1:P`) : 32 octets du générateur du système, en hexadécimal, qui disent l'agent et son projet. La porte n'en garde que l'empreinte (SHA-256), en mémoire ; le jeton n'est écrit ni dans un fichier ni au journal. Il revient tel quel aux redémarrages du process de l'agent, et ne vaut plus rien quand l'agent est supprimé ; un serveur redémarré en donne de nouveaux.
- **Les terminaux, les lancements (et les étapes de préparation et de nettoyage des worktrees) et les `git` du cœur** d'un projet ont chacun un jeton du projet (`terminal`, `launch`, `git` au journal à la place de l'agent).
- Reçu en HTTP par `Proxy-Authorization: Basic` (le jeton comme nom d'utilisateur, ce que les clients font d'une adresse `http://<jeton>@…`), en SOCKS5 par le nom d'utilisateur de la RFC 1929. Sans jeton, ou un jeton que la porte n'a pas donné : refusé (`407`, avec `Proxy-Authenticate` ; en SOCKS5, aucune méthode acceptée ou l'identification refusée).

### Les routes

Pour chaque connexion (`CONNECT hôte:port`, une requête en HTTP clair sous sa forme absolue, un `CONNECT` SOCKS5), l'hôte mis en forme (minuscules, sans point final, nom international en `xn--`) :

1. **Dans la liste du projet** : vers le connecteur de cet hôte (R12). Son canal coupé : refusé aussitôt ; hors de son plafond connu : refusé aussitôt ; sinon le connecteur ouvre le flux, ou dit son refus. Un connecteur qui n'est pas (plus) appairé : refusé, dit.
2. **Un nom d'un réseau privé hors de la liste** : refusé. Un nom est d'un réseau privé quand un plafond de connecteur ou la liste d'un projet le nomment, **sur n'importe quel port** : il n'est jamais résolu par le serveur ni envoyé dehors (décision proposée `reseau-prive-rp-1-les-routes-de-la-porte`).
3. **Le reste** (API des modèles, npm public, GitHub) : **directement**, ou **par le proxy réglé dans l'espace** (Réglages › Avancé › Réseau), qui passe derrière la porte, avec son mot de passe et ses exclusions (`NO_PROXY`, qui sortent directement). Un proxy réglé que la porte ne sait pas parler (`https://`, `socks5://`) : rien ne sort que ses exclusions, plutôt que de le contourner (comme « Ne jamais sortir sans le proxy »).

- **En SOCKS5, par nom** (`socks5h`) : une adresse que l'agent a résolue lui-même est prise pour une adresse, jamais pour un nom d'un réseau privé ; elle suit la route d'une adresse (un connecteur refuse une adresse littérale que son plafond n'écrit pas).
- **Une requête en HTTP clair** (`GET http://hôte/chemin`, ce que `HTTP_PROXY` fait d'une adresse `http://`) part sous sa forme d'origine vers l'hôte (sa forme absolue vers le proxy de l'espace, avec son autorisation), sans les en-têtes du proxy ni le jeton, avec `Connection: close` : une requête par connexion.

### Les refus

- **En HTTP** : `403` pour ce que la porte ou un connecteur refuse, `407` pour un jeton absent ou inconnu, `502` pour un hôte injoignable (« … est injoignable : … », « le proxy de l'espace refuse … ») ; la phrase est le corps de la réponse (en UTF-8) et sa raison, en ASCII. Les phrases, dans la langue du serveur : « gitlab.acme.internal:22 n'est pas dans la liste du projet Démo », « … n'est pas permis par le connecteur d'Acme » (avec la raison du connecteur entre parenthèses quand c'est lui qui refuse), « le réseau privé d'Acme est injoignable depuis 14:32 » (la date devant l'heure quand ce n'est pas le jour même), « la porte d'Escouade ne connaît pas ce jeton… ».
- **En SOCKS5** : « connexion non permise » (`0x02`) pour un refus, « hôte injoignable » (`0x04`) pour un échec.
- **Pour `ssh`** : `escouade-server via` écrit la phrase sur sa sortie d'erreur, qu'affiche `ssh`.
- **Un connecteur coupé** : les flux ouverts par lui sont fermés aussitôt, sans reprise (un `git fetch`, un `npm install` se relancent), et tout nouveau flux vers son réseau est refusé sans attendre.

### Le journal

Chaque connexion, permise ou refusée : l'heure, l'hôte, le port, HTTP ou SOCKS, la route (connecteur et lequel, direct, proxy de l'espace, refusé), la raison d'un refus, les octets dans chaque sens, la durée, l'agent et le projet ; **jamais le contenu ni un jeton**. Les 1 000 dernières sont gardées en mémoire et servies à l'espace (`private_network_journal`, RP-4) ; chacune est aussi une ligne du journal du serveur.

### Les variables

- **Les agents** (leur process, leur vue dans le terminal), **les terminaux**, **les lancements** et **les étapes des worktrees** d'un projet reçoivent la porte **à la place** du proxy réglé : `HTTPS_PROXY`, `HTTP_PROXY` et leurs minuscules vers `http://<jeton>@127.0.0.1:P`, `ALL_PROXY` et `all_proxy` vers `socks5h://<jeton>@127.0.0.1:P`, `NO_PROXY=127.0.0.1,localhost` (les serveurs de test des agents ne passent pas par elle). Les terminaux et les lancements la reçoivent que « le proxy aussi pour les terminaux » soit coché ou non : elle est la seule route vers le réseau privé (décision proposée `reseau-prive-rp-1-les-variables-et-ssh-t`). Le fichier d'autorités et `NODE_TLS_REJECT_UNAUTHORIZED` restent comme avant.
- **`GIT_SSH_COMMAND`** = la commande ssh de l'utilisateur (le `core.sshCommand` du dépôt, sinon le `GIT_SSH_COMMAND` du serveur, sinon `ssh`) suivie de `-o ProxyCommand="<escouade-server> via %h %p"` (le chemin du programme qui tourne). Posé par le cœur seul : un plugin ne peut pas le poser (`escouade-plugins::procs`).
- **`core.sshCommand`** : au démarrage d'un agent, le dépôt de son worktree reçoit la même commande dans sa configuration (celle du dépôt, partagée par ses worktrees), la commande de l'utilisateur gardée dedans ; déjà passée par la porte, rien n'est écrit. Un `git` lancé sans les variables de l'agent passe ainsi par `escouade-server via`, qui sort directement sans porte dans son environnement.
- **Les `git` du cœur** qui joignent un dépôt distant (`fetch`, fetch de fond, `pull`, `push`) dans un projet reçoivent les mêmes variables, avec le jeton `git` de leur projet ; une variable que le cœur leur donnait déjà (le ssh de Windows d'un fetch de fond) reste.
- **Un process partagé de harness** (plusieurs agents sur un même process) en a désormais un par agent, chacun ses variables.
- **Ce qui ne passe pas par la porte** : les connexions d'Escouade lui-même (le cœur, les programmes des plugins), comme avant.

### `escouade-server via <hôte> <port>`

Le `ProxyCommand` de `ssh` : il ouvre `hôte:port` comme son environnement le dit (par un `CONNECT` à la porte de son `HTTPS_PROXY`, avec le jeton, ses `NO_PROXY` en direct), passe les octets entre la connexion et son entrée et sa sortie standard, et sort à la fin de la connexion. Sans proxy dans son environnement, il se connecte directement. Il ne lit rien d'autre et n'écrit rien au journal du serveur.

### La liste par projet

- Dans `private-network.json`, au dossier des données du serveur : `{"projects": {"<id du projet>": {"hosts": [{"host": "gitlab.acme.internal", "port": 22, "connector": "<id du connecteur>"}]}}}`. Absent, ou un projet sans liste : il n'atteint rien d'un réseau privé. Un hôte illisible est laissé de côté, dit au journal ; un fichier illisible vaut une liste vide.
- Relu deux secondes au plus après un changement. L'API distante l'écrit (`private_network_set_project`, RP-4) : chaque hôte mis en forme, d'un connecteur appairé, dans son plafond connu ; l'interface (Réglages du projet › Réseau privé) est celle de RP-5. **Aucun outil MCP ne le lit ni ne l'écrit** (R5).

## L'entrée des connecteurs, construite (RP-4, ESC-600)

Dans `escouade-server` : le module `entry` de la crate `escouade-gate` (l'entrée, ses bornes, le registre des connecteurs) et le module `privateentry` du cœur (ce qu'il en garde, la stratégie, les commandes). Sa forme est la décision proposée `reseau-prive-rp-4-l-entree-des-connecteu`. Les essais passent par un faux connecteur fait de la crate du protocole, sur 127.0.0.1.

### Quand elle tourne

- **Éteinte par défaut.** `escouade-server --connector-listen <adresse:port>` (`ESCOUADE_SERVER_CONNECTOR_LISTEN`), par exemple `0.0.0.0:8443`, l'ouvre, et la porte avec elle (sans `--gate`). Une adresse et un port, toujours nommés : un port seul est refusé.
- **Elle ne parle que le protocole des connecteurs** : `/connector` (un canal) et `/connector/pair` (l'appairage), en WebSocket. Tout autre chemin répond 404 ; une requête qui n'est pas une WebSocket, un message texte, une poignée de main de l'API distante sont fermés sans réponse. L'API distante reste sur 127.0.0.1, sur son port à elle.
- **Le TLS** : un certificat à elle (`--connector-cert FICHIER` et `--connector-key FICHIER`, en PEM, ensemble ; `ESCOUADE_SERVER_CONNECTOR_CERT`, `ESCOUADE_SERVER_CONNECTOR_KEY`), ou aucun, derrière le proxy TLS d'un domaine (Traefik de Coolify, Caddy), ce que le journal dit. Noise ne dépend pas du certificat.
- **Son adresse**, que l'appairage donne au connecteur : `--connector-url wss://…/connector` (`ESCOUADE_SERVER_CONNECTOR_URL`), sinon faite de l'adresse d'écoute (le nom de la machine pour `0.0.0.0`), dite « devinée » au journal. Le serveur qui tourne l'écrit dans `connectors.json`, pour `connector pair` lancé à côté.
- `--trust-proxy` vaut aussi pour elle : la source d'une connexion d'un proxy déclaré est la dernière adresse de son `X-Forwarded-For`.
- 64 connexions au plus dont la poignée de main est en cours ; 10 s pour le TLS et la WebSocket, 30 s pour un appairage.

### L'appairage

- `escouade-server connector pair <nom>` (ou « Ajouter un connecteur » dans l'espace, RP-5, par l'API distante) : un code de **8 lettres** d'un alphabet de 32 (sans 0, O, 1 ni I : **40 bits**), montré `ABCD-EFGH`, **valable 10 minutes, une fois**, avec l'adresse de l'entrée et la commande à lancer chez l'entreprise (`escouade-connector pair <adresse> <code>`). Tapé en minuscules ou sans tiret, c'est le même code.
- Plusieurs codes peuvent attendre, un par nom ; un nouveau code pour un nom remplace celui d'avant. Un nom déjà appairé est refusé (« retire-le d'abord »). Un nom : 1 à 64 caractères, sans caractère de contrôle.
- **L'échange** (le module `pairing` de la crate du protocole, accordé avec RP-3) : `Noise_XXpsk0_25519_ChaChaPoly_BLAKE2s` sur `/connector/pair`, la clé prépartagée tirée du code (BLAKE2s) : le code ne passe jamais, même sous une inspection TLS. Le connecteur donne sa clé et sa version ; le serveur épingle la clé et répond la sienne. Une version refusée l'est avant, sans consommer le code. Ensuite, un canal n'est accepté que par ces deux clés.
- **Un code faux, expiré ou déjà utilisé** est fermé sans un mot, comme une clé inconnue, et **n'annule jamais le code qui attend** (comme pour un appareil, ESC-654).
- **Les bornes** :
  - **par source**, 20 essais ratés en une minute (pas de poignée de main à temps, pas ce protocole, une clé qu'aucun connecteur n'a, un code faux, un TLS raté, un autre chemin) : ses connexions sont fermées sans être lues jusqu'à la fin de la minute ;
  - **par source**, 5 codes faux pendant la vie d'un code (10 minutes) : ses appairages sont refusés sans être lus pour le reste de ces 10 minutes ; le code attend toujours, une autre source peut le donner ;
  - **pour toute l'entrée**, 20 essais ratés en une minute de toutes les sources : l'appairage est fermé à tous jusqu'à la fin de la minute. Un connecteur appairé entre toujours (sa clé se lit, elle ne se devine pas) : un inconnu ne coupe jamais le réseau privé.
  - **Ce que la durée et la taille du code bornent** : au plus 20 codes faux par minute de tout Internet, soit 200 pendant les 10 minutes d'un code, contre 32⁸ = 2⁴⁰ codes : environ une chance sur 5,5 milliards. Négligeable : le code garde ses 10 minutes et ses 8 lettres.
  - Derrière un proxy non déclaré, toutes ses connexions comptent pour une seule source : déclarer le proxy (`--trust-proxy`).
- **Ni le code ni une clé privée au journal** : il dit « un code d'appairage fait pour « Acme » », « « Acme » appairé (son id, sa version) », un code faux et sa source.

### Ce qui est gardé

- **`secrets.json`** (droits 0600, voir *Le mode serveur*) : la clé privée du serveur pour les connecteurs (`connector-server-key`, faite au premier besoin), la clé publique de chaque connecteur (`connector-key-<id>`), les codes qui attendent (`connector-pairing` : le nom, la fin et la clé prépartagée de chacun, jamais le code). Un processus à la fois l'écrit (la commande et le serveur à côté).
- **`connectors.json`** (dossier des données, 0600) : chaque connecteur (id, nom, date d'appairage, connecté depuis, dernière connexion, sa version, une version refusée, son plafond tel qu'il l'a envoyé), la trace des 32 derniers retirés (nom, date, par qui, l'empreinte SHA-256 de sa clé), l'adresse de l'entrée ouverte. Jamais une clé ni un code.

### L'état de chaque connecteur

- **Son plafond**, reçu à chaque connexion et à chaque relecture de son fichier, gardé et montré tel qu'il l'écrit, ses plages d'adresses comprises ; la porte n'en prend que les règles `hôte:port` et `*.suffixe:port` (les plages ne servent qu'au connecteur, après sa résolution).
- **Son canal** : connecté (depuis quand), coupé (depuis quand), jamais connecté ; sa dernière connexion. Un serveur redémarré repart de « coupé » ; le connecteur rouvre seul. Un connecteur qui revient alors qu'un canal à lui est encore ouvert prend sa place, l'ancien fermé.
- **Sa version** : à jour ; « à mettre à jour » à deux mineures du serveur (accepté) ; « trop ancien » au-delà, ou plus récent que le serveur : le canal refusé, le connecteur le sait par le protocole, et l'état dit quoi faire (« escouade-connector update --to 2.3.0 »). Une version refusée vient d'une clé appairée : elle ne compte pas comme un essai raté.
- **Plusieurs connecteurs** (R12), un par réseau : chaque hôte de la liste d'un projet nomme le sien.

### Les commandes du serveur

| Commande | Ce qu'elle fait |
| --- | --- |
| `escouade-server connector pair <nom>` | Le code d'appairage d'un connecteur, l'adresse de l'entrée et la commande à lancer chez l'entreprise |
| `escouade-server connectors` (`list`) | Les connecteurs : id, nom, état, version, le nombre de règles de son plafond, ce qu'il faut faire de sa version ; puis les retirés |
| `escouade-server connectors remove <nom ou id>` | Retire un connecteur : sa clé effacée (il n'entre plus), sa trace gardée ; un serveur qui tourne ferme son canal dans les deux secondes |

Elles marchent que le serveur tourne ou non, dans la langue de ses réglages ; un nom peut avoir des espaces (`connector pair Client d'Acme`). `connector pair` est refusé par la stratégie `servers.privateNetwork`.

### Les commandes servies à l'espace

Pour Réglages de l'espace › Réseau privé (l'interface est celle de RP-5), par l'API distante, à un poste seulement (jamais un téléphone), et **jamais par un outil MCP** (R5) :

- `private_network_status` : servi ou non, la stratégie qui l'interdit, l'adresse de l'entrée ouverte, si la porte tourne, les connecteurs et leur état, les codes qui attendent (le nom et la fin, jamais le code), les retirés ;
- `private_network_pair` (`name`) : un code, sa fin et l'adresse de l'entrée, montrés une fois à qui l'a demandé ;
- `private_network_remove` (`id`) : le connecteur retiré, « par » l'appareil qui l'a demandé ;
- `private_network_project` et `private_network_set_project` (`project`, `hosts`) : la liste d'un projet, écrite dans `private-network.json`, chaque hôte mis en forme, d'un connecteur appairé, **dans son plafond connu** (hors du plafond : refusé, « seul le service informatique l'élargit, dans connector.toml ») ; vide, le projet n'atteint plus rien d'un réseau privé ; la porte la relit aussitôt ;
- `private_network_journal` : le journal de la porte, ses 1 000 dernières connexions.

### La porte branchée

- La porte demande ses connecteurs à l'entrée par le trait `Connectors` de RP-1 : leur état, leur plafond, un flux par le canal du connecteur. Un canal coupé la fait refuser aussitôt (« le réseau privé d'Acme est injoignable depuis 14:32 ») ; un connecteur retiré n'est plus connu d'elle.
- `--gate` seul (sans `--connector-listen`) garde la porte sans entrée : les connecteurs appairés y sont coupés.

### La stratégie `servers.privateNetwork`

- À `false` (voir *Réglages imposés*), **au démarrage** : ni l'entrée ni la porte ne s'ouvrent, et le journal le dit. `connector pair`, « Ajouter un connecteur » et `private_network_pair` sont refusés, avec qui l'impose (« Le réseau privé (l'entrée des connecteurs, la porte, l'appairage d'un connecteur) est interdit par la stratégie de ce poste (servers.privateNetwork). »).
- **Arrivée pendant que le serveur tourne** : l'entrée se ferme et tous les canaux sont coupés aussitôt ; la porte des agents déjà lancés reste, sans plus rien atteindre d'un réseau privé (leur couper ce proxy leur couperait Internet). Retirée, l'entrée se rouvre.
- Lister et retirer les connecteurs restent permis.

## Le connecteur

- **Le tunnel** : une WebSocket sur TLS, sortante, vers l'entrée des connecteurs du serveur, à travers le proxy de l'entreprise s'il y en a un ; à l'intérieur, Noise, que le proxy ne peut ni lire ni imiter ; des flux « ouvre `hôte:port` ».
- **L'appairage** : Réglages de l'espace › Réseau privé › « Ajouter un connecteur » (ou `escouade-server connector pair`) donne un code (8 lettres, 10 minutes, une fois) et l'adresse de l'entrée ; comme celui d'un appareil, un code faux ne l'annule pas et les essais sont bornés par source et en tout (*L'entrée des connecteurs, construite*). Le service informatique lance `escouade-connector pair <adresse> <code>`. Ni le code ni une clé privée au journal. Retirer un connecteur, d'un côté ou de l'autre, ferme son canal.
- **Deux listes** : le plafond, dans `connector.toml`, et la liste d'un projet, au serveur (R4, R5).
- **Les contrôles** : nom et port dans le plafond après mise en forme ; adresse littérale refusée sauf écrite dans le plafond ; jamais la boucle, le lien local, les métadonnées d'un cloud ni la machine du connecteur ; 256 flux, 32 ouvertures par seconde, 10 minutes de silence.
- **L'agent l'emprunte sans le savoir** (R6, *La porte, construite*). Le reste d'Internet sort comme avant.
- **Les refus** sont lisibles par l'agent : « gitlab.acme.internal:22 n'est pas dans la liste du projet Démo », « … n'est pas permis par le connecteur d'Acme », « le réseau privé d'Acme est injoignable depuis 14:32 ».
- **La coupure** : le connecteur se reconnecte seul ; pendant la coupure, la porte refuse aussitôt ; les flux ouverts sont fermés, sans reprise. Un connecteur coupé depuis 10 minutes dont un projet a besoin fait une demande dans la boîte « À toi ».
- **Ce que voit Escouade, l'éditeur** : rien ; aucun relais à lui. Le serveur voit les noms, ports, octets et durées, pas le contenu chiffré de bout en bout.

## Le protocole (RP-2, fait)

La crate `crates/escouade-connector-proto`, partagée par `escouade-server` (RP-1, RP-4) et `escouade-connector` (RP-3), sans rien du cœur ni de Tauri. Sa forme exacte est la décision proposée `reseau-prive-rp-2-la-forme-du-protocole`.

- **Le transport** : `wss://<serveur>:<port C>/connector`, ouvert par le connecteur, directement ou par le proxy de l'entreprise (`CONNECT`, authentification Basic seulement ; `HTTPS_PROXY` et `NO_PROXY` du connecteur, `http://` seulement). Le TLS est vérifié par les autorités du système, celles de Mozilla et un **fichier d'autorités en plus** (celle d'une inspection TLS). Un autre chemin que `/connector` (et `/connector/pair`, l'appairage) répond 404. Le certificat ne dit pas qui est le serveur : Noise le dit.
- **La poignée de main** : `Noise_IK_25519_ChaChaPoly_BLAKE2s`, dans les deux premiers messages de la WebSocket. Le connecteur y donne sa version ; le serveur lit sa clé et, **sans clé connue, ferme sans rien répondre** (ni message ni fermeture WebSocket) ; il compte alors un essai raté pour les bornes de l'entrée (RP-4), comme un message qui n'est pas de ce protocole ou une poignée de main qui ne vient pas en 10 s. Une clé changée, d'un côté ou de l'autre, est refusée de la même façon. Une inspection TLS ne lit que la demande de WebSocket, puis du Noise.
- **L'appairage** : `Noise_XXpsk0_25519_ChaChaPoly_BLAKE2s` sur `/connector/pair` (le module `pairing`, ESC-599 et ESC-600), la clé prépartagée tirée du code ; quatre messages, le dernier du serveur dit « appairé » ou « refusé » avec la raison.
- **Les versions** : le serveur dit la sienne et son verdict dans sa réponse. Sa mineure ou celle d'avant : acceptée ; deux mineures d'avant : acceptée, « à mettre à jour » ; plus vieille, ou un autre majeur : « trop ancien », refusée ; **plus récente que le serveur : refusée aussi**, c'est le serveur qu'il faut mettre à jour. Le connecteur refusé reçoit les versions acceptées et dit quoi faire (« escouade-connector update --to 2.3.0 », ou « mettez escouade-server à jour »). Une version refusée vient d'une clé appairée : elle ne compte pas comme un essai raté.
- **Les trames** : `open {id, hôte, port, agent, projet}` (du serveur), `opened {id}` et `refused {id, raison}` (du connecteur, la raison lisible avec un code : non permis, non résolu, injoignable, bornes, arrêt), `data {id, octets}` (une `data` vide clôt un sens, comme le FIN de TCP), `close {id}` (le flux est fini dans les deux sens), `window {id, incrément}`, `ping`, `pong`, et le **plafond** envoyé par le connecteur à chaque connexion, et de nouveau quand il relit son fichier.
- **Le contrôle de débit** : chaque flux a une fenêtre de 256 Kio dans chaque sens, rendue à mesure que le lecteur lit ; un flux lent n'arrête que lui (un `git clone` lent ne bloque pas un `npm install`). Un pair qui dépasse sa fenêtre est coupé.
- **La vie** : un `ping` toutes les 15 s, des deux côtés ; rien reçu pendant 45 s, le canal est coupé et tous ses flux fermés. Le connecteur rouvre après 250 ms, 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s.
- **Les bornes**, réglables par l'appelant : une trame de 64 Kio au plus (32 Kio au moins), 256 flux ouverts, 32 ouvertures demandées par seconde (refusées comprises), un flux muet fermé après 10 min ; au-delà, l'ouverture est refusée (`bornes`), ou le canal coupé pour une trame trop grande.
- **L'appairage** (ajouté par RP-3, ESC-599, le module `pairing`, pris tel quel par RP-4) : sur le même port, au chemin de l'entrée suivi de `/pair` (`wss://…:8443/connector/pair`). Une poignée de main `Noise_XXpsk0_25519_ChaChaPoly_BLAKE2s` dont la clé prépartagée est tirée du code mis en forme (majuscules, sans tiret ni blanc ; BLAKE2s-256 de `escouade-connector-pair/1` suivi du code) : **le code ne passe jamais**, même sous une inspection TLS, et le serveur peut n'en garder que cette clé. Les trois messages de XX (le premier porte la version du connecteur, le deuxième le verdict du serveur), puis un quatrième du serveur, chiffré : appairé, ou refusé avec sa raison. Un code faux (ou aucun en attente) ne déchiffre pas le premier message : le serveur ferme sans un mot, et compte un essai raté. Une version refusée l'est par le quatrième message, avec ce qu'il faut faire, sans consommer le code. Chaque côté prouve qu'il tient la clé privée de la clé publique qu'il donne.
- **Les essais** : des vecteurs de test (chaque trame, octet par octet), la poignée de main avec des clés faites dans les tests, un faux proxy d'entreprise et une fausse inspection TLS en local, deux flux dont un lent, le temps simulé pour le ping et la coupure ; deux cibles de fuzzing dans `fuzz/` (`connector-frames`, `connector-handshake`) ; l'appairage (bon code parmi plusieurs, code faux, aucun en attente, version refusée, refus du serveur, silence).

## Le connecteur, construit (RP-3, ESC-599)

Le programme `escouade-connector`, la crate `crates/escouade-connector`, en Rust, sans rien du cœur ni de Tauri (le protocole, `tokio`, `rustls`, `serde`, TOML). Sa version est celle de l'app (`scripts/version.mjs`) : c'est elle que le serveur juge. Il n'est pas encore distribué ni installé en service (RP-6). Ses textes (commandes, journal, raisons des refus) sont en anglais, comme ceux du protocole ; la porte les met entre parenthèses dans ses propres phrases. Les choix de ce ticket sont la décision proposée `reseau-prive-rp-3-le-connecteur-connecto`.

### `connector.toml`

Écrit par le service informatique (`--config`, `ESCOUADE_CONNECTOR_CONFIG`, sinon `/etc/escouade-connector/connector.toml`, ou `%ProgramData%\escouade-connector\connector.toml` sous Windows) :

```toml
server = "wss://escouade.exemple.fr:8443/connector"     # facultatif : pair le donne aussi
proxy = "http://it:motdepasse@proxy.acme.internal:3128"  # facultatif : sinon HTTPS_PROXY et NO_PROXY ; http:// et Basic seulement
authorities = "/etc/escouade-connector/acme-root.pem"    # autorités en plus (l'inspection TLS)
allow = ["gitlab.acme.internal:22", "*.npm.acme.internal:443", "10.20.1.5:8443"]   # le plafond
ranges = ["10.20.0.0/16"]                                # facultatif : toute adresse jointe y est
auto_update = false                                      # lu seulement (RP-6 le fera)
[limits]
max_streams = 256        # de 1 à 4 096
opens_per_second = 32    # de 1 à 1 024
idle_minutes = 10        # de 1 à 1 440
```

- **Une clé inconnue est une erreur** (une faute de frappe ne passe pas pour une règle oubliée), comme une règle illisible (`gitlab` sans port, `*.internal:22` sous un nom d'un seul mot, une plage dont les bits passent sa longueur), avec sa ligne ; jamais le mot de passe du proxy dans le message.
- **Une règle** : `hôte:port` ; `*.suffixe:port` pour chaque nom sous le suffixe, pas le suffixe lui-même ; une adresse écrite telle quelle (`10.20.1.5:8443`, `[fd00::5]:22`), seule façon de permettre une adresse. Les hôtes sont mis en forme comme à la porte : minuscules, sans point final, nom international en `xn--`, IPv6 sans crochets, IPv4 dans IPv6 lue comme IPv4.
- **Relu à chaud**, toutes les 2 s : le changement est dit au journal (ce qui a été ajouté et retiré du plafond, les bornes) ; le plafond est renvoyé au serveur ; un flux ouvert que le nouveau plafond (ou ses plages) ne permet plus est **fermé**, dit au journal ; un autre serveur, proxy ou fichier d'autorités rouvre le canal ; les nouvelles bornes de flux et d'ouvertures valent aussitôt, le silence à la prochaine connexion. Un fichier qui ne se lit plus : dit au journal, et le dernier lu est gardé. Au démarrage, un fichier illisible arrête le programme avec son erreur.
- **Le plafond envoyé au serveur** : ses règles mises en forme, puis ses plages, sans port (`10.20.0.0/16`), pour que l'interface les montre.

### Les commandes

- **`pair <adresse> <code>`** (ou `-` pour lire le code sur l'entrée standard, hors de la liste des process) : refuse s'il est déjà appairé, ou si `connector.toml` nomme un autre serveur ; fait une nouvelle paire de clés, s'appaire (le proxy et les autorités de `connector.toml`), et ne garde les clés qu'une fois le serveur d'accord. Il dit l'adresse, la clé publique du serveur et la sienne ; jamais le code. Un code faux : « the code is wrong, expired or already used (ask a new one) ».
- **`unpair`** : écrit des zéros sur la clé privée, puis efface les deux fichiers. Le serveur garde le connecteur jusqu'à ce qu'il soit retiré de l'espace ; sans sa clé, rien ne passe.
- **`run`** : le service, le journal sur la sortie standard ; il s'arrête proprement sur Ctrl+C et, sous Unix, SIGTERM (le canal fermé, les flux fermés).
- **`check`** : pour chaque règle du plafond, la résolution, les contrôles et une connexion, comme un `open` : « ok gitlab.acme.internal:22: … answers at 10.20.1.5:22 (12 ms) », ou la raison (non résolu, une adresse jamais jointe, ne répond pas) ; un motif est dit, pas essayé. Il dit aussi s'il est appairé. Code de sortie 1 si un hôte ne répond pas.
- **`version`**, et `--state <dossier>` pour le dossier des clés.

### Ce qu'il garde

Dans son dossier d'état (`--state`, sinon `$STATE_DIRECTORY` de systemd, `/var/lib/escouade-connector`, ou `%ProgramData%\escouade-connector`) : `connector.key`, la clé privée, en mode `0600` dans un dossier `0700` sous Unix, **scellée par DPAPI** sous Windows pour le compte qui a lancé `pair` (lui seul l'ouvre) ; `paired.toml`, l'adresse du serveur, sa clé publique et la sienne. Rien d'autre. Une clé privée qui n'est pas celle que dit `paired.toml` est refusée (« pair again »).

### Les contrôles de chaque `open`

Dans cet ordre, chaque refus revenant au serveur avec le code du protocole et sa raison :

1. **Les bornes** : plus de `max_streams` flux à la fois, ou plus de `opens_per_second` ouvertures dans la seconde (refusées comprises) : refusé (`bornes`). Le protocole garde au-dessus ses propres limites dures (4 096 flux, 1 024 par seconde).
2. **Le plafond**, après mise en forme : un nom hors du plafond, refusé ; une adresse littérale que le plafond n'écrit pas telle quelle, refusée (« an address the ceiling of this connector does not write »).
3. **Une seule résolution**, par le DNS de la machine (10 s au plus) : un nom inconnu, refusé (`non résolu`).
4. **Chaque adresse résolue vérifiée**, et celle qui est écrite aussi : jamais la boucle (`127.0.0.0/8`, `::1`), le lien local (`169.254.0.0/16`, `fe80::/10`), les métadonnées connues d'un cloud (`169.254.169.254`, `100.100.100.200`, `fd00:ec2::254`), l'adresse non précisée, la diffusion ou le multicast, une adresse de la machine du connecteur (reconnue parce qu'une socket peut s'y lier, sans écouter), ni une adresse hors des plages ; les formes cachées (IPv4 dans IPv6, NAT64 `64:ff9b::/96`) sont lues comme l'IPv4 qu'elles portent. Les adresses interdites sont écartées et dites au journal ; s'il n'en reste aucune, refusé.
5. **L'adresse vérifiée est jointe telle quelle** (10 s au plus, la première qui répond), sans seconde résolution : un DNS qui change sa réponse (le *rebinding*) ne fait rien joindre d'autre que ce qui a été vérifié. Rien ne répond : refusé (`injoignable`).

### La reconnexion

- Au calendrier du protocole : 250 ms, 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s ; de nouveau 250 ms une fois un canal ouvert puis coupé. À la coupure, les flux ouverts sont fermés, sans reprise.
- **Un serveur qui ne connaît plus le connecteur** (retiré de l'espace) ou **qui refuse sa version** : dit au journal avec ce qu'il faut faire (« unpair, then pair again with a new code », « escouade-connector update --to … », « update escouade-server »), et réessayé **toutes les 60 s** seulement (chaque essai compte comme un essai raté au serveur). Une version « à mettre à jour » est acceptée et dite.
- **Non appairé** : dit une fois, rien n'est essayé ; `pair` lancé à côté du service est vu dans les 2 s (le canal s'ouvre), `unpair` aussi (le canal est fermé). Un `server` de `connector.toml` qui n'est pas celui de l'appairage : dit, rien n'est essayé tant qu'il ne change pas.

### Le journal

Une ligne JSON par événement sur la sortie standard, avec `time` (UTC, RFC 3339) et `event` :

- **`allowed`** (à l'ouverture : hôte, port, adresse jointe, agent et projet annoncés par le serveur, les adresses écartées), **`closed`** (à la fin : les mêmes, `bytes_to_host`, `bytes_from_host`, `duration_ms`, et la raison d'une fermeture par le connecteur), **`refused`** (hôte, port, agent, projet, `code` — `not-allowed`, `unresolved`, `unreachable`, `limits` —, la raison) ;
- la vie du service : `started`, `connected` (le serveur, sa version, le verdict, le plafond), `outdated`, `unreachable`, `cut`, `not-known`, `version-refused`, `not-paired`, `paired`, `server-mismatch`, `config-reloaded`, `config-error`, `state-error`, `auto-update`, `stopped`.
- **Jamais le contenu, ni le code d'appairage, ni une clé privée** : aucun événement ne les porte, et en dernier garde le journal remplace par `held-back` toute ligne qui contiendrait la clé privée.

### Ce qu'il ne fait pas

Il n'écoute sur aucun port (aucune socket à l'écoute dans son code) ; il ne se connecte qu'à son serveur (et au proxy de l'entreprise) et aux hôtes de son plafond ; ni UDP ni ICMP.

### Les essais

Rust (`cargo nextest run -p escouade-connector`, une quarantaine) : un faux serveur en mémoire et un faux serveur en TLS réel sur 127.0.0.1 (une autorité faite dans le test), des hôtes d'écho sur 127.0.0.1 (la boucle permise par une option des tests seulement, jamais par `connector.toml` ni la ligne de commande), un faux DNS qui change de réponse ; les refus (métadonnées, lien local en IPv4, IPv6 et IPv4 dans IPv6, boucle par défaut, adresse littérale, hors plage, la machine du connecteur, nom inconnu, bornes), le rebinding, le plafond relu à chaud, `check`, la reconnexion en temps simulé, la coupure, les versions, `pair` puis `run` de bout en bout, et aucun code ni clé dans le journal, dans ce que dit `pair` ni dans un fichier autre que la clé.

## Limites

- TCP seulement : ni UDP, ni ICMP ; rien du réseau de l'entreprise vers le serveur.
- La porte est une consigne, pas une cloison : les agents d'un serveur partagent un compte, et un agent détourné peut ignorer les variables, lire celles d'un autre agent et se présenter pour un autre projet ; ce qui tient, c'est le plafond du connecteur. L'interface le dit.
- Le connecteur borne l'entrée dans le réseau de l'entreprise, pas la sortie du serveur vers Internet (R11).
- Le proxy de l'entreprise, pour la sortie du connecteur : Basic seulement (ni NTLM, ni Kerberos), et en `http://`.
- Ce qui ne suit pas les variables de proxy (Java : Maven, Gradle ; le `fetch` de Node sans `NODE_USE_ENV_PROXY` ; un programme qui ouvre ses sockets lui-même) ne passe pas par la porte : les noms en boucle locale de l'image (RP-7) le couvriront.
- Le journal de la porte n'est gardé qu'en mémoire : il repart de zéro au redémarrage du serveur ; il est servi à l'espace, que RP-5 montrera.
- Un attaquant réparti sur beaucoup de sources peut fermer l'appairage à tous, minute après minute (jamais les canaux des connecteurs appairés) : on réessaie la minute suivante, ou on filtre l'entrée par adresse.
- Sous Windows, la clé du connecteur est scellée pour le compte qui a lancé `pair` : le service doit tourner sous ce compte (RP-6 lancera `pair` sous le compte du service).
- Le connecteur ne se met pas encore à jour lui-même : `auto_update` est lu et dit, pas fait (RP-6).

## Les tickets

- **RP-1** (ESC-597) : la porte dans `escouade-server`, les variables des agents, `escouade-server via`. **Fait.**
- **RP-2** (ESC-598) : la crate `escouade-connector-proto`. **Fait.**
- **RP-3** (ESC-599) : le programme `escouade-connector`, et l'appairage dans la crate du protocole. **Fait.**
- **RP-4** (ESC-600) : l'entrée des connecteurs, l'appairage côté serveur, la stratégie `servers.privateNetwork`. **Fait.**
- **RP-5** (ESC-601) : Réglages de l'espace et du projet › Réseau privé, le `.escouade.yml`, la boîte « À toi ».
- **RP-6** (ESC-602) : la distribution et la mise à jour du connecteur.
- **RP-7** (ESC-603) : les noms en boucle locale dans l'image.
- **RP-9** (ESC-604) : les guides du site, la fiche pour le service informatique.
- **RP-8** (ESC-605) : les trois cas de bout en bout, en dernier.

Ce qui attend l'utilisateur ensuite : un essai réel dans un vrai réseau d'entreprise (un connecteur derrière un vrai proxy avec inspection TLS, vers un vrai GitLab), et la fiche lue par un vrai service informatique.
