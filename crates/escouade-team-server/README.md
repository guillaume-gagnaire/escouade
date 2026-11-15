# escouade-team-server : le serveur d'équipe, aveugle (TS-1)

Ticket ESC-456, ligne TS-1 de `docs/TEAM-SERVER.md` §16. Choix tranchés par l'utilisateur le 2026-11-14 (§15.2) : un binaire à part (C1-A), MLS (C9-B, maquetté dans `crates/escouade-team-crypto`), le chiffrement de bout en bout comme seul mode (C10-A). Rien n'est publié : les binaires et l'image viendront à la sortie de la 2.3.

## En bref

- **Aveugle.** Le serveur ne garde que des octets chiffrés et les métadonnées du §3.8. Il n'exécute rien : aucun agent, aucune commande, aucun plugin, aucun harness. Il n'a aucune clé de contenu.
- **Les appareils** : un id, une clé publique Ed25519, une équipe, un rôle (`admin` ou `member`), révoqué ou non. Le premier appareil s'inscrit avec le jeton à usage unique que donne `init`, et devient administrateur de l'équipe qu'il crée. Un administrateur demande ensuite un jeton d'inscription pour chaque nouvel appareil : c'est le point d'entrée des invitations de TS-3.
- **L'authentification par défi signé** (§2.6) : le serveur donne un défi, l'appareil le signe, le serveur rend un jeton de session de 15 minutes. Aucun mot de passe. Toutes les routes demandent une session, sauf `/healthz`, l'inscription par jeton et les deux temps du défi.
- **Deux sortes de flux ordonnés** : un par équipe (les messages MLS) et un par projet (les opérations chiffrées de TS-5). Le serveur numérote, garde et diffuse. Pour les commits MLS, le premier d'une époque gagne, comme dans `relay.rs`. Un second pour la même époque est refusé (409 `epoch_taken`), avec les entrées depuis le commit gagnant. La reprise se fait par `?since=n`.
- **Les fichiers chiffrés** (documents, images) : sur le disque ou dans un stockage compatible S3 (MinIO, Garage, AWS), derrière le trait `files::Blobs`. Une taille maximale par fichier, et un quota par équipe qui compte les fichiers et les flux.
- **La diffusion en direct** par WebSocket (`/v1/live`) : un appareil abonné reçoit les nouvelles entrées de tous les flux de son équipe.
- **La base** : SQLite en WAL par défaut, PostgreSQL en option, derrière le trait `db::Store`.

## Ce que le serveur voit et garde

C'est la table du §3.8, et rien de plus.

| Où | Ce qui y est | Ce qui n'y est jamais |
| --- | --- | --- |
| `teams` | un id opaque (16 octets, tiré par le serveur), l'heure de création, le dernier numéro du flux d'équipe, le nombre de commits (l'époque), les octets gardés | un nom |
| `devices` | un id opaque, l'équipe, la clé publique Ed25519, le rôle, les heures de création et de révocation | un nom, une adresse, le membre derrière l'appareil |
| `tokens` | le **SHA-256** des jetons d'inscription non utilisés, leur sorte, leur équipe, leur expiration (un jeton utilisé est effacé) | le jeton lui-même |
| `team_entries`, `project_entries` | le numéro, la sorte (`commit` et son époque, ou `message`), l'appareil auteur, l'heure, **les octets chiffrés** | ce qu'ils disent : un commit MLS part en message privé, une opération est chiffrée par la clé du projet |
| `projects` | un id opaque choisi par le client, son dernier numéro | le nom du projet, ses dépôts |
| `files` | un id opaque choisi par le client, la taille, l'appareil auteur, l'heure | le nom du fichier, son type, son contenu |
| les fichiers (disque ou S3) | les octets chiffrés, sous `<équipe>/<fichier>` en hexadécimal | |
| la mémoire du processus | les défis en cours (60 s), le SHA-256 des jetons de session (15 min), les seaux des limites de débit (par adresse IP, par appareil) | |
| le journal (`RUST_LOG`) | la méthode, **le modèle de la route** (`/v1/projects/{project}/stream`), le statut, la durée | un id d'équipe, de projet, d'appareil ou de fichier, une adresse IP, un jeton, un contenu |
| les métriques (`/metrics`) | des compteurs par méthode, modèle de route et statut ; les nombres d'équipes, d'appareils et d'appareils révoqués ; les octets écrits ; les connexions en direct | aucun id d'équipe, de projet ni d'appareil |

Les tests le vérifient : `ops::the_metrics_say_no_id` cherche chaque id dans les métriques, et `ops::init_then_serve` dans le journal du vrai binaire.

Ce que ça révèle (§3.8) : l'activité (qui écrit quand, sur combien de projets, à quel rythme), la taille des objets, le nombre d'époques. Les adresses IP ne servent qu'à la limite de débit, en mémoire.

## Le protocole (version 1)

- **CBOR** (RFC 8949) dans les deux sens, `Content-Type: application/cbor`. Les objets chiffrés y voyagent en chaînes d'octets, jamais en base64. Un fichier voyage en `application/octet-stream`, brut. Les corps sont dans `api::wire`.
- **La poignée de main des versions** : chaque requête de `/v1` dit `Escouade-Protocol: <n>`. Le serveur accepte sa version et la précédente. Une plus vieille, ou pas d'en-tête, est refusée en 426 `client_too_old`, avec le message « Mets Escouade à jour ». Une plus récente est refusée en 400 `server_too_old` : le serveur est en retard. Chaque réponse dit la version du serveur dans le même en-tête.
- **Un refus** est un corps CBOR `{error, message, detail?}`, avec un `error` stable pour le client.
- **Le défi signé** : l'appareil signe `"escouade-team-server/auth/v1\0" ‖ id (16 octets) ‖ défi (32 octets)` avec sa clé Ed25519 (vérification stricte). Un défi sert une fois, pendant 60 s.

| Route | Session | Rôle |
| --- | --- | --- |
| `GET /healthz` | non | `ok`, si la base répond |
| `POST /v1/enroll` `{token, public_key}` | non | inscrit un appareil avec un jeton à usage unique : celui d'`init` crée l'équipe |
| `POST /v1/auth/challenge` `{device}` | non | un défi, pour un appareil connu et non révoqué |
| `POST /v1/auth/session` `{device, challenge, signature}` | non | un jeton de session (`Authorization: Bearer …`) |
| `GET /v1/devices` | oui | les appareils de l'équipe : id, clé publique, rôle, révoqué |
| `POST /v1/devices/enrollments` `{expires_in?}` | administrateur | un jeton d'inscription pour un appareil de plus (24 h par défaut, 7 jours au plus) |
| `POST /v1/devices/{id}/revoke` | administrateur, ou l'appareil lui-même | révoque : ses sessions et ses connexions en direct se ferment |
| `POST /v1/team/stream` `{kind, epoch?, payload}` | oui | ajoute au flux d'équipe ; un `commit` doit être pour l'époque suivante |
| `GET /v1/team/stream?since=n&limit=m` | oui | les entrées après `n`, une page au plus (`more` dit s'il en reste) |
| `POST`, `GET /v1/projects/{id}/stream` | oui | de même pour un projet (des `message` seulement) |
| `PUT`, `GET`, `DELETE /v1/files/{id}` | oui | un fichier chiffré, écrit une fois (409 s'il existe) |
| `GET /v1/files/usage` | oui | les octets gardés, le quota, les tailles maximales |
| `GET /v1/live` (WebSocket) | oui | des trames CBOR `{stream: "team" \| "project" \| "lagged", project?, entry?}` ; fermée en 4001 à la fin de la session, en 4003 à la révocation |

Les ids passent en hexadécimal dans les chemins (32 chiffres), en octets dans les corps.

## Les défenses (§2.3)

- **Aucune route sans session**, hors `/healthz`, l'inscription et le défi.
- **Une limite de débit par adresse et par appareil** (seaux à jetons ; 30 requêtes/s et 120 d'un coup par adresse, 20/s et 80 par appareil). Au-delà : 429 et `Retry-After`. Derrière un proxy inverse, `--trust-proxy` prend la dernière adresse de `X-Forwarded-For`, celle que le proxy a vue.
- **Des tailles bornées partout** : une entrée de flux (1 Mio), un fichier (25 Mio, `--max-file`), le quota d'une équipe (2 Gio, `--team-quota`), une page de lecture (500 entrées, 8 Mio), les corps des autres routes (64 Kio), 500 appareils et 10 000 projets par équipe, 4 connexions en direct par appareil, les trames reçues sur le WebSocket (16 Kio), les en-têtes HTTP (64 Kio, lus en 15 s). **Une longueur annoncée trop grande** (`Content-Length`) est refusée en 413 avant de lire un octet. Une chaîne CBOR qui annonce plus d'octets qu'elle n'en porte est refusée en 400, sans allouer.
- **TLS** : par rustls avec un certificat donné (`--tls-cert`, `--tls-key`, PEM), ou derrière un proxy inverse (Caddy, nginx) qui le tient. Le compose d'exemple prend Caddy, qui obtient et renouvelle le certificat (ACME).
- **Les métadonnées** du §3.8, et rien de plus (ci-dessus).
- `/healthz` et les métriques Prometheus sans id. Les métriques ont leur propre adresse (`--metrics-listen`), jamais l'adresse exposée.

## La base : SQLite ou PostgreSQL, et pourquoi pas sqlx

Le §14 proposait sqlx pour les deux. Mais `libsqlite3-sys` déclare `links = "sqlite3"` : un workspace n'en tient qu'une version. Le verrou de l'app (`src-tauri/Cargo.lock`) tient la 0.39.1, par rusqlite 0.41. Or sqlx 0.8.6 demande la 0.30.1, et sqlx 0.9.0 une version `>=0.30.1, <0.38`. **Le serveur prend donc rusqlite 0.41 pour SQLite**, à la version du verrou, **et tokio-postgres pour PostgreSQL**, tous deux derrière le trait `db::Store`. Le verrou n'a changé aucune version existante : il a seulement gagné les crates nouvelles (axum, ciborium, tokio-postgres…).

- **SQLite** : en WAL, `synchronous = NORMAL`, une connexion, chaque écriture dans une transaction `BEGIN IMMEDIATE`.
- **PostgreSQL** : une connexion, chaque écriture dans une transaction qui verrouille la ligne de l'équipe (`SELECT … FOR UPDATE`). La numérotation et le premier commit d'une époque tiennent donc aussi sous la concurrence ; un index unique `(équipe, époque)` sur les commits le garantit en plus. La connexion est en clair (`NoTls`) : la base est à côté du serveur (le réseau du compose, une socket Unix, un VPN).

## Installer

### Le binaire

```sh
cd src-tauri && cargo build --release -p escouade-team-server   # target/release/escouade-team-server
escouade-team-server init  --data /var/lib/escouade-team-server  # crée la base, affiche le jeton de création
escouade-team-server serve --data /var/lib/escouade-team-server --listen 127.0.0.1:8443 --trust-proxy
escouade-team-server --help                                       # toutes les options, et leurs variables
```

`init` affiche un jeton à usage unique : le premier administrateur le donne à Escouade avec l'adresse du serveur, et crée ainsi l'équipe (le lien de création viendra avec TS-3). Relancer `init` donne un autre jeton, pour une autre équipe sur le même serveur.

PostgreSQL : `--database postgres://user:pass@host/base` (ou `ESCOUADE_TEAM_DATABASE_URL`). S3 : `--s3-endpoint`, `--s3-bucket`, `--s3-region`, `--s3-prefix`, et les clés par `ESCOUADE_TEAM_S3_ACCESS_KEY` et `ESCOUADE_TEAM_S3_SECRET_KEY`, jamais en argument.

### Docker et Caddy

`Dockerfile` (contexte : la racine du dépôt) et `deploy/docker-compose.yml`, avec `deploy/Caddyfile` :

```sh
cd crates/escouade-team-server/deploy
ESCOUADE_TEAM_DOMAIN=team.example.org docker compose build
docker compose run --rm team-server init
ESCOUADE_TEAM_DOMAIN=team.example.org docker compose up -d
```

Le serveur n'est pas publié hors du réseau du compose : seul Caddy l'atteint, et le WebSocket passe par lui.

### systemd

`deploy/escouade-team-server.service` : son propre utilisateur, distinct du mode serveur qui lance des agents (`docs/REMOTE.md`), et un bac à sable systemd (`ProtectSystem=strict`, aucune capacité, `MemoryDenyWriteExecute`, filtre d'appels système). Les commandes d'installation sont en tête du fichier.

### Les sauvegardes

La base et les fichiers (le volume, ou le seau S3). Ils ne contiennent que des octets chiffrés, si bien qu'une sauvegarde est peu sensible. Mais elle ne sert à rien sans les clés des membres.

## Les tests

```sh
cd src-tauri && cargo nextest run -p escouade-team-server
```

- `tests/server.rs` lance le vrai serveur sur localhost, sur une base SQLite temporaire : inscription par jeton (une seconde fois refusée), ajout et révocation d'un appareil, défi signé (une mauvaise signature, une signature d'une autre clé, un défi réutilisé ou inventé, un appareil révoqué et une session expirée refusés), flux et reprise depuis n, le premier commit d'une époque qui gagne (aussi à huit en même temps), fichiers et quotas, WebSocket (les entrées en ordre, rien pour une autre équipe, fermeture à la révocation et à la fin de la session), poignée de main des versions, limite de débit par adresse et par appareil, bornes de taille (dont une longueur annoncée de 10 Go et une chaîne CBOR de 2⁶² octets), équipes étanches.
- `tests/crypto_scenario.rs` rejoue un scénario d'`escouade-team-crypto` à travers le vrai serveur, au lieu de `relay.rs` : création, ajout de deux membres (Welcome et historique lus sur le serveur), retrait d'un membre (qui ne lit plus rien de nouveau, et dont l'appareil est révoqué), rotation, puis deux administrateurs qui committent à la même époque : le serveur garde le premier, refuse le second avec ce qui l'a précédé, et la branche perdante se voit (`Error::Fork`) au premier objet qui passe.
- `tests/ops.rs` : `init` puis `serve` par le vrai binaire, le journal sans id, TLS avec un certificat donné, la base en WAL, les métriques sans id.
- **PostgreSQL** : chaque test de `server.rs` et le scénario ont une variante `postgres::`, qui tourne quand `ESCOUADE_TEST_POSTGRES_URL` est posée (un schéma par test, effacé à la fin). La CI la pose dans le job Linux (`linux-rust`, service `postgres:17`).
- **S3** : `files_and_quotas` écrit dans S3 quand `ESCOUADE_TEST_S3_ENDPOINT` (et `_BUCKET`, `_REGION`, `_ACCESS_KEY`, `_SECRET_KEY`) est posée. Vérifié sur Garage 1.1.0 ; la signature SigV4 est aussi testée sur l'exemple d'AWS.

## Ce qui reste

- **TS-3** : les invitations sur le point d'entrée `POST /v1/devices/enrollments` (le secret après le `#`, le lien `escouade://team/join` et le lien de création qu'affichera `init`), l'ajout d'un appareil par QR code et par code court, les rôles propriétaire et lecteur (le serveur refuse alors les écritures d'un lecteur), le changement de rôle.
- **TS-5** : les droits par projet (le serveur ne sert un projet qu'aux appareils qui y ont accès : aujourd'hui, tout appareil de l'équipe), les instantanés et l'élagage des flux, l'horloge hybride côté client.
- **TS-6** : la prise atomique des tickets (bail et jeton de prise, en clair, §4.2), le compteur des numéros de tickets d'un projet.
- **TS-2** (fait, ESC-458) : le client, `crates/escouade-team`, parle ce protocole. Il garde les mêmes formes que `api::wire`, testées contre ce serveur. Reste au serveur, pour TS-3 : laisser un membre couper son propre autre appareil (le serveur ne sait pas quels appareils sont à quel membre), et changer le rôle d'un appareil, pour suivre le registre.
- **TS-18** : le fuzzing du décodage des corps et des flux ; l'e2e avec deux clients.
- **Les limites de cette version** : un seul processus (les sessions, les défis, les limites de débit et la diffusion sont en mémoire : plusieurs serveurs sur une même base PostgreSQL demanderaient `LISTEN/NOTIFY` et des sessions en base) ; une connexion à la base ; un fichier est tenu en mémoire le temps de l'écrire (25 Mio au plus) ; PostgreSQL sans TLS.
- **Le module `transport`** (CBOR, versions, bornes, débit, TLS) n'utilise rien du reste de la crate : il deviendra une crate commune avec le mode serveur (ESC-451) quand celui-ci aura son transport.
