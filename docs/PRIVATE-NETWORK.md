# Atteindre un réseau privé depuis un serveur Escouade (RP-0, l'étude)

Des agents qui tournent sur un serveur Escouade (`escouade-server`, sur un VPS ou dans un conteneur) doivent joindre un git, un registre de paquets ou une API qui ne sont visibles que dans le réseau d'une entreprise, derrière son VPN. Ce document pose le besoin, compare les quatre voies, dessine celle qu'il recommande (`escouade-connector`, une sortie bornée à une liste d'hôtes) et rend les choix à trancher (R1 à R12), avec le découpage en tickets (RP-1 à RP-9).

**Rien n'est tranché ici, et aucun code de produit n'est écrit.** Chaque choix a une recommandation et sa raison ; les choix structurants ont chacun une décision proposée, à accepter ou non.

> **Tranché le 2026-11-21 (ESC-587)** : l'utilisateur a choisi « A · Les recommandations : V0 guidée, V2 construite ». R1 à R12 suivent les recommandations du § 5 ; les tickets RP-1 à RP-9 du § 6 sont ESC-597 (RP-1), ESC-598 (RP-2), ESC-599 (RP-3), ESC-600 (RP-4), ESC-601 (RP-5), ESC-602 (RP-6), ESC-603 (RP-7), ESC-605 (RP-8) et ESC-604 (RP-9).
>
> **RP-1 construit (ESC-597, 2026-11-26)** : la porte est la crate `crates/escouade-gate`, le cœur s'en sert dans `src-tauri/src/privatenet.rs` ; `escouade-server --gate` l'allume en attendant l'entrée des connecteurs (RP-4). Ce qui est construit, et les choix pris en route (décisions proposées `reseau-prive-rp-1-la-porte-s-allume-par`, `reseau-prive-rp-1-les-routes-de-la-porte`, `reseau-prive-rp-1-les-variables-et-ssh-t`), est dans la spec « Escouade à distance › Atteindre un réseau privé », section « La porte, construite ».

La demande de l'utilisateur, le 2026-11-20, en choisissant « Atteindre un réseau privé depuis un serveur » : des agents qui tournent sur un serveur Escouade, avec un git (et d'autres services : registre de paquets, API internes) derrière le VPN d'une entreprise. Il avait dit « pas encore » le 2026-11-15, le mode serveur n'étant pas fait ; il l'est sur `release/2.3` (ESC-514, 515, 537 à 539, 541).

Les fondations sur lesquelles l'étude s'appuie, sans les répéter :

- [`REMOTE.md`](REMOTE.md) : le mode serveur, son API distante sur 127.0.0.1, l'appairage par code (ESC-514), les ports relayés par SSH (ESC-538), WireGuard embarqué prévu pour le téléphone (pas encore construit), « Les agents sur le serveur, en auto-hébergé » ;
- [`SERVER-UX.md`](SERVER-UX.md) : les espaces et « Se connecter à un serveur » ;
- [`TEAM-SERVER.md`](TEAM-SERVER.md) § 13 : une machine en mode serveur est un appareil de l'équipe ;
- la spec « Réseau : proxy et certificats » (le proxy et le fichier d'autorités donnés aux agents) et « Projets privés » (`src-tauri/src/fence.rs`) ;
- l'audit de la 2.2.0 ([`security/audit-2026-11/rapport.md`](security/audit-2026-11/rapport.md)), § 6.1 (« Ce qui sort du poste, ce qui écoute ») et SEC-02 (« Projet privé » est une consigne, pas un cloisonnement).

## 0. En bref

- **Le besoin** tient en trois cas : cloner et pousser sur un GitLab interne en SSH, installer des paquets depuis un registre npm ou Maven interne en HTTPS, faire appeler une API interne par les tests d'un agent (§ 1).
- **Quatre voies** (§ 2, § 3) :
  - **V0, le serveur dans le réseau de l'entreprise** : rien à construire ; l'entreprise héberge la machine, son pare-feu fait la borne. **La meilleure quand l'entreprise l'accepte.**
  - **V1, le serveur client du VPN** : le VPS entre dans le VPN. Un compte VPN à une machine, souvent refusé ; l'authentification forte casse le H24 ; tout le VPN devient joignable par un agent.
  - **V2, `escouade-connector`** : un petit programme dans le réseau de l'entreprise ouvre une connexion **sortante** vers le serveur, et ne laisse passer que les connexions des agents vers **une liste d'hôtes** que le service informatique tient. **Recommandée quand le serveur est hors de l'entreprise.**
  - **V3, le poste en relais** : le portable de l'utilisateur, branché au VPN, relaie pour le serveur. Rien à installer chez l'entreprise, mais il ne marche que quand le portable est allumé et sur le VPN, et il contourne la règle du VPN.
- **La sécurité tient en une phrase** : ce qu'un agent détourné par un contenu piégé peut joindre, c'est ce que le dernier point de passage dans le réseau de l'entreprise laisse passer. Dans V2, c'est la liste du connecteur, entre les mains du service informatique ; la liste par projet du serveur n'est qu'une défense de plus, comme la clôture d'un projet privé (SEC-02).
- **Escouade, l'éditeur, ne voit rien** dans aucune voie : aucun relais à lui, le tunnel va du connecteur au serveur de l'utilisateur, et ce qui passe dedans (SSH, TLS) est chiffré de bout en bout entre l'agent et l'hôte interne.
- **Pour V2** (§ 4) : un tunnel TLS sortant sur le port 443, multiplexé, avec une authentification Noise épinglée à l'appairage (et non WireGuard ni SSH inversé) ; l'agent l'emprunte sans le savoir par un proxy posé dans son environnement (`HTTPS_PROXY`, `ALL_PROXY`) et par le `ProxyCommand` de son `ssh` ; un code d'appairage comme celui d'ESC-514 ; un binaire signé pour Linux et Windows, une image Docker, un service systemd ou Windows ; une mise à jour décidée par le service informatique ; du TCP seulement.
- **À trancher** : R1 à R12 (§ 5), dont six structurants avec une décision proposée chacun. **Puis neuf tickets** (§ 6) : ≈ 27 h d'agent, ≈ 10 h de bout en bout à six agents en parallèle, plus un essai réel que seul l'utilisateur peut faire dans un vrai réseau d'entreprise.

## 1. Le besoin

### 1.1 Les trois cas

Une entreprise, Acme, a son réseau `*.acme.internal`, joignable par son VPN (WireGuard, OpenVPN, ou un client d'entreprise : GlobalProtect, AnyConnect, FortiClient, Zscaler Private Access). L'utilisateur a un serveur Escouade sur un VPS (Coolify, Hetzner…), hors de ce réseau. Ses agents y tournent H24.

| Cas | Le service | Ce que l'agent fait | Ce qui passe |
| --- | --- | --- | --- |
| **C1, le git** | `gitlab.acme.internal`, en SSH (port 22, ou 2222) | `git clone`, `git fetch`, `git push` du dépôt du projet, depuis son worktree ; Escouade fait aussi ses propres `git fetch` (Kanban, PR) | Du SSH : une clé de déploiement ou la clé d'un compte de service, sur le serveur ; la clé d'hôte du GitLab dans `known_hosts` |
| **C2, le registre** | `nexus.acme.internal` (npm, Maven), en HTTPS (443) | `npm install`, `mvn package`, `./gradlew build` ; les paquets internes (`@acme/…`) et le miroir des paquets publics | Du TLS, signé par l'autorité interne d'Acme ; un jeton du registre dans `.npmrc` ou `settings.xml` |
| **C3, l'API des tests** | `api-recette.acme.internal:8443`, en HTTPS | Les tests d'intégration lancés par l'agent (`npm test`, un lancement de test) appellent l'API de recette | Du TLS ; parfois un certificat client ; un jeton de test dans l'environnement du lancement |

Ce qui ne fait pas partie du besoin : que le réseau de l'entreprise joigne le serveur (un webhook d'une CI interne vers Escouade) ; de l'UDP ou de l'ICMP (un DNS interne interrogé par l'agent, un `ping`) ; un partage de fichiers Windows (SMB) ; le bureau à distance d'une machine interne.

### 1.2 Ce que l'utilisateur et le service informatique font, voie par voie

Dans chaque voie, la part de l'utilisateur est la même pour les identifiants des trois cas : une clé SSH sur le serveur, ajoutée au GitLab comme clé de déploiement (C1) ; le jeton du registre dans `.npmrc` ou `settings.xml` du serveur (C2) ; le jeton de recette dans les variables du lancement de test (C3) ; le certificat racine d'Acme dans Réglages › Avancé › Réseau de l'espace du serveur (« Autorités de certification supplémentaires », C2 et C3). Le tableau dit le reste.

| Voie | L'utilisateur | Le service informatique |
| --- | --- | --- |
| **V0, dans le réseau** | Demande une machine Linux ; y installe `escouade-server` (`install.sh` ou l'image) ; s'y connecte par SSH à travers son VPN (« Se connecter à un serveur », avec le `ProxyJump` d'un bastion s'il le faut) ; règle le proxy de sortie d'Acme et son certificat racine dans l'espace, pour que les harness joignent leurs modèles | Fournit la machine (une VM) et son segment réseau ; ouvre, depuis ce segment, le GitLab (C1), le registre (C2) et l'API de recette (C3) ; ouvre la sortie vers Internet pour les API des modèles (par son proxy) ; accepte que des agents qui exécutent des commandes tournent dans son réseau |
| **V1, client du VPN** | Installe le client VPN sur le VPS, et un passage vers lui (un SOCKS en espace utilisateur, § 3.2) ; le relance quand l'authentification expire | Crée un compte VPN pour une machine (souvent interdit : un compte est personnel, avec MFA et contrôle du poste) ; le limite aux trois hôtes s'il sait le faire sur son concentrateur |
| **V2, `escouade-connector`** | Ajoute le connecteur à son espace (Réglages de l'espace › Réseau privé › « Ajouter un connecteur ») et donne le code au service informatique ; choisit, par projet, les hôtes de la liste dont le projet a besoin | Lit la fiche du connecteur (ce qu'il fait, ce qu'il ouvre, ce qu'il journalise) ; l'installe sur une machine de son réseau (un service Windows, systemd ou un conteneur) ; écrit sa liste d'hôtes (`gitlab.acme.internal:22`, `nexus.acme.internal:443`, `api-recette.acme.internal:8443`) ; l'appaire avec le code ; permet sa sortie vers le serveur sur le port 443 dans son proxy ; lit son journal |
| **V3, poste en relais** | Active le relais dans son client de bureau, pendant qu'il est sur le VPN ; écrit la liste d'hôtes ; laisse le portable allumé et sur le VPN tant que les agents en ont besoin | Rien, s'il ne le sait pas ; s'il le sait, il doit l'accepter : son VPN sert alors une machine qu'il ne gère pas |

## 2. Les quatre voies, en une page

```
V0  Réseau d'Acme ─────────────────────────────────────────────┐
     escouade-server (VM d'Acme) ──▶ gitlab, nexus, api-recette │
     poste de l'utilisateur ──VPN──▶ SSH ──▶ escouade-server    │
    ────────────────────────────────────────────────────────────┘

V1  VPS : escouade-server + client VPN ══ tunnel VPN ══▶ concentrateur d'Acme ──▶ tout le VPN

V2  VPS : escouade-server ◀══ TLS 443, sortant d'Acme ══ escouade-connector (machine d'Acme)
           │ la porte (127.0.0.1)                           │ sa liste : gitlab:22, nexus:443, api-recette:8443
           agents                                           ▼
                                                            gitlab, nexus, api-recette

V3  VPS : escouade-server ◀══ API distante (SSH) ══ poste de l'utilisateur ══VPN══▶ Acme
                                                     (le relais et sa liste)
```

## 3. Les voies, comparées

### 3.1 Le tableau

| | V0, dans le réseau | V1, client du VPN | V2, `escouade-connector` | V3, poste en relais |
| --- | --- | --- | --- | --- |
| **Ce qu'elle demande à l'entreprise** | Une VM, son segment, ses règles de pare-feu, la sortie vers les modèles ; accepter des agents chez elle | Un compte VPN pour une machine, hors de sa règle (MFA, poste contrôlé) | Installer un programme ; écrire sa liste ; permettre **une sortie** vers un nom sur le 443. **Aucun port entrant** | Rien, ou accepter un usage de son VPN qu'elle n'a pas prévu |
| **Ports entrants ouverts chez elle** | Aucun (la VM est jointe par son VPN) | Aucun | Aucun | Aucun |
| **Ce qu'un agent détourné peut joindre** | Ce que le segment de la VM permet, et Internet par le proxy d'Acme : la borne est celle d'Acme, avec ses outils | **Tout ce que le compte VPN permet**, sauf si le concentrateur borne ce compte | La liste du connecteur, au plus ; la liste du projet, en défense de plus | La liste du relais, tenue par l'utilisateur ; à défaut, tout ce que le portable joint |
| **La liste d'hôtes** | Le pare-feu d'Acme (par segment) | Le concentrateur d'Acme, s'il sait ; la porte du serveur (§ 4.3) en défense de plus | Au connecteur (le plafond, Acme) et par projet au serveur (l'utilisateur) | Au relais (l'utilisateur) et par projet au serveur |
| **Qui peut la changer** | Acme | Acme (son concentrateur), l'utilisateur (la porte) | Le plafond : Acme seul, sur la machine du connecteur ; la liste d'un projet : l'utilisateur, dans le plafond ; jamais un agent | L'utilisateur |
| **Le DNS interne** | Celui d'Acme, tel quel | Celui du VPN (poussé par le concentrateur) ; à reprendre dans le passage en espace utilisateur | Résolu par le connecteur, dans le réseau d'Acme : l'agent ne voit que des noms, le serveur n'interroge aucun DNS interne | Résolu par le poste |
| **Le journal des connexions** | Celui d'Acme (pare-feu, proxy) | Celui du concentrateur (par compte, pas par agent) | Au connecteur (chaque connexion : quand, l'hôte, le port, les octets, la durée, l'agent et le projet que le serveur annonce) et au serveur | Au poste et au serveur |
| **Ce que voit Escouade, l'éditeur** | Rien | Rien | Rien : aucun relais de l'éditeur ; le tunnel va du connecteur au serveur de l'utilisateur | Rien |
| **Ce que voit le serveur du trafic** | Sans objet | Les paquets IP du VPN | Les noms, ports, octets et durées ; jamais le contenu de SSH ni de TLS (de bout en bout entre l'agent et l'hôte) ; le contenu d'un HTTP en clair passe, sans être lu ni gardé | Les mêmes que V2 |
| **Les systèmes** | Le serveur Linux (x64, arm64) | Le serveur Linux ; le client VPN doit exister pour Linux et tourner sans droits dans l'image | Le connecteur sur Linux (x64, arm64), Windows (x64), en conteneur ; le serveur Linux | Le client de bureau (Windows, macOS, Linux) |
| **L'hors ligne et la reprise** | Comme le mode serveur | L'authentification expire (8 à 12 h souvent, avec MFA) : le VPN tombe et ne se relève pas seul | Le connecteur se reconnecte seul (250 ms … 5 s, puis toutes les 5 s) ; les connexions en cours sont coupées, un `git fetch` se relance ; le serveur répond vite « réseau privé injoignable » | Tombe quand le portable dort, quitte le VPN ou se ferme ; le H24 est perdu |
| **L'effort** | 1 ticket, ≈ 1 h 30 d'agent (le guide) | 2 tickets, ≈ 4 h (la porte de V2, le guide d'un SOCKS en espace utilisateur) | 9 tickets, ≈ 27 h d'agent, ≈ 10 h de bout en bout (§ 6) | 1 ticket de plus après V2, ≈ 3 h (le connecteur en bibliothèque, dans le client) |

### 3.2 V0, le serveur auto-hébergé dans le réseau de l'entreprise

Ce n'est pas une fonction à construire, c'est le mode serveur posé ailleurs. `escouade-server` sur une VM d'Acme joint ce que son segment joint ; le client de l'utilisateur s'y connecte par SSH à travers le VPN qu'il a déjà, comme aujourd'hui (ESC-537 : `~/.ssh/config`, `ProxyJump`).

- **Pour** : rien de neuf, ni dans Escouade ni chez Acme, qu'une VM ; la borne est le pare-feu d'Acme, avec ses outils, ses journaux et sa prévention des fuites (son proxy de sortie voit ce que les agents envoient sur Internet) ; un agent détourné qui voudrait sortir des données d'Acme passe par le proxy qui les surveille déjà.
- **Contre** : il faut qu'Acme héberge des agents qui exécutent des commandes chez elle ; c'est la décision la plus lourde à demander à un RSSI, même si c'est la plus simple techniquement. Le téléphone de l'utilisateur ne joint le serveur que par le VPN d'Acme.
- **Ce qui manque** : un guide (« Escouade dans le réseau de ton entreprise » : la VM, le segment, le proxy et son certificat racine dans l'espace, la connexion des harness à travers le proxy, le `ProxyJump`) et une vérification : que Réglages › Avancé › Réseau, dans l'espace d'un serveur, règle bien le proxy du serveur et le donne à ses agents (servi par l'API distante, à essayer).

### 3.3 V1, le serveur client du VPN

Le VPS entre lui-même dans le VPN d'Acme.

- **Le compte** : un VPN d'entreprise donne un compte à une personne, avec une authentification forte (MFA, SAML) et souvent un contrôle du poste (antivirus, chiffrement). Un compte pour une machine hors de l'entreprise est l'exception ; quand Acme le refuse, la voie est fermée.
- **Le H24** : avec MFA, la session expire et demande un geste ; le serveur perd le réseau privé pendant la nuit.
- **Sans droits** : l'image du serveur tourne sans aucune capacité (`--cap-drop ALL`, `no-new-privileges`, lecture seule). Un client VPN classique veut une interface `tun` et `NET_ADMIN`. Restent les passages en espace utilisateur, qui exposent un SOCKS local sans rien demander au système : `wireproxy` (WireGuard), `openconnect --script-tun` avec `ocproxy` ou `tunsocks` (AnyConnect, GlobalProtect). FortiClient (`openfortivpn`, par PPP) et Zscaler Private Access (son client) veulent les droits du système.
- **La sécurité** : tout ce que le compte VPN joint, un agent détourné le joint, sauf si Acme borne ce compte sur son concentrateur. Escouade peut ajouter la porte du serveur (§ 4.3) devant le SOCKS : une défense de plus, côté serveur, que l'agent contourne s'il parle au SOCKS directement (même compte, même machine : SEC-02).
- **Ce qui manque** : la porte de V2 avec une route « vers un SOCKS local » ; un guide pour `wireproxy` et `ocproxy`. Pas un client VPN dans Escouade : ce serait refaire ce que les VPN font, pour chacun d'eux.

### 3.4 V2, `escouade-connector`

Un programme qu'Acme fait tourner chez elle, sur une machine qui joint les trois hôtes. Il ouvre une connexion **sortante** vers le serveur Escouade et la garde ouverte ; le serveur lui demande, connexion par connexion, « ouvre `gitlab.acme.internal:22` pour l'agent X du projet Y » ; le connecteur vérifie sa liste, résout le nom dans le réseau d'Acme, ouvre la connexion et passe les octets dans les deux sens. Le § 4 le dessine.

- **Pour** : aucun port entrant chez Acme, une seule sortie vers un nom connu, sur le 443 (le port qu'un proxy d'entreprise laisse passer) ; la liste est chez Acme, sur sa machine, et le serveur ne peut pas l'élargir ; Acme a son propre journal, connexion par connexion ; le serveur peut être n'importe où, H24 ; un connecteur se retire d'un geste, des deux côtés.
- **Contre** : un programme de plus à faire accepter et à tenir à jour chez Acme ; un tunnel sortant qui laisse entrer des connexions est exactement ce qu'un RSSI surveille (Cloudflare Tunnel, ngrok) : la fiche du connecteur doit répondre d'avance à ses questions (§ 4.9) ; du TCP seulement.
- **La sécurité** : un agent détourné joint au plus la liste du connecteur. Il peut, en revanche, **sortir sur Internet** ce qu'il y lit (le code du GitLab, une réponse de l'API) : le serveur a Internet, il en a besoin pour les modèles. Le connecteur borne l'entrée dans le réseau d'Acme, pas la sortie du serveur vers Internet (R11).

### 3.5 V3, le poste en relais

Le client de bureau de l'utilisateur, branché au VPN, relaie pour le serveur : le serveur lui envoie ses demandes de connexion par l'API distante (ou par une redirection SSH inverse sur le tunnel qu'il tient déjà), et le poste les ouvre dans le VPN. C'est le sens inverse des ports relayés d'ESC-538.

- **Pour** : rien à installer chez Acme ; quelques heures d'agent une fois le connecteur écrit (le relais est le connecteur, en bibliothèque, dans le client).
- **Contre** : il ne marche que quand le portable est allumé, réveillé, sur le VPN et le client ouvert : le H24 du mode serveur est perdu ; le VPN d'Acme sert alors une machine qu'Acme ne gère pas, ce que la règle d'usage d'un VPN d'entreprise interdit le plus souvent ; un client VPN à découpage du trafic (split tunnel) ou à contrôle de l'appli (Zscaler) peut aussi le refuser.
- **La sécurité** : la liste est celle du relais, tenue par l'utilisateur, pas par Acme. Une redirection SSH inverse dynamique (`ssh -R <port>`, un SOCKS sur le serveur qui sort par le poste) n'a aucune liste : tout ce que joint le portable deviendrait joignable par un agent. À écarter.

## 4. `escouade-connector`, la voie recommandée hors de l'entreprise

### 4.1 Les pièces

```
escouade-server (VPS)                                   machine d'Acme
┌────────────────────────────────────────────┐          ┌──────────────────────────────────┐
│ agents, lancements, terminaux, git          │          │ escouade-connector               │
│   │ HTTPS_PROXY / ALL_PROXY (127.0.0.1:P)   │          │  - le plafond (connector.toml)   │
│   │ ssh -o ProxyCommand="escouade-server    │          │  - la résolution (DNS d'Acme)    │
│   │      via %h %p"                         │          │  - les contrôles (§ 4.4)         │
│   ▼                                          │          │  - le journal                    │
│ la porte : liste du projet, routes, journal │          │        │                         │
│   │                                          │  TLS 443 │        ▼                         │
│ l'entrée des connecteurs (port C, éteinte ◀─┼──────────┼── gitlab:22, nexus:443,          │
│   par défaut)                                │ sortant  │    api-recette:8443              │
└────────────────────────────────────────────┘ d'Acme   └──────────────────────────────────┘
```

- **La porte** (dans `escouade-server`) : un proxy sur 127.0.0.1, HTTP `CONNECT` et SOCKS5 (avec le nom de l'hôte, jamais une adresse résolue par l'agent), par lequel passe toute connexion sortante des agents. Pour chaque connexion : un hôte de la liste du projet de l'agent va au connecteur de ce réseau ; un hôte privé hors de la liste est refusé ; le reste (les API des modèles, npm public) sort directement, ou par le proxy réglé dans l'espace.
- **L'entrée des connecteurs** (dans `escouade-server`) : un port à part, éteint par défaut, qui ne parle que le protocole des connecteurs, jamais l'API distante (R3).
- **Le connecteur** (`escouade-connector`, un programme à part, sans rien d'Escouade d'autre) : sa liste, sa résolution, ses contrôles, son journal ; il n'écoute sur aucun port.

### 4.2 Le protocole : trois candidats

| | WireGuard embarqué (celui du téléphone) | Un tunnel TLS sortant multiplexé | SSH inversé |
| --- | --- | --- | --- |
| **Ce qui sort de chez Acme** | De l'UDP vers le port WireGuard du serveur | Du TCP vers le 443 du serveur, à travers le proxy d'Acme s'il y en a un (`CONNECT`) | Du TCP vers le 22 du serveur |
| **Passe le pare-feu d'une entreprise** | Souvent non : l'UDP sortant est fermé hors du DNS, et WireGuard se reconnaît (le RSSI y voit un VPN pirate) | Oui, c'est le cas courant : une sortie HTTPS vers un nom, que le proxy d'Acme autorise par une ligne | Souvent non : le 22 sortant est fermé ; il faut un compte SSH sur le serveur pour la machine d'Acme |
| **L'unité de passage** | Des paquets IP (couche 3) : le connecteur doit terminer le TCP lui-même (`smoltcp`) puis le rouvrir dans le réseau d'Acme | Des flux, chacun « ouvre `hôte:port` » : la liste par nom et le DNS chez Acme vont de soi | Des flux ; `ssh -R <port>` sans destination donne un SOCKS sur le serveur, **sans aucune liste côté Acme** ; avec une destination, une redirection par hôte |
| **La liste et le DNS** | Par adresse IP ; le DNS d'Acme à relayer à part | Par nom et port, résolus par le connecteur | Aucune liste côté Acme en dynamique ; `permitlisten` borne seulement l'écoute sur le serveur |
| **Ce qu'un scan voit du serveur** | Rien : un paquet non signé par une clé connue est ignoré | Un serveur TLS ; sans clé connue, la connexion est fermée après le premier message (§ 4.5) | Un `sshd` |
| **Sous une inspection TLS (Zscaler)** | Sans objet (UDP) | Le TLS extérieur est ouvert par le proxy ; l'authentification intérieure (Noise, épinglée à l'appairage) tient : le proxy ne peut ni lire ni se faire passer pour le serveur | Sans objet |
| **Ce qui existe dans Escouade** | Rien encore : `boringtun` et `smoltcp` sont prévus pour le téléphone, pas commencés | La WebSocket de l'API distante, ses bornes, l'appairage par code et les jetons (ESC-478, 495, 514) ; `rustls` | Le `ssh` du système, côté client (ESC-537) ; rien côté serveur |
| **Ce que le connecteur demande à sa machine** | Rien (espace utilisateur) | Rien | Un client `ssh` et une clé à garder |

**Recommandation : le tunnel TLS sortant multiplexé** (R2). C'est le seul qui passe le pare-feu d'une entreprise sans lui demander plus qu'une ligne dans son proxy, c'est celui qui tient la liste par nom et le DNS chez Acme sans traduire de paquets, et il reprend ce que l'API distante a déjà (WebSocket, bornes, appairage). WireGuard reste meilleur face à un scan, mais l'UDP sortant est le premier fermé chez une entreprise ; SSH inversé n'a aucune liste côté Acme.

Sa forme :

- **Le transport** : une WebSocket sur TLS (`wss://<serveur>:<port C>/connector`), passée par le proxy d'Acme s'il y en a un (`HTTPS_PROXY` du connecteur, et le fichier d'autorités d'Acme pour une inspection TLS), comme les connexions de l'app (spec « Réseau : proxy et certificats »).
- **L'authentification, à l'intérieur** : une poignée de main Noise (`Noise_IK`, la crate `snow`) dans les premiers messages de la WebSocket, avec la clé statique du serveur et celle du connecteur, épinglées l'une chez l'autre à l'appairage. Le TLS extérieur ne sert qu'à passer : un proxy qui l'ouvre ne voit que du Noise chiffré, et ne peut pas se faire passer pour le serveur. Le certificat TLS du serveur peut être celui d'un proxy devant lui (Traefik, Caddy, Let's Encrypt) ou un certificat à lui : rien n'en dépend.
- **Les flux** : dans le canal Noise, des trames `open {id, hôte, port, agent, projet}`, `opened {id}` ou `refused {id, raison}`, `data {id, octets}`, `close {id}`, et un contrôle de débit par flux (une fenêtre, comme HTTP/2 : un `git clone` lent ne bloque pas un `npm install`). Une crate à part, `escouade-connector-proto`, partagée par le serveur et le connecteur.
- **Les versions** : le connecteur et le serveur se disent la leur ; le serveur accepte la sienne et les deux mineures d'avant (le service informatique met à jour à son rythme, pas à celui de l'utilisateur), et dit au connecteur trop vieux ce qu'il faut faire. C'est différent de l'API distante, qui veut la même version des deux côtés.
- **La vie** : un `ping` toutes les 15 s ; muet 45 s, le canal est dit coupé ; le connecteur rouvre (250 ms, 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s, comme le client), le serveur rejette tout flux pendant la coupure (§ 4.8).
- **Construit** (RP-2, ESC-598) : `crates/escouade-connector-proto`. Sa forme exacte (la suite Noise, les versions dans la poignée de main, le format des trames, la `data` vide qui clôt un sens, les fenêtres de 256 Kio, le connecteur plus récent que le serveur refusé) est dans la décision `reseau-prive-rp-2-la-forme-du-protocole`.

### 4.3 Comment l'agent l'emprunte sans le savoir

L'agent ne sait rien du connecteur : il tape `git clone git@gitlab.acme.internal:acme/app.git`, `npm install`, `npm test`. Trois entrées, chacune pour ce qu'elle sert :

1. **Un proxy dans l'environnement** (C2, C3, git en HTTPS, les harness eux-mêmes) : `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` et leurs minuscules, vers la porte (`http://<jeton de l'agent>@127.0.0.1:P`, `socks5h://…`), avec `NO_PROXY=127.0.0.1,localhost` (les serveurs de test des agents ne passent pas par elle). Escouade pose déjà ces variables pour le proxy réglé (`Settings::proxy_env`, aux agents par `network_env`, aux lancements et aux terminaux quand « le proxy aussi pour les terminaux » est coché) : la porte prend leur place, et devient le seul proxy des agents ; le proxy d'Acme ou de l'hébergeur, s'il y en a un, passe derrière elle. Ce qui suit ces variables : git en HTTPS, curl, npm, pip, Go, Python (`requests`), les harness en Node qui les lisent. Ce qui ne les suit pas : Java (Maven, Gradle, qui veulent `settings.xml` ou `-Dhttps.proxyHost`), le `fetch` de Node sans `NODE_USE_ENV_PROXY`, un programme qui ouvre ses sockets lui-même.
2. **Le `ProxyCommand` de `ssh`** (C1) : `GIT_SSH_COMMAND=ssh -o ProxyCommand="escouade-server via %h %p"`, posé par le cœur (un plugin ne peut pas poser `GIT_SSH_COMMAND` : `crates/escouade-plugins/src/procs.rs` le refuse, et c'est bien). `escouade-server via` parle à la porte sur sa sortie et son entrée standard : un hôte de la liste passe par le connecteur, un autre est joint directement (GitHub reste joignable). La config SSH de l'utilisateur et ses clés restent les siennes ; le cœur écrit aussi `core.sshCommand` dans les worktrees qu'il crée, pour un `git` lancé sans l'environnement de l'agent.
3. **Des noms en boucle locale** (C2 pour Java, C3 pour un client qui ignore les proxys), dans l'image seulement : `extra_hosts` du Compose fait de `nexus.acme.internal` l'adresse `127.77.0.3` du conteneur, où la porte écoute sur le 443 et passe les octets tels quels (le certificat est vérifié de bout en bout, par le nom). Il faut `net.ipv4.ip_unprivileged_port_start=0` dans le Compose (un `sysctl` du réseau du conteneur, permis sans capacité) pour écouter sous 1024. Hors de l'image, `/etc/hosts` veut root : cette entrée n'y est pas (R6).

Ce qui ne passe pas par la porte : les connexions d'Escouade lui-même (le cœur, ses plugins), sauf les `git fetch` qu'il lance pour un projet, qui prennent les mêmes variables que ses agents.

**La porte est une consigne, pas une cloison** (comme la clôture d'un projet privé, SEC-02) : tous les agents du serveur tournent sous le même compte ; un agent détourné peut ignorer les variables, lire celles d'un autre agent (`/proc/<pid>/environ`, même compte) et se présenter à la porte pour un autre projet. Ce qui tient vraiment, c'est le plafond du connecteur, chez Acme, et le fait que le réseau d'Acme n'est joignable **que** par la porte (le VPS n'a aucune route vers lui). Un vrai cloisonnement par projet veut un compte du système ou un conteneur par projet (`REMOTE.md`, « L'isolement ») : hors de ce chantier, à dire dans l'interface.

### 4.4 La liste, le DNS et les contrôles du connecteur

- **Deux listes** (R4) :
  - **le plafond**, dans `connector.toml` sur la machine du connecteur, écrit par le service informatique : des `hôte:port` (`gitlab.acme.internal:22`), des motifs (`*.npm.acme.internal:443`), au besoin des plages d'adresses permises pour la résolution (`10.20.0.0/16`). Le connecteur l'envoie au serveur à chaque connexion, pour que l'interface la montre ; le serveur ne peut pas l'élargir, il ne fait que demander ;
  - **la liste d'un projet**, au serveur : les hôtes du plafond dont ce projet a besoin, choisis par l'utilisateur. Un projet sans liste n'atteint rien du réseau privé.
- **Qui change quoi** (R5) : le plafond, Acme seule, sur sa machine (le connecteur le relit à chaud, et le journal le dit) ; la liste d'un projet, l'utilisateur, dans Réglages du projet › Réseau privé, ou par le `.escouade.yml` du dépôt, qui la **propose** et qu'il accepte par son empreinte, comme les commandes d'un `.escouade.yml` (`projectfile.rs`) ; une stratégie imposée ou le `.escouade.yml` d'équipe peuvent la fixer. **Jamais un agent** : aucun outil MCP ne la lit ni ne l'écrit.
- **Le DNS** (R7) : l'agent ne donne que des noms (`socks5h`, `CONNECT hôte:port`) ; la porte ne résout pas un nom privé, le connecteur le résout avec le DNS de sa machine, dans le réseau d'Acme. Le serveur n'a besoin d'aucun DNS interne, et l'agent non plus.
- **Les contrôles du connecteur**, pour chaque `open` :
  - le nom et le port dans le plafond, comparés après mise en forme (minuscules, sans point final, IDN en `xn--`) ;
  - une adresse IP littérale refusée, sauf écrite telle quelle dans le plafond ;
  - l'adresse résolue vérifiée **après** la résolution, et c'est elle qui est jointe (pas une seconde résolution : pas de DNS rebinding) : jamais la boucle locale, le lien local (`169.254.0.0/16`, dont `169.254.169.254`, les métadonnées d'un cloud), l'adresse de la machine du connecteur elle-même, ni, si le plafond en donne, une adresse hors de ses plages ;
  - des bornes : 256 flux ouverts à la fois, 32 ouvertures par seconde, un flux muet fermé après 10 min (réglables dans `connector.toml`).
- **Le refus** revient à l'agent comme un refus de proxy lisible : une réponse `403` de `CONNECT` avec la raison (« gitlab.acme.internal:22 n'est pas dans la liste du projet Démo », « … n'est pas permis par le connecteur d'Acme »), un code SOCKS « connexion non permise », une ligne sur la sortie d'erreur pour `ssh`.

### 4.5 L'appairage

Comme celui d'un appareil (ESC-514), avec les clés de Noise à la place d'un jeton :

1. Dans l'espace du serveur, Réglages de l'espace › Réseau privé › « Ajouter un connecteur » (ou `escouade-server connector pair` sur le serveur) : un nom (« Acme »), puis un code de 8 lettres (40 bits), valable 10 minutes, une fois, 5 essais faux au plus, et l'adresse de l'entrée (`wss://escouade.exemple.fr:8443/connector`).
2. Le service informatique : `escouade-connector pair wss://escouade.exemple.fr:8443/connector 8NUC-W6HS`. Le connecteur fait sa paire de clés (gardée dans un fichier lisible du seul compte du service, ou dans le magasin de Windows), envoie sa clé publique et le code ; le serveur répond sa propre clé publique ; chacun garde celle de l'autre.
3. Ensuite, une connexion n'est acceptée que par ces deux clés : sans clé connue, le serveur ferme après le premier message Noise, sans rien dire d'autre ; comme pour les appareils, chaque essai raté compte (20 par minute, puis tout est fermé jusqu'à la fin de la minute).
4. **Retirer** : de l'espace (« Retirer le connecteur » : sa clé ne vaut plus, son canal est fermé), ou chez Acme (`escouade-connector unpair`, ou arrêter le service). Une trace des connecteurs retirés, comme celle des appareils révoqués.

Ni le code ni une clé privée ne vont au journal, d'un côté comme de l'autre.

### 4.6 La distribution

- **Un programme à part**, `escouade-connector`, en Rust, sans Tauri, sans rien du cœur : `rustls`, `tokio`, `snow`, la crate du protocole. Environ 5 à 8 Mo, statique (`musl`) sous Linux.
- **Linux** x64 et arm64 : l'archive signée (la clé de l'updater, comme `escouade-server`, avec sa SHA-256 et son attestation de provenance), un `install.sh` qui pose un service systemd du système sous un compte `escouade-connector` sans shell (`DynamicUser=yes`, `ProtectSystem=strict`, `ProtectHome=yes`, `NoNewPrivileges`, `RestrictAddressFamilies=AF_INET AF_INET6`, `CapabilityBoundingSet=` vide).
- **Windows** x64 : un `.exe` signé par Azure Artifact Signing (comme l'app), qui s'installe lui-même en service (`escouade-connector install`, le compte `LocalService`), son journal dans l'Observateur d'événements ; un `.msi` plus tard si un service informatique le demande (déploiement par Intune ou GPO).
- **L'image** `ghcr.io/…/escouade-connector` (distroless, sans shell, uid non root, lecture seule, sans capacité), pour Docker ou Kubernetes, multi-architecture, attestée.
- **macOS** : pas au départ (un connecteur tourne sur une machine de serveur, rarement un Mac) ; le binaire se construit, sans service ni signature.
- **Aucune télémétrie**, aucune connexion qu'à son serveur (et au proxy d'Acme, s'il en a un) ; pas même pour sa mise à jour, sauf si on la lui demande (§ 4.7).

### 4.7 La mise à jour

- **Par défaut, manuelle** : le service informatique décide. `escouade-connector update [--to X.Y.Z] [--check]`, vérifiée par la signature comme `escouade-server update` (l'ancien gardé en `.old`, le service relancé) ; dans l'image, une nouvelle image.
- **Automatique, à la demande** (`auto_update = true` dans `connector.toml`) : il regarde `latest.json` une fois par jour et se met à jour quand aucun flux n'est ouvert.
- **Ce que voit l'utilisateur** : la version du connecteur dans Réglages de l'espace › Réseau privé ; « à mettre à jour » quand il est à deux mineures du serveur ; « trop ancien » au-delà, le canal refusé avec ce qu'il faut faire, dit aussi dans le journal du connecteur.

### 4.8 L'hors ligne et la reprise

- **Le connecteur coupé** (sa machine éteinte, le réseau d'Acme coupé, un proxy qui ferme le canal) : il rouvre seul ; pendant la coupure, la porte refuse aussitôt un flux vers ce réseau (« le réseau privé d'Acme est injoignable depuis 14:32 ») au lieu de laisser l'agent attendre ; les flux ouverts sont fermés, sans reprise (un `git fetch`, un `npm install` se relancent ; un TCP ne se reprend pas au milieu).
- **Le serveur redémarré** (une mise à jour) : les connecteurs rouvrent leur canal à son retour ; rien n'est à refaire.
- **Un proxy d'entreprise qui coupe les connexions longues** (souvent après 1 h à 8 h) : le connecteur rouvre, comme pour une coupure.
- **L'interface** : l'état de chaque connecteur dans l'espace (connecté, coupé depuis, dernière connexion, version, son plafond), et une demande de la boîte « À toi » quand un connecteur dont des projets ont besoin est coupé depuis plus de 10 minutes.

### 4.9 La fiche pour le service informatique

Le guide du site a une page à lui, écrite pour un RSSI, qui répond d'avance :

- **ce qu'il ouvre** : une sortie TCP vers un nom, sur un port (le 443 conseillé), à travers son proxy s'il le veut ; aucun port entrant ; aucun port qui écoute sur la machine du connecteur ;
- **ce qu'il laisse entrer** : des connexions TCP vers les seuls `hôte:port` de son plafond, qu'il écrit lui-même ; le serveur ne peut pas l'élargir ;
- **ce qu'il journalise** : chaque connexion, permise ou refusée, en JSON sur la sortie standard (journald, l'Observateur d'événements, les logs du conteneur) : l'heure, l'hôte, le port, l'adresse résolue, les octets, la durée, l'agent et le projet annoncés par le serveur (à titre indicatif : c'est le serveur qui les dit) ; jamais le contenu ;
- **ce qu'il garde** : sa clé privée et celle, publique, du serveur ; rien d'autre ;
- **comment l'arrêter** : arrêter le service, ou `unpair` ;
- **ce qu'il ne fait pas** : de l'UDP, de l'ICMP, un relais vers Internet, un accès au réseau d'Acme pour qui que ce soit d'autre que le serveur appairé ;
- **ce qui reste à sa charge** : la sortie des données. Un agent qui lit le GitLab peut envoyer ce qu'il lit sur Internet depuis le serveur ; le connecteur ne l'empêche pas (R11).

### 4.10 Les limites

- **TCP seulement** : ni UDP (un DNS interne interrogé directement, Kerberos en UDP, QUIC/HTTP3 vers un hôte interne, qui retombe en TCP), ni ICMP (`ping`, `traceroute`), ni multicast.
- **Rien du réseau d'Acme vers le serveur** : un webhook d'une CI interne ne joint pas Escouade par le connecteur.
- **L'adresse source** que voient les hôtes d'Acme est celle de la machine du connecteur : une règle qui ne permet que certaines adresses doit la permettre.
- **L'authentification d'un proxy d'Acme par NTLM ou Kerberos**, pour la sortie du connecteur : pas au départ (Basic seulement) ; une machine de serveur a souvent une règle à elle dans le proxy.
- **Java** : Maven et Gradle ne suivent pas `HTTPS_PROXY` ; il faut leur config de proxy (que l'étape de préparation d'un projet peut écrire), ou les noms en boucle locale de l'image (§ 4.3) ; l'autorité interne d'Acme doit aussi être dans leur magasin de certificats (le fichier d'autorités d'Escouade ne va qu'aux harness en Node et à Codex).
- **Le débit** est celui de la sortie d'Acme et du serveur ; un gros dépôt se clone une fois, dans le worktree de base.
- **Un agent joint ce que liste son projet, pas moins** : la liste n'est pas une liste de chemins ni de méthodes (un `push` vers une autre branche du GitLab passe) ; les droits fins restent ceux des comptes (la clé de déploiement en lecture seule, un jeton du registre en lecture).

## 5. Les choix à trancher

Chaque choix a une recommandation, retenue tant que l'utilisateur ne la change pas. Les choix marqués **(décision)** ont chacun une décision proposée dans le registre, à accepter ou non à la validation de ce ticket : `reseau-prive-r1-le-serveur-dans-l-entrep`, `reseau-prive-r2-le-connecteur-parle-par`, `reseau-prive-r3-une-entree-des-connecteu`, `reseau-prive-r4-un-plafond-au-connecteur`, `reseau-prive-r5-la-liste-d-un-projet-est`, `reseau-prive-r6-l-agent-emprunte-la-port`.

1. **R1. Les voies offertes (décision).** Recommandé : **V0 d'abord, dite et guidée** (rien à construire) ; **V2 construite** pour un serveur hors de l'entreprise ; **V1 en recette du guide seulement** (un SOCKS en espace utilisateur, derrière la porte) ; **V3 non construite**. Pourquoi : V0 laisse la borne au pare-feu de l'entreprise et ne coûte rien ; V2 est la seule qui garde le H24, n'ouvre aucun port chez l'entreprise et lui laisse la liste ; V1 dépend d'un compte VPN qu'une entreprise refuse le plus souvent, et d'un client par VPN ; V3 perd le H24 et contourne la règle du VPN. Autre : V3 en dépannage, derrière un avertissement (RP-10, ≈ 3 h, après V2).
2. **R2. Le protocole du connecteur (décision).** Recommandé : **un tunnel TLS sortant multiplexé** (WebSocket sur TLS, Noise IK à l'intérieur, épinglé à l'appairage, des flux `hôte:port`). Pourquoi : il passe le proxy d'une entreprise par une ligne, la liste par nom et le DNS restent chez l'entreprise, il reprend la WebSocket et l'appairage de l'API distante. Autres : WireGuard embarqué (silencieux face à un scan, mais l'UDP sortant est fermé chez la plupart des entreprises, la liste se fait par adresse, et le WireGuard du téléphone n'est pas commencé) ; SSH inversé (aucune liste côté entreprise).
3. **R3. L'entrée du serveur pour les connecteurs (décision).** Recommandé : **un port à part** (`--connector-listen`, `ESCOUADE_SERVER_CONNECTOR_LISTEN`), éteint par défaut, qui ne parle que le protocole des connecteurs ; il peut aussi être servi derrière le proxy TLS d'un domaine (Traefik de Coolify, Caddy), le Noise intérieur ne dépendant pas du certificat. Pourquoi : l'API distante reste sur 127.0.0.1, jamais sur Internet ; ce qui est exposé se réduit à TLS, une WebSocket et une poignée de main Noise qui ferme sans clé connue. Autre : l'entrée dans l'API distante (un seul port, mais l'API sur Internet, ce que le mode serveur s'interdit).
4. **R4. Où se tient la liste (décision).** Recommandé : **un plafond au connecteur**, écrit par l'entreprise, que le serveur ne peut pas élargir, **et une liste par projet au serveur**, dans ce plafond, vide par défaut. Pourquoi : la borne qui tient face à un agent détourné doit être hors de la machine des agents ; la liste par projet évite qu'un projet personnel du même serveur joigne le GitLab d'Acme par erreur, et dit à l'utilisateur ce dont chaque projet a besoin. Autres : la liste au serveur seul (l'entreprise n'a plus la main), au connecteur seul (tous les projets du serveur joignent tout le plafond).
5. **R5. Qui change la liste d'un projet (décision).** Recommandé : **l'utilisateur**, dans Réglages du projet › Réseau privé ; le `.escouade.yml` du dépôt la **propose**, accordée par son empreinte comme ses commandes ; une stratégie ou le `.escouade.yml` d'équipe peuvent la fixer ; **jamais un agent** (aucun outil MCP). Pourquoi : un dépôt piégé ne doit pas ouvrir seul le réseau d'une entreprise ; un agent détourné non plus.
6. **R6. Comment l'agent l'emprunte (décision).** Recommandé : **le proxy dans l'environnement** (`HTTPS_PROXY`, `ALL_PROXY` vers la porte, qui devient le seul proxy des agents) **et le `ProxyCommand` de `ssh`** (`GIT_SSH_COMMAND`, `core.sshCommand`) d'abord ; **les noms en boucle locale** ensuite, dans l'image seulement, pour Java et ce qui ignore les proxys. Pourquoi : les deux premiers couvrent C1, C2 en npm et C3 sans rien demander au système ; le troisième veut un `sysctl` du conteneur et ne marche pas hors de l'image. Autre : les trois d'emblée.
7. **R7. Le DNS.** Recommandé : **résolu par le connecteur**, des noms seulement (une adresse littérale refusée sauf écrite dans le plafond), l'adresse résolue vérifiée puis jointe telle quelle, jamais la boucle, le lien local ni la machine du connecteur. Pourquoi : le serveur et l'agent n'ont besoin d'aucun DNS interne ; une seule résolution ferme le DNS rebinding.
8. **R8. Le journal.** Recommandé : **aux deux bouts**, en JSON, chaque connexion permise ou refusée, l'agent et le projet annoncés par le serveur, jamais le contenu ; au serveur, Réglages de l'espace › Réseau privé › Journal (les 1 000 dernières). Pourquoi : l'entreprise veut le sien, sur sa machine, sans faire confiance au serveur ; l'utilisateur veut voir ce que ses agents ont demandé.
9. **R9. Les systèmes du connecteur.** Recommandé : **Linux (x64, arm64), Windows (x64) et l'image** ; macOS sans service ni signature. Pourquoi : une entreprise met un tel programme sur une VM Linux, un serveur Windows ou son Kubernetes.
10. **R10. La mise à jour du connecteur.** Recommandé : **manuelle par défaut, automatique sur demande**, signée ; le serveur accepte sa version et les deux mineures d'avant. Pourquoi : le service informatique ne met pas à jour au rythme de l'utilisateur, et ne veut pas d'un programme qui change seul dans son réseau.
11. **R11. La sortie du serveur vers Internet.** Recommandé : **hors de ce chantier, dite dans la fiche et dans l'interface.** Un agent qui lit le réseau d'Acme peut envoyer ce qu'il lit sur Internet ; le borner vraiment veut que les agents n'aient aucune autre route que la porte (un réseau Docker `internal` et la porte dans un conteneur à part, avec une liste des API des modèles), un chantier à lui. Autre : une option « sortie bornée aussi vers Internet » dans la porte, en consigne (les variables), que l'agent peut contourner.
12. **R12. Plusieurs connecteurs.** Recommandé : **un serveur, plusieurs connecteurs**, un par réseau (Acme, un client d'Acme), chacun son plafond, chaque hôte de la liste d'un projet rattaché à son connecteur ; deux connecteurs pour le même réseau (la redondance), plus tard. Pourquoi : un consultant travaille pour plusieurs entreprises depuis le même serveur ; la redondance n'est pas le premier besoin.

## 6. Le découpage en tickets (voie V2)

Estimé au rythme d'agents de Claude Code (Opus, effort élevé), en heures d'agent, six en parallèle. Tous partent de `release/2.3` (le mode serveur fait), après les choix R1 à R6.

| # | Ticket | Ce qu'il livre | Estimation |
| --- | --- | --- | --- |
| **RP-1** · la porte | Dans `escouade-server` | Le proxy sur 127.0.0.1 (HTTP `CONNECT`, SOCKS5 par nom), un jeton par agent dans l'adresse du proxy, les routes (un connecteur, direct, le proxy de l'espace derrière), la liste par projet (lue, appliquée), les refus lisibles, le journal ; les variables données aux agents, aux lancements, aux terminaux et aux `git` du cœur à la place de celles du proxy ; `escouade-server via %h %p` et `GIT_SSH_COMMAND`, `core.sshCommand` dans les worktrees. Essais contre un faux connecteur en mémoire | 3 h 30 |
| **RP-2** · le protocole | La crate `escouade-connector-proto` | La WebSocket sur TLS (par un proxy `CONNECT`, le fichier d'autorités), Noise IK (`snow`), les trames et le contrôle de débit par flux, les versions (la sienne et deux mineures d'avant), le `ping`, les bornes ; des vecteurs de test et deux cibles de fuzzing (`fuzz/`) | 3 h |
| **RP-3** · le connecteur | Le programme `escouade-connector` | `connector.toml` (le serveur, le plafond, les bornes, relu à chaud), `pair`, `unpair`, `run`, `check` (vérifie que chaque hôte du plafond se résout et répond), la résolution et ses contrôles (§ 4.4), la reconnexion, le journal en JSON ; ni code ni clé au journal. Essais : un faux serveur, des hôtes locaux, les refus (métadonnées, boucle, adresse littérale, rebinding) | 3 h 30 |
| **RP-4** · l'entrée | Dans `escouade-server` | `--connector-listen`, l'appairage (`escouade-server connector pair`, `connectors`, `connectors remove`), les clés dans `secrets.json`, le plafond reçu, l'état de chaque connecteur, la trace des retirés ; les commandes servies pour l'espace ; la stratégie `servers.privateNetwork` (à `false`, aucune entrée) | 3 h |
| **RP-5** · l'interface | Dans le client | Réglages de l'espace › Réseau privé (les connecteurs, leur état et leur plafond, « Ajouter un connecteur » et son code, le journal) ; Réglages du projet › Réseau privé (les hôtes choisis dans le plafond) ; la liste proposée par le `.escouade.yml`, accordée par empreinte ; la demande de la boîte « À toi » d'un connecteur coupé ; la limite « une consigne, pas une cloison » dite ; textes en français et en anglais ; la spec (cette page, Réglages, `.escouade.yml`) | 4 h |
| **RP-6** · la distribution | La release du connecteur | Les jobs de la release (Linux `musl` x64 et arm64, Windows signé, l'image distroless multi-architecture), les signatures et les attestations, `install.sh` et son service systemd, `escouade-connector install` en service Windows, `update` signé et son mode automatique ; les essais du service (systemd dans un conteneur, comme `server-install-test.sh`) | 3 h 30 |
| **RP-7** · les noms en boucle locale | Dans l'image du serveur | La porte qui écoute sur `127.77.x.y:<port>` pour chaque hôte d'une liste, `extra_hosts` et le `sysctl` dans les Compose d'exemple, les octets passés tels quels ; un essai avec un client qui ignore les proxys (Java) | 2 h |
| **RP-8** · les trois cas de bout en bout | Les essais | Un faux réseau privé local (un `sshd` git, un registre npm, une API en TLS signée par une autorité de test), le connecteur, le serveur, un agent du faux `claude` qui clone (C1), installe (C2) et lance des tests (C3) ; un hôte refusé, un connecteur coupé puis repris, un connecteur trop ancien ; un e2e dans la CI Linux, sans aucun réseau réel | 2 h 30 |
| **RP-9** · les guides | Le site, en français et en anglais | « Atteindre un réseau privé » (les quatre voies, comment choisir), « Escouade dans le réseau de ton entreprise » (V0), « Le connecteur » et la fiche pour le service informatique (§ 4.9), la recette d'un SOCKS en espace utilisateur (V1) ; la FAQ « Où vont mes données ? » | 2 h |

- **Ordre** : RP-1 et RP-2 en parallèle ; puis RP-3 et RP-4 en parallèle ; puis RP-5, RP-6, RP-7 et RP-9 en parallèle ; RP-8 en dernier. **Total : ≈ 27 h d'agent, ≈ 10 h de bout en bout.**
- **V0 seule** (si R1 ne garde qu'elle) : RP-9 réduit à son guide, ≈ 1 h 30 ; **V1** : RP-1 avec une route « vers un SOCKS local » et sa recette, ≈ 4 h.
- **RP-10** (si R1 garde V3) : le relais dans le client de bureau, le connecteur en bibliothèque, ≈ 3 h, après RP-3.
- **Ce qui attend l'utilisateur** : les choix R1 à R6 avant RP-1 ; puis un **essai réel** dans un vrai réseau d'entreprise (un connecteur derrière un vrai proxy d'entreprise, avec inspection TLS, vers un vrai GitLab), que seul l'utilisateur peut faire, et la lecture de la fiche par un vrai service informatique.
- **Hors de ces tickets** : la sortie du serveur vers Internet bornée (R11) ; un compte du système ou un conteneur par projet ; un connecteur pour le téléphone ou le poste lui-même (le poste a déjà son VPN).

## 7. Sources

- La demande de l'utilisateur du 2026-11-20 (le ticket ESC-551) et celle du 2026-11-15 (« pas encore »).
- `docs/REMOTE.md` : « La fondation commune », « Le mode serveur », MS-1 (ESC-514, l'appairage), MS-3 (ESC-515, la distribution, `install.sh`, l'image), « Ce qui change de machine, MS-4 » (ESC-538, les ports relayés), « Le téléphone » (WireGuard embarqué, pas encore construit), « Les agents sur le serveur, en auto-hébergé » (l'isolement).
- `docs/SERVER-UX.md` (les espaces, les états de connexion), `docs/TEAM-SERVER.md` § 13.
- La spec : « Escouade à distance › Le mode serveur », « Fenêtre › Réseau : proxy et certificats », « Projets › Projets privés ».
- Le code : `crates/escouade-model/src/model.rs` (`proxy_env`, `network_env`), `src-tauri/src/commands.rs` et `src-tauri/src/core.rs` (le proxy aux lancements et aux terminaux), `crates/escouade-plugins/src/procs.rs` (les variables refusées aux plugins, `GIT_SSH_COMMAND` compris), `src-tauri/src/fence.rs` (la clôture), `src-tauri/src/projectfile.rs` (l'accord par empreinte).
- L'audit de la 2.2.0 : § 6.1, § 6.3, SEC-02.
