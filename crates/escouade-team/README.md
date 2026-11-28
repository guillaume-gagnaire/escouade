# escouade-team : le client du serveur d'équipe (TS-2)

Tickets ESC-458 (TS-2), ESC-461 (TS-3), ESC-466 (TS-5), ESC-471 (TS-13), ESC-480 (TS-8), ESC-479 (TS-6), ESC-482 (TS-9) et ESC-547 (TS-16) de `docs/TEAM-SERVER.md` §16. L'équipe est un groupe MLS (C9-B). Cette crate est le client : elle tient un appareil dans une équipe, avec ses clés au trousseau, son état sur le disque et son serveur, et fait entrer dans l'équipe (création, invitation, autre appareil, récupération). Le cœur la branche (`src-tauri/src/teams.rs`) : le `SecretStore` de l'app pour trousseau, son client HTTP (proxy et certificats des réglages), le dossier `teams` des données. Elle synchronise les projets de l'équipe (TS-5) : le journal d'opérations chiffrées et signées, la copie locale, la file d'envoi hors ligne, les instantanés. Elle tient la liste chiffrée des projets de l'équipe (TS-8).

## La maquette devient la couche cryptographique

`escouade-team-crypto` (TS-0a) reste une crate à part : **la couche cryptographique** de celle-ci, et non une copie reprise ici. Pourquoi :

- elle reste pure et synchrone, sans disque, sans réseau ni horloge. C'est elle que l'audit externe (TS-20) lira, et que le fuzzing visera ;
- ses vecteurs (`vectors/team-crypto-v1.json`) et ses tests restent à leur place, inchangés et verts ;
- le scénario du serveur (`escouade-team-server/tests/crypto_scenario.rs`) la joue toujours sans le client.

Ce que TS-2 y a changé :

- **`TeamDevice` est générique sur son fournisseur** (`OpenMlsProvider`). Les tests de la maquette prennent celui en mémoire, le client celui sur SQLite.
- **Le commit se fait en deux temps** : `stage(&Change)`, puis `confirm_pending()` une fois que le serveur l'a pris, ou `abandon_pending()` s'il le refuse. Les appels de la maquette (`rotate`, `admit`…) font les deux d'un coup, pour le serveur en mémoire.
- **L'état hors MLS** (les invitations, le lien d'entrée, le commit en attente) sort par `saved_state()` et revient par `restore()`.
- **Les liens d'historique** : `history_link(e)` (tout membre qui tient `e` et `e − 1`) et `learn_history(lien)`, lien par lien.
- **Les balises de tête** (`beacon.rs`) : `beacon(at)`, `check_beacon()`, et `head_words()`, les six mots de la tête à comparer hors bande.
- **Aucune fausse alerte de fourche** : une balise ne compte que si un membre de l'époque courante l'a signée (un membre retiré garde les clés des anciennes époques). Un objet doit être signé par un membre avant qu'on compare sa tête (sinon le serveur forgerait une alerte).

## Ce que fait la crate

| Module | Ce qu'il fait |
| --- | --- |
| `keychain.rs` | Le trait `Keychain` (trois appels, comme le `SecretStore` de l'app : le cœur l'implémente en quelques lignes), `MemoryKeychain` pour les tests, et `Vault`, qui garde un secret comme l'app (écrit, puis relu pour vérifier). Quand le trousseau refuse (Linux sans Secret Service), le secret va dans un fichier lisible du seul propriétaire, et `KeyPlace::File` le dit à l'interface. Le fichier passe au trousseau dès qu'il l'accepte |
| `storage.rs` | `SqliteStorage`, le `StorageProvider` d'openmls sur SQLite, un fichier par équipe (`<dossier>/<équipe>/state.sqlite3`), chaque valeur chiffrée (XChaCha20-Poly1305) sous la clé d'état du trousseau |
| `client.rs` | `ServerClient`, le client du protocole v1 du serveur, derrière le trait `Transport`. `HttpTransport` est celui de l'app : le client reqwest du cœur, et le même proxy (tunnel CONNECT) et le même TLS pour `/v1/live`. `LiveFeed` revient seul |
| `team.rs` | `Team` : créer, rejoindre, ouvrir, lire le flux, committer, faire tourner les clés, retirer, révoquer, inviter et admettre, écrire et lire les objets et les fichiers ; `serve` (le travail d'un appareil en ligne pour les autres), la récupération, le code court |
| `pairing.rs` | Le code court de 10 caractères (TS-3) : SPAKE2 sur le relais du serveur, un seul essai |
| `escrow.rs` | La clé de séquestre de l'organisation en texte : la moitié de l'équipe (sa clé publique et l'ancre de ses enveloppes), son secret |
| `wire.rs` | Les corps CBOR du serveur, et `TeamMessage`, ce que le client écrit dans le flux d'équipe à côté des commits (liens, enveloppes, balises, demandes, Welcome) |
| `replica.rs` | La copie locale d'un projet (TS-5) : SQLite, un fichier par projet, le journal rejoué, la fusion champ par champ, le conflit gardé, les versions des pages (leur historique et leur base commune, TS-9), les numéros des compteurs (TS-9), la file d'envoi |
| `merge.rs` | La fusion à trois voies d'un texte, ligne par ligne (TS-6 et TS-9) : une page, une description écrites des deux côtés |
| `team/sync.rs` | La synchronisation des projets (TS-5) : écrire, envoyer, lire (chaînes, signatures, tête des membres, serveur menteur), les instantanés, les groupes des projets gardés à quelques membres |
| `fuzzing/` | Le corps des cibles de fuzzing (le paquet `fuzz/` à la racine, TS-18 : « Le fuzzing » ci-dessous) |

### Les clés d'appareil, au trousseau, jamais exportées

- Chaque appareil a, par équipe : une clé Ed25519 pour le serveur (elle signe les défis), une clé Ed25519 de feuille MLS (elle signe ses commits et ses objets), et la clé de son état sur le disque. Toutes sont créées sur l'appareil, gardées dans une seule entrée du trousseau (`escouade-team.<équipe>.keys`), et effacées de la mémoire après usage.
- Deux clés plutôt qu'une : le serveur ne peut pas relier un appareil à sa feuille MLS, et un défi signé ne peut jamais passer pour un message MLS.
- Rien ne les exporte : aucun accès hors de la crate, et un `Debug` qui ne les montre pas. Les clés HPKE des feuilles et des KeyPackages sont dans l'état d'openmls, chiffré sous la clé d'état.
- L'id de l'appareil sur le serveur est aussi celui de son identité MLS. Un administrateur sait donc quel appareil couper sur le serveur avant de le retirer du groupe.

### La persistance, et l'effacement des époques passées

- L'implémentation suit `openmls_memory_storage` 0.6 (MIT) appel pour appel, sa `HashMap` remplacée par une table `kv`. Elle corrige au passage son `clear_proposal_queue`, qui gardait les propositions. `openmls_sqlite_storage` aurait voulu rusqlite 0.37, or le verrou tient `libsqlite3-sys` 0.39.1 (une seule version par workspace).
- Chaque étape (une entrée du flux, un commit) est une transaction. En cas d'erreur, l'appareil est rechargé depuis le disque : la mémoire ne diverge jamais de l'état.
- L'effacement des clés passées (avis GHSA-qr9h-x63w-vqfm, point 9 de l'audit) :
  - openmls écrit l'arbre des secrets à chaque message traité, au moment même (c'était l'objet de l'avis) ;
  - aucun secret de message d'une époque passée n'est gardé (`max_past_epochs` 0) ;
  - `secure_delete` met à zéro ce que tenait une ligne effacée ;
  - à chaque nouvelle époque, le journal WAL est versé dans la base puis tronqué (`compact`).
  
  Le test `the_past_epochs_keys_leave_the_disk` cherche les secrets de l'époque 0 dans les fichiers après deux rotations, et ne les trouve pas.
- Les enregistrements d'époque (la clé de contenu et la clé d'historique de chaque époque) sont gardés exprès : un Kanban dure. C'est le point 2 de l'audit.

### Le point de contrôle, hors de l'état restaurable

La plus haute époque vue et sa tête vont dans leur propre entrée du trousseau (`escouade-team.<équipe>.checkpoint`), et jamais seulement sur le disque. Ils y vont après l'état, si bien qu'un arrêt entre les deux laisse l'état en avance, jamais en retard. À l'ouverture, un état plus ancien que ce point de contrôle (une sauvegarde remise, une image disque) est refusé (`Rollback`). C'est le point 8 de l'audit. Sans trousseau, le point de contrôle va dans un fichier à part, à côté des données de l'app et non dans le dossier de l'équipe, et l'interface dit alors que cette garde est affaiblie.

### Le client du serveur

- L'inscription par jeton (`ServerClient::enroll`), le défi signé et la session, renouvelée seule 30 s avant sa fin, ou sur un 401 (le serveur a redémarré).
- Les flux : l'écriture d'un commit (`append_commit`, refusé en `EpochTaken`) ou d'un message, et la lecture par `since` et page.
- Les fichiers chiffrés : `Team::put_file` les scelle comme un objet du projet, avant l'envoi.
- `/v1/live` : il revient seul après une coupure (attente croissante), signe de nouveau quand le serveur ferme la session (4001), et s'arrête à la révocation (4003, ou une connexion refusée). Il dit `Connected` à chaque retour, pour relire les flux depuis leur dernier numéro.
- La poignée de main des versions : `ClientTooOld` (« Mets Escouade à jour ») et `ServerTooOld`.

### L'équipe : le flux, les commits, les fourches

- **Le journal des membres vérifié**, c'est le groupe MLS : chaque commit est vérifié par MLS, puis par les règles de `policy.rs`, avant sa fusion. `Team::members()` donne le registre et les appareils de chaque membre.
- **Le commit nul** (SR2-02, ESC-635) : un commit de l'époque courante que MLS ou les règles refusent (`team::is_null`, et le groupe n'a pas bougé) est gardé dans `Meta::nulls` (`Team::null_alerts`, la carte de l'équipe des administrateurs) et la lecture continue ; l'appareil administrateur signe une annulation (`escouade_team_crypto::Void`, `Team::void_nulls`) puis reprend l'époque réservée (`retake_voided_epoch` : une rotation au `serve` si rien d'autre ne la prend). Une annulation signée par un administrateur pour un commit que l'appareil a pris, ou l'époque d'après un commit nul, est une fourche. Les commits d'avant l'entrée (`early`, `early_from`) : un nul ne bloque pas le Welcome, et l'annulation fait oublier le commit qu'elle nomme. Le groupe d'un projet de même (`sync.rs` : `take_project_void`, `void_project_nulls`, `project_rotate`). Un membre envoie ses commits avec `own` (sa portée signée est `CommitScope::Own`) ; un `409 epoch_reserved` est `Error::Busy`. Tests : `tests/nulls.rs`.
- **Les commits concurrents** : sur un 409 `epoch_taken`, l'appareil abandonne le sien, lit le commit gagnant dans le flux, puis refait le sien s'il reste à faire (`is_done`). Il ne fusionne jamais un commit que le serveur n'a pas pris : jamais deux époques divergentes chez un même appareil. Un commit envoyé sans réponse reste en attente sur le disque. La lecture suivante le retrouve dans le flux (et le fusionne) ou ne l'y trouve pas (et l'abandonne).
- **La rotation au départ** (C8-A) : `remove_member` fait couper par le serveur tous les appareils du membre, puis un commit les retire avec le membre. La nouvelle époque leur échappe, et l'ancien contenu n'est pas rechiffré. `rotate` ouvre une époque à la demande.
- **La révocation d'un appareil** : le serveur le coupe d'abord (ses sessions et ses connexions en direct se ferment), puis le commit le retire. Le serveur ne laisse que ses administrateurs couper un autre appareil. Un membre qui révoque son propre autre appareil, ou un administrateur nommé après son inscription, s'en remet donc au commit seul.
- **Les liens d'historique manquants** : à chaque lecture, l'appareil écrit le lien des époques qu'il tient avec la précédente et dont le flux n'a pas de lien (le commettant s'est arrêté avant de l'envoyer). Un nouvel appareil remonte le premier lien qui s'ouvre. Un lien que le serveur aurait forgé ne s'ouvre pas.
- **Les balises de tête** : à chaque nouvelle époque, et toutes les 6 h (`Options::beacon_every`). Une balise d'un membre courant qui cite une autre tête pour une époque que l'appareil tient, c'est `Fork`.
- **Le serveur menteur** : l'appareil relit la dernière entrée lue avant de lire plus loin. Si elle a disparu, c'est `ServerRollback` ; si elle a changé, `Rewritten`. Un autre commit pour une époque déjà vue, c'est `Fork`. Une numérotation qui recule ou un commit sauté, c'est `Server`.
- **La boîte d'envoi** : liens, enveloppes, balises et Welcome attendent sur le disque que le serveur les prenne.

## L'entrée dans l'équipe (TS-3)

### Créer une équipe

- `Team::create` avec l'adresse du serveur et le jeton d'`init` : l'appareil s'inscrit, crée le groupe (la genèse), et rend la **clé de secours**, 24 mots (C7-B, obligatoire). L'app les montre une fois ; le propriétaire en retape trois (`RecoveryKey::confirms`). Ils ne sont gardés qu'en mémoire jusque-là : l'app arrêtée avant, elle propose une nouvelle clé (`Team::replace_recovery`).
- **L'empreinte en six mots** est celle de la genèse (`Team::fingerprint`) : chaque lien la porte, l'invité la voit avant d'entrer.
- **La clé de séquestre**, en option, au registre : chaque membre la voit (`Team::escrow`). `escrow::new_pair` fait la paire pour l'organisation, dont le secret (la graine de la paire, `escouade-escrow-secret:1:…`) est montré une fois ; `EscrowKey::from_seed` la redonne. Avec les enveloppes de séquestre du flux et les liens d'historique, elle lit tout le contenu.

### Inviter : le secret après le `#` (C6-A)

- `Team::invite_for(objet, maintenant, durée)` : 72 h par défaut, un seul usage. Le lien `escouade://team/join/<serveur>/<id>#s=<secret>&t=<empreinte>` (et `&m=<membre>` pour un autre appareil d'un membre) : il ouvre Escouade, jamais une page web, que le serveur d'équipe servirait et dont le script lirait le secret (SR2-08). Un lien de l'ancienne forme (`https://<serveur>/join/<id>#…`) est refusé (`InvitationError::OldForm`), et une invitation faite avant (`Invitation::app_link` faux) n'admet plus personne.
- **Le jeton du serveur est dérivé du secret** (`invite::server_token`, HKDF) : l'administrateur en enregistre le SHA-256 auprès du serveur (`POST /v1/devices/enrollments {token_hash}`), qui ne voit jamais le secret. Le lien seul suffit donc à l'invité (`Team::join_link`).
- **Usage unique, des deux côtés** : le serveur ne laisse servir le jeton qu'une fois (un second usage : 409 `token_used`, `Error::InvitationUsed` pour l'invité légitime, et une alerte pour les administrateurs, que `serve` lit et que l'app montre sur l'invitation) ; le groupe ne l'admet qu'une fois (`Invitation::used`).
- **L'expiration, des deux côtés** : le serveur oublie le jeton à son terme (`Error::InvitationUnknown`), et un administrateur en ligne après lui n'admet plus la demande.
- **L'invité vérifie l'empreinte** : le Welcome doit être d'un groupe de l'empreinte du lien, et porter la confirmation du secret (`invite.rs`).
- **Aucune admission sans les six mots** (SR2-08) : `Team::serve` n'admet plus rien. Il écarte les demandes qui ne se prouvent pas et compte les autres (`Served::waiting`) ; `Team::admissions` les rend vérifiées (secret, KeyPackage signé), chacune avec son `AdmissionCode` (six mots tirés de l'id de l'invitation et du KeyPackage). L'invité montre les siens (`Team::join_words`, gardés dans l'état jusqu'au Welcome). `Team::admit_invitation(id, mots)` n'admet que la demande dont les mots sont ceux confirmés (`InvitationError::CodeMismatch` sinon) ; `Team::refuse(id)` annule l'invitation (`Invitation::refused`), écarte ses demandes, coupe au serveur les appareils qu'elles nomment (hors du groupe) et scelle une alerte `TeamMessage::Refused` que les autres appareils lisent (`Team::refusals`).

### Ajouter un de ses appareils (§3.4)

- **Par QR code** : le lien d'une invitation `Purpose::OwnDevice(membre)` (une heure), montré en QR code par l'app, ou collé sur l'autre appareil. L'appareil du membre l'admet lui-même, les six mots des deux écrans comparés : la règle « un membre n'ajoute que ses propres appareils ».
- **Le jeton d'un membre** passe par une demande dans le flux : un membre n'est pas administrateur du serveur, qui refuse alors l'enregistrement (403). Son appareil écrit `TeamMessage::TokenRequest`, un objet de l'équipe scellé et signé par sa feuille, qui ne porte que le SHA-256 du jeton. Un administrateur du serveur en ligne vérifie l'auteur (un membre de l'époque) et son droit (son propre appareil, ou un administrateur pour un nouveau membre), enregistre le jeton, et le dit (`TokenGranted`).
- **Par code court**, sans caméra (`pairing.rs`) : 10 caractères (base 32 de Crockford), les 5 premiers le créneau du relais choisi par le serveur (un parmi 33 millions, SR2-05), les 5 autres le mot de passe d'un **SPAKE2** (la crate `spake2` de RustCrypto). L'appareil dans l'équipe ouvre le créneau avec son message ; le nouveau y laisse le sien et une confirmation de clé : **un seul essai** par créneau, qui lui rend le jeton (128 bits) sans lequel la réponse ne se prend pas. Un mauvais code, l'appareil dans l'équipe le voit à la confirmation : il ferme le créneau, rien n'est parti. Le bon code, il envoie le lien de l'invitation scellé (XChaCha20-Poly1305) sous la clé du PAKE, une fois son jeton enregistré. Le serveur ne voit que les messages du PAKE et la réponse scellée : rien qui permette d'essayer des codes hors ligne.

### Une machine de l'équipe (OE-1)

- **Son entrée** : `Team::pair_start_service` (un administrateur seul) montre un code court comme `pair_start`, mais sous une invitation `Purpose::NewMember(Role::Service)` ; la machine le joue par `pairing::fetch_link` et entre par le lien, avec son propre id de membre, ses six mots comparés comme pour tout le monde. Le cœur l'appelle par `escouade-server team pair` (`src-tauri/src/server/teampair.rs`).
- **Ce qu'elle signe**, vu du client : `Team::may_sign(Act)` (la table de `policy::may_sign`), `Team::is_service` (cet appareil), `Team::is_machine(membre)` (le registre, ou `Meta::services`, les machines vues un jour : le rôle ne change jamais, une machine retirée l'était pour ce qu'elle a signé avant). Les réglages, le coupe-circuit, la liste des projets et la clé des coûts ne prennent déjà que des administrateurs (`admins_at`) ; le store privé (`plugins.rs`, `Refused::Machine`) et les propositions (`proposals.rs`) refusent en plus toute entrée d'une machine, et son client ne l'écrit pas.
- **Les tests** (`tests/service.rs`, SQLite puis PostgreSQL) : l'inscription par le code, l'id à elle, le rôle « service » au serveur ; les routes d'administrateur et les flux de l'équipe refusés (403), `service_stays` ; un réglage, un coupe-circuit, une clé et un retrait du store scellés par la machine et joués par un serveur menteur, refusés par Bob ; un projet lu et écrit, un ticket pris, son nom vu ; la machine retirée, les clés tournées, Bob qui écrit encore.

### Les rôles du serveur au pas du registre

`serve` compare, à chaque nouvelle époque, le rôle de chaque appareil sur le serveur à celui de son membre au registre (administrateur, membre, ou service pour une machine de l'équipe), et le corrige (`POST /v1/devices/{id}/role`) quand cet appareil est administrateur du serveur. Un administrateur nommé après son inscription le devient donc aussi sur le serveur : il enregistre ses invitations et coupe les appareils.

### La récupération par la clé de secours, à travers le serveur

**L'ancre** (SR2-09, `escouade_team_crypto::anchor`) : `Team::create` et `Team::replace_recovery` gardent l'ancre des mots sur l'appareil (dans son état chiffré, jamais les mots) ; `Team::serve` d'un appareil administrateur la donne aux autres appareils administrateurs (`TeamMessage::Anchor`, scellée à leur feuille ; `Meta::anchors_given` retient ce qui est donné) et écrit les enveloppes de l'époque quand le commit d'un membre n'en a pas fait (`Meta::recovery_wrapped`, `escrow_wrapped`). Le commit d'un membre ne fait jamais d'enveloppe. Tests : `tests/onboarding.rs` (une enveloppe d'un groupe de membre écartée, l'ancre jusqu'au second administrateur, l'enveloppe après le commit d'un membre) et `escouade-team-crypto/src/tests.rs`.

`Team::recover_onto` : le jour où plus aucun administrateur n'a d'appareil, l'hébergeur donne un jeton (`escouade-team-server recovery-token --team <id>`). Avec lui et les 24 mots :
1. la feuille de récupération s'inscrit (administrateur du serveur), lit la dernière enveloppe de récupération du flux (`TeamMessage::Recovery {group, wrap}`) **signée par l'ancre de ces mots** (les autres sont écartées), vérifie les six mots de l'équipe tapés depuis la feuille de secours et la GroupInfo (empreinte, clé de secours, feuille administrateur qui a signé), puis entre dans le groupe par un commit externe ;
2. elle invite un nouvel appareil administrateur, celui de la machine, et l'admet avec les six mots qu'elle lit elle-même sur le nouvel appareil (les deux écrans sont la même machine) ; puis elle le met administrateur du serveur ;
3. le nouvel appareil remplace la clé de secours (la feuille de l'ancienne quitte le groupe dans le même commit) ; l'appareil serveur de la feuille est révoqué, ses clés oubliées.

Les mots sont vérifiés avant d'utiliser le jeton : une faute ne le gâche pas. Les anciens mots n'ouvrent plus rien ensuite.

### L'alerte de fourche

`Error::alarm()` dit ce qu'une erreur montre du serveur : une fourche, un retour en arrière, une entrée réécrite. Le cœur arrête alors l'équipe sur cet appareil : il ne lit plus son flux et n'y envoie plus rien (ni invitation, ni entrée d'un invité, ni nouvel appareil, ni nouvelle clé de secours). L'app dit « Le serveur montre deux versions de l'équipe : contacte un administrateur », avec les six mots de la tête à comparer, « Réessayer » (une lecture qui passe lève l'alerte) et « Retirer l'équipe de cet appareil ».

### Le décodage

`wire::decode` lit le CBOR avec un tampon de la taille de ce qu'il lit : celui de ciborium (4 Kio) refusait une chaîne plus longue, comme l'hexadécimal d'un GroupInfo (une enveloppe de récupération) ou d'un Welcome d'une équipe qui grandit.

## La synchronisation des projets (TS-5)

Choix tranchés (§15.2) : C2-A (journal d'opérations, champ par champ, le dernier qui écrit gagne sous une horloge hybride), C9-B (MLS), C10-A (bout en bout seul mode). La page de spec est « La synchronisation des projets ».

### Le journal d'opérations

- **Une opération** (`escouade-team-crypto/src/ops.rs`) : son auteur (membre, appareil, clé de feuille), son rang `dseq` et l'empreinte de l'opération d'avant du même appareil, la tête du journal des membres que l'auteur connaissait (époque et tête de l'équipe), son horodatage (`hlc.rs`), et ce qu'elle fait : `Create`, `Set` (avec la base vue), `Move`, `Delete`, `Version` (une page en version entière, après ses bases).
- **Signée** par la feuille, puis **scellée** (`Sealed`) sous la clé du projet de l'époque. Le serveur en voit l'époque et la portée (de l'équipe ou du groupe du projet), sa taille, son heure, l'appareil qui l'envoie : rien d'autre (§3.8). Le test `two_devices_share_a_projects_tickets` cherche le titre, le nom du champ, l'id de l'objet et la clé de l'auteur dans ce que garde le serveur, et ne les trouve pas.
- **Un projet de toute l'équipe** prend la clé de l'équipe ; sa première opération le dit (`Create` de genre `project`), pour qu'un appareil qui le suit le sache avant d'y écrire. Une opération sous l'autre portée que celle du projet est refusée : un membre ne peut pas rendre lisible à toute l'équipe un projet gardé à quelques membres.

### Un groupe MLS par projet gardé à quelques membres

- `Team::create_project(Some(membres))` (un administrateur) : le groupe du projet (`escouade-team-crypto/src/project.rs`), dont les feuilles sont les appareils de ces membres. Ses commits vont dans le flux du projet (le premier d'une époque gagne au serveur, comme pour l'équipe), son Welcome et ses liens d'historique dans le flux de l'équipe.
- **Les KeyPackages** : chaque appareil publie le sien (de dernier recours, réutilisable d'un projet à l'autre) dans le flux de l'équipe, au premier `serve` puis tous les 30 jours. Il n'est pris que pour un appareil qui est une feuille de l'équipe, avec la même identité et la même clé.
- **Au pas de l'équipe** : `Team::serve` (et l'envoi d'une opération) tient chaque groupe de projet au pas : les nouveaux appareils des membres ajoutés, les appareils partis de l'équipe retirés. Seul un administrateur change qui a accès (`set_project_members`).
- **Les projets d'une invitation** (la ligne de TS-3) : `Team::invite_with_projects` ; le membre qui entre avec elle est ajouté aux groupes de ces projets par l'administrateur en ligne, une fois son KeyPackage lu.
- **Les règles** (`check_project_commit`) : le committeur est une feuille du groupe et de l'équipe ; il garde son identité et sa clé ; un appareil ajouté est une feuille de l'équipe dont le membre a accès ; un appareil est retiré s'il a quitté l'équipe, si son membre n'a plus accès, par son propre membre ou par un administrateur. Le commit dit dans ses données authentifiées l'époque de l'équipe qu'avait son committeur : un appareil en retard lit d'abord l'équipe.

### La fusion, champ par champ

`replica.rs` garde chaque écriture d'un champ avec son horodatage, dont les octets se trient comme les horodatages : la valeur d'un champ est l'écriture la plus récente, calculée en SQL, quel que soit l'ordre d'arrivée. Deux écritures depuis la même base sont concurrentes : la perdante est gardée (`conflicts`), pour que TS-6 la montre. Une suppression est définitive. Les versions d'une page : les têtes (aucune autre ne les cite en base) sont la version courante (la plus récente) et les conflits, que le cœur fusionne à trois voies (TS-9, ci-dessous).

### La copie locale et la file d'envoi

- `<teams>/<équipe>/projects/<projet>.sqlite3`, en clair comme le Kanban local de `.escouade/` (les clés restent dans l'état chiffré). Le journal y est gardé ; `Replica::rebuild` rejoue l'état depuis le dernier instantané et le journal.
- `Team::write` (et `create_ticket`, `set_field`, `move_ticket`, `delete_ticket`, `write_version`) : l'opération s'applique à la copie tout de suite et attend dans la file. `flush_project` l'envoie dans l'ordre, chiffrée à l'époque du moment (un membre parti entre-temps ne la lit pas). Hors ligne, elle attend ; `Team::sync_state` dit « hors ligne » et le nombre de changements en attente, et le cœur l'expose (`team_sync_status`).
- **Reprise après coupure** : la lecture reprend au dernier numéro vu (`ProjectMeta::cursor`), dans la copie sur le disque.

### Le serveur qui ment

À la lecture (`Team::read_project`), chaque opération est ouverte (une feuille de son époque, sa signature), sa tête des membres vérifiée (la fourche), puis sa place dans la chaîne de son appareil :
- un rang déjà passé : `Replayed` ; un rang sauté (une opération retirée, ou deux d'un appareil inversées) : `Reordered` ; deux opérations pour un même rang, ou une qui ne cite pas la précédente : `Equivocation` ;
- une opération de cet appareil que le serveur a prise à un numéro et qui n'y est pas : `Withheld` ;
- une opération qui ne s'ouvre pas ou n'est signée par aucune feuille : `Forged` ;
- la dernière entrée lue disparue ou changée : `ServerRollback`, `Rewritten`, comme pour le flux de l'équipe.

Ces erreurs sont des alarmes (`Alarm::Journal`) : le cœur arrête l'équipe sur l'appareil, comme pour une fourche. Une opération renvoyée après une réponse perdue revient en double au même rang, avec la même empreinte : elle compte une fois.

### Les instantanés

Au-delà de `Options::snapshot_every` (10 000) entrées depuis le dernier, l'appareil qui lit écrit un instantané (`Snapshot`) : l'état de la copie et la chaîne de chaque appareil au dernier numéro lu, scellé et signé. Le serveur garde les opérations d'avant 30 jours, puis les oublie (pas les commits), sauf si un appareil a signalé l'instantané (SR2-01). Un appareil qui arrive, ou qui était absent au-delà, part de l'instantané : sa signature, sa tête et son numéro vérifiés, chaque chaîne au moins aussi loin que ce qu'il a lu, ses propres opérations prises par le serveur dedans. Un appareil déjà au-delà le vérifie une fois (`Entries::latest`) : aucune chaîne plus loin que ce qu'il a lu.

Un instantané qui ne passe pas (`Error::Forged` dans `open_snapshot`, ou une époque de l'équipe jamais atteinte une fois l'équipe relue) est **écarté**, jamais une alarme : `put_aside` le signale au serveur (`flag_snapshot`), le note dans `ProjectMeta::discarded` (il n'est plus repris) et dans les alertes de l'équipe (`Team::snapshot_alerts`, l'appareil qui l'a posé), puis la lecture prend l'instantané d'avant (`get_snapshot(before)`, 16 au plus) ou le journal. Un administrateur lit au `serve` la liste du serveur (`read_snapshot_alerts`) et retire un instantané (`Team::remove_snapshot`) ; un membre masque son alerte (`dismiss_snapshot_alert`).

### La rotation (C8-A)

Après le départ d'un membre, les nouvelles opérations sont à l'époque suivante : celle de l'équipe, ou celle du groupe du projet, que le premier appareil du projet en ligne fait tourner. L'ancien membre ne lit plus rien de nouveau ; l'historique n'est pas rechiffré.

### Le direct dans le cœur

`src-tauri/src/teams.rs` tient la connexion en direct de chaque équipe (`LiveFeed`) : une trame (une entrée de l'équipe ou d'un projet, une connexion revenue, des trames perdues) déclenche la lecture de l'équipe puis de ses projets suivis, les trames d'une rafale lues une fois. La relecture de toutes les équipes passe de 3 s à 15 s et ne sert plus que si le direct ne passe pas. Le test du cœur `a_projects_operation_comes_by_the_live_feed` le vérifie avec la relecture à 60 s.

## Le Kanban d'équipe (TS-6)

Choix tranchés (§15.2) : C2-A, C11-A (le pilote auto ne prend que ce qu'on lui confie), C5-A (pas de passerelle `.yml`). La page de spec est « Le Kanban d'équipe » ; le cœur le branche dans `src-tauri/src/teamboard.rs`.

### La prise d'un ticket (`team/claims.rs`)

- `Team::take_claim`, `renew_claim`, `release_claim`, `claims` : la prise d'un ticket par son **id opaque** (16 octets tirés par le cœur, champ `claim` du ticket chiffré, jamais son `ObjectId`), par le serveur. Hors ligne, `ClientError::Net` : un ticket de l'équipe ne se prend jamais sans le serveur. Un refus est `ClientError::Claim { code, claim }` (`taken`, `stale_version`, `claim_lost`, `stale_token`), la prise du serveur en détail.
- **Le jeton** : `Team::write_fenced(projet, opération, Some(Fence { claim, token }))` met l'opération dans la file avec sa garde (colonne `fence` de l'`outbox`, ajoutée à une copie plus ancienne). À l'envoi, le serveur refuse celle d'un jeton dépassé : `drop_stale` reprend toute la file (`Replica::take_unsent`, qui retire ces opérations du journal local et rejoue l'état), remet la chaîne de l'appareil à sa dernière opération prise par le serveur, et réécrit les autres dans l'ordre, sans celles de ce jeton ; `Team::take_refused` dit au cœur ce qui a été retiré. La chaîne de l'appareil n'a jamais de trou : un autre appareil la lit sans `Reordered`.
- **Les droits** : `Team::sync_access` dit au serveur qui a accès à un projet (toute l'équipe, ou les appareils des membres d'un projet gardé), à sa création (avant le premier commit de son groupe : sa première écriture l'ouvrirait à toute l'équipe) et après chaque commit de son groupe par un administrateur. Un refus `no_access` du serveur devient `Error::NoAccess`.
- **Les clés** : `Team::next_key`, le compteur du serveur ; le préfixe est un champ (`prefix`) de l'objet du projet dans le journal.
- **Les noms** : `Team::set_name` écrit dans le projet un objet de genre `person` dont l'id est celui de l'appareil (son nom, son membre), chiffré comme le reste ; `name_of` traduit l'appareil d'une prise en nom (ou celui d'un autre appareil du même membre). C'est l'annuaire de TS-6, projet par projet, en attendant celui de l'équipe (TS-4).

### Les conflits et la fusion à trois voies

- `replica::Conflict` porte désormais la valeur de la base des deux écritures (`base`), lue dans `writes` à son horodatage.
- `merge::merge3(base, mien, sien)` : une fusion ligne par ligne (plus longue sous-suite commune contre la base, puis morceaux entre lignes stables) ; `None` quand les deux côtés changent les mêmes lignes autrement, ou au-delà de 4 000 lignes. Le cœur réécrit la description fusionnée ; sinon la carte offre « Garder le mien » et « Prendre le sien ».

### Les tests (`tests/claims.rs`, SQLite puis PostgreSQL)

Deux prises en même temps (une passe, l'autre apprend le nom du preneur), le bail renouvelé puis fini (« interrompu », repris sous un nouveau jeton, le renouvellement trop tard refusé, le rendu), une écriture d'un jeton repris refusée et retirée sans trou dans la chaîne (les autres écritures parties), un membre retiré qui ne prend plus rien et un appareil hors d'un projet gardé (ni lecture, ni prise, ni clé), ce que garde le serveur (sa base lue : ni titre, ni préfixe, ni nom, ni objet), les numéros du compteur, et hors ligne puis retour (aucune prise, la file qui part, le titre au dernier, la description fusionnée à trois voies, « Reprendre le mien »). `merge.rs` a ses tests.

## Les tests

```sh
cd src-tauri && cargo nextest run -p escouade-team -p escouade-team-crypto -p escouade-team-server
```

`tests/team.rs` et `tests/devices.rs` lancent le vrai `escouade-team-server` dans le test, sur une base SQLite temporaire, derrière un relais TCP qu'on peut couper. Chaque appareil a son dossier et son trousseau en mémoire, et un transport qui ment quand le test le lui dit.

- **Les membres** : création, ajout de deux membres (le second lit ce qui a été écrit avant lui), retrait (le serveur le coupe, puis la nouvelle époque lui échappe), rotation, et un nouvel appareil qui remonte jusqu'à l'époque 0.
- **Les commits concurrents** : le commit de Dave est retenu en route pendant qu'Alice committe. Le serveur refuse celui de Dave (409), qui rejoue celui d'Alice puis refait le sien : une seule branche. Deux retraits du même membre en même temps : le second trouve le travail fait.
- **Le redémarrage** : l'appareil rouvre son état (même époque, même tête, même historique) et reprend le flux où il l'avait laissé. Un commit envoyé dont la réponse s'est perdue est retrouvé au redémarrage.
- **La sauvegarde ancienne** : le dossier remis à l'époque 1 est refusé, car le trousseau a vu l'époque 3.
- **Le serveur qui ment** : il cache ses dernières entrées (retour en arrière refusé) ou change une entrée lue (refusé). Pour la fourche, il montre à Bob un autre commit valide pour une époque : la balise d'Alice le montre à Bob, et un objet de chaque branche le montre à l'autre.
- **Un lien d'historique manquant** est écrit par un autre membre, et un appareil qui arrive après lit tout.
- **Un appareil volé** : coupé par le serveur (son flux en direct dit `Revoked`), puis retiré par le commit.
- **Le direct** : il revient après une coupure, puis après un redémarrage du serveur (les sessions sont alors signées de nouveau). La session expirée se renouvelle seule. Les fichiers voyagent chiffrés.
- **Sans trousseau** : les clés et le point de contrôle vont dans un fichier, ce qui se dit, et l'équipe se rouvre.
- **L'effacement** des secrets des époques passées sur le disque, et rien de lisible en clair dans les fichiers.
- **L'entrée** (`tests/onboarding.rs`, TS-3) : création (24 mots, trois retapés, six mots), une invitation qui sert une fois (le second usage dit à l'invité et à l'administrateur), son expiration sur le serveur et dans l'équipe, les rôles du serveur au pas du registre, l'appareil d'un membre par le lien du QR code (son jeton demandé dans le flux), le code court (un mauvais code qui coûte son seul essai, une devinette qui prend l'essai du bon), le séquestre montré à chaque membre et qui lit le contenu, la récupération par les 24 mots à travers le serveur, la fourche comme alerte.

- **La synchronisation** (`tests/sync.rs`, TS-5), chacun sur SQLite puis sur PostgreSQL (`ESCOUADE_TEST_POSTGRES_URL`, comme TS-1 ; la macro `both!` du banc) : deux appareils qui partagent les tickets (créer, changer un champ, déplacer, supprimer ; ce que voit le serveur), trois appareils qui écrivent en même temps sur le même champ et sur des champs différents (les copies pareilles, le conflit gardé), l'écriture hors ligne puis le retour (l'état « hors ligne », l'ordre gardé, une réponse perdue comptée une fois), le serveur qui ment (rejouée, réordonnée, retirée, tue, forgée, réécrite, retour en arrière), une signature fausse, un membre parti qui ne lit plus (projet de toute l'équipe et projet gardé à deux), un instantané puis un nouvel appareil (et un appareil absent au-delà ; un instantané forgé écarté, signalé, vu par l'administrateur et retiré ; un instantané d'une époque jamais atteinte écarté par un appareil au-delà), un instantané illisible seul qui laisse le journal, la reprise après coupure (rouvert depuis le disque, relu depuis le dernier numéro, rejoué depuis le journal), une invitation qui donne un projet. Le transport `Liar` ment aussi sur les flux des projets.
- **La convergence** (`replica.rs`, proptest) : quel que soit l'ordre d'arrivée des opérations de trois appareils (et rejouées), toutes les copies finissent pareilles, rejouées depuis leur journal aussi ; un instantané puis le reste égale le journal entier.

Les tests unitaires couvrent le trousseau, le chiffrement du stockage (une autre clé n'ouvre rien, une valeur déplacée non plus), les transactions, et chaque cible de fuzzing sur ses graines et sur des entrées au hasard (TS-18). La maquette garde ses 18 tests et ses vecteurs, et en gagne 6 (commit en deux temps, invitation rendue, état sauvegardé, balises, liens d'historique, fausses alertes).

- **Les bornes du client** (`tests/bounds.rs`, TS-18) : une réponse du serveur annoncée ou lue au-delà de `MAX_REPLY` est refusée, un message du direct au-delà de `MAX_LIVE_MESSAGE` ferme la connexion, une réponse dans les bornes passe.
- **Les propriétés** (`tests/properties.rs`, proptest, TS-18) : l'horloge hybride ne recule jamais et passe ce qu'elle a vu (dans la dérive permise), ses octets se trient comme ses horodatages ; la fusion à trois voies rend l'autre côté quand un côté n'a pas changé, garde deux changements éloignés, garde ce que les deux côtés ont ajouté.

## Le fuzzing (TS-18)

`fuzz/`, à la racine du dépôt, est le paquet cargo-fuzz du serveur d'équipe et de son client : son workspace, son verrou et sa nightly (`fuzz/rust-toolchain.toml`) sont à lui, rien n'en passe dans `src-tauri/Cargo.lock` ni dans `cargo deny`. Les corps des cibles sont ici (`src/fuzzing/`, derrière la feature `fuzzing`) et dans `escouade-team-server` (`src/fuzzing.rs`). La CI fait une passe courte (le job `fuzz` de `ci.yml`, 15 s par cible), `.github/workflows/fuzz.yml` une longue (chaque dimanche, ou à la main).

```sh
cargo install cargo-fuzz          # Linux ou macOS ; sous Windows, une distribution WSL
sh scripts/fuzz.sh                # toutes les cibles, 20 s chacune
sh scripts/fuzz.sh 3600 journal   # une cible, une heure
```

Chaque cible part de ses graines commitées (`fuzz/seeds/<cible>/` : des entrées choisies, les vecteurs de `escouade-team-crypto` sous la forme où le serveur les sert, et ce que le fuzzing a trouvé). `ESCOUADE_WRITE_SEEDS=1` les réécrit (`fuzzing::tests::write_the_seeds`). Les tests de la crate jouent chaque cible sur ses graines et sur des entrées au hasard, à chaque passage de nextest : une trouvaille devient une graine, donc un test de non-régression. `scripts/fuzz.sh` refuse ce qui alloue plus de 256 Mio d'un coup ou tient plus de 2 Gio : ce que donne un pair ne fait jamais allouer sans borne.

Ce que vérifie chaque cible, au-delà de « ne panique pas » :

| Cible | Ce qui vient du réseau ou du disque | Les propriétés |
| --- | --- | --- |
| `commit` | un commit MLS d'un administrateur ou d'un membre, construit par les octets, et les octets bruts | ce qu'un appareil prend laisse l'équipe saine : genèse gardée, un administrateur au moins, chaque appareil d'un membre du registre, rien qu'un membre n'a le droit de faire |
| `roster` | le registre, les identités, `TeamMessage`, les entrées et les trames du serveur, les objets scellés, les enregistrements d'époque | ce qui se décode se réencode pareil |
| `journal` | une tête citée pour une époque, l'état que montre le serveur, un point de contrôle remis, une balise, un objet et un lien d'historique d'Alice changés par les octets | une tête autre que la sienne pour une époque tenue est une fourche ; une époque plus vieille que le point de contrôle est un retour en arrière ; une balise, un objet, un lien changés ne s'ouvrent que tels qu'Alice les a signés |
| `operation` | une opération scellée changée, une opération, un instantané, un registre de projet, un horodatage ; une opération qui se dit d'Alice mais signée d'une autre clé | elle s'ouvre telle qu'elle a été scellée, ou pas du tout ; une signature fausse n'est jamais prise ; appliquée deux fois, la seconde ne change rien |
| `convergence` | les opérations de trois appareils faites par les octets, dans un ordre qu'ils choisissent, certaines rejouées, un instantané au milieu | toutes les copies finissent pareilles, rejouées depuis leur journal aussi |
| `snapshot` | l'état d'un instantané tel qu'il arrive, un instantané scellé changé | refusé, ou chargé entier (et rechargé pareil) ; il ne s'ouvre que tel que scellé |
| `envelope` | les enveloppes de séquestre et de secours (HPKE) changées, les textes de la clé de séquestre | elles ne s'ouvrent que sur l'époque et le GroupInfo scellés |
| `invite` | un lien d'invitation, une demande d'entrée changée, une confirmation de Welcome changée | le lien se relit pareil, son hôte n'a ni chemin ni utilisateur, ce qui part au serveur n'a ni secret ni empreinte ; le MAC ne tient que pour l'invitation et le KeyPackage d'origine, la confirmation que pour son groupe et son époque |
| `handshake` | un code court tapé, l'essai du nouvel appareil (message SPAKE2 et confirmation) changé par le relais, une réponse scellée | un essai changé ou un mauvais code n'est jamais pris, l'essai ne sert qu'une fois, la réponse ne s'ouvre que par la bonne clé et le bon créneau |
| `catalog` | une adresse, une entrée de la liste des projets, une entrée scellée changée | une adresse qui passe est https ou ssh, sans option de git, ni assistant distant en tête, ni espace, ni `..` ; un projet que `check` laisse passer a des dossiers qui restent dans le projet ; l'entrée ne s'ouvre que telle qu'écrite |
| `settings` | un `.escouade.yml` d'équipe, des versions et des cosignatures de trois membres dans n'importe quel ordre | ce qui passe `never` n'impose rien qui s'exécute ; la chaîne choisie va 1, 2, 3… chacune citant la précédente, lisible, au format, signée par assez d'administrateurs d'aujourd'hui (deux sous le quorum) ; relue avec la suite, elle n'est jamais défaite |
| `server-bodies` | chaque corps CBOR d'une requête, l'en-tête de la poignée de main des versions, `X-Forwarded-For` | la poignée de main n'accepte que la version du serveur ; l'adresse est celle du pair ou celle que l'en-tête nomme |
| `server-requests` | une requête faite des octets (route, ids, session ou non, version, type, corps) sur le vrai routeur en mémoire, de personne, d'un appareil révoqué, d'un membre, d'un administrateur | jamais de 5xx ; rien que les routes ouvertes ne répond sans session ni à l'appareil révoqué ; un membre ne fait rien de ce qui est à l'administrateur |

**Ce qui a été trouvé** (corrigé, avec sa graine ou son test) :
- par le fuzzing (`catalog`) : le nom de dossier que l'assistant propose pour cloner un projet de la liste reprenait la fin de l'adresse telle quelle, si bien qu'une adresse `…/.ssh` proposait le dossier caché `.ssh` du dossier parent (`folderName`, `src/lib/team-projects.ts`, et son test) ;
- par la relecture : le client lisait une réponse du serveur sans borne (le corps HTTP entier, et un message du direct jusqu'aux 64 Mio de tungstenite). `client::MAX_REPLY` (32 Mio, refusé dès la longueur annoncée, sinon dès qu'il est dépassé) et `MAX_LIVE_MESSAGE` (4 Mio) les bornent, testés dans `tests/bounds.rs`.

**Ce qu'il ne couvre pas**, pour l'audit externe (TS-20) :
- MLS lui-même (openmls a sa propre suite et ses vecteurs) : `commit` construit des commits valides au sens de MLS, pour viser nos règles ;
- la lecture d'un flux entier par `Team` contre un serveur qui ment : c'est le transport `Liar` des tests d'intégration (`tests/sync.rs`, `tests/team.rs`), pas le fuzzing ;
- le stockage SQLite chiffré d'openmls et le trousseau (le disque n'est pas une entrée d'un pair) ;
- le cœur de l'app (`src-tauri`) : le lien `escouade://team/join`, l'application des réglages d'équipe, le Kanban d'équipe ; ses tests le couvrent, pas le fuzzing ;
- `merge3` (la fusion à trois voies) est vérifiée par proptest (`tests/properties.rs`), pas fuzzée : sa table de plus longue sous-suite commune va jusqu'à 64 Mio pour deux textes de 4 000 lignes, une borne voulue mais large ;
- le temps : `server-requests` ne mesure ni le débit ni la durée d'une requête, que les tests du serveur bornent (`tests/bounds.rs`, `server.rs`) ;
- `commit` reconstruit son équipe à chaque entrée (quelques centaines par minute) : c'est la passe longue qui le fait avancer.

## Le .escouade.yml d’équipe (TS-13)

`settings.rs` tient le fichier de réglages de l’équipe (docs/TEAM-SERVER.md §6.5, §6.6, C12-B). Le cœur lui donne la sorte de chaque clé (trait `Classify`, `src-tauri/src/team_settings.rs`) et l’applique ; la crate dit ce qui est en vigueur et ce qui attend.

- **Où** : le flux de projet `SETTINGS_STREAM` (`b"escouade/setting"`, un id qu’aucun client ne tire), sur ce que TS-1 offre déjà. Chaque entrée est un `SealedObject` sous la clé d’équipe de son époque, signé par la feuille de son auteur : `Entry::Version { body }` (le `Version` encodé : numéro, empreinte de la précédente, fichier en JSON, nom de l’auteur, heure, note) ou `Entry::Cosign { number, digest }`. L’empreinte d’une version est `digest(body)`, sur les octets signés, jamais sur un réencodage.
- **Qui signe** : un administrateur de l’époque de son scellement (`Team::admins_at`, que `team.rs` garde pour chaque époque vécue) et d’aujourd’hui, dans `signers` quand la version d’avant en nomme. Chaque membre compte une fois.
- **La chaîne** : au numéro `k`, la première candidate (dans l’ordre du flux) qui cite la version `k − 1` choisie et a ses signatures ; la chaîne choisie est gardée (`chosen`) et jamais défaite. Les autres : `Lost`, `Behind`, `BadChain`, `Replayed` (même empreinte), `NotAdmin`, `RemovedAdmin`, `Never`, `Unreadable` ; une entrée qui n’ouvre pas : `Unsigned`.
- **Le quorum** : 2 signatures quand le quorum de la version en vigueur tient (`quorum`, sinon deux administrateurs ou plus) et que la version change une clé sensible, `signers` ou `quorum` (`needs_quorum`).
- **Ce qui n’entre jamais** (`never`) : la liste en dur, avec des clés qui s’exécutent connues ici même si le cœur ne les a pas (`RUNS`). Une ligne trouvée refuse la version entière.
- **Ce qui s’applique** (`Effective`) : `enforced` et `defaults`, sauf le sensible qui attend l’accord de cet appareil ; `proposed` accepté (`accepted`). Un accord (`consent`) nomme `fingerprint(part, clé, valeur)` : une valeur changée redemande l’accord. `withdraw` le retire.
- **Le serveur menteur** : comme le flux d’équipe, la dernière entrée lue est relue avant de lire plus loin (`Alarm::Rollback`, `Rewritten`, `Server`, `Fork`) ; sous l’alarme, rien n’est lu jusqu’à `retry`.
- **`serve`** : un appareil d’administrateur cosigne les versions de la chaîne dont les signatures ne comptent plus (leur auteur est parti), pour qu’un appareil qui arrive après lise toute la chaîne.
- L’état (`State`) va dans la base chiffrée de l’équipe (`Team::save_extra`, `extra/settings`).

Les tests (`tests/settings.rs`, contre le vrai serveur) : version signée appliquée, chaîne et historique ; un membre qui signe ; une entrée forgée ou non signée ; un administrateur retiré depuis (et l’appareil venu après, servi par la cosignature) ; une version rejouée, qui recule, ou dont le numéro est pris ; un serveur qui cache ou change une version ; le quorum (une signature ne suffit pas, deux fois le même membre non plus, deux oui ; les signataires et le quorum gardés de même) ; un seul administrateur ; chaque sorte ; chaque ligne de ce qui n’entre jamais.

## Les projets de l'équipe (TS-8)

`catalog.rs` tient la liste des projets de l'équipe (docs/TEAM-SERVER.md §5). La page de spec est « Les projets de l'équipe ».

- **Où** : le flux de projet `CATALOG_STREAM` (`b"escouade/project"`), comme le fichier d'équipe. Chaque entrée est un `SealedObject` sous la clé d'équipe de son époque, signé par la feuille de son auteur, avec un id d'objet tiré à chaque fois : `Entry::Put { id, project, at }` (un projet ajouté ou changé, entier) ou `Entry::Remove { id, at }`. Le serveur voit qu'une entrée est écrite, sa taille, son heure ; ni nom, ni adresse, ni branche.
- **Un projet** (`Listed`) : son nom, sa couleur et son logo (une image PNG, JPEG ou WebP en `data:`, jamais un SVG), proposés et changeables sur le poste ; ses dépôts (`Repo { url, folder }`, un dossier chacun pour un projet à plusieurs dépôts) ; sa branche par défaut ; qui y a accès (`Access::Team` ou des membres).
- **Qui écrit** : un administrateur. Le client refuse l'écriture d'un membre (`CryptoError::NotAdmin`) ; le serveur, qui connaît les rôles, refuse une entrée d'un membre dans ce flux (403, défense en profondeur) ; chaque appareil refuse à la lecture une entrée scellée par un membre qui n'était pas administrateur à son époque (ou, pour une époque qu'il n'a pas vécue, qui ne l'est pas aujourd'hui). `serve` : un appareil d'administrateur réécrit les projets d'un administrateur parti, pour un appareil venu après. Un nouvel administrateur n'écrit qu'une fois son rôle porté au serveur par le `serve` d'un administrateur.
- **Les adresses** (`address`) : `https://` et ssh (`ssh://`, `user@hôte:chemin`) seulement. Refusés : `file://`, `http://`, `git://`, les transports `ext::` et autres, une adresse qui commence par un tiret ou dont une partie du chemin en commence un (une option de git), un chemin local (`C:\`, `/`, `./`, `~`, `\\serveur`), un identifiant dans l'adresse (`user:mot@`, un jeton en nom d'utilisateur sur https), un espace ou un caractère de contrôle. Un projet dont une adresse, un dossier, une branche ou un logo ne passe pas est refusé entier, à l'écriture comme à la lecture (`Refused::Bad`).
- **La reconnaissance d'un clone** (`key`, `Catalog::holding`) : l'hôte en minuscules, sans port ni utilisateur, et le chemin sans `/` autour ni `.git` ; https et ssh sont la même adresse. Sur un hôte connu (GitHub, GitLab, Bitbucket, Azure DevOps, Codeberg), le chemin aussi en minuscules ; ailleurs sa casse compte.
- **Le serveur qui ment** : comme le fichier d'équipe, la dernière entrée lue est relue avant de lire plus loin (`Alarm::Rollback`, `Rewritten`, `Server`, `Fork`) ; sous l'alarme, rien n'est écrit ni lu jusqu'à `retry`.
- L'état (la liste lue, les refus, le curseur) va dans la base chiffrée de l'équipe (`Team::save_extra`, `extra/catalog`).

Le cœur (`src-tauri/src/teams_catalog.rs`, `team_projects.rs`) lit la liste à chaque lecture de l'équipe, la met près des projets du poste (les adresses de leurs remotes, telles que configurées, avant tout `insteadOf`), et clone : `git clone --origin origin [--branch b] -- <adresse> <dossier>`, l'adresse relue depuis la liste et revérifiée, par les identifiants git du membre (aucun ne passe par le serveur d'équipe). Un refus d'accès de git devient « Demande l'accès à acme/api sur GitHub » ; un projet à plusieurs dépôts les clone tous et dit lequel manque.

Les tests (`tests/catalog.rs`, contre le vrai serveur, sur SQLite puis PostgreSQL) : un administrateur ajoute, change et retire, un membre lit, et le disque du serveur ne garde ni nom, ni adresse, ni branche, ni hôte ; un membre refusé par le client, par le serveur (403) et à la lecture ; une adresse dangereuse refusée à l'écriture et, signée quand même, à la lecture ; un clone reconnu sous sept formes de son adresse ; les projets d'un administrateur parti réécrits pour un appareil venu après ; un serveur qui cache ou change une entrée. Le cœur (`team_projects_tests.rs`) : la reconnaissance près des projets du poste, le clone depuis un dépôt local (`insteadOf`), un projet à plusieurs dépôts dont un manque, les refus d'accès de git, une adresse dangereuse qui n'atteint jamais git. L'e2e `tests/e2e/team-projects.spec.ts` (serveur sur `ESCOUADE_PORT_BASE + 4`) : deux clients, le clone par l'assistant.

## Rattacher à l'équipe (TS-7)

Les pages de spec sont « Le Kanban d'équipe » (« Rattacher un Kanban à l'équipe », « Verser à l'équipe »), « Les projets de l'équipe » et « .escouade.yml » (« La clé `team` »). Choix C5-A : pas de passerelle `.yml`.

- **Le Kanban dans la liste** : `Listed::board` (`catalog.rs`), l'id hex du journal du Kanban, chiffré avec l'entrée. Vide pour une entrée d'avant (`#[serde(default)]` : elle se décode sans le champ, testé) ; `check` refuse ce qui n'est pas 32 chiffres hexadécimaux ; `Listed::board_id`.
- **Les propositions** (`proposals.rs`) : le flux `PROPOSAL_STREAM` (`b"escouade/propose"`), ouvert à toute l'équipe. `Entry::Board { item, board, at }`, scellée sous la clé d'équipe, signée par la feuille de l'auteur ; refusée à la lecture si aucune feuille de l'équipe ne l'a signée. `propose`, `sync` (la dernière par projet de la liste), `view`. Un administrateur l'applique en écrivant l'entrée (`catalog::put`) ; rien ne s'applique seul.
- **Le serveur** (`escouade-team-server`, `api::TEAM_STREAMS`) refuse `PUT /access` sur les flux de l'équipe (liste, fichier d'équipe, propositions) : un membre ne peut pas en fermer l'accès aux autres, même avant leur première écriture.
- **Le cœur** : `src-tauri/src/teamjoin.rs` (rattacher, détacher, la clé `team` du `.escouade.yml`, inscrire ou proposer un Kanban, rejoindre celui de la liste à la lecture suivante de l'équipe), `src-tauri/src/teampour.rs` (l'aperçu et « Verser à l'équipe » : l'objet d'un ticket de fichier est `SHA-256("escouade/ticket-file/" + id)[..16]`, le même partout ; jamais réécrit s'il est dans le journal ou supprimé), `sharing::freeze` (la ligne « Frozen » en tête des `.yml`, une fois) ; les docs de TS-9 : `teampour::PourDocs` dans l'aperçu, et `teamdocs` qui n'importe plus seul au premier passage (tant que l'équipe n'a ni choisi ni importé, `Core::team_docs_on` est faux et les docs restent celles du dépôt ; `team_docs_state`) ; `projectfile::team_hint` (l'adresse vérifiée par `teams::address`, le nom nettoyé, jamais joint). Les anciennes clés sont le champ `formerKeys` des tickets.

Les tests : `tests/catalog.rs` (une entrée sans Kanban puis avec, une proposition d'un membre lue et inscrite par un administrateur, une proposition forgée refusée, le disque du serveur sans le Kanban sous SQLite, les flux de l'équipe qu'un membre ne restreint pas) ; le cœur, contre le vrai serveur : `teampour_tests.rs` (deux postes qui versent en même temps puis une seconde fois, sans doublon, clés communes et anciennes clés, fichiers figés et lus par le lecteur d'ESC-190 ; le Kanban de la liste rejoint au rattachement, un Kanban de membre proposé puis inscrit, la clé `team` écrite puis retirée), `projectfile_tests.rs` (adresses, ids et noms piégés, un fichier changé entre deux lectures) ; les e2e `tests/e2e/team-attach.spec.ts` (deux clients : rattacher et verser, puis cloner et voir le Kanban) et `team-hint.spec.ts` (la bande, avec un serveur qui compte ses requêtes : aucune).

## Les docs d'un projet d'équipe (TS-9)

Choix tranchés (§15.2) : C2-A (les pages en versions entières, fusion à trois voies sur le poste, un vrai conflit à l'agent rédacteur), C4-B (au choix par projet, le serveur par défaut), C5-A (pas de passerelle). La page de spec est « Les docs d'un projet d'équipe ». Le cœur (`src-tauri/src/teamdocs.rs`) tient la copie des docs sur le poste et ce qui les relie au journal ; la crate lui donne :

- **Les versions d'une page** (`OpBody::Version`, TS-5) : `Replica::versioned` (les documents qui en ont), `history` (toutes leurs versions), `common_base` (la plus récente version dont deux versions viennent toutes deux, en remontant leurs bases) ; `merge::merge3` fusionne à trois voies. Le cœur fusionne les têtes dans l'ordre de leurs horodatages, la plus ancienne d'abord : chaque copie fusionne pareil, et deux fusions écrites en même temps ont le même texte (des têtes au même contenu ne sont pas un conflit).
- **Le compteur des numéros** : `OpBody::Number { object, counter, at }` (`ops.rs` d'`escouade-team-crypto`), écrit par `Team::take_number`. Le numéro n'est pas dans l'opération : c'est sa place parmi les demandes du compteur dans l'ordre du serveur, après le plus haut donné avant (`at` le donne tel quel quand il est libre : un import). `Replica::assign_numbers` ne le donne qu'aux demandes dont le numéro du flux est déjà lu, et tout ce qui le précède aussi (`take_project_entry`, après chaque entrée) : une opération de ce poste envoyée mais pas relue attend (`number_waits`). Les numéros donnés sont dans l'état (`numbers`), donc dans un instantané ; `rebuild` les redonne. Deux acceptations au même moment, sur deux postes, ont deux numéros, les mêmes partout ; le serveur ne voit ni le compteur ni les numéros.
- **Les documents** : `Team::put_document` scelle les octets sous une clé tirée sur le poste (XChaCha20-Poly1305, les données authentifiées liées à l'id du fichier), puis les envoie comme un fichier de l'équipe ; la clé, l'id, la taille et l'empreinte vont dans le journal, sous la clé du projet de l'époque. `get_document` les rouvre. Un membre parti, ou hors d'un projet gardé, ne lit pas la clé d'un document écrit après.
- **Hors ligne** : `Team::sync` dit « hors ligne » quand le serveur ne répond pas (pas seulement la lecture d'un projet).

Les tests (`tests/docs.rs`, contre le vrai serveur, sur SQLite puis PostgreSQL) : une page écrite ici lue là, et le disque du serveur sans son chemin, son titre ni son texte ; une page écrite des deux côtés, sa base retrouvée et la fusion écrite après les deux têtes ; trois acceptations au même moment (1, 2, 3, les mêmes sur chaque copie, une demande refaite qui ne change rien) ; une acceptation hors ligne qui attend et prend le numéro d'après ; des numéros gardés tels quels (un import), puis un instantané et un appareil venu après qui donne le suivant, et une reconstruction qui donne pareil. Le cœur (`src-tauri/src/teamdocs_tests.rs`, deux dossiers de données contre le vrai serveur) : page, décision et document lus de l'autre côté, ce que garde le serveur, fusion et conflit montré puis réglé, champ d'une décision perdu, deux décisions acceptées serveur coupé, brouillon d'un ticket invisible puis versé, hors ligne puis retour, import du dépôt, « dans le dépôt » et retour, membre retiré ; `mcp/teamdocs_tests.rs` : les outils des docs et la marque de donnée de tiers. L'e2e `tests/e2e/team-docs.spec.ts` (serveur sur `ESCOUADE_PORT_BASE + 6`) : deux clients, une page et une décision lues de l'autre côté, puis un conflit montré.

## La mémoire d'équipe (TS-10)

Choix tranché (§15.2) : C14-B (la validation des souvenirs d'agent est un réglage de l'équipe, allumé d'office, imposable). La page de spec est « La mémoire d'équipe ». Le cœur (`src-tauri/src/teammemory.rs`) étend le passage des docs de TS-9 (même journal, même choix, même file d'envoi) ; la crate lui donne :

- **Un objet par écriture** (genre `memory`, id tiré sur le poste) : `name`, `description`, `type`, `body` et `provenance` (membre, appareil, nom dit par le poste, date, `hand` ou `agent` avec ticket, harness et modèle, données de tiers lues, import), écrits dans un `Create` ; puis, par d'autres, `accepted` (l'empreinte du contenu accepté), `rejected`, `removed` (la pierre tombale). Les quarantaines sont des objets `memory-rule` (`rule` : ticket, membre, appareil, depuis), levés par `lifted`.
- **La provenance tient à la signature** : l'horodatage d'un champ porte l'appareil qui l'a écrit, vérifié égal à l'auteur de l'opération (`ops.rs`), dont la signature est vérifiée à la lecture. Le cœur refuse un souvenir dont un champ du contenu vient d'un autre appareil que celui de `provenance`, ou dont l'appareil n'est pas du membre qu'il nomme. `Replica::author_member` donne le membre d'un appareil d'après une opération du journal (pour un appareil révoqué depuis, ou absent du registre) ; le cœur garde ce qu'il a vu, au-delà d'un instantané.
- **Les droits, lus du registre** : une acceptation ou un écart compte de l'auteur ou d'un administrateur, une quarantaine d'un administrateur seul ; le serveur ne voit que des opérations chiffrées.

Les tests du cœur (`src-tauri/src/teammemory_tests.rs`, deux dossiers de données contre le vrai serveur) : une proposition lue des seuls agents de son auteur, puis gardée et lue de l'autre poste avec sa provenance, et le disque du serveur sans nom ni texte ; un écart, une adresse inconnue, un souvenir gardé qui reste une donnée après des données de tiers ; la validation éteinte, puis imposée par l'équipe ; une pierre tombale qui va partout, l'historique au journal ; une quarantaine posée hors ligne, qui tient, puis va à l'autre poste à son retour et prend ce qu'il a écrit hors ligne, puis levée ; une provenance qui ne tient pas à sa signature, refusée ; un secret refusé ; l'import du dépôt par le geste. `mcp/teammemory_tests.rs` : la provenance, les marques et l'avis, la session marquée après une donnée de tiers, aucun outil qui valide. L'e2e `tests/e2e/team-memory.spec.ts` (serveur sur `ESCOUADE_PORT_BASE + 6`) : deux clients et le faux `claude`.

## Le coupe-circuit (TS-15)

`breaker.rs` tient les coupe-circuits de l'équipe (docs/TEAM-SERVER.md §9). La page de spec est « Le coupe-circuit de l'administrateur ».

- **Où** : le flux `BREAKER_STREAM` (`b"escouade/breaker"`), comme la liste des projets : chaque entrée est un `SealedObject` sous la clé d'équipe de son époque, signé par la feuille de son auteur, avec un id d'objet tiré à chaque fois. `Entry::Set { id, serial, scope, what, reason, by, at }` pose un coupe-circuit (ou le pose de nouveau, entier) ; `Entry::Lift { id, serial, by, at }` le lève. La portée (`Scope::Team`, ou `Scope::Project` par l'id hex du journal du Kanban du projet) et ce qui est retiré (`What::Plugin { id, uninstall }`, `Harness`, `Model`, `Agents`) sont vérifiés (`check`) à l'écriture et à la lecture.
- **Qui écrit** : un administrateur. Le client refuse d'abord (`CryptoError::NotAdmin`) ; le serveur refuse une entrée d'un membre dans ce flux (`api::ADMIN_STREAMS`, 403), et personne ne restreint son accès (`api::TEAM_STREAMS`) ; chaque appareil refuse à la lecture une entrée qui n'est pas d'un administrateur de son époque (`Refused::NotAdmin`) ou dont le sceau ou la signature ne tient pas (`Refused::Unsigned`).
- **Le `serial`** : une entrée dont le `serial` n'est pas au-dessus du plus haut lu est refusée (`Refused::Replayed`), une levée de rien aussi (`Refused::Unknown`) ; une entrée refusée ne prend pas le `serial`. `set` et `lift` écrivent au-dessus du plus haut lu ; si un autre administrateur a pris ce `serial` d'abord, ils réécrivent au-dessus (trois fois au plus).
- **`serve`** : un appareil d'administrateur réécrit les coupe-circuits en vigueur d'un administrateur parti, pour un appareil venu après.
- **Le serveur menteur** : comme la liste, la dernière entrée lue est relue avant de lire plus loin (`Alarm::Rollback`, `Rewritten`, `Server`, `Fork`) ; sous l'alarme, rien n'est lu ni écrit jusqu'à `retry`, et ce qui était en vigueur le reste.
- **`online`** : les autres appareils qui ont une connexion en direct ouverte (le champ `online` de `/v1/devices`, `Hub::is_live` du serveur), pour la confirmation « sur N postes en ligne ».
- L'état (en vigueur, levés, refus, plus haut `serial`, curseur) va dans la base chiffrée de l'équipe (`Team::save_extra`, `extra/breaker`).

Le cœur (`src-tauri/src/teams_breakers.rs`, `team_breakers.rs`) le lit en premier à chaque lecture de l'équipe et l'applique par ses verrous : `LockedBy::Breaker`, des `denies` sur `harnesses.allowed`, `harnesses.allowedModels` et `plugins.denied`, `Locks::halt` pour tous les agents, `Locks::for_project` pour un coupe-circuit d'un projet. Les tests (`tests/breaker.rs`, contre le vrai serveur, sur SQLite puis PostgreSQL) : pose et levée lues d'un membre, et le disque du serveur sans ce qui est retiré ni la raison ; un membre refusé par le client, le serveur et à la lecture ; une signature forgée, des octets qui ne sont pas un objet, un `serial` rejoué ; deux administrateurs en même temps ; un administrateur parti, réécrit pour un appareil venu après ; un serveur qui cache une entrée ; le compte des appareils en ligne. Ceux du cœur : `src-tauri/src/team_breakers_tests.rs` et `team_breakers_live_tests.rs` (deux dossiers de données contre le vrai serveur : l'application dans la seconde, mesurée ; tous les agents ; hors ligne puis au retour ; « sans contact depuis N heures » ; un agent perso jamais arrêté ; un coupe-circuit d'un projet).

## Passer un agent (TS-17)

`handoff.rs`, sur les prises de `team/claims.rs`.

- **La prise passée** : `Team::hand_claim` la donne à un appareil du projet sous un jeton neuf (le serveur refuse `not_in_project` un appareil révoqué, sans accès, ou soi-même) ; `Claim::handed_by` dit qui l'a passée tant qu'elle n'est pas prise ; `Team::withdraw_claim` la reprend tant que le destinataire ne l'a pas prise (`claim_lost` ensuite). Le bail d'une prise passée est de 14 jours (`Limits::hand_lease`).
- **L'offre** : un objet `handoff` du journal du projet (`Offer` : le ticket, sa prise, de quel appareil à quel autre, la branche et son dépôt distant, patch ou non, le harness). Elle s'efface à la reprise et à l'abandon (`end_handoff`). Ne la croire qu'avec la prise : celle que le serveur dit passée à cet appareil, par l'appareil qu'elle nomme.
- **Le dépôt** : un fichier du serveur scellé comme un objet du projet (`Team::deposit`), la conversation et le patch ; pour un agent de Claude Code passé entre deux tours, sa session aussi (`SessionCopy` : son id, le worktree où elle a tourné, la version de `claude`, son fichier tel quel ; TS-17b, C16-B), que l'offre annonce (`Offer::session`, absent des offres d'avant : faux). Effacé avec l'offre (`delete_file`). La réécriture et la reprise sont au cœur (`src-tauri/src/sessioncopy.rs`).
- **Les machines** : `set_machine` écrit `machine: true` sur l'objet `person` de l'appareil, `machines` les lit.

## Les coûts de l'équipe (TS-16)

Choix tranché (§15.2) : C15, un réglage de l'équipe, « par projet » d'office. La page de spec est « Les coûts et les quotas de l'équipe ». `costs.rs` tient ce que la crate fait des coûts ; le cœur (`src-tauri/src/teamcosts.rs`, `teams_costs.rs`) fait les totaux depuis les Stats et pèse les budgets.

- **Ce qui part** (`Totals`, `Day`, `Row`, `TicketRow`) : des sommes par jour, par projet (l'id du journal), harness, modèle et forfait ; le total de chaque ticket par sa clé. `deny_unknown_fields` partout : un champ de plus ne passe ni à l'envoi ni à la lecture. Le membre n'y est que « par membre » (`member`).
- **La clé des administrateurs** (`escouade-team-crypto/src/costkey.rs`, `CostKey`, un `EpochKey` de l'usage `Costs` depuis AU-3 : mêmes étiquettes, mêmes octets) : une paire HPKE à part. Chaque appareil d'administrateur tire sa propre clé des coûts (dans l'état chiffré, `extra/costs`) et la publie (`KeyEntry::Device`) ; un appareil d'administrateur tire la paire de l'époque (`KeyEntry::Key`) et l'enveloppe pour chaque appareil d'administrateur dont il a lu la clé, puis pour ceux qui viennent après (`KeyEntry::Wrap`). Les données authentifiées lient un chiffré à l'équipe, à l'époque de la clé et à l'appareil (l'auteur des totaux, ou le destinataire de l'enveloppe).
- **Les flux** : `KEY_STREAM` (`b"escouade/costkey"`, un administrateur seul y écrit : le serveur refuse un membre, `api.rs`, et chaque appareil refuse à la lecture une entrée qui n'est pas d'un administrateur de son époque) et `TOTALS_STREAM` (`b"escouade/costsum"`, ouvert à tous). Les deux sont dans `api::TEAM_STREAMS` : personne n'en restreint l'accès. Le premier `Key` d'une époque dans l'ordre du flux gagne ; la dernière entrée lue de la clé est relue avant de lire plus loin (`Alarm::Rollback`, `Rewritten`).
- **La rotation** : une clé dont un appareil enveloppé n'est plus un appareil d'administrateur du registre est « périmée » (`View::stale`) : aucun poste n'y scelle, un appareil d'administrateur en tire une nouvelle à son passage. `wants_month` dit au cœur de renvoyer tout le mois à une nouvelle clé.
- **La file** : `queue` met des jours entiers en attente (un jour remplace celui qui attendait), `send` les scelle à la clé en cours et les envoie ; hors ligne ou sans clé, ils attendent. L'appareil d'un administrateur garde, par appareil et par jour, l'envoi le plus récent (son heure, puis son numéro) : un envoi refait compte une fois.
- **Le résumé du mois** (`KeyEntry::Summary`) : le total de chaque projet et de l'équipe, jamais un membre, que l'appareil d'un administrateur publie sous la clé d'équipe quand il change : chaque membre pèse ses budgets avec.
- `read` (les flux, et pour un administrateur les totaux et son service), `send`, `sync` (les deux), `view`, `retry`.
- **Le flux de la clé est commun avec l'audit des postes** (AU-3) : `keyring.rs` tient la lecture (`read_keys`, qui juge chaque entrée par la règle de l'usage), le service d'un détenteur (`serve` : sa clé publiée, une paire tirée quand il n'y en a pas ou qu'elle est périmée, les enveloppes manquantes) et la péremption (`is_stale`). L'état des coûts le garde aplati (`#[serde(flatten)] ring`), sous les noms de TS-16 : un état écrit avant se relit entier (`a_state_written_before_the_ring_reads_back_whole`). Piège : sous `flatten`, les clés numériques d'une table reviennent en chaînes (`"1"`) ; `keyring::by_epoch` les relit.

Les tests (`tests/costs.rs`, contre le vrai serveur, sur SQLite puis PostgreSQL) : les champs de ce qui part, et rien d'autre ; un membre qui envoie, l'administrateur qui lit, un autre membre qui ne lit que le résumé, et le disque du serveur sans modèle, projet, jour ni clé de ticket ; un membre refusé dans le flux de la clé ; « par membre » ; la rotation au départ d'un administrateur (les totaux attendent, puis vont à la nouvelle clé, et le mois est redemandé) ; hors ligne, puis une réponse perdue, comptée une fois. Le cœur : `src-tauri/src/teamcosts_tests.rs` (deux dossiers de données contre le vrai serveur). L'e2e `tests/e2e/team-costs.spec.ts` (serveur sur `ESCOUADE_PORT_BASE + 6`).

## L'audit des postes (AU-3)

La conception est `docs/TEAM-AUDIT.md` (§3.3, S3) ; la page de spec « L'audit vers un SIEM ». `audit.rs` est le modèle des coûts appliqué aux faits des postes : ce qu'un poste dit de lui-même (un refus du cœur, une autorisation donnée, une commande approuvée, un plugin, une clé sensible, une validation, une consigne, le coupe-circuit appliqué), scellé pour les seuls appareils qui émettent l'audit. Rien ne l'appelle encore : AU-4 (ESC-612) y branche les faits du poste et ce que voit le membre, AU-5 (ESC-614) la machine d'audit qui les prend et les émet. Aucun outil du MCP n'y lit ni n'y dépose.

- **Ce qui part** (`StationFact`) : l'id du fait (32 chiffres hexadécimaux, tiré par le poste), l'heure du poste (`at`, ms), l'événement et sa cible et son détail, les structures fermées d'`escouade-audit` (AU-1). Seuls les événements de `escouade_audit::event::STATION_EVENTS` (ceux que le §1 donne à S3 ; un test les recoupe avec le document) passent. **Jamais d'acteur** : l'émetteur le prend de la signature de l'entrée (le membre et l'appareil), un poste ne parle pas pour un autre. `deny_unknown_fields` sur le fait et le lot ; les parties sont relues comme un événement de la liste à la file (`queue`) et à l'ouverture (`StationFact::fact`).
- **Les émetteurs** (`emitters`) : les appareils des machines de l'équipe (rôle de service, OE-1) quand elle en a une, sinon ceux des administrateurs (le recours d'A2-A). À la lecture, une entrée du flux de la clé n'est prise que si son signataire émettait à son époque, d'après les feuilles et les rôles de cette époque (`Team::leaves_at`, `admins_at`, `is_machine`) : un administrateur ne glisse pas sa paire quand une machine émet.
- **La clé** (`escouade-team-crypto/src/auditkey.rs`, `AuditKey`) : une paire HPKE par époque, ses propres étiquettes (`escouade/audit-facts/v1`, `escouade/audit-key-wrap/v1`), enveloppée pour les émetteurs seuls par `keyring.rs` comme celle des coûts. Elle est périmée dès qu'un appareil pour qui elle est enveloppée n'émet plus (une machine retirée, un administrateur parti, ou une machine venue : les administrateurs passent la main) ; un émetteur en tire une nouvelle, au-dessus de la plus haute époque nommée dans le flux, même par une entrée refusée (`Ring::top` : une machine arrivée après ne prend pas la paire des administrateurs, mais ne rejoue pas son numéro).
- **Les flux** : `KEY_STREAM` (`b"escouade/audit/k"`, écrit par un administrateur ou une machine : le serveur refuse un membre) et `FACTS_STREAM` (`b"escouade/audit/f"`, le flux `escouade/audit` de la conception : chaque appareil y dépose ; le serveur ne le donne qu'aux administrateurs et aux machines, par le flux, le direct ou un fichier, et n'y prend aucun fichier). Les deux sont dans `api::TEAM_STREAMS`.
- **La file hors ligne** : `queue` met le fait au bout de la file (base chiffrée de l'équipe, `extra/audit`) ; `send` les scelle par lots (64 faits, 256 Kio au plus) à la clé en cours ; hors ligne, sans clé ou clé périmée, ils attendent dans leur ordre. Au-delà de `MAX_QUEUE` (10 000), les plus vieux tombent et le lot suivant dit combien (`Taken::dropped`). Un lot envoyé deux fois (une réponse perdue) est reconnu par les ids : l'émetteur prend chaque fait une fois.
- **L'émetteur** : `read` (le flux de la clé, et pour un émetteur son service), `take` (les faits depuis sa dernière lecture, chacun avec son appareil et son membre, `at` et `seen`, l'heure où le serveur l'a reçu, en ms : le serveur la garde à la seconde). Un lot scellé à la clé en cours dont l'enveloppe n'est pas encore là arrête la lecture d'un émetteur ; un autre appareil le compte illisible (`unreadable`). `view` dit l'époque, `holds`, `emitter`, `stale`, ce qui attend, les 50 derniers faits partis.

Les tests (`tests/audit.rs`, contre le vrai serveur, sur SQLite puis PostgreSQL) : un membre qui dépose, l'administrateur qui prend le fait avec son acteur, `at` et `seen`, une fois ; le serveur qui refuse le flux des faits à un membre (les siens compris) et le flux de la clé à son écriture ; le disque du serveur sans clé de réglage ni nom d'événement ; une machine qui vient (la clé des administrateurs périmée, les faits qui attendent, la paire de la machine seule, l'administrateur qui n'ouvre rien) ; la rotation au départ d'un administrateur émetteur ; hors ligne, puis une réponse perdue, chaque fait pris une fois et dans l'ordre. Les tests unitaires d'`audit.rs` : les champs d'un fait et son acteur écarté, les seuls événements d'un poste, les émetteurs, la file bornée et ce qui en tombe, le lot borné. Côté serveur, `crates/escouade-team-server/tests/stations.rs`.

## Le store de plugins privés (TS-14)

Choix tranché (§15.2) : C13-A, une paire à part, hors ligne, avec une clé de secours. La page de spec est « Le store de plugins privés ». `signing.rs` tient la clé de l'équipe, `plugins.rs` le flux du store ; les contrôles sont ceux d'`escouade-plugin` (`crates/escouade-plugin-cli/src/team.rs`), le cœur (`src-tauri/src/teamplugins.rs`, `teams_plugins.rs`) les joue et garde la clé scellée.

- **La clé de l'équipe** (`signing.rs`, `SigningKey`) : une paire minisign (Ed25519, la forme pré-hachée `ED`, BLAKE2b-512, que `minisign-verify` et `minisign` lisent), tirée sur le poste d'un administrateur. Sa moitié secrète n'est jamais écrite en clair : `seal` la scelle par un mot de passe (12 caractères au moins ; PBKDF2-HMAC-SHA256, 600 000 tours, un sel à elle ; XChaCha20-Poly1305, l'id de la clé en données associées), `open` la relit. Deux paires : la clé de tous les jours (gardée scellée par le cœur dans `<données>/team-signing/<équipe>/`) et la clé de secours (écrite dans le dossier que choisit l'administrateur, à garder ailleurs).
- **Le flux** `PLUGINS_STREAM` (`b"escouade/plugins"`, dans `api::TEAM_STREAMS` : personne n'en restreint l'accès). Ses entrées, scellées et signées par la feuille de leur auteur :
  - `Key` (un administrateur) : les deux clés publiques et **l'espace de noms** (`fr.acme.*`). C'est la déclaration au journal de l'équipe (§3.7) : chaque appareil refuse une entrée d'un membre qui n'était pas administrateur à son époque, et un espace qui n'en est pas un (`namespace` : un domaine à l'envers de deux parties au moins, jamais `dev.escouade.*` ni un espace qui le contient, `dev.*`).
  - `Submit` (tout membre) : une version proposée, ses contrôles passés sur le poste de son auteur ; son archive et son module sont des fichiers chiffrés du serveur (`Team::put_file`, la clé d'équipe de l'époque), son empreinte (`digest`) celle que l'administrateur doit retrouver.
  - `Publish` (un administrateur) : l'énoncé (`Statement` : l'id, la version, l'étiquette, l'espace, l'empreinte de l'équipe, les SHA-256 de l'archive et du module, l'empreinte des contrôles) signé par la clé de l'équipe, le commentaire de confiance `escouade-team-v1 <id>@<version> <espace>` (jamais celui du store public). Chaque appareil le vérifie à la lecture (`statement_signed`) : une clé de confiance, l'id dans l'espace, jamais `dev.escouade.*`, cette équipe-ci.
  - `Revocations` (un administrateur) : la liste de révocation de l'équipe, au format de `revocations.json`, signée par une clé de confiance avec le commentaire du store (`escouade-revocations-v1 <serial> …`) ; son `serial` ne recule jamais (plus bas, ou le même avec un autre contenu : refusé, `Refused::Older`). Une liste qui révoque la clé qui la signe est refusée ; `keys_revoked` retire une clé des clés de confiance (`Store::trusted_keys`) : la clé de secours prend la suite.
  - `Dismiss` (un administrateur, ou l'auteur) : une proposition écartée.
- **La publication** : `submit` (refuse hors de l'espace, ou une archive qui n'est pas celle contrôlée), `publish` (l'empreinte trouvée par l'administrateur doit être celle de la proposition, sinon rien n'est signé ; une signature par une clé qui n'est pas de confiance n'est pas envoyée), `revoke` (le `serial` au-delà du dernier), `files` (l'archive et le module rouverts, vérifiés par leurs SHA-256).
- **Le serveur qui ment** : comme la liste des projets, la dernière entrée lue est relue avant de lire plus loin (`Alarm::Rollback`, `Rewritten`).

Les tests (`tests/plugins.rs`, contre le vrai serveur, sur SQLite puis PostgreSQL) : la clé déclarée par un administrateur seul, jamais `dev.escouade.*` ; une proposition hors de l'espace refusée ; une empreinte qui diffère, ou une clé non déclarée, ne publient rien ; un membre ne publie pas ; le membre lit la version et rouvre son archive ; le disque du serveur sans id ni archive ; une clé, une version signée d'une autre clé et une version hors de l'espace glissées dans le flux, refusées à la lecture ; la liste qui ne recule pas, la clé de secours qui révoque la clé de tous les jours. Les contrôles : `crates/escouade-plugin-cli/src/team.rs` (la double compilation, dont une qui diffère). Le cœur : `src-tauri/src/teamplugins_tests.rs`. L'e2e : `tests/e2e/team-plugins.spec.ts` (serveur sur `ESCOUADE_PORT_BASE + 5`).

## Ce qui reste

- **TS-3** (fait, ESC-461), sauf :
  - une invitation que seul l'appareil qui l'a faite honore : un autre administrateur en ligne ne la connaît pas (il faudrait la sceller pour les seuls administrateurs) ;
  - `escouade-server team pair` pour une machine en mode serveur.
- **TS-13** (fait, ESC-471), sauf : le fichier d’un projet d’équipe par-dessus celui de l’équipe (il passera par le groupe du projet gardé de TS-5) ; un serveur qui cache une version à un appareil qui ne l’a jamais vue (à croiser avec les balises de tête) ; le flux des réglages pourrait passer par le journal d’opérations de TS-5 (copie locale, direct) plutôt que par une lecture à chaque relecture.
- **TS-4** : montrer l'équipe dans les espaces et ses réglages (membres, appareils, rôles, état du trousseau, hors ligne), retirer un appareil de soi qu'on oublie.
- **TS-5** (fait, ESC-466, avec les projets d'une invitation et le direct dans le cœur de TS-3), sauf :
  - les écrans (« Hors ligne · N changements en attente », les conflits, le Kanban) : TS-4 et TS-6 ; la prise d'un ticket : TS-6 ; la fusion à trois voies des pages est faite (TS-9) ;
  - les droits par projet sur le serveur : faits par TS-6 ;
  - l'écriture d'un lien d'historique manquant d'un groupe de projet par un autre appareil que son committeur (pour l'équipe, c'est fait) ;
  - la liste chiffrée des projets de l'équipe porte l'id du journal du Kanban d'un projet (TS-7, ESC-493) ;
- **TS-8** (fait, ESC-480), sauf : la barre des projets par espace (avec les espaces d'Escouade à distance) ; un serveur qui cache une entrée de la liste à un appareil qui ne l'a jamais vue (comme pour TS-13) ; le choix des membres d'un accès par leur nom (l'annuaire chiffré est TS-4) ; l'accès d'un projet de la liste n'est pas encore celui de son groupe MLS (TS-7).
  - `write_object` et `read_objects` (l'objet scellé de TS-2) restent pour les fichiers ; les projets passent par les opérations.
- **TS-9** (fait, ESC-482), sauf : le nom de l'auteur d'une version dans un conflit (l'annuaire chiffré, TS-4) ; un document de plus de 20 Mo ne part pas ; les écarts des comparaisons au code restent par poste.
- **TS-7** (fait, ESC-493), sauf : un poste qui suit un autre Kanban que celui de la liste n'en change pas seul (l'écran le dit) ; deux postes qui versent le même ticket en même temps consomment deux numéros du compteur.
- **TS-6** (fait, ESC-479), sauf : passer l'agent d'un ticket interrompu (TS-17) ; l'annuaire des noms de toute l'équipe (TS-4 : ici, chaque projet porte les noms de ses appareils) ; un administrateur qui rend la prise d'un autre ; la règle de colonne d'un ticket pris est tenue par les clients (le serveur ne voit pas les déplacements).
- **TS-14** (fait, ESC-579), sauf : une nouvelle version publiée n'est pas proposée comme mise à jour dans « Mises à jour » (la page « Équipe » des Plugins l'offre) ; un fichier du serveur plus gros que sa borne (`--max-file`, 25 Mo d'office) ne part pas, même sous les 50 Mo de l'archive.
- **TS-18** (fait, ESC-519) : « Le fuzzing » ci-dessus ; l'e2e à deux clients tient dans la CI (la part « team server », `docs/TEAM-SERVER.md` §14). Reste : la révocation d'un appareil dans un e2e, quand l'app aura son écran (TS-4).
- **L'audit externe (TS-20)** : les points 1 à 10 du README de la maquette. Pour cette crate, en plus :
  - le format du stockage (les clés en clair, dont l'id du groupe et les époques) ;
  - l'ordre état puis trousseau du point de contrôle ;
  - le repli sans trousseau, où le point de contrôle est sur le même disque ;
  - le fait qu'une alerte de fourche bloque la lecture du flux jusqu'à ce qu'un humain s'en occupe ;
  - le jeton du serveur dérivé du secret de l'invitation, et la demande de jeton d'un membre dans le flux ;
  - le code court : 25 bits de mot de passe, un essai par créneau, SPAKE2 sur Ed25519 sans liaison au créneau ;
  - la graine de la clé de séquestre montrée une fois, en clair, à l'administrateur qui fait la paire.
  - TS-5 : la copie locale en clair sur le disque ; la confiance faite à un membre pour un instantané (il remplace le début du journal) et pour son horodatage (une horloge en avance gagne) ; les règles d'un groupe de projet évaluées contre la vue de l'équipe du vérificateur ; la tête des membres d'une opération d'une époque que l'appareil ne tient pas, acceptée sans contrôle (`HeadCheck::Unknown`).
