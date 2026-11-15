# escouade-team : le client du serveur d'équipe (TS-2)

Tickets ESC-458 (TS-2) et ESC-461 (TS-3) de `docs/TEAM-SERVER.md` §16. L'équipe est un groupe MLS (C9-B). Cette crate est le client : elle tient un appareil dans une équipe, avec ses clés au trousseau, son état sur le disque et son serveur, et fait entrer dans l'équipe (création, invitation, autre appareil, récupération). Le cœur la branche (`src-tauri/src/teams.rs`) : le `SecretStore` de l'app pour trousseau, son client HTTP (proxy et certificats des réglages), le dossier `teams` des données. Elle ne synchronise pas encore les opérations des projets (TS-5).

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
| `pairing.rs` | Le code court de 8 caractères (TS-3) : SPAKE2 sur le relais du serveur, un seul essai |
| `escrow.rs` | La clé de séquestre de l'organisation en texte : sa moitié publique, son secret |
| `wire.rs` | Les corps CBOR du serveur, et `TeamMessage`, ce que le client écrit dans le flux d'équipe à côté des commits (liens, enveloppes, balises, demandes, Welcome) |
| `fuzzing.rs` | Le corps des cibles de fuzzing (`fuzz/`) |

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

- `Team::invite_for(objet, maintenant, durée)` : 72 h par défaut, un seul usage. Le lien `https://<serveur>/join/<id>#s=<secret>&t=<empreinte>` (et `&m=<membre>` pour un autre appareil d'un membre).
- **Le jeton du serveur est dérivé du secret** (`invite::server_token`, HKDF) : l'administrateur en enregistre le SHA-256 auprès du serveur (`POST /v1/devices/enrollments {token_hash}`), qui ne voit jamais le secret. Le lien seul suffit donc à l'invité (`Team::join_link`).
- **Usage unique, des deux côtés** : le serveur ne laisse servir le jeton qu'une fois (un second usage : 409 `token_used`, `Error::InvitationUsed` pour l'invité légitime, et une alerte pour les administrateurs, que `serve` lit et que l'app montre sur l'invitation) ; le groupe ne l'admet qu'une fois (`Invitation::used`).
- **L'expiration, des deux côtés** : le serveur oublie le jeton à son terme (`Error::InvitationUnknown`), et un administrateur en ligne après lui n'admet plus la demande.
- **L'invité vérifie l'empreinte** : le Welcome doit être d'un groupe de l'empreinte du lien, et porter la confirmation du secret (`invite.rs`).
- **Un administrateur en ligne fait entrer l'invité** : `Team::serve`, après chaque lecture, admet les demandes des invitations de cet appareil par un Welcome MLS.

### Ajouter un de ses appareils (§3.4)

- **Par QR code** : le lien d'une invitation `Purpose::OwnDevice(membre)` (une heure), montré en QR code par l'app, ou collé sur l'autre appareil. L'appareil du membre l'admet lui-même : la règle « un membre n'ajoute que ses propres appareils ».
- **Le jeton d'un membre** passe par une demande dans le flux : un membre n'est pas administrateur du serveur, qui refuse alors l'enregistrement (403). Son appareil écrit `TeamMessage::TokenRequest`, un objet de l'équipe scellé et signé par sa feuille, qui ne porte que le SHA-256 du jeton. Un administrateur du serveur en ligne vérifie l'auteur (un membre de l'époque) et son droit (son propre appareil, ou un administrateur pour un nouveau membre), enregistre le jeton, et le dit (`TokenGranted`).
- **Par code court**, sans caméra (`pairing.rs`) : 8 caractères (base 32 de Crockford), les 3 premiers le créneau du relais choisi par le serveur, les 5 autres le mot de passe d'un **SPAKE2** (la crate `spake2` de RustCrypto). L'appareil dans l'équipe ouvre le créneau avec son message ; le nouveau y laisse le sien et une confirmation de clé : **un seul essai** par créneau. Un mauvais code, l'appareil dans l'équipe le voit à la confirmation : il ferme le créneau, rien n'est parti. Le bon code, il envoie le lien de l'invitation scellé (XChaCha20-Poly1305) sous la clé du PAKE, une fois son jeton enregistré. Le serveur ne voit que les messages du PAKE et la réponse scellée : rien qui permette d'essayer des codes hors ligne.

### Les rôles du serveur au pas du registre

`serve` compare, à chaque nouvelle époque, le rôle de chaque appareil sur le serveur à celui de son membre au registre, et le corrige (`POST /v1/devices/{id}/role`) quand cet appareil est administrateur du serveur. Un administrateur nommé après son inscription le devient donc aussi sur le serveur : il enregistre ses invitations et coupe les appareils.

### La récupération par la clé de secours, à travers le serveur

`Team::recover_onto` : le jour où plus aucun administrateur n'a d'appareil, l'hébergeur donne un jeton (`escouade-team-server recovery-token --team <id>`). Avec lui et les 24 mots :
1. la feuille de récupération s'inscrit (administrateur du serveur), lit la dernière enveloppe de récupération du flux (`TeamMessage::Recovery {group, wrap}`), l'ouvre avec les mots et entre dans le groupe par un commit externe ;
2. elle invite un nouvel appareil administrateur, celui de la machine, et l'admet ; puis elle le met administrateur du serveur ;
3. le nouvel appareil remplace la clé de secours (la feuille de l'ancienne quitte le groupe dans le même commit) ; l'appareil serveur de la feuille est révoqué, ses clés oubliées.

Les mots sont vérifiés avant d'utiliser le jeton : une faute ne le gâche pas. Les anciens mots n'ouvrent plus rien ensuite.

### L'alerte de fourche

`Error::alarm()` dit ce qu'une erreur montre du serveur : une fourche, un retour en arrière, une entrée réécrite. Le cœur arrête alors l'équipe sur cet appareil : il ne lit plus son flux et n'y envoie plus rien (ni invitation, ni entrée d'un invité, ni nouvel appareil, ni nouvelle clé de secours). L'app dit « Le serveur montre deux versions de l'équipe : contacte un administrateur », avec les six mots de la tête à comparer, « Réessayer » (une lecture qui passe lève l'alerte) et « Retirer l'équipe de cet appareil ».

### Le décodage

`wire::decode` lit le CBOR avec un tampon de la taille de ce qu'il lit : celui de ciborium (4 Kio) refusait une chaîne plus longue, comme l'hexadécimal d'un GroupInfo (une enveloppe de récupération) ou d'un Welcome d'une équipe qui grandit.

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

Les tests unitaires couvrent le trousseau, le chiffrement du stockage (une autre clé n'ouvre rien, une valeur déplacée non plus), les transactions, et un passage des deux cibles de fuzzing sur quelques entrées. La maquette garde ses 18 tests et ses vecteurs, et en gagne 6 (commit en deux temps, invitation rendue, état sauvegardé, balises, liens d'historique, fausses alertes).

## Le fuzzing

`fuzz/` est un paquet cargo-fuzz à part, hors du workspace de l'app, qui ne tourne pas en CI :

```sh
cargo install cargo-fuzz
cd crates/escouade-team/fuzz
cargo +nightly fuzz run commit   # la vérification des commits
cargo +nightly fuzz run roster   # le décodage du registre, des identités, des messages du flux
```

- `commit` : les octets construisent un commit par MLS brut, sur l'appareil d'un administrateur ou d'un membre. Ce commit ajoute des inconnus ou les appareils d'un autre, retire des feuilles, réécrit le registre (y compris par des octets bruts). Un troisième appareil le traite. S'il le prend, l'équipe doit rester saine : la genèse gardée, un administrateur au moins, chaque appareil d'un membre du registre, et rien qu'un membre n'a pas le droit de faire. Les octets bruts passent aussi par le décodeur des commits.
- `roster` : le registre (et son aller-retour), les identités, `TeamMessage`, les entrées et les trames du serveur, les objets scellés, les enregistrements d'époque.

## Ce qui reste

- **TS-3** (fait, ESC-461), sauf :
  - les projets d'une invitation (il n'y a pas encore de projets d'équipe : TS-5) ;
  - une invitation que seul l'appareil qui l'a faite honore : un autre administrateur en ligne ne la connaît pas (il faudrait la sceller pour les seuls administrateurs) ;
  - le flux en direct dans le cœur : il relit les équipes toutes les 3 s ;
  - `escouade-server team pair` pour une machine en mode serveur.
- **TS-4** : montrer l'équipe dans les espaces et ses réglages (membres, appareils, rôles, état du trousseau, hors ligne), retirer un appareil de soi qu'on oublie.
- **TS-5** : la synchronisation des opérations des projets, sur `write_object` et `read_objects`. Il faudra un groupe MLS par projet limité, et les instantanés.
- **TS-18** : faire tourner les cibles de fuzzing longtemps, et l'e2e avec deux clients.
- **L'audit externe (TS-20)** : les points 1 à 10 du README de la maquette. Pour cette crate, en plus :
  - le format du stockage (les clés en clair, dont l'id du groupe et les époques) ;
  - l'ordre état puis trousseau du point de contrôle ;
  - le repli sans trousseau, où le point de contrôle est sur le même disque ;
  - le fait qu'une alerte de fourche bloque la lecture du flux jusqu'à ce qu'un humain s'en occupe ;
  - le jeton du serveur dérivé du secret de l'invitation, et la demande de jeton d'un membre dans le flux ;
  - le code court : 25 bits de mot de passe, un essai par créneau, SPAKE2 sur Ed25519 sans liaison au créneau ;
  - la graine de la clé de séquestre montrée une fois, en clair, à l'administrateur qui fait la paire.
