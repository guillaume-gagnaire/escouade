# Installer le serveur d’équipe

Pour qui héberge le serveur d’une équipe : le construire, le lancer avec Docker ou systemd, créer l’équipe, choisir la base et le stockage, le mettre derrière TLS, le sauvegarder, le mettre à jour et le surveiller.

> [!WARNING]
> Le serveur d’équipe est un aperçu de la 2.3. Rien n’est encore publié : ni binaire, ni image. On le construit depuis le dépôt d’Escouade. Les binaires signés et l’image publiée sont à venir.

## Ce que fait le serveur

`escouade-team-server` garde, numérote et diffuse ce que les appareils de l’équipe lui envoient, chiffré de bout en bout. Il n’a aucune clé de contenu et ne lance rien : ni agent, ni commande, ni plugin. Ce qu’il voit est détaillé dans [le guide du RSSI](team/security#ce-que-voit-le-serveur).

C’est un programme à part du mode serveur d’Escouade, qui lance des agents et ne s’expose jamais. Le serveur d’équipe, lui, doit être joignable de tous les membres, en HTTPS.

## Les prérequis

- Une machine Linux joignable de tous les membres, et un nom de domaine qui pointe vers elle (`team.exemple.fr`).
- Pour le compose d’exemple : Docker avec Compose, et les ports 80 et 443 ouverts (Caddy obtient le certificat).
- Pour le binaire : la chaîne Rust du dépôt (`src-tauri/rust-toolchain.toml`).
- En option, une base PostgreSQL et un stockage compatible S3 (MinIO, Garage, AWS).
- Côté membres : Escouade joint un serveur en `https://` ; en `http://` seulement sur la machine même (`127.0.0.1`, `localhost`), pour un essai.

## Construire le binaire

Depuis le dépôt d’Escouade :

```sh
cd src-tauri && cargo build --release -p escouade-team-server
# le binaire : src-tauri/target/release/escouade-team-server
```

`escouade-team-server --help`, ou le programme lancé sans argument, liste les commandes et les options.

> [!WARNING]
> Les sous-commandes ne lisent pas `--help` : `escouade-team-server init --help` fait un vrai `init` dans `./data`. Demande l’aide sans sous-commande.

## Les commandes

| Commande | Ce qu’elle fait |
|---|---|
| `escouade-team-server init` | Crée la base si elle n’existe pas, puis affiche un jeton à usage unique qui crée une équipe. Relancée, elle donne un autre jeton, pour une autre équipe sur le même serveur. |
| `escouade-team-server serve` | Sert, jusqu’à `Ctrl+C` ou l’arrêt du service. |
| `escouade-team-server recovery-token --team <identifiant>` | Affiche un jeton à usage unique, valable 7 jours au plus, pour le détenteur de la clé de secours d’une équipe dont tous les administrateurs ont perdu leurs appareils (voir [Récupérer une équipe](#recuperer-une-equipe)). |

Les trois prennent les mêmes options de base et de fichiers : passe à `init` et à `recovery-token` les mêmes que celles de `serve`.

## Les options

Chaque option a sa variable d’environnement ; l’option passe devant la variable. Une option s’écrit `--listen 0.0.0.0:8443` ou `--listen=0.0.0.0:8443`. Une variable vide compte comme absente.

| Option | Variable | Par défaut | Ce qu’elle règle |
|---|---|---|---|
| `--data <dossier>` | `ESCOUADE_TEAM_DATA` | `./data` | Le dossier de la base SQLite (`team.db`) et des fichiers (`files/`). |
| `--database <adresse>` | `ESCOUADE_TEAM_DATABASE_URL` | SQLite dans `--data` | Une base PostgreSQL : `postgres://…` ou `postgresql://…`. |
| `--files <dossier>` | `ESCOUADE_TEAM_FILES` | `<data>/files` | Le dossier des fichiers chiffrés. |
| `--s3-endpoint <adresse>` | `ESCOUADE_TEAM_S3_ENDPOINT` | aucun | Les fichiers dans un seau S3, adressé par chemin (`<adresse>/<seau>/<préfixe><clé>`). |
| `--s3-bucket <nom>` | `ESCOUADE_TEAM_S3_BUCKET` | aucun | Le seau, obligatoire avec S3. |
| `--s3-region <nom>` | `ESCOUADE_TEAM_S3_REGION` | `us-east-1` | La région. |
| `--s3-prefix <texte>` | `ESCOUADE_TEAM_S3_PREFIX` | vide | Mis devant chaque clé. |
| aucune | `ESCOUADE_TEAM_S3_ACCESS_KEY`, `ESCOUADE_TEAM_S3_SECRET_KEY` | aucune | Les clés de S3, seulement par l’environnement, jamais en argument. |
| `--listen <adresse:port>` | `ESCOUADE_TEAM_LISTEN` | `127.0.0.1:8443` | Où `serve` écoute. |
| `--tls-cert <pem>`, `--tls-key <pem>` | `ESCOUADE_TEAM_TLS_CERT`, `ESCOUADE_TEAM_TLS_KEY` | aucun | TLS par le serveur lui-même ; les deux vont ensemble. Sans eux, mets un proxy inverse devant. |
| `--trust-proxy` | `ESCOUADE_TEAM_TRUST_PROXY=1` | éteint | L’adresse du client est la dernière de `X-Forwarded-For`, celle que le proxy a vue. |
| `--metrics-listen <adresse:port>` | `ESCOUADE_TEAM_METRICS_LISTEN` | éteint | Les métriques Prometheus sur `/metrics`, à leur propre adresse. |
| `--max-file <octets>` | `ESCOUADE_TEAM_MAX_FILE` | `26214400` (25 Mio) | La taille maximale d’un fichier chiffré. |
| `--team-quota <octets>` | `ESCOUADE_TEAM_QUOTA` | `2147483648` (2 Gio) | Ce qu’une équipe garde en tout : ses flux et ses fichiers. |
| aucune | `RUST_LOG` | `info` | Le niveau du journal. |

`ESCOUADE_TEAM_TRUST_PROXY` vaut `1` ou `true`. `recovery-token` lit aussi l’identifiant de l’équipe dans `ESCOUADE_TEAM_ID`.

## Avec Docker et Caddy

Le dépôt donne une image (`crates/escouade-team-server/Dockerfile`, construite depuis la racine du dépôt) et un compose d’exemple dans `crates/escouade-team-server/deploy/` : le serveur, et Caddy devant lui, qui obtient et renouvelle le certificat. Le serveur n’est publié que sur le réseau du compose : seul Caddy l’atteint, WebSocket compris.

```sh
cd crates/escouade-team-server/deploy
export ESCOUADE_TEAM_DOMAIN=team.exemple.fr   # le nom que joignent les membres, pointé vers cette machine
docker compose build
docker compose run --rm team-server init      # affiche le jeton qui crée l’équipe, une fois
docker compose up -d
```

- Les données sont dans le volume `team-data` (la base SQLite et les fichiers).
- Les métriques écoutent sur le port 9464 du réseau du compose, sans être publiées.
- Pour PostgreSQL ou S3, le compose a les lignes en commentaire : la variable `ESCOUADE_TEAM_DATABASE_URL` et le service `postgres`, les variables `ESCOUADE_TEAM_S3_*`.

## Avec systemd

`crates/escouade-team-server/deploy/escouade-team-server.service` lance le serveur sous son propre utilisateur, distinct de tout mode serveur d’Escouade, dans un bac à sable systemd (`ProtectSystem=strict`, aucune capacité, `MemoryDenyWriteExecute`, filtre d’appels système). Il écoute sur `127.0.0.1:8443`, derrière un proxy inverse sur la même machine, et ses métriques sur `127.0.0.1:9464`.

```sh
sudo install -m 0755 escouade-team-server /usr/local/bin/
sudo useradd --system --home-dir /var/lib/escouade-team-server --shell /usr/sbin/nologin escouade-team
sudo install -m 0644 escouade-team-server.service /etc/systemd/system/
sudo install -d -o escouade-team -m 0700 /var/lib/escouade-team-server
sudo -u escouade-team ESCOUADE_TEAM_DATA=/var/lib/escouade-team-server escouade-team-server init
sudo systemctl daemon-reload && sudo systemctl enable --now escouade-team-server
```

Les clés de S3 vont dans un fichier que seul root lit, nommé par `EnvironmentFile=` (la ligne est en commentaire dans le fichier du service).

## Créer l’équipe

1. Lance `init` : il affiche un jeton, une seule fois. Garde-le secret jusqu’à son usage.
2. Le premier administrateur ouvre Escouade, Réglages › Avancé › « Serveur d’équipe (aperçu) » › « Ouvrir les équipes… », puis « Créer une équipe ».
3. Il y met l’adresse du serveur, le jeton (« Jeton de création ») et un nom. Escouade crée l’équipe, puis lui montre sa clé de secours de 24 mots et l’empreinte de l’équipe en six mots (voir [Rejoindre une équipe](team/join)).

Le jeton ne sert qu’une fois. Pour une autre équipe sur le même serveur, relance `init`.

## La base de données

- **SQLite, par défaut** : `team.db` dans `--data`, en WAL (les fichiers `team.db-wal` et `team.db-shm` sont là tant que le serveur tourne).
- **PostgreSQL** : `--database postgres://utilisateur:motdepasse@hôte/base`. La connexion se fait sans TLS : mets la base à côté du serveur (le réseau du compose, une socket Unix, un VPN). Le fichier du service donne l’exemple d’une socket Unix : `postgres://team@/team?host=/run/postgresql`.

Au démarrage, le serveur crée ce qui manque à sa base.

## Les fichiers

Les documents et les images chiffrés vont, par défaut, sur le disque (`<data>/files`, ou `--files`). Avec `--s3-endpoint` et `--s3-bucket`, ils vont dans un seau S3 adressé par chemin : MinIO, Garage, AWS. Chaque fichier est rangé sous `<équipe>/<fichier>`, en hexadécimal, sans nom ni type.

Aujourd’hui, l’app n’y dépose encore aucun fichier : ils viendront avec la spec, les décisions et les documents de l’équipe (à venir).

## TLS et le proxy inverse

Les membres joignent le serveur en HTTPS. Deux façons :

- **Un proxy inverse** (Caddy, nginx) tient TLS et passe les requêtes au serveur, qui écoute sur `127.0.0.1` ou sur le réseau du compose. Mets `--trust-proxy` pour que la limite de débit compte par adresse de client, et non par adresse du proxy. Le proxy doit :
  - passer le WebSocket de `/v1/live` (la diffusion en direct) ;
  - ajouter l’adresse du client à `X-Forwarded-For` ;
  - laisser passer des corps jusqu’à la taille maximale d’un fichier (25 Mio par défaut).

  Le `Caddyfile` d’exemple, en quelques lignes, fait tout cela. Le dépôt ne donne pas de configuration pour nginx.
- **Le serveur lui-même**, par rustls : `--tls-cert` et `--tls-key`, des fichiers PEM lisibles par l’utilisateur du service. Le certificat est lu au démarrage : après un renouvellement, redémarre le serveur.

N’active `--trust-proxy` que derrière un proxy qui est le seul à joindre le serveur : sinon un client choisirait l’adresse que compte la limite de débit.

## Les défenses en place

- Toutes les routes demandent une session, sauf `/healthz`, l’inscription par un jeton, les deux temps du défi signé, la page d’un lien d’invitation et le côté du nouvel appareil dans l’appairage par code court.
- Une limite de débit par adresse (30 requêtes par seconde, 120 d’un coup) et par appareil (20 par seconde, 80 d’un coup). Au-delà : `429` et `Retry-After`.
- Des tailles bornées partout : 1 Mio par entrée d’un flux, 25 Mio par fichier, 2 Gio par équipe, 500 appareils et 10 000 projets par équipe, 4 connexions en direct par appareil, 64 Kio d’en-têtes lus en 15 secondes.

## Sauvegarder et restaurer

Sauvegarde ensemble la base et les fichiers :

- avec SQLite et le disque : le dossier `--data` entier (le volume `team-data` du compose, `/var/lib/escouade-team-server` avec systemd) ;
- avec PostgreSQL : la base, par ses outils ;
- avec S3 : le seau.

Pour une copie cohérente de SQLite, arrête le serveur, copie le dossier (avec `team.db-wal` s’il est là), puis relance-le. Pour restaurer, arrête le serveur, remets le dossier, relance-le.

Une sauvegarde ne contient que des octets chiffrés et les métadonnées listées dans [le guide du RSSI](team/security#ce-que-voit-le-serveur) : elle est peu sensible, et elle ne sert à rien sans les clés des membres, qui ne quittent jamais leurs appareils.

> [!WARNING]
> Chaque appareil retient ce qu’il a déjà lu de l’équipe. Une sauvegarde plus ancienne que ce qu’ont lu les membres leur montre un retour en arrière : leur app le dit (« Le serveur est revenu en arrière sur ce que cet appareil avait déjà lu. ») et cesse de lire l’équipe. Restaure donc la sauvegarde la plus récente, et ce que les membres ont écrit depuis est perdu pour le serveur.

## Récupérer une équipe

Si tous les administrateurs d’une équipe perdent leurs appareils, son propriétaire garde la clé de secours de 24 mots. Toi, l’hébergeur, tu lui donnes un jeton :

```sh
escouade-team-server recovery-token --team <identifiant> --data /var/lib/escouade-team-server
```

L’identifiant de l’équipe (32 chiffres hexadécimaux) est montré dans Escouade, à la création de l’équipe et sur sa carte (« Identifiant de l’équipe »). Le jeton inscrit un appareil comme administrateur du serveur ; il ne donne aucune clé : les 24 mots les donnent. Le détenteur les tape dans « Récupérer une équipe », avec l’adresse du serveur et le jeton.

Avec Docker : `docker compose run --rm team-server recovery-token --team <identifiant>`.

## Mettre à jour

1. Sauvegarde la base et les fichiers.
2. Construis la nouvelle version (`cargo build --release -p escouade-team-server`, ou `docker compose build`).
3. Remplace le binaire et redémarre le service (`sudo systemctl restart escouade-team-server`, ou `docker compose up -d`).

Les sessions et les connexions en direct sont en mémoire : au redémarrage, les apps des membres se reconnectent et signent une nouvelle session d’elles-mêmes.

**La poignée de main des versions.** Chaque requête dit la version du protocole (`Escouade-Protocol`, aujourd’hui `1`). Le serveur accepte sa version et la précédente :

- une app plus ancienne est refusée, et dit « Mets Escouade à jour : le serveur d’équipe ne parle plus cette version » ;
- une app plus récente que le serveur est refusée, et dit que le serveur d’équipe est en retard et que son administrateur doit le mettre à jour.

Mets donc à jour le serveur d’abord : il garde les apps d’une version en arrière.

## Surveiller

- **`/healthz`** répond `200` et `ok` quand la base répond, `503` et `base unavailable` sinon. Elle se passe de la poignée de main des versions : un sondeur la joint sans en-tête.
- **Les métriques** Prometheus, sur `/metrics` à l’adresse de `--metrics-listen` seulement, jamais à l’adresse exposée : les requêtes par méthode, modèle de route et statut, les connexions et les sessions refusées, les refus de la limite de débit, les entrées et les fichiers écrits et leurs octets, les commits refusés parce que leur époque était prise, les trames envoyées en direct, les nombres d’équipes, d’appareils et d’appareils révoqués, les connexions en direct. Aucun identifiant d’équipe, de projet ni d’appareil.
- **Le journal** va sur la sortie d’erreur (`journalctl -u escouade-team-server`, `docker compose logs team-server`), au niveau de `RUST_LOG`. Une ligne par requête : la méthode, le modèle de la route (`/v1/projects/{project}/stream`), le statut et la durée. Jamais un identifiant, une adresse IP, un jeton ni un contenu. Le journal de ton proxy inverse, lui, garde les adresses des clients : règle sa durée de conservation.

## Ce qui n’est pas encore là

- Les binaires signés et l’image publiée (à venir).
- Le lien de création de l’équipe affiché par `init` : aujourd’hui, le jeton se colle dans Escouade (à venir).
- Les rôles propriétaire et lecteur sur le serveur, qui ne connaît que administrateur et membre (à venir).
- Les droits par projet sur le serveur : il sert un projet à tout appareil de l’équipe, et seul le chiffrement protège un projet gardé à quelques membres (à venir).
- Plusieurs serveurs sur une même base : les sessions, les défis, les limites de débit et la diffusion sont en mémoire, un seul processus sert.
