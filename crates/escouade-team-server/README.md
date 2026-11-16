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
| `projects` | un id opaque choisi par le client, son dernier numéro, le nombre de commits de son groupe (son époque) | le nom du projet, ses dépôts |
| `project_snapshots` | le dernier instantané de chaque projet : son numéro, l'appareil, l'heure, **les octets chiffrés** | l'état du projet |
| `files` | un id opaque choisi par le client, la taille, l'appareil auteur, l'heure | le nom du fichier, son type, son contenu |
| `project_access` (TS-6) | pour chaque projet : ouvert à toute l'équipe, ou la liste des ids d'appareils qui y ont accès | le membre derrière un appareil, le nom du projet |
| `claims` (TS-6) | la prise de chaque ticket : son **id opaque** (tiré par le client, distinct de l'objet du ticket dans le journal), l'appareil qui le tient, la fin du bail, le jeton, la version, les heures de prise et de dernier renouvellement, l'appareil d'avant | le titre, la clé, la colonne ou l'objet du ticket |
| `project_keys` (TS-6) | le compteur des numéros de tickets de chaque projet | le préfixe (`DEM`), chiffré dans le journal |
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
| `POST /v1/enroll` `{token, public_key}` | non | inscrit un appareil avec un jeton à usage unique : celui d'`init` crée l'équipe. Un jeton déjà servi est refusé en 409 `token_used` (« invitation déjà utilisée »), et ce second usage est noté pour les administrateurs |
| `POST /v1/auth/challenge` `{device}` | non | un défi, pour un appareil connu et non révoqué |
| `POST /v1/auth/session` `{device, challenge, signature}` | non | un jeton de session (`Authorization: Bearer …`) |
| `GET /v1/devices` | oui | les appareils de l'équipe : id, clé publique, rôle, révoqué |
| `POST /v1/devices/enrollments` `{expires_in?, token_hash?}` | administrateur | un jeton d'inscription pour un appareil de plus (24 h par défaut, 7 jours au plus). Avec `token_hash`, le serveur enregistre le SHA-256 d'un jeton que l'administrateur a fait lui-même (celui d'une invitation, dérivé du secret après le `#`, TS-3) et ne rend que sa durée |
| `GET /v1/devices/enrollments/alerts` | administrateur | les jetons de l'équipe servis une seconde fois : leur SHA-256, quand ils ont servi, quand ils ont resservi |
| `POST /v1/devices/{id}/role` `{role}` | administrateur | le rôle d'un appareil sur le serveur, tenu au pas du registre par les appareils des administrateurs ; le dernier administrateur ne devient jamais membre (409 `last_admin`) |
| `POST /v1/devices/{id}/revoke` | administrateur, ou l'appareil lui-même | révoque : ses sessions et ses connexions en direct se ferment |
| `POST /v1/team/stream` `{kind, epoch?, payload}` | oui | ajoute au flux d'équipe ; un `commit` doit être pour l'époque suivante |
| `GET /v1/team/stream?since=n&limit=m` | oui | les entrées après `n`, une page au plus (`more` dit s'il en reste) |
| `POST`, `GET /v1/projects/{id}/stream` | accès au projet (TS-6) | de même pour un projet : ses opérations (`message`) et les commits du groupe MLS d'un projet gardé à quelques membres (`commit`, le premier d'une époque gagne, TS-5). La lecture dit `snapshot: n` quand le dernier instantané va au-delà de `since` |
| `PUT /v1/projects/{id}/snapshot` `{upto, payload}` | oui | garde l'instantané chiffré d'un projet (TS-5) : les messages jusqu'à `upto` sont oubliés (les commits restent) et leurs octets rendus au quota. Refusé en 409 `snapshot_refused` s'il n'est pas après celui gardé ou va au-delà du flux |
| `GET /v1/projects/{id}/snapshot` | oui | le dernier instantané (`upto`, l'appareil, l'heure, les octets), 404 s'il n'y en a pas |
| `GET`, `PUT /v1/projects/{id}/access` `{everyone, devices}` | oui ; changer l'accès d'un projet existant : administrateur | qui a accès au projet (TS-6). Un projet inconnu est ouvert à l'équipe, et sa première écriture le rend tel ; ensuite seul un administrateur le change (403 `not_admin`). Chaque appareil listé est de l'équipe |
| `POST /v1/projects/{id}/claims/{ticket}` `{expected?}` | accès au projet | prend un ticket en une transaction (§4.2) : bail de 10 min (`claim_lease`), nouveau jeton. Refusé en 409 `taken` (un autre appareil le tient, la prise en détail) ou `stale_version` (sa version a changé depuis `expected`). Un détenteur révoqué ou sorti du projet ne tient plus rien |
| `POST /v1/projects/{id}/claims/{ticket}/renew`, `…/release` `{token}` | accès au projet | renouvelle le bail (aussi une fois fini, tant que personne n'a repris), ou le rend ; 409 `claim_lost` si le jeton n'est plus le sien |
| `GET /v1/projects/{id}/claims` | accès au projet | les prises du projet, l'heure du serveur et la durée du bail |
| `POST /v1/projects/{id}/keys` | accès au projet | le numéro suivant des tickets du projet (1, 2, 3…, jamais deux fois) |
| `PUT`, `GET`, `DELETE /v1/files/{id}` | oui | un fichier chiffré, écrit une fois (409 s'il existe) |
| `GET /v1/files/usage` | oui | les octets gardés, le quota, les tailles maximales |
| `POST /v1/pair` `{message}` | oui | ouvre un créneau d'appairage (TS-3) avec le premier message du PAKE : ses trois caractères, la tête du code court ; 10 minutes |
| `GET /v1/pair/{slot}` | non | le premier message, pour le nouvel appareil |
| `POST /v1/pair/{slot}/try` `{message, confirm}` | non | l'unique essai du nouvel appareil ; un second est refusé en 409 `pair_tried` |
| `GET /v1/pair/{slot}/try` | l'appareil du créneau | l'essai, 204 tant qu'il n'y en a pas |
| `POST /v1/pair/{slot}/answer` `{sealed}`, `DELETE /v1/pair/{slot}` | l'appareil du créneau | la réponse scellée sous la clé du PAKE, ou la fermeture sur un mauvais code |
| `GET /v1/pair/{slot}/answer` | non | la réponse, prise une fois (le créneau part avec) ; 204 tant qu'il n'y en a pas, 404 `pair_gone` quand il est fermé |
| `GET /join/{id}` | non | la page d'un lien d'invitation ouvert dans un navigateur : son script lit le secret après le `#`, là où il est, et passe le lien entier à Escouade (`escouade://team/join/<serveur>/<id>#…`). Aucune ressource d'ailleurs, un script par son nonce seul (CSP) |
| `GET /v1/live` (WebSocket) | oui | des trames CBOR `{stream: "team" \| "project" \| "claim" \| "lagged", project?, entry?, claim?}` ; celles d'un projet (ses entrées, ses prises) ne vont qu'aux appareils qui y ont accès ; fermée en 4001 à la fin de la session, en 4003 à la révocation |

**Le jeton de prise** (TS-6) : une entrée d'un projet peut porter `fence: {ticket, token}` ; le serveur la refuse en 409 `stale_token` (la prise en détail) si ce jeton n'est plus celui de la prise ou si l'appareil n'est plus son détenteur, dans la même transaction que l'ajout. Le serveur ne sait pas ce que fait l'entrée : seulement qu'elle vient du preneur.

Les ids passent en hexadécimal dans les chemins (32 chiffres), en octets dans les corps.

`escouade-team-server recovery-token --team <id>` affiche un jeton à usage unique (7 jours au plus) qui inscrit un appareil dans une équipe existante comme administrateur du serveur : pour le détenteur de la clé de secours, le jour où tous les administrateurs ont perdu leurs appareils (§3.5). Il ne donne aucune clé : les 24 mots les donnent.

Le relais d'appairage (`pair.rs`) est en mémoire, comme les sessions : un créneau prend un seul essai, et ne porte que les messages du PAKE et une réponse scellée, qui ne permettent pas d'essayer des codes hors ligne.

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
- `tests/onboarding.rs` (TS-3) : un jeton fait par l'administrateur sert une fois, son second usage est dit à l'invité et aux administrateurs, il expire ; les rôles suivent l'administrateur et le dernier reste ; le jeton de récupération inscrit un administrateur de l'équipe ; le relais d'appairage prend un seul essai ; la page d'un lien d'invitation ne charge rien d'ailleurs.
- **TS-5** (`tests/server.rs`) : les commits d'un projet (le premier d'une époque gagne), et `a_snapshot_takes_the_place_of_the_start_of_a_projects_stream` (les messages oubliés, les commits gardés, le quota rendu, un instantané plus ancien ou au-delà du flux refusé, rien pour une autre équipe). La synchronisation entière se joue contre ce serveur dans `crates/escouade-team/tests/sync.rs`.
- **TS-6** (`tests/claims.rs`, SQLite puis PostgreSQL) : dix appareils prennent un ticket en même temps, un seul passe et les neuf autres apprennent qui le tient ; le bail renouvelé, fini (le ticket listé avec son détenteur, puis repris sous un nouveau jeton), le renouvellement arrivé trop tard, le rendu, la version attendue ; l'écriture d'un jeton dépassé refusée sans rien garder ; un projet gardé à quelques appareils (les autres ne lisent, n'écrivent, ne prennent, ne numérotent rien, et un membre n'ouvre pas l'accès) ; un détenteur révoqué ne tient plus rien ; le compteur donne chaque numéro une fois ; la trame d'une prise arrive en moins d'une seconde aux seuls appareils du projet. Les tests purs de `db::decide` et `db::fenced` sont dans `db/mod.rs`.
- `tests/ops.rs` : `init` puis `serve` par le vrai binaire, le journal sans id, TLS avec un certificat donné, la base en WAL, les métriques sans id.
- **PostgreSQL** : chaque test de `server.rs` et le scénario ont une variante `postgres::`, qui tourne quand `ESCOUADE_TEST_POSTGRES_URL` est posée (un schéma par test, effacé à la fin). La CI la pose dans le job Linux (`linux-rust`, service `postgres:17`).
- **S3** : `files_and_quotas` écrit dans S3 quand `ESCOUADE_TEST_S3_ENDPOINT` (et `_BUCKET`, `_REGION`, `_ACCESS_KEY`, `_SECRET_KEY`) est posée. Vérifié sur Garage 1.1.0 ; la signature SigV4 est aussi testée sur l'exemple d'AWS.

## Ce qui reste

- **TS-3** (fait, ESC-461) : le jeton fourni par l'administrateur (celui d'une invitation), le second usage dit et noté, les rôles tenus au pas du registre, le jeton de récupération, le relais d'appairage du code court, la page `/join/<id>`. Restent les rôles propriétaire et lecteur (le serveur refusera alors les écritures d'un lecteur, TS-4) et un lien de création qu'afficherait `init`.
- **TS-5** (fait, ESC-466) : les commits du groupe d'un projet dans son flux, les instantanés et l'élagage des messages.
- **TS-6** (fait, ESC-479) : les droits par projet, la prise atomique des tickets (bail, version, jeton de prise vérifié à l'ajout), le compteur des numéros de tickets, les trames `claim`. Reste : la liste d'accès d'un projet gardé est tenue par les appareils des administrateurs (un nouvel appareil d'un membre n'y entre qu'à leur passage) ; le serveur ne voit pas qu'une entrée déplace un ticket, donc la règle « la colonne d'un ticket pris ne change que par son preneur » est tenue par les clients ; pas de route pour qu'un administrateur rende la prise d'un autre.
- **TS-2** (fait, ESC-458) : le client, `crates/escouade-team`, parle ce protocole. Il garde les mêmes formes que `api::wire`, testées contre ce serveur. Reste au serveur : laisser un membre couper son propre autre appareil (le serveur ne sait pas quels appareils sont à quel membre ; le commit seul le coupe du groupe).
- **TS-18** : le fuzzing du décodage des corps et des flux ; l'e2e avec deux clients.
- **Les limites de cette version** : un seul processus (les sessions, les défis, les limites de débit et la diffusion sont en mémoire : plusieurs serveurs sur une même base PostgreSQL demanderaient `LISTEN/NOTIFY` et des sessions en base) ; une connexion à la base ; un fichier est tenu en mémoire le temps de l'écrire (25 Mio au plus) ; PostgreSQL sans TLS.
- **Le module `transport`** (CBOR, versions, bornes, débit, TLS) n'utilise rien du reste de la crate : il deviendra une crate commune avec le mode serveur (ESC-451) quand celui-ci aura son transport.
