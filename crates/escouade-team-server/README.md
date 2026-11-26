# escouade-team-server : le serveur d'équipe, aveugle (TS-1)

Ticket ESC-456, ligne TS-1 de `docs/TEAM-SERVER.md` §16. Choix tranchés par l'utilisateur le 2026-11-14 (§15.2) : un binaire à part (C1-A), MLS (C9-B, maquetté dans `crates/escouade-team-crypto`), le chiffrement de bout en bout comme seul mode (C10-A). Publié avec chaque version d'Escouade depuis la 2.3 (ESC-586, `.github/workflows/release.yml`) : les archives signées pour Linux x64 et arm64 et l'image `ghcr.io/guillaume-gagnaire/escouade/escouade-team-server` (`latest` pour une version stable seulement, `beta` pour une bêta).

## En bref

- **Aveugle.** Le serveur ne garde que des octets chiffrés et les métadonnées du §3.8. Il n'exécute rien : aucun agent, aucune commande, aucun plugin, aucun harness. Il n'a aucune clé de contenu.
- **Les appareils** : un id, une clé publique Ed25519, une équipe, un rôle (`admin` ou `member`), révoqué ou non. Le premier appareil s'inscrit avec le jeton à usage unique que donne `init`, et devient administrateur de l'équipe qu'il crée. Un administrateur demande ensuite un jeton d'inscription pour chaque nouvel appareil : c'est le point d'entrée des invitations de TS-3.
- **L'authentification par défi signé** (§2.6) : le serveur donne un défi, l'appareil le signe, le serveur rend un jeton de session de 15 minutes. Aucun mot de passe. Toutes les routes demandent une session, sauf `/healthz`, l'inscription par jeton et les deux temps du défi.
- **Deux sortes de flux ordonnés** : un par équipe (les messages MLS) et un par projet (les opérations chiffrées de TS-5). Le serveur numérote, garde et diffuse. Pour les commits MLS, le premier d'une époque gagne, comme dans `relay.rs`. Un second pour la même époque est refusé (409 `epoch_taken`), avec les entrées depuis le commit gagnant. La reprise se fait par `?since=n`.
- **Les commits sont aux administrateurs** (SR2-02) : le commit d'un membre dit en clair (`own`) qu'il ne change que ses propres appareils, sinon `403` ; le commit le signe aussi, et seuls les appareils le vérifient. **Une annulation** (genre `void`, d'un appareil administrateur seul, `403` sinon) déclare nul le dernier commit du flux, une fois (`409 void_refused` sinon) : il reste servi tel quel, son époque redevient libre, réservée aux administrateurs jusqu'à leur prochain commit (`409 epoch_reserved` pour un membre). Colonnes `voided` des entrées et `reserved` des équipes et des projets, migrées en SQLite comme en PostgreSQL.
- **Les fichiers chiffrés** (documents, images) : sur le disque ou dans un stockage compatible S3 (MinIO, Garage, AWS), derrière le trait `files::Blobs`. Une taille maximale par fichier, et un quota par équipe qui compte les fichiers et les flux.
- **La diffusion en direct** par WebSocket (`/v1/live`) : un appareil abonné reçoit les nouvelles entrées de tous les flux de son équipe.
- **La base** : SQLite en WAL par défaut, PostgreSQL en option, derrière le trait `db::Store`.

## Ce que le serveur voit et garde

C'est la table du §3.8, et rien de plus.

| Où | Ce qui y est | Ce qui n'y est jamais |
| --- | --- | --- |
| `teams` | un id opaque (16 octets, tiré par le serveur), l'heure de création, le dernier numéro du flux d'équipe, le nombre de commits (l'époque), les octets gardés | un nom |
| `devices` | un id opaque, l'équipe, la clé publique Ed25519, le rôle (`admin`, `member`, ou `service` pour une machine de l'équipe, OE-1 : sa sorte seule), les heures de création et de révocation | un nom, une adresse, le membre derrière l'appareil |
| `tokens` | le **SHA-256** des jetons d'inscription non utilisés, leur sorte, leur équipe, leur expiration (un jeton utilisé est effacé) | le jeton lui-même |
| `team_entries`, `project_entries` | le numéro, la sorte (`commit` et son époque, ou `message`), l'appareil auteur, l'heure, **les octets chiffrés** | ce qu'ils disent : un commit MLS part en message privé, une opération est chiffrée par la clé du projet |
| `projects` | un id opaque choisi par le client, son dernier numéro, le nombre de commits de son groupe (son époque), le numéro jusqu'où ses messages sont oubliés (`pruned`) | le nom du projet, ses dépôts |
| `snapshots` | les instantanés de chaque projet (SR2-01 : plusieurs, tant que la rétention court) : leur numéro, l'appareil, l'heure, **les octets chiffrés**, et l'appareil qui l'a dit illisible et quand | l'état du projet |
| `files` | un id opaque choisi par le client, la taille, l'appareil auteur, l'heure, le projet de rattachement (SR2-03) | le nom du fichier, son type, son contenu |
| `project_access` (TS-6) | pour chaque projet : ouvert à toute l'équipe, ou la liste des ids d'appareils qui y ont accès | le membre derrière un appareil, le nom du projet |
| `claims` (TS-6) | la prise de chaque ticket : son **id opaque** (tiré par le client, distinct de l'objet du ticket dans le journal), l'appareil qui le tient, la fin du bail, le jeton, la version, les heures de prise et de dernier renouvellement, l'appareil d'avant | le titre, la clé, la colonne ou l'objet du ticket |
| `project_keys` (TS-6) | le compteur des numéros de tickets de chaque projet | le préfixe (`DEM`), chiffré dans le journal |
| les fichiers (disque ou S3) | les octets chiffrés, sous `<équipe>/<fichier>` en hexadécimal | |
| la mémoire du processus | les défis en cours (60 s), le SHA-256 des jetons de session (15 min), les seaux des limites de débit (par adresse IP, par appareil) | |
| le journal (`RUST_LOG`) | la méthode, **le modèle de la route** (`/v1/projects/{project}/stream`), le statut, la durée | un id d'équipe, de projet, d'appareil ou de fichier, une adresse IP, un jeton, un contenu. Une erreur de la base n'y dit que sa sorte (le code SQLite, le SQLSTATE de PostgreSQL), jamais son texte qui peut citer une valeur ; une erreur de S3, que sa sorte (délai, connexion, envoi, corps) ou le statut rendu, jamais l'URL qui porte l'équipe et le fichier (SR2-10) |
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
| `POST /v1/enroll` `{token, public_key}` | non | inscrit un appareil avec un jeton à usage unique : celui d'`init` crée l'équipe, dans les 24 h (SR2-11 ; 401 `bad_token` après). Un jeton déjà servi est refusé en 409 `token_used` (« invitation déjà utilisée »), et ce second usage est noté pour les administrateurs |
| `POST /v1/auth/challenge` `{device}` | non | un défi, pour un appareil connu et non révoqué |
| `POST /v1/auth/session` `{device, challenge, signature}` | non | un jeton de session (`Authorization: Bearer …`) |
| `GET /v1/devices` | oui | les appareils de l'équipe : id, clé publique, rôle, révoqué |
| `POST /v1/devices/enrollments` `{expires_in?, token_hash?}` | administrateur | un jeton d'inscription pour un appareil de plus (24 h par défaut, 7 jours au plus). Avec `token_hash`, le serveur enregistre le SHA-256 d'un jeton que l'administrateur a fait lui-même (celui d'une invitation, dérivé du secret après le `#`, TS-3) et ne rend que sa durée |
| `GET /v1/devices/enrollments/alerts` | administrateur | les jetons de l'équipe servis une seconde fois : leur SHA-256, quand ils ont servi, quand ils ont resservi |
| `POST /v1/devices/{id}/role` `{role}` | administrateur | le rôle d'un appareil sur le serveur, tenu au pas du registre par les appareils des administrateurs ; le dernier administrateur ne devient jamais membre (409 `last_admin`) |
| `POST /v1/devices/{id}/revoke` | administrateur, ou l'appareil lui-même | révoque : ses sessions et ses connexions en direct se ferment. L'appareil révoqué reste listé (l'histoire de l'équipe) mais ne compte plus dans ses 500 (SR2-12). Le dernier administrateur non révoqué ne l'est jamais, par lui-même ou un autre (409 `last_admin`, « a team keeps one administrator on the server at least », SR2-13) : on en nomme un autre d'abord |
| `POST /v1/team/stream` `{kind, epoch?, payload}` | oui | ajoute au flux d'équipe ; un `commit` doit être pour l'époque suivante |
| `GET /v1/team/stream?since=n&limit=m` | oui | les entrées après `n`, une page au plus (`more` dit s'il en reste) |
| `POST`, `GET /v1/projects/{id}/stream` | accès au projet (TS-6) | de même pour un projet : ses opérations (`message`) et les commits du groupe MLS d'un projet gardé à quelques membres (`commit`, le premier d'une époque gagne, TS-5). La lecture dit `snapshot: n` quand le dernier instantané va au-delà de `since` |
| `PUT /v1/projects/{id}/snapshot` `{upto, payload}` | accès au projet | garde l'instantané chiffré d'un projet (TS-5) à côté des précédents ; ses octets comptent dans le quota. **Rien n'est oublié à sa pose** (SR2-01) : les messages jusqu'à `upto` restent 30 jours (`snapshot_retention`), puis le balayage les oublie (les commits restent, et les instantanés d'avant), seulement si aucun appareil ne l'a dit illisible. Refusé en 409 `snapshot_refused` s'il n'est pas après le dernier ou va au-delà du flux, et en 403 sur les flux de l'équipe (`TEAM_STREAMS` : liste des projets, réglages, propositions, coûts, store privé, coupe-circuit), pour tous |
| `GET /v1/projects/{id}/snapshot?before=n` | accès au projet | le dernier instantané (`upto`, l'appareil, l'heure, les octets), ou le dernier avant `n` ; 404 s'il n'y en a pas. La lecture du flux dit aussi `latest: n`, le dernier instantané quel que soit `since` |
| `POST /v1/projects/{id}/snapshot/unreadable` `{upto}` | accès au projet | un appareil dit l'instantané `upto` illisible (SR2-01) : ses messages ne sont plus oubliés tant qu'il est là, et les administrateurs le lisent ; 404 sans cet instantané |
| `DELETE /v1/projects/{id}/snapshot?upto=n` | administrateur | retire un instantané : celui d'avant et le journal reprennent sa place, ses octets reviennent au quota ; 404 sans lui, 409 `snapshot_refused` pour celui où le flux est élagué (ses messages sont partis) |
| `GET /v1/snapshots/unreadable` | administrateur | les instantanés de l'équipe dits illisibles : le projet, le numéro, l'appareil qui l'a posé et quand, le premier appareil qui l'a signalé et quand |
| `GET /v1/dropped` | oui | les entrées que l'opérateur a retirées des flux de l'équipe (`drop-entry`) : le flux (celles d'un projet aux seuls appareils qui y ont accès), le numéro, le genre et l'époque, l'appareil qui l'avait écrite et quand, son empreinte, quand elle a été retirée ; jamais son contenu ni qui l'a retirée |
| `GET`, `PUT /v1/projects/{id}/access` `{everyone, devices}` | oui ; changer l'accès d'un projet existant : administrateur | qui a accès au projet (TS-6). Un projet inconnu est ouvert à l'équipe, et sa première écriture le rend tel ; ensuite seul un administrateur le change (403 `not_admin`). Chaque appareil listé est de l'équipe, une fois (400 `duplicate_device`, SR2-04) ; les octets de la liste comptent dans le quota |
| `POST /v1/projects/{id}/claims/{ticket}` `{expected?}` | accès au projet | prend un ticket en une transaction (§4.2) : bail de 10 min (`claim_lease`), nouveau jeton. Refusé en 409 `taken` (un autre appareil le tient, la prise en détail) ou `stale_version` (sa version a changé depuis `expected`). Un détenteur révoqué ou sorti du projet ne tient plus rien. Chaque nouvelle prise compte 128 octets dans le quota (SR2-04 ; 507 `quota` au-delà) |
| `POST /v1/projects/{id}/claims/{ticket}/renew`, `…/release` `{token}` | accès au projet | renouvelle le bail (aussi une fois fini, tant que personne n'a repris), ou le rend ; 409 `claim_lost` si le jeton n'est plus le sien |
| `GET /v1/projects/{id}/claims` | accès au projet | les prises du projet, l'heure du serveur et la durée du bail |
| `POST /v1/projects/{id}/keys` | accès au projet | le numéro suivant des tickets du projet (1, 2, 3…, jamais deux fois) |
| `PUT /v1/projects/{id}/files/{file}` | accès au projet | un fichier chiffré rattaché au projet (SR2-03), écrit une fois (409 s'il existe). Un fichier de l'équipe se rattache à l'un de ses flux (`TEAM_STREAMS`, ouverts à tous ses appareils) |
| `GET /v1/projects/{id}/files/{file}` | accès au projet | le fichier ; 404 s'il est rattaché à un autre projet. Un fichier rattaché à rien n'est servi qu'à son auteur et aux administrateurs, et cette lecture le rattache au projet nommé ; 404 pour les autres |
| `DELETE /v1/projects/{id}/files/{file}` | l'appareil auteur ou un administrateur | efface le fichier ; 403 pour un autre membre qui le voit, 404 pour qui ne le voit pas |
| `PUT`, `GET`, `DELETE /v1/files/{file}` | comme ci-dessus | l'ancienne route, gardée une version : un fichier posé là n'est rattaché à rien (son auteur et les administrateurs seuls) ; un fichier rattaché se lit selon les droits de son projet |
| `GET /v1/files/usage` | oui | les octets gardés, le quota, les tailles maximales |
| `POST /v1/pair` `{message}` | oui | ouvre un créneau d'appairage (TS-3) avec le premier message du PAKE : ses cinq caractères, la tête du code court de 10 ; 10 minutes |
| `GET /v1/pair/{slot}` | non | le premier message, pour le nouvel appareil ; débit à lui par adresse |
| `POST /v1/pair/{slot}/try` `{message, confirm}` | non | l'unique essai du nouvel appareil, qui rend `{token}` (128 bits) ; un second est refusé en 409 `pair_tried` ; débit à lui par adresse |
| `GET /v1/pair/{slot}/try` | l'appareil du créneau | l'essai, 204 tant qu'il n'y en a pas |
| `POST /v1/pair/{slot}/answer` `{sealed}`, `DELETE /v1/pair/{slot}` | l'appareil du créneau | la réponse scellée sous la clé du PAKE, ou la fermeture sur un mauvais code |
| `POST /v1/pair/{slot}/take` `{token}` | non, le jeton de l'essai | la réponse, prise une fois (le créneau part avec) ; 204 tant qu'il n'y en a pas, 403 sous un autre jeton, 404 `pair_gone` quand il est fermé |
| `GET /join/{id}` | non | la page d'un lien d'invitation de l'ancienne forme (`https://<serveur>/join/<id>#…`) ouvert dans un navigateur : un texte fixe qui dit d'en demander un neuf. Aucun script (CSP `script-src 'none'`), aucune ressource d'ailleurs, rien du lien lu : le secret d'une invitation ne passe jamais par une page du serveur (SR2-08) ; les liens sont maintenant `escouade://team/join/<serveur>/<id>#…` |
| `GET /v1/live` (WebSocket) | oui | des trames CBOR `{stream: "team" \| "project" \| "claim" \| "lagged", project?, entry?, claim?}` ; celles d'un projet (ses entrées, ses prises) ne vont qu'aux appareils qui y ont accès ; fermée en 4001 à la fin de la session, en 4003 à la révocation |

**Le jeton de prise** (TS-6) : une entrée d'un projet peut porter `fence: {ticket, token}` ; le serveur la refuse en 409 `stale_token` (la prise en détail) si ce jeton n'est plus celui de la prise ou si l'appareil n'est plus son détenteur, dans la même transaction que l'ajout. Le serveur ne sait pas ce que fait l'entrée : seulement qu'elle vient du preneur.

Les ids passent en hexadécimal dans les chemins (32 chiffres), en octets dans les corps.

`escouade-team-server recovery-token --team <id>` affiche un jeton à usage unique (7 jours au plus) qui inscrit un appareil dans une équipe existante comme administrateur du serveur : pour le détenteur de la clé de secours, le jour où tous les administrateurs ont perdu leurs appareils (§3.5). Il ne donne aucune clé : les 24 mots les donnent.

### Dégeler une équipe ou un projet : `drop-entry` (SR2-02)

Le serveur garde le premier commit d'une époque sans pouvoir le lire, puis refuse tout autre commit de cette époque. Un commit que les appareils ne peuvent pas lire (des octets quelconques d'un membre, ou un commit contraire aux règles de l'équipe) ne gèle plus l'équipe : les appareils le passent et un administrateur l'annule (ESC-635, ci-dessus). Quand aucun administrateur ne le peut (le groupe d'un projet qui n'en a plus), l'opérateur du serveur le retire, sans toucher à la base à la main ; il ne retire ni une annulation, ni le commit qu'elle a déclaré nul :

```sh
escouade-team-server drop-entry --team <id> --stream team --seq <n>          # le flux de l'équipe
escouade-team-server drop-entry --team <id> --project <id> --seq <n>         # le flux d'un projet
escouade-team-server drop-entry --team <id> --stream escouade/setting --seq <n>   # un flux propre à l'équipe
escouade-team-server drop-entry --team <id> --list                           # l'audit de ce qui a été retiré
```

- **Elle montre l'entrée** (genre, époque, appareil qui l'a écrite, date, taille, empreinte SHA-256) **et demande `yes`** ; `--yes` s'en passe. Sans l'id de l'équipe, elle ne fait rien : elle ne devine jamais l'équipe.
- **Un commit doit être le dernier de son flux** : son époque redevient libre, et le prochain commit (celui de l'administrateur) la prend. Un commit que d'autres suivent est refusé : retirer les plus récents d'abord.
- **L'audit** : une ligne dans la table `dropped_entries` de la base, dans la même transaction que le retrait : l'équipe, le flux, le numéro, le genre et l'époque, l'appareil qui l'avait écrite, sa taille, son empreinte, qui (`--by`, ou le nom du compte) et quand. Jamais ses octets. Le journal du serveur n'en dit qu'une ligne sans identifiant. `--list` relit l'audit d'une équipe.
- **Le flux garde sa numérotation** : le trou se voit. Les appareils lisent la liste `GET /v1/dropped` : un trou que l'opérateur a fait n'est pas un retour en arrière, même quand l'entrée retirée était la dernière qu'un appareil avait lue (son empreinte doit être celle qu'il en a gardée). Les administrateurs le voient dans la carte de l'équipe (« Entrées retirées par l'opérateur du serveur », 30 jours).
- **Ce qu'elle ne permet pas** : lire une entrée ou la rendre (le serveur n'a que des octets chiffrés), ni retirer une entrée en silence (les appareils la voient, les administrateurs sont prévenus). Ne retire **que l'entrée qui bloque** : un commit valide que des appareils ont lu, retiré puis remplacé, est une fourche à leurs yeux ; une opération valide d'un projet retirée casse la chaîne de son appareil, et ses opérations suivantes sont refusées ; une entrée signée d'un administrateur (liste des projets, réglages imposés, store privé) retirée disparaît pour tous.
- Elle ouvre la base comme `serve` (mêmes options) et peut tourner pendant qu'il sert : SQLite attend son verrou, PostgreSQL verrouille la ligne de l'équipe.

Le relais d'appairage (`pair.rs`) est en mémoire, comme les sessions : un créneau prend un seul essai, et ne porte que les messages du PAKE et une réponse scellée, qui ne permettent pas d'essayer des codes hors ligne.

## Les défenses (§2.3)

- **Aucune route sans session**, hors `/healthz`, l'inscription et le défi.
- **Une machine de l'équipe** (rôle `service`, OE-1) n'a aucune route d'administrateur (jetons, rôles, révocation d'un autre, alertes des seconds usages et des entrées retirées) et n'écrit dans aucun flux propre de l'équipe (`api::SERVICE_DENIED_STREAMS` : liste des projets, réglages, propositions, clé des coûts, store privé, coupe-circuit ; elle garde les totaux des coûts) : 403. Son rôle ne passe à rien d'autre (409 `service_stays`). Les postes refusent de même ce qu'elle signerait hors de son rôle : le serveur ne fait que doubler la défense.
- **Une limite de débit par adresse et par appareil** (seaux à jetons ; 30 requêtes/s et 120 d'un coup par adresse, 20/s et 80 par appareil, et pour les étapes de l'appairage sans session une toutes les 5 s, 10 d'un coup, par adresse). Une adresse IPv6 compte par son /64. Au-delà : 429 et `Retry-After`.
- **Les défis et les sessions bornés par appareil et par équipe** (SR2-06) : deux défis en attente par appareil (un nouveau prend la place du plus ancien), 1 024 par équipe, 100 000 en tout ; huit sessions par appareil. Une équipe qui demande sans fin ne remplit que ses places.
- **Des délais et un plafond des connexions** (SR2-07, `transport/listen.rs`, `transport/body.rs`) : le corps d'une requête arrive en 30 s plus sa longueur à 64 Kio/s, sinon 408 `slow_body` et la connexion se ferme ; une connexion où rien ne passe 60 s se ferme ; 8 192 connexions ouvertes à la fois et 512 par adresse (une adresse IPv6 par son /64 ; derrière `--trust-proxy`, le plafond global seul), au-delà la nouvelle est fermée aussitôt et comptée (`escouade_team_connections_refused_total`). Un WebSocket compte jusqu'à sa fermeture. **Exposé sur Internet, garde-le derrière le proxy** (Caddy, Traefik), qui encaisse d'abord. Derrière un proxy inverse, `--trust-proxy` prend la dernière adresse de `X-Forwarded-For`, celle que le proxy a vue.
- **Des tailles bornées partout** : une entrée de flux (1 Mio), un fichier (25 Mio, `--max-file`), le quota d'une équipe (2 Gio, `--team-quota`), une page de lecture (500 entrées, 8 Mio), les corps des autres routes (64 Kio), 500 appareils et 10 000 projets par équipe, 4 connexions en direct par appareil, les trames reçues sur le WebSocket (16 Kio), les en-têtes HTTP (64 Kio, lus en 15 s). **Chaque ligne qu'un appareil fait compte** (SR2-04) : un projet est connu du serveur dès sa première écriture de quelque sorte (une entrée, son accès, une prise, un numéro) et compte dès lors dans les 10 000 de l'équipe, si bien qu'un projet inconnu est refusé au-delà (403 `too_many`) quelle que soit l'écriture ; une prise compte 128 octets dans le quota, une liste d'accès les octets de ses appareils (chacun une fois) ; le compteur et l'accès d'un projet sont une ligne par projet. **Une longueur annoncée trop grande** (`Content-Length`) est refusée en 413 avant de lire un octet. Une chaîne CBOR qui annonce plus d'octets qu'elle n'en porte est refusée en 400, sans allouer.
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
escouade-team-server --help                                       # les commandes ; `<commande> --help` : ses options et leurs variables
```

`init` affiche un jeton à usage unique, valable 24 h (SR2-11 : un jeton resté dans un terminal ou un historique ne crée plus rien ensuite ; une base d'avant donne aussi 24 h à partir de sa création à un jeton non servi) : le premier administrateur le donne à Escouade avec l'adresse du serveur, et crée ainsi l'équipe (le lien de création viendra avec TS-3). Relancer `init` donne un autre jeton, pour une autre équipe sur le même serveur.

`init --help`, `serve --help`, `recovery-token --help` et `drop-entry --help` affichent l’aide de la commande, sans rien créer ni écouter ; `--version` la version. Une option inconnue, ou d’une autre commande, est refusée avec le code de sortie 2.

PostgreSQL : `--database postgres://user:pass@host/base` (ou `ESCOUADE_TEAM_DATABASE_URL`). S3 : `--s3-endpoint`, `--s3-bucket`, `--s3-region`, `--s3-prefix`, et les clés par `ESCOUADE_TEAM_S3_ACCESS_KEY` et `ESCOUADE_TEAM_S3_SECRET_KEY`, jamais en argument.

### Docker et Caddy

`Dockerfile` : depuis les sources (contexte : la racine du dépôt), ou depuis l'archive d'une version (`--build-arg PROGRAM=archive`, ce que fait le Release). `deploy/docker-compose.yml` lance l'image publiée (`ESCOUADE_TEAM_TAG`, `latest` par défaut), avec `deploy/Caddyfile` ; `deploy/coolify.yml` est celui de Coolify :

```sh
cd crates/escouade-team-server/deploy
export ESCOUADE_TEAM_DOMAIN=team.example.org ESCOUADE_TEAM_TAG=beta
docker compose run --rm team-server init
docker compose up -d
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
- `tests/files.rs` : les fichiers rattachés à un projet (SR2-03) : un appareil hors du projet refusé en pose, lecture et effacement, ni lu ni effacé sous un autre projet ; l'auteur et un administrateur effacent, pas un autre membre ; un fichier rattaché à rien (ancienne route, ou base d'avant migrée) servi à son auteur et aux administrateurs seuls, rattaché par leur lecture sous un projet ; sur SQLite et PostgreSQL, sur le disque et dans S3 (`ESCOUADE_TEST_S3_ENDPOINT`).
- `tests/crypto_scenario.rs` rejoue un scénario d'`escouade-team-crypto` à travers le vrai serveur, au lieu de `relay.rs` : création, ajout de deux membres (Welcome et historique lus sur le serveur), retrait d'un membre (qui ne lit plus rien de nouveau, et dont l'appareil est révoqué), rotation, puis deux administrateurs qui committent à la même époque : le serveur garde le premier, refuse le second avec ce qui l'a précédé, et la branche perdante se voit (`Error::Fork`) au premier objet qui passe.
- `tests/onboarding.rs` (TS-3) : un jeton fait par l'administrateur sert une fois, son second usage est dit à l'invité et aux administrateurs, il expire ; les rôles suivent l'administrateur et le dernier reste ; le jeton de récupération inscrit un administrateur de l'équipe ; le relais d'appairage prend un seul essai ; la page d'un lien d'invitation ne charge rien d'ailleurs.
- **TS-5** (`tests/server.rs`) : les commits d'un projet (le premier d'une époque gagne), et `a_snapshot_takes_the_place_of_the_start_of_a_projects_stream` (l'instantané gardé, le quota compté, un instantané plus ancien ou au-delà du flux refusé, rien pour une autre équipe). La synchronisation entière se joue contre ce serveur dans `crates/escouade-team/tests/sync.rs`.
- **SR2-01** (`tests/snapshots.rs`, SQLite puis PostgreSQL) : aucun instantané sur les flux de l'équipe, pour un administrateur comme pour un membre ; les messages gardés 30 jours puis oubliés (les commits restent, le quota rendu) ; un instantané signalé gardé au-delà, les administrateurs prévenus (pas les membres) ; le retrait par un administrateur seul, celui d'avant qui reprend sa place, celui de la base refusé ; l'instantané d'avant un numéro ; une base d'avant la rétention migrée (aussi dans `db/sqlite.rs`).
- **TS-6** (`tests/claims.rs`, SQLite puis PostgreSQL) : dix appareils prennent un ticket en même temps, un seul passe et les neuf autres apprennent qui le tient ; le bail renouvelé, fini (le ticket listé avec son détenteur, puis repris sous un nouveau jeton), le renouvellement arrivé trop tard, le rendu, la version attendue ; l'écriture d'un jeton dépassé refusée sans rien garder ; un projet gardé à quelques appareils (les autres ne lisent, n'écrivent, ne prennent, ne numérotent rien, et un membre n'ouvre pas l'accès) ; un détenteur révoqué ne tient plus rien ; le compteur donne chaque numéro une fois ; la trame d'une prise arrive en moins d'une seconde aux seuls appareils du projet. Les tests purs de `db::decide` et `db::fenced` sont dans `db/mod.rs`.
- **SR2-02** (`tests/drop.rs`, SQLite puis PostgreSQL) : le commit qui bloque retiré, son époque libre pour le commit de l'administrateur, la numérotation gardée, l'empreinte de l'entrée telle que servie, le quota rendu ; un commit que d'autres suivent refusé ; l'entrée d'un projet listée à ses seuls appareils ; rien sans l'équipe ou l'entrée. `tests/cli.rs` : la commande et ses refus (sans équipe, sans flux, deux flux, un flux inconnu, sans numéro, une entrée absente, une équipe absente, un commit suivi d'un autre, sans confirmation), le retrait confirmé ou par `--yes`, l'audit et `--list`, rien de l'entrée ni d'identifiant au journal. Côté appareils : `crates/escouade-team/tests/team.rs` (l'équipe gelée puis dégelée), `sync.rs` (un projet), `catalog.rs` (un flux de l'équipe).
- **SR2-02, l'annulation** (`tests/voids.rs`, SQLite puis PostgreSQL) : le commit d'un membre refusé sans `own`, pris avec ; l'annulation refusée à un membre, prise d'un administrateur pour le dernier commit seul et une fois ; l'époque réservée aux administrateurs puis reprise par leur commit, le perdant suivant renvoyé au nouveau gagnant ; rien de réécrit (le commit nul servi à l'identique, l'annulation après lui) ; `drop-entry` ne retire ni l'un ni l'autre. Le flux de l'équipe et celui d'un projet. Côté appareils : `crates/escouade-team/tests/nulls.rs`.
- `tests/ops.rs` : `init` puis `serve` par le vrai binaire, le journal sans id, TLS avec un certificat donné, la base en WAL, les métriques sans id.
- **PostgreSQL** : chaque test de `server.rs`, de `bounds.rs` et le scénario ont une variante `postgres::`, qui tourne quand `ESCOUADE_TEST_POSTGRES_URL` est posée (un schéma par test, effacé à la fin). La CI la pose dans le job Linux (`linux-rust`, service `postgres:17`), avec `ESCOUADE_TEST_POSTGRES_REQUIRED=1` : une variante sans base y échoue au lieu de passer sans rien dire.
- **Les bornes** (`tests/bounds.rs`, TS-18) : les appareils et les projets d'une équipe, le temps d'un défi (et la même réponse pour un appareil inconnu ou révoqué), une page en octets et en entrées, la durée d'un jeton d'inscription et la longueur d'un jeton, les trames du direct, ce que porte le relais d'appairage, tout ce qu'un appareil révoqué n'obtient plus, la taille et le temps de l'en-tête HTTP, le temps d'un corps, le délai d'une connexion muette, le plafond des connexions par adresse et en tout, un appareil qui demande des défis sans fin sans gêner les autres, le débit du relais d'appairage et son jeton ; chaque ligne comptée dans les projets ou le quota, un appareil listé deux fois refusé (SR2-04), le jeton d'`init` servi une fois et expiré après 24 h (SR2-11), les appareils révoqués hors du plafond (SR2-12), le dernier administrateur jamais révoqué (SR2-13) ; une erreur de la base ou de S3 sans id (SR2-10 : `db/sqlite.rs`, `files/s3.rs`) ; dans les modules, les défis et sessions en attente, par appareil, par équipe et en tout (`auth.rs`), le /64 d'une adresse IPv6 (`transport/rate.rs`), les prises d'un projet (`db/sqlite.rs`), les créneaux d'appairage (`pair.rs`).
- **S3** : `files_and_quotas` écrit dans S3 quand `ESCOUADE_TEST_S3_ENDPOINT` (et `_BUCKET`, `_REGION`, `_ACCESS_KEY`, `_SECRET_KEY`) est posée. Vérifié sur Garage 1.1.0 ; la signature SigV4 est aussi testée sur l'exemple d'AWS.

## Ce qui reste

- **TS-3** (fait, ESC-461) : le jeton fourni par l'administrateur (celui d'une invitation), le second usage dit et noté, les rôles tenus au pas du registre, le jeton de récupération, le relais d'appairage du code court, la page `/join/<id>`. Restent les rôles propriétaire et lecteur (le serveur refusera alors les écritures d'un lecteur, TS-4) et un lien de création qu'afficherait `init`.
- **TS-5** (fait, ESC-466) : les commits du groupe d'un projet dans son flux, les instantanés et l'élagage des messages. La rétention de 30 jours, le signalement d'un instantané illisible et son retrait par un administrateur sont venus avec SR2-01 (ESC-637).
- **TS-6** (fait, ESC-479) : les droits par projet, la prise atomique des tickets (bail, version, jeton de prise vérifié à l'ajout), le compteur des numéros de tickets, les trames `claim`. Reste : la liste d'accès d'un projet gardé est tenue par les appareils des administrateurs (un nouvel appareil d'un membre n'y entre qu'à leur passage) ; le serveur ne voit pas qu'une entrée déplace un ticket, donc la règle « la colonne d'un ticket pris ne change que par son preneur » est tenue par les clients ; pas de route pour qu'un administrateur rende la prise d'un autre.
- **TS-2** (fait, ESC-458) : le client, `crates/escouade-team`, parle ce protocole. Il garde les mêmes formes que `api::wire`, testées contre ce serveur. Reste au serveur : laisser un membre couper son propre autre appareil (le serveur ne sait pas quels appareils sont à quel membre ; le commit seul le coupe du groupe).
- **TS-18** (fait, ESC-519) : un test par borne (`tests/bounds.rs` dit en tête où est chacun) ; le fuzzing de ce qu'un client envoie (`src/fuzzing.rs`, cibles `server-bodies` et `server-requests` de `fuzz/` : les corps CBOR, la poignée de main, `X-Forwarded-For`, et le vrai routeur en mémoire, de personne, d'un appareil révoqué, d'un membre, d'un administrateur) ; l'e2e avec deux clients dans la CI (la part « team server », `docs/TEAM-SERVER.md` §14). Reste, dit par la relecture : le nombre de jetons d'inscription qu'un administrateur fait n'est pas borné (un administrateur est de confiance, et le balayage retire les jetons servis et expirés) ; le seau de débit d'une adresse vit jusqu'au balayage suivant (30 s), si bien que la mémoire des seaux suit le débit des adresses nouvelles, pas leur nombre total.
- **Les limites de cette version** : un seul processus (les sessions, les défis, les limites de débit et la diffusion sont en mémoire : plusieurs serveurs sur une même base PostgreSQL demanderaient `LISTEN/NOTIFY` et des sessions en base) ; une connexion à la base ; un fichier est tenu en mémoire le temps de l'écrire (25 Mio au plus) ; PostgreSQL sans TLS.
- **Le module `transport`** (CBOR, versions, bornes, débit, TLS) n'utilise rien du reste de la crate : il deviendra une crate commune avec le mode serveur (ESC-451) quand celui-ci aura son transport.
