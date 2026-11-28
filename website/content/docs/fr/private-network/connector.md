# Le connecteur

Le programme escouade-connector, que l’entreprise fait tourner chez elle : il ouvre une connexion sortante vers ton serveur et ne laisse passer que les hôtes de sa liste. L’installer, l’appairer, écrire sa liste, choisir celle d’un projet, le vérifier, le mettre à jour, le retirer, et ses limites.

> [!WARNING]
> Le connecteur est un aperçu de la 2.3, comme le [mode serveur](server) : ses fichiers, son image et son script d’installation sont publiés avec la release de la 2.3.

## Comment ça marche

```
Serveur Escouade (ton VPS)                          Réseau d’Acme
  agents ──▶ la porte (127.0.0.1) ──▶ l’entrée ◀══ TLS, sortant ══ escouade-connector
                                                                    │ son plafond : gitlab:22, nexus:443
                                                                    ▼
                                                             gitlab, nexus, api-recette
```

- **Le connecteur ouvre la connexion**, de chez l’entreprise vers ton serveur : une WebSocket sur TLS, à travers le proxy de l’entreprise s’il y en a un. Aucun port entrant chez elle, aucun port qui écoute sur sa machine. À l’intérieur, une poignée de main Noise dont les deux clés sont épinglées à l’appairage : ni un proxy qui inspecte le TLS ni un autre serveur ne peuvent se faire passer pour l’un ou l’autre.
- **Le serveur lui demande**, connexion par connexion, « ouvre `gitlab.acme.internal:22` pour tel agent de tel projet ». Le connecteur vérifie son plafond, résout le nom dans le réseau de l’entreprise, vérifie l’adresse, l’ouvre et passe les octets.
- **Deux listes** : le **plafond**, dans `connector.toml`, écrit par le service informatique sur sa machine, que le serveur ne peut pas élargir ; et la **liste de chaque projet**, sur le serveur, choisie par toi dans ce plafond, vide par défaut.
- **L’agent l’emprunte sans le savoir.** Sur le serveur, la porte devient le seul proxy des agents (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` vers `127.0.0.1`), et leur `ssh` passe par elle (`ProxyCommand="escouade-server via %h %p"`). Un hôte de la liste du projet va au connecteur ; le reste d’Internet (les modèles, npm public, GitHub) sort comme avant, directement ou par le proxy de l’espace.

## Ouvrir l’entrée du serveur

L’entrée des connecteurs est éteinte par défaut. Elle est à part de l’API distante : un port à elle, qui ne parle que le protocole des connecteurs.

```sh
escouade-server --connector-listen 0.0.0.0:8443 --connector-url wss://escouade.exemple.fr:8443/connector
```

| Option | Variable | Ce qu’elle fait |
|---|---|---|
| `--connector-listen <adresse:port>` | `ESCOUADE_SERVER_CONNECTOR_LISTEN` | Ouvre l’entrée, et la porte des agents avec elle. Une adresse et un port, toujours : `0.0.0.0:8443` |
| `--connector-cert <fichier>`, `--connector-key <fichier>` | `ESCOUADE_SERVER_CONNECTOR_CERT`, `ESCOUADE_SERVER_CONNECTOR_KEY` | Son certificat TLS, en PEM. Sans eux, l’entrée est derrière le proxy TLS d’un domaine (le Traefik de Coolify, Caddy) |
| `--connector-url wss://…/connector` | `ESCOUADE_SERVER_CONNECTOR_URL` | Son adresse telle que les connecteurs la joignent, que l’appairage leur donne. Sans elle, faite de l’adresse d’écoute, et dite « devinée » au journal |
| `--gate` | `ESCOUADE_SERVER_GATE` | La porte seule, sans entrée |

- **Le port 443 passe partout.** Un proxy d’entreprise laisse sortir le 443 ; un autre port demande une règle de plus au service informatique. Le plus simple : un domaine derrière le proxy TLS de ton hébergeur, qui mène au port de l’entrée, et `--connector-url wss://escouade.exemple.fr/connector`. Déclare ce proxy (`--trust-proxy`) pour que chaque source compte ses essais à part.
- **En Docker**, publie le port de l’entrée (`-p 8443:8443`) ou mène-le par le proxy de ton hébergeur ; l’API distante, elle, reste sur `127.0.0.1`.
- **Une stratégie** peut l’interdire : `servers.privateNetwork` à `false` (voir [les stratégies du poste](managed-policies)) empêche l’entrée, la porte et l’appairage.

## Ajouter un connecteur

Sur l’espace du serveur, Réglages de l’espace › Réseau privé › « Ajouter un connecteur » : donne-lui le nom du réseau (« Acme »), puis « Créer le code ». Ou, sur le serveur :

```sh
escouade-server connector pair Acme
```

Tu reçois un code de 8 lettres, comme `ABCD-EFGH`, valable 10 minutes et une seule fois, l’adresse de l’entrée et la commande à donner au service informatique : `escouade-connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH`. Le code n’est montré qu’une fois ; Escouade ne le garde pas.

- Le code ne passe jamais sur le réseau, même sous une inspection TLS : il sert de clé à la poignée de main de l’appairage.
- Un code faux, expiré ou déjà utilisé est refusé sans un mot, et n’annule jamais le code qui attend. Les essais sont bornés : 20 ratés par minute et par source, 5 codes faux par source pendant la vie d’un code, 20 ratés par minute pour toute l’entrée ; un connecteur déjà appairé entre toujours.
- Ni le code ni une clé privée ne sont écrits au journal.

## Installer le connecteur

Sur une machine de l’entreprise qui joint les hôtes dont tes projets ont besoin. Le service informatique le fait, avec [la fiche](private-network/it-sheet) en main.

### Linux

Linux x64 ou arm64 avec systemd. Le script vérifie la somme SHA-256 et la signature de l’archive, pose le programme dans `/usr/local/bin` et un service systemd verrouillé, et ne lance rien avant l’appairage.

```sh
curl -fsSLo install-connector.sh https://github.com/guillaume-gagnaire/escouade/releases/latest/download/install-connector.sh
less install-connector.sh
sudo sh install-connector.sh
sudoedit /etc/escouade-connector/connector.toml
sudo escouade-connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH
sudo escouade-connector check
sudo systemctl enable --now escouade-connector
```

Le service tourne sous un compte à lui sans shell ni dossier personnel, le système en lecture seule, sans aucune capacité, IPv4 et IPv6 seulement. Son journal va à journald : `journalctl -u escouade-connector`. Le mot de passe d’un proxy va plutôt dans `/etc/escouade-connector/proxy.env` (`HTTPS_PROXY=…`, mode 0600, lu par systemd seul), `connector.toml` étant lisible par les comptes de la machine.

### Windows

Windows x64 : `escouade-connector-<version>-windows-x64.exe`, signé. Dans une console d’administrateur :

```powershell
escouade-connector install
notepad C:\ProgramData\escouade-connector\connector.toml
escouade-connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH
escouade-connector check
sc start escouade-connector
```

`install` copie le programme dans `%ProgramFiles%\escouade-connector` et fait le service `escouade-connector` sous `LocalService`, démarré avec la machine et relancé après un échec. Son journal va dans l’Observateur d’événements (Journaux Windows › Application, source `escouade-connector`). Sa clé est scellée par DPAPI pour la machine, son dossier ouvert à SYSTEM, aux administrateurs et à LocalService seuls.

### L’image

`ghcr.io/guillaume-gagnaire/escouade/escouade-connector`, pour `linux/amd64` et `linux/arm64` : le programme seul, sans shell, un utilisateur sans privilège, en lecture seule, sans capacité. Le dépôt donne un Compose et un manifeste Kubernetes d’exemple (`deploy/connector/`) :

```sh
docker compose run --rm connector pair wss://escouade.exemple.fr:8443/connector ABCD-EFGH
docker compose up -d
docker compose logs -f connector
```

`connector.toml` est monté en lecture seule à côté du Compose, les clés gardées dans un volume. Aucun port n’est publié. Une seule réplique : une paire de clés, un connecteur.

## Écrire `connector.toml`

Le service informatique l’écrit. Sous Linux `/etc/escouade-connector/connector.toml`, sous Windows `%ProgramData%\escouade-connector\connector.toml`, ailleurs `--config <fichier>`.

```toml
server = "wss://escouade.exemple.fr:8443/connector"     # facultatif : pair le donne aussi
proxy = "http://proxy.acme.internal:3128"               # facultatif : sinon HTTPS_PROXY et NO_PROXY
authorities = "/etc/escouade-connector/acme-root.pem"    # autorités en plus (une inspection TLS)
allow = [
  "gitlab.acme.internal:22",
  "*.npm.acme.internal:443",
  "10.20.1.5:8443",
]
ranges = ["10.20.0.0/16"]                                # facultatif : toute adresse jointe y est
auto_update = false

[limits]
max_streams = 256        # de 1 à 4096
opens_per_second = 32    # de 1 à 1024
idle_minutes = 10        # de 1 à 1440
```

- **Le plafond**, `allow` : `hôte:port` ; `*.suffixe:port` pour chaque nom sous le suffixe, pas le suffixe lui-même ; une adresse écrite telle quelle (`10.20.1.5:8443`, `[fd00::5]:22`), seule façon de permettre une adresse. Vide, rien ne passe.
- **`ranges`** : facultatif ; chaque adresse jointe doit y être.
- **Le proxy de l’entreprise**, pour la sortie du connecteur : `http://` seulement, avec une authentification Basic.
- **Une clé inconnue est une erreur**, comme une règle illisible, dite avec sa ligne : une faute de frappe ne passe pas pour une règle oubliée.
- **Relu à chaud**, toutes les 2 secondes : le changement est dit au journal et renvoyé au serveur ; un flux que le nouveau plafond ne permet plus est fermé. Un fichier qui ne se lit plus est dit, et le dernier lu gardé.

### Ce que le connecteur refuse toujours

Pour chaque connexion, dans l’ordre : ses bornes (`max_streams`, `opens_per_second`) ; le plafond, après mise en forme du nom ; une seule résolution, par le DNS de sa machine ; puis chaque adresse résolue vérifiée. Jamais la boucle locale, le lien local, les métadonnées d’un cloud (`169.254.169.254`), l’adresse non précisée, la diffusion, une adresse de sa propre machine ni une adresse hors de `ranges`, même cachée dans une IPv6. L’adresse vérifiée est jointe telle quelle, sans seconde résolution : un DNS qui change sa réponse ne fait rien joindre d’autre.

## Appairer et vérifier

- **`escouade-connector pair <adresse> <code>`** (ou `-` à la place du code, pour le lire sur l’entrée standard, hors de la liste des process) : il fait une nouvelle paire de clés, s’appaire, et ne les garde qu’une fois le serveur d’accord. Il dit l’adresse et les deux clés publiques, jamais le code. Un code faux : « the code is wrong, expired or already used (ask a new one) ».
- **`escouade-connector check`** : pour chaque règle du plafond, la résolution, les contrôles et une connexion, comme pour un agent : « ok gitlab.acme.internal:22: … answers at 10.20.1.5:22 (12 ms) », ou la raison. Il dit aussi s’il est appairé. Code de sortie 1 si un hôte ne répond pas.
- **Dans l’espace**, Réglages de l’espace › Réseau privé montre chaque connecteur : connecté depuis, coupé depuis, sa version, son plafond tel qu’il l’a envoyé.

Les textes du connecteur (commandes, journal, raisons) sont en anglais.

## Choisir la liste d’un projet

Un projet n’atteint rien d’un réseau privé tant que tu ne l’as pas choisi : Réglages du projet › Réseau privé, les hôtes du plafond de chaque connecteur dont ce projet a besoin. Un motif du plafond (`*.npm.acme.internal:443`) se précise en un nom (`registry.npm.acme.internal`). Un hôte hors du plafond est refusé : seul le service informatique l’élargit, dans `connector.toml`.

Le `.escouade.yml` du dépôt peut la proposer :

```yaml
private_network:
  hosts:
    - gitlab.acme.internal:22
    - nexus.acme.internal:443
```

Rien n’est atteint avant ton accord (« Accepter cette liste ») ; si la liste change, l’accord est redemandé. Le `.escouade.yml` d’une équipe peut la recommander ou l’imposer (voir [.escouade.yml](escouade-yml)). **Aucun agent** ne la lit ni ne l’écrit : un dépôt piégé ou un agent détourné n’ouvrent pas seuls le réseau d’une entreprise.

## Ce que voit l’agent

Il lance `git clone git@gitlab.acme.internal:acme/api.git`, `npm install` ou ses tests comme d’habitude. Un refus lui est dit en clair, et il peut te le rapporter :

- « gitlab.acme.internal:22 n’est pas dans la liste du projet Démo » : ajoute l’hôte dans Réglages du projet › Réseau privé ;
- « … n’est pas permis par le connecteur d’Acme » : il est hors du plafond, à demander au service informatique ;
- « le réseau privé d’Acme est injoignable depuis 14:32 » : le connecteur est coupé.

Un connecteur coupé : la porte refuse aussitôt au lieu de laisser l’agent attendre ; les connexions ouvertes sont fermées, sans reprise (un `git fetch`, un `npm install` se relancent) ; le connecteur se reconnecte seul. Coupé depuis plus de 10 minutes alors qu’un projet en a besoin, il fait une demande dans [la boîte « À toi »](inbox).

### Java et ce qui ignore les proxys

Maven, Gradle, le `fetch` de Node sans `NODE_USE_ENV_PROXY` ou un programme qui ouvre ses sockets lui-même ne suivent pas `HTTPS_PROXY`. Dans l’image du serveur, `--gate-loopback` (`ESCOUADE_SERVER_GATE_LOOPBACK=1`) donne à chaque hôte des listes une adresse de la boucle locale où la porte écoute ; `escouade-server gate-hosts` écrit les lignes `extra_hosts` à mettre dans le Compose. Le TLS reste de bout en bout : l’autorité interne de l’entreprise doit être dans le magasin de Java.

## Le journal

- **Au connecteur** : une ligne JSON par événement, sur la sortie standard (journald, les journaux du conteneur) ou dans l’Observateur d’événements. Chaque connexion permise (`allowed`, puis `closed` avec les octets et la durée) ou refusée (`refused`, avec son code et sa raison) : l’heure, l’hôte, le port, l’adresse jointe, l’agent et le projet annoncés par le serveur. Jamais le contenu, le code d’appairage ni une clé privée.
- **Au serveur** : Réglages de l’espace › Réseau privé › « Le journal », les 1 000 dernières connexions des agents par la porte, permises ou refusées, avec la route et la raison d’un refus. Il repart de zéro au redémarrage du serveur.

## Mettre à jour

Le serveur accepte un connecteur de sa version et des deux mineures d’avant ; à deux mineures, l’espace le dit « à mettre à jour ». Au-delà, ou plus récent que le serveur, le canal est refusé, et l’état dit quoi faire.

- **À la main**, par défaut : `sudo escouade-connector update` (en administrateur sous Windows), `--to X.Y.Z` pour une version précise, `--check` pour seulement la dire. La signature est vérifiée avant de rien toucher, jamais une version plus ancienne, l’ancien programme gardé en `escouade-connector.old`, le service relancé.
- **Automatique**, si le service informatique le demande : `auto_update = true` dans `connector.toml`. Il regarde une fois par jour, et ne se met à jour que quand aucune connexion n’est ouverte.
- **Dans l’image**, une nouvelle image : `docker compose pull && docker compose up -d`.

## Retirer un connecteur

- **De ton côté** : Réglages de l’espace › Réseau privé › « Retirer », ou `escouade-server connectors remove Acme`. Sa clé ne vaut plus rien, son canal est fermé dans les deux secondes, et sa trace est gardée (nom, date, par qui). `escouade-server connectors` les liste.
- **Du côté de l’entreprise** : arrêter le service, ou `escouade-connector unpair`, qui écrit des zéros sur la clé privée puis l’efface. Pour retirer le programme : `sudo sh install-connector.sh --uninstall` (`--purge` retire aussi `connector.toml` et la clé), `escouade-connector uninstall` sous Windows.

Pour le remettre, il faut un nouveau code et un nouvel appairage.

## Les limites

- **TCP seulement** : ni UDP (un DNS interne interrogé directement, Kerberos en UDP, QUIC), ni ICMP (`ping`, `traceroute`).
- **Rien du réseau de l’entreprise vers le serveur** : un webhook d’une CI interne ne joint pas Escouade par le connecteur.
- **L’adresse source** que voient les hôtes de l’entreprise est celle de la machine du connecteur : une règle qui ne permet que certaines adresses doit la permettre.
- **Le proxy de l’entreprise**, pour la sortie du connecteur : `http://` et Basic seulement, ni NTLM ni Kerberos. Une machine de serveur a souvent une règle à elle dans le proxy.
- **Java** : Maven et Gradle ne suivent pas `HTTPS_PROXY` ; leur config de proxy, ou les noms en boucle locale de l’image, et l’autorité interne dans leur magasin de certificats.
- **Une consigne, pas une cloison** : la liste d’un projet est une consigne donnée à ses agents. Tous les agents d’un serveur partagent un même compte, et un agent détourné peut ignorer ses variables ou se présenter pour un autre projet. Ce qui tient vraiment, c’est le plafond du connecteur.
- **La sortie vers Internet n’est pas bornée** : le connecteur borne l’entrée dans le réseau de l’entreprise, pas la sortie de ton serveur vers Internet, dont les agents ont besoin pour leurs modèles. Un agent qui lit un hôte interne peut envoyer ce qu’il lit ailleurs.
- **Ce que la liste ne dit pas** : ni chemins ni méthodes. Un `push` vers une autre branche du GitLab passe ; les droits fins restent ceux des comptes (une clé de déploiement en lecture seule, un jeton du registre en lecture).
