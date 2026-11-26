---
title: "Atteindre un réseau privé"
subject: "a-distance"
sources: ["docs/PRIVATE-NETWORK.md (ESC-551, l'étude RP-0 ; § 4.3 la porte, § 4.4 les refus, § 4.8 la coupure, § 5 les choix R1 à R12, § 6 les tickets RP-1 à RP-9)","Réponse de l'utilisateur du 2026-11-21 (la demande a-22 de la boîte « À toi », ESC-587) : « A · Les recommandations : V0 guidée, V2 construite »","docs/REMOTE.md : « Le mode serveur », MS-1 (ESC-514, l'appairage), MS-3 (ESC-515), « Ce qui change de machine, MS-4 » (ESC-538), « Le téléphone », « Les agents sur le serveur, en auto-hébergé »","docs/SERVER-UX.md","docs/TEAM-SERVER.md § 13","docs/security/audit-2026-11/rapport.md, § 6.1 et SEC-02","src-tauri/src/fence.rs","crates/escouade-model/src/model.rs (proxy_env, network_env)","crates/escouade-plugins/src/procs.rs (variables refusées aux plugins)","crates/escouade-gate (la porte, RP-1 : lib.rs, http.rs, socks.rs, outside.rs, env.rs, via.rs, journal.rs, lists.rs ; tests/gate.rs, le faux connecteur)","src-tauri/src/privatenet.rs et privatenet_tests.rs (le cœur et la porte, RP-1)","src-tauri/src/server/mod.rs (--gate, ESCOUADE_SERVER_GATE, via)","crates/escouade-model/src/git.rs (set_net_env, les git du cœur)","Demande de l'utilisateur du 2026-11-20 (ESC-551) : des agents sur un serveur Escouade qui joignent un git et des services derrière le VPN d'une entreprise"]
tickets: ["ESC-551","ESC-587","ESC-597","ESC-598","ESC-599","ESC-600","ESC-601","ESC-602","ESC-603","ESC-604","ESC-605","ESC-654"]
decisions: ["reseau-prive-r1-le-serveur-dans-l-entrep","reseau-prive-r2-le-connecteur-parle-par","reseau-prive-r3-une-entree-des-connecteu","reseau-prive-r4-un-plafond-au-connecteur","reseau-prive-r5-la-liste-d-un-projet-est","reseau-prive-r6-l-agent-emprunte-la-port",336,337,338]
compared: null
---

> **Décidé ; la porte construite (RP-1), le reste à venir.** L'utilisateur a choisi le 2026-11-21 (ESC-587) les recommandations de l'étude RP-0 (`docs/PRIVATE-NETWORK.md`, ESC-551) : « V0 guidée, V2 construite ». Les choix R1 à R12 ci-dessous sont donc ceux de l'étude ; R1 à R6 ont chacun une décision proposée. La construction passe par les tickets RP-1 à RP-9 (ESC-597 à ESC-605), qui mettent cette page à jour à mesure : **RP-1 (ESC-597) a construit la porte** (*La porte, construite*, plus bas) ; sans connecteur (RP-4), elle refuse ce qui est d'un réseau privé et laisse sortir le reste.

## Ce que c'est

Des agents qui tournent sur un serveur Escouade (voir *Le mode serveur*) joignent des services qui ne sont visibles que dans le réseau d'une entreprise, derrière son VPN : un git en SSH (`gitlab.acme.internal:22`), un registre de paquets en HTTPS (`nexus.acme.internal:443`), une API appelée par leurs tests (`api-recette.acme.internal:8443`). Ils ne joignent que les hôtes d'une liste, et l'entreprise tient cette liste.

## Les voies (R1)

- **V0, le serveur dans le réseau de l'entreprise** : guidée, rien à construire. `escouade-server` sur une machine de l'entreprise joint ce que son réseau permet ; le client s'y connecte par SSH à travers le VPN de l'utilisateur. Le guide le dit, avec le proxy de sortie de l'entreprise et son certificat racine réglés dans l'espace du serveur. La meilleure quand l'entreprise l'accepte.
- **V2, `escouade-connector`** : **construite**, pour un serveur hors de l'entreprise. Un programme que l'entreprise fait tourner chez elle ouvre une connexion **sortante** vers le serveur et ne laisse passer que les connexions des agents vers sa liste. Aucun port entrant chez l'entreprise.
- **V1, le serveur client du VPN** : non construite ; le guide donne une recette par un SOCKS en espace utilisateur, derrière la porte.
- **V3, le poste en relais** : non construite (elle perd le H24 et contourne la règle du VPN).

## Les choix décidés

- **R1, les voies** : V0 guidée, V2 construite, V1 en recette du guide, V3 non construite (décision proposée `reseau-prive-r1-le-serveur-dans-l-entrep`).
- **R2, le protocole** : une WebSocket sur TLS, sortante, avec à l'intérieur une poignée de main Noise IK dont les deux clés sont épinglées à l'appairage, puis des flux « ouvre `hôte:port` » à contrôle de débit ; une crate `escouade-connector-proto` partagée ; le serveur accepte la version du connecteur et les deux mineures d'avant (`reseau-prive-r2-le-connecteur-parle-par`).
- **R3, l'entrée du serveur** : un port à part (`--connector-listen`, `ESCOUADE_SERVER_CONNECTOR_LISTEN`), éteint par défaut, qui ne parle que le protocole des connecteurs, jamais l'API distante ; la stratégie `servers.privateNetwork` à `false` l'empêche (`reseau-prive-r3-une-entree-des-connecteu`).
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

Dans `escouade-server`, la crate `escouade-gate` (la porte) et le module `privatenet` du cœur (ce que le cœur en fait). Les essais passent par un faux connecteur en mémoire : RP-4 branchera l'entrée réelle des connecteurs sur le trait `Connectors` de la porte (leur état, leur plafond, l'ouverture d'un flux).

### Quand elle tourne

- **Éteinte par défaut**, allumée par `escouade-server --gate` ou `ESCOUADE_SERVER_GATE=1` (aussi `true`, `on`, `yes`), avant le premier agent ; RP-4 l'allumera avec l'entrée des connecteurs. Jamais sur le poste de bureau, dont les process gardent le proxy réglé comme avant (décision proposée `reseau-prive-rp-1-la-porte-s-allume-par`).
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

Chaque connexion, permise ou refusée : l'heure, l'hôte, le port, HTTP ou SOCKS, la route (connecteur et lequel, direct, proxy de l'espace, refusé), la raison d'un refus, les octets dans chaque sens, la durée, l'agent et le projet ; **jamais le contenu ni un jeton**. Les 1 000 dernières sont gardées en mémoire pour l'interface (RP-5) ; chacune est aussi une ligne du journal du serveur.

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
- Relu deux secondes au plus après un changement. Ce ticket ne l'écrit pas : Réglages du projet › Réseau privé (RP-5) et l'appairage (RP-4) le feront. **Aucun outil MCP ne le lit ni ne l'écrit** (R5).

## Le connecteur

- **Le tunnel** : une WebSocket sur TLS, sortante, vers l'entrée des connecteurs du serveur, à travers le proxy de l'entreprise s'il y en a un ; à l'intérieur, Noise, que le proxy ne peut ni lire ni imiter ; des flux « ouvre `hôte:port` ».
- **L'appairage** : Réglages de l'espace › Réseau privé › « Ajouter un connecteur » (ou `escouade-server connector pair`) donne un code (8 lettres, 10 minutes, une fois) et l'adresse de l'entrée ; comme celui d'un appareil, un code faux ne l'annule pas et les essais sont bornés par source et en tout (voir *L'API distante*, « Les bornes », ESC-654). Le service informatique lance `escouade-connector pair <adresse> <code>`. Ni le code ni une clé privée au journal. Retirer un connecteur, d'un côté ou de l'autre, ferme son canal.
- **Deux listes** : le plafond, dans `connector.toml`, et la liste d'un projet, au serveur (R4, R5).
- **Les contrôles** : nom et port dans le plafond après mise en forme ; adresse littérale refusée sauf écrite dans le plafond ; jamais la boucle, le lien local, les métadonnées d'un cloud ni la machine du connecteur ; 256 flux, 32 ouvertures par seconde, 10 minutes de silence.
- **L'agent l'emprunte sans le savoir** (R6, *La porte, construite*). Le reste d'Internet sort comme avant.
- **Les refus** sont lisibles par l'agent : « gitlab.acme.internal:22 n'est pas dans la liste du projet Démo », « … n'est pas permis par le connecteur d'Acme », « le réseau privé d'Acme est injoignable depuis 14:32 ».
- **La coupure** : le connecteur se reconnecte seul ; pendant la coupure, la porte refuse aussitôt ; les flux ouverts sont fermés, sans reprise. Un connecteur coupé depuis 10 minutes dont un projet a besoin fait une demande dans la boîte « À toi ».
- **Ce que voit Escouade, l'éditeur** : rien ; aucun relais à lui. Le serveur voit les noms, ports, octets et durées, pas le contenu chiffré de bout en bout.

## Limites

- TCP seulement : ni UDP, ni ICMP ; rien du réseau de l'entreprise vers le serveur.
- La porte est une consigne, pas une cloison : les agents d'un serveur partagent un compte, et un agent détourné peut ignorer les variables, lire celles d'un autre agent et se présenter pour un autre projet ; ce qui tient, c'est le plafond du connecteur. L'interface le dit.
- Le connecteur borne l'entrée dans le réseau de l'entreprise, pas la sortie du serveur vers Internet (R11).
- Ce qui ne suit pas les variables de proxy (Java : Maven, Gradle ; le `fetch` de Node sans `NODE_USE_ENV_PROXY` ; un programme qui ouvre ses sockets lui-même) ne passe pas par la porte : les noms en boucle locale de l'image (RP-7) le couvriront.
- Le journal de la porte n'est gardé qu'en mémoire : il repart de zéro au redémarrage du serveur, et rien ne le montre encore (RP-5).

## Les tickets

- **RP-1** (ESC-597) : la porte dans `escouade-server`, les variables des agents, `escouade-server via`. **Fait.**
- **RP-2** (ESC-598) : la crate `escouade-connector-proto`.
- **RP-3** (ESC-599) : le programme `escouade-connector`.
- **RP-4** (ESC-600) : l'entrée des connecteurs, l'appairage, la stratégie `servers.privateNetwork`.
- **RP-5** (ESC-601) : Réglages de l'espace et du projet › Réseau privé, le `.escouade.yml`, la boîte « À toi ».
- **RP-6** (ESC-602) : la distribution et la mise à jour du connecteur.
- **RP-7** (ESC-603) : les noms en boucle locale dans l'image.
- **RP-9** (ESC-604) : les guides du site, la fiche pour le service informatique.
- **RP-8** (ESC-605) : les trois cas de bout en bout, en dernier.

Ce qui attend l'utilisateur ensuite : un essai réel dans un vrai réseau d'entreprise (un connecteur derrière un vrai proxy avec inspection TLS, vers un vrai GitLab), et la fiche lue par un vrai service informatique.
