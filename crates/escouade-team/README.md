# escouade-team : le client du serveur d'équipe (TS-2)

Ticket ESC-458, ligne TS-2 de `docs/TEAM-SERVER.md` §16. L'équipe est un groupe MLS (C9-B). Cette crate est le client : elle tient un appareil dans une équipe, avec ses clés au trousseau, son état sur le disque et son serveur. Elle n'a pas d'interface (TS-3 et TS-4 la montreront) et ne synchronise pas encore les opérations des projets (TS-5). Elle n'est branchée nulle part dans l'app : le cœur lui donnera son trousseau et son client HTTP.

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
| `team.rs` | `Team` : créer, rejoindre, ouvrir, lire le flux, committer, faire tourner les clés, retirer, révoquer, inviter et admettre, écrire et lire les objets et les fichiers |
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

- **TS-3** :
  - l'interface pour créer l'équipe, montrer les 24 mots et les six mots de la tête ;
  - inviter par lien `escouade://team/join` ;
  - ajouter un appareil par QR code et par code court (PAKE), sur `Purpose::OwnDevice`. Le jeton du serveur pour l'appareil d'un membre devra passer par une demande dans le flux : aujourd'hui seul un administrateur l'obtient ;
  - les rôles du serveur tenus au pas du registre : une route pour qu'un administrateur nommé après son inscription le devienne aussi sur le serveur ;
  - le séquestre, la récupération par la clé de secours à travers le serveur ;
  - l'alerte « Le serveur montre deux versions de l'équipe » et ce qu'on fait ensuite ;
  - le branchement dans le cœur : le `Keychain` sur le `SecretStore`, `HttpTransport` sur `tls::app_builder`, `tls::config` et `tls::proxy_for`, le dossier `teams` des données.
- **TS-4** : montrer l'équipe (membres, appareils, état du trousseau, hors ligne).
- **TS-5** : la synchronisation des opérations des projets, sur `write_object` et `read_objects`. Il faudra un groupe MLS par projet limité, et les instantanés.
- **TS-18** : faire tourner les cibles de fuzzing longtemps, et l'e2e avec deux clients.
- **L'audit externe (TS-20)** : les points 1 à 10 du README de la maquette. Pour cette crate, en plus :
  - le format du stockage (les clés en clair, dont l'id du groupe et les époques) ;
  - l'ordre état puis trousseau du point de contrôle ;
  - le repli sans trousseau, où le point de contrôle est sur le même disque ;
  - le fait qu'une alerte de fourche bloque la lecture du flux jusqu'à ce qu'un humain s'en occupe.
