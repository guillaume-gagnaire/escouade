# escouade-team-crypto : la maquette de la cryptographie du serveur d'équipe (TS-0a)

Ticket ESC-453, ligne TS-0a de `docs/TEAM-SERVER.md` §16. Le choix C9-B (MLS dès le départ) a été tranché par l'utilisateur le 2026-11-14 (§15.2). Depuis TS-2 (ESC-458), c'est la couche cryptographique du client `escouade-team`, qui y ajoute le trousseau, le stockage SQLite et le serveur (`crates/escouade-team/README.md`). Elle reste pure et synchrone, sans disque ni réseau.

## En bref

- **L'équipe est un groupe MLS** (RFC 9420), par `openmls`. Le journal signé des membres du §3.7 devient le groupe et ses commits.
- **Le registre de l'équipe** est une extension du GroupContext (type 0xff00, plage privée) : la genèse, les rôles, la clé de secours, la clé de séquestre. Tous les membres d'une époque ont le même, et seul un commit le change. Les commits partent en messages privés MLS : le serveur ne le lit pas.
- **Les règles des commits** (`policy.rs`) sont notre service d'authentification. Chaque appareil les vérifie avant de fusionner un commit. Un administrateur ajoute et retire ; un membre n'ajoute ou ne retire que ses propres appareils ; un commit externe n'est admis que pour la clé de secours. Chaque commit signe sa portée dans ses données authentifiées (`CommitScope`, SR2-02) : celle d'un administrateur, que seul un administrateur porte, ou « ses propres appareils », vérifiée comme celle d'un membre ; dans le groupe d'un projet, l'époque de l'équipe puis cette portée. **L'annulation** d'un commit nul (`void.rs`, `TeamDevice::sign_void`, `void_signer`) est signée par la feuille d'un administrateur, sur le groupe, l'époque, le numéro et l'empreinte du commit.
- **Les clés de projet par époque** sont dérivées du groupe : un export MLS par époque, puis HKDF par projet, puis par objet. Le contenu est chiffré en XChaCha20-Poly1305 et signé par l'auteur (Ed25519).
- **L'historique** est une chaîne de liens : le lien de l'époque `e` contient l'époque `e − 1`. Un nouvel appareil remonte ainsi jusqu'à l'époque 0.
- **L'invitation** (C6-A) : le lien ne porte que le secret, après le `#`. Les clés arrivent par un Welcome MLS d'un administrateur en ligne.
- **La récupération** (C7-B) : une clé de secours de 24 mots, et une clé de séquestre en option, visible de tous. Leurs enveloppes sont ancrées (SR2-09, `anchor.rs`) : signées par la feuille d'un administrateur et par une ancre que seuls les appareils administrateurs tiennent (tirée des mots, ou de la graine du séquestre), elles portent l'empreinte de l'équipe, que la récupération compare aux six mots tapés depuis la feuille de secours.
- **Une fourche** se voit à la tête d'époque citée par chaque objet. **Un retour en arrière** est refusé grâce au point de contrôle de l'appareil.

## openmls : licence, maturité, version, suite

| | |
| --- | --- |
| Crates | `openmls` 0.9.2, `openmls_rust_crypto` 0.6.0, `openmls_basic_credential` 0.6.0, `openmls_traits` 0.6.0 (les dernières au 2026-11-14) |
| Licence | MIT (toutes quatre), acceptée par `deny.toml` |
| Auteurs | OpenMLS (Phoenix R&D, Cryspen) |
| MSRV | 1.91 : la crate le déclare, et la chaîne du dépôt est en 1.98 |
| Maturité | RFC 9420 implémentée, vecteurs de test du groupe de travail MLS dans sa propre suite. Un audit de SRLabs, financé par la Sovereign Tech Agency, a relevé 8 points, dont 1 élevé. Les correctifs sont parus dans 0.8.1 et 0.7.3, donc présents en 0.9.2. Avis passés : GHSA-8x3w-qj7j-gqhf (contournement de l'authentification par une étiquette tronquée, corrigé en 0.7.2) et GHSA-qr9h-x63w-vqfm. Pour 0.9.2, `cargo deny check` ne signale aucun avis |
| Ce qui bouge encore | L'API, avant la 1.0 : `join_by_external_commit` est dépréciée depuis 0.7.1 au profit de `external_commit_builder`, et certaines fonctions de test (`export_group_context`, `into_welcome`) ne sont pas publiques. Il faut épingler la version et relire le CHANGELOG à chaque mise à jour |
| Suite retenue | `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519` (0x0003). X25519 et Ed25519 comme le §3.2. ChaCha20-Poly1305 comme notre XChaCha20 : temps constant sans instructions AES, mobile compris, tout en RustCrypto. La suite obligatoire 0x0001 (AES-128-GCM) n'apporterait que l'interopérabilité avec d'autres clients MLS, dont nous n'avons pas besoin |
| Poids | 104 crates de plus dans `src-tauri/Cargo.lock`, aucune version existante changée. `cargo deny check` donne advisories, bans, licenses et sources « ok » |

## Ce que fait MLS, ce qui reste à nous

| MLS (openmls) | Nous, au-dessus |
| --- | --- |
| — | **Le rôle de service** (`roster.rs`, `policy.rs`, OE-1) : une machine de l'équipe (`Role::Service`), son propre id au registre, ni membre ni administrateur. Ses commits sont ceux d'un membre (ses appareils seuls) ; son rôle ne se donne qu'à l'entrée et ne change jamais (`check_roles`) ; `may_sign` dit ce que signe chaque rôle (`Act`), et les flux de l'équipe (`escouade-team`) le vérifient à la lecture |
| Le groupe, ses époques, TreeKEM : confidentialité persistante et sécurité après compromission à chaque commit (chaque commit de la crate force un chemin) | **Le registre et ses règles** (`roster.rs`, `policy.rs`) : qui est administrateur, qui peut ajouter qui. Le service d'authentification est hors du standard |
| KeyPackages, Add, Welcome : l'invité reçoit les secrets de l'époque où il entre | **L'invitation** (`invite.rs`) : le MAC `HMAC(s, id ‖ KeyPackage)` empêche le serveur de substituer sa clé. La confirmation `HMAC(s, groupe ‖ époque ‖ epoch_authenticator)`, avec l'empreinte du lien, empêche de montrer une fausse équipe à l'invité. Le code d'admission (`AdmissionCode`, six mots de `SHA-256(id ‖ KeyPackage)`) est confirmé par qui invite avant le Welcome : un lien qui fuit ne fait pas entrer qui l'a (SR2-08) |
| Remove : le retiré n'a pas les secrets de l'époque suivante | **Les clés de projet** (`keys.rs`) : `export("escouade/content/v1")`, puis HKDF par projet et par objet. Le retiré ne dérive plus rien de nouveau |
| L'export de secrets (RFC 9420 §8.5), l'epoch_authenticator (§8.7) | **L'historique** (`history.rs`) : MLS oublie les époques passées, la chaîne des liens les rend à un nouvel appareil |
| Le commit externe (§12.4.3.2) | **La clé de secours** (`recovery.rs`) : 24 mots BIP-39, qui donnent par HKDF une clé Ed25519 et une clé HPKE. À chaque époque, l'enregistrement et le GroupInfo lui sont scellés (HPKE). Elle revient par un commit externe |
| — | **Le séquestre** (`recovery.rs`) : l'enregistrement de chaque époque est scellé (HPKE) à la clé de l'organisation. Il est inscrit au registre, donc montré à chaque membre. Il lit tout, il n'écrit rien |
| Un commit rejoué d'une autre époque est refusé | **La fourche et le retour en arrière** (`device.rs`) : chaque objet cite `head = SHA-256(groupe ‖ époque ‖ epoch_authenticator)`, et un appareil qui connaît une autre tête pour cette époque dit « fourche ». Le point de contrôle (la plus haute époque vue et sa tête) refuse un état plus ancien, que ce soit celui du serveur ou une sauvegarde remise |
| Un groupe par ensemble de feuilles, les KeyPackages de dernier recours | **Le groupe d'un projet gardé à quelques membres** (`project.rs`, TS-5) : un second groupe MLS par projet, ses feuilles les appareils de l'équipe dont le membre a accès, son registre (`ProjectRoster`, extension 0xff01) changé par un administrateur de l'équipe seul, ses règles (`check_project_commit`) vérifiées contre la vue de l'équipe, ses liens d'historique |
| — | **Les opérations** (`ops.rs`, `hlc.rs`, TS-5) : une opération signée par la feuille, chaînée par appareil (rang et empreinte de la précédente), qui cite la tête du journal des membres et porte son horodatage hybride, puis chiffrée sous la clé du projet de l'époque (de l'équipe ou du groupe du projet). Le serveur n'en voit que l'époque. Les instantanés sont scellés et signés de même, sous un autre label |

`relay.rs` joue le serveur aveugle : il ordonne les commits (le premier pour une époque gagne), garde les liens, les enveloppes, les demandes et les Welcome, et ne lit rien. Ses champs sont publics pour que les tests jouent un serveur qui ment.

## Les tests et les vecteurs

`cargo nextest run -p escouade-team-crypto` lance 30 tests (24 de TS-0a et TS-2, 3 de l'horloge hybride et 3 des opérations et des projets, TS-5), en quelques secondes. Les 6 derniers sont venus avec TS-2 : le commit en deux temps (préparé, puis fusionné une fois pris par le serveur, ou abandonné), l'invitation rendue par un commit perdu, l'état sauvegardé, les balises de tête, les liens d'historique écrits par un autre membre, et l'absence de fausse alerte de fourche (ni un membre retiré, ni le serveur).

- **Groupe** : la suite, le registre, la genèse, les six mots ; ajout et retrait d'un membre ; rotation sans rechiffrer l'ancien contenu (C8-A) ; clés différentes par projet et par époque.
- **Un membre retiré ne lit plus rien de nouveau**, ni par l'API, ni avec aucune des clés qu'il a eues : il n'ouvre ni les objets, ni les liens d'historique, ni les commits suivants.
- **Un nouvel appareil lit tout l'historique**, y compris celui d'avant l'arrivée du membre, et les auteurs sont vérifiés époque par époque.
- **Le rôle de service** (OE-1) : la table de `may_sign` ; une machine de l'équipe lit et écrit le contenu, ajoute ses propres appareils, et chacun de ses autres commits (un étranger ajouté, elle-même faite administrateur, un membre ôté du registre) est refusé par MLS brut, quelle que soit sa portée ; ni une personne faite machine, ni une machine faite personne, ni le propriétaire fait machine, même par un administrateur ; retirée, elle ne lit plus rien.
- **Les règles** : un membre qui ajoute un étranger ou se fait administrateur par MLS brut est refusé ; le serveur qui glisse son KeyPackage ou tente un commit externe est refusé ; un faux groupe ou une confirmation changée sont refusés.
- **Une invitation qui fuit** : le serveur ne voit pas le secret, le lien ne porte aucune clé, et rien de ce que garde le serveur ne s'ouvre avec le secret. Une seconde utilisation et un lien expiré sont refusés.
- **La fourche** est détectée par le premier objet qui passe d'une branche à l'autre, ou par une tête montrée. **Le retour en arrière** est refusé : rejeu d'un commit, état ancien montré, sauvegarde ancienne, lien d'historique déplacé.
- **La clé de secours récupère l'équipe** : tous les administrateurs perdus, les 24 mots (retapés en majuscules) rejoignent, font entrer un nouvel appareil qui lit tout, puis une nouvelle clé de secours remplace l'ancienne. Les anciens mots n'ouvrent plus rien, et leur commit externe est refusé.
- **Le séquestre** lit toutes les époques, se voit chez chaque membre, et ne reçoit plus rien une fois retiré.
- **Les mots** suivent BIP-39 (vecteurs officiels de 256 bits).

`vectors/team-crypto-v1.json` donne des entrées fixes et les sorties attendues : mots et clés de secours, empreinte et ses six mots, tête, clés de projet et d'objet, objet scellé et signé, lien d'historique, lien d'invitation, MAC et confirmation, enveloppe de secours (aléatoire, le test l'ouvre). `ESCOUADE_WRITE_VECTORS=1` les réécrit. Les vecteurs de MLS lui-même sont ceux de la suite d'openmls.

## Les points à soumettre à l'audit externe (TS-20)

1. **Les règles des commits** (`policy.rs`) : sont-elles complètes ? Elles couvrent toutes les propositions, les commits externes, la conservation de l'identité, l'état d'après. Ce sont elles qui empêchent de glisser une clé : la cible `commit` de `fuzz/` les vise (TS-18), en construisant des commits valides au sens de MLS ; un commit que MLS refuse n'atteint pas les règles.
2. **L'historique contre la confidentialité persistante** : qui tient une époque lit toutes les précédentes. C'est voulu (un Kanban dure), mais un appareil compromis livre tout l'historique, comme les clés persistantes du premier schéma.
3. **Les labels d'export et de HKDF**, leur séparation de domaine et le format des données associées (`ObjectHeader::aad`, `link_aad`, `wrap_aad`).
4. **L'invitation** : la force du MAC face à un serveur actif, le fait qu'un faux Welcome **consomme le KeyPackage** de l'invité (déni de service, l'invité redemande ; vu par les tests), et le partage de l'invitation entre appareils administrateurs (ici gardée par l'appareil qui invite, en équipe un objet chiffré).
5. **La clé de secours** : dériver Ed25519 et HPKE d'une entropie BIP-39 par HKDF, sans phrase de passe ; le GroupInfo scellé à chaque époque (qui le tient peut tenter un commit externe, refusé par les règles) ; la clé de secours à changer après chaque usage. **L'ancre** (SR2-09) : une troisième clé tirée des mots, Ed25519, tenue des seuls appareils administrateurs et donnée scellée (HPKE) à la forme Montgomery (X25519) de la clé de feuille Ed25519 du destinataire : la même clé sert donc à signer et à recevoir, comme le font age et libsodium ; à revoir. Ce qui reste : un ancien administrateur garde l'ancre des mots de son temps (retiré, il lui faut le serveur pour écrire dans le flux ; redevenu membre, non : le client dit alors de refaire la clé de secours).
6. **Le séquestre** : sa visibilité dépend de l'interface, et rien n'oblige un committer malveillant à sceller pour le séquestre (le séquestre ne lit alors plus, il ne lit jamais plus que voulu).
7. **La détection de fourche** n'a lieu que si quelque chose passe d'une branche à l'autre. Un serveur qui partitionne l'équipe pour toujours reste invisible sans comparaison hors bande : il faut prévoir des balises de tête régulières, signées, et les six mots de la tête dans l'interface.
8. **Le point de contrôle** doit être gardé hors de l'état restaurable (au trousseau), sinon une sauvegarde ancienne le ramène avec elle.
9. **La persistance d'openmls** (`MemoryStorage` ici, SQLite chiffré dans `escouade-team`) : son format, son chiffrement au repos, l'effacement des clés passées (avis GHSA-qr9h-x63w-vqfm).
10. **Les métadonnées** : le serveur voit la taille et l'heure des commits, des liens et des enveloppes, et le nombre d'époques.
11. **Les opérations et les projets (TS-5)** : le label et les données associées de `Sealed` (`ops.rs`), la signature à l'intérieur du chiffré (un membre qui tient la clé peut rechiffrer l'opération signée d'un autre, sans la changer : la chaîne la dédouble) ; l'horodatage, qu'un membre malveillant peut mettre en avance pour gagner (la dérive bornée à une heure ne vaut que pour les horloges des autres) ; les règles des groupes de projet évaluées contre la vue de l'équipe du vérificateur (un vérificateur en avance sur le committeur peut refuser un ajout que l'autre a fait) ; le KeyPackage de dernier recours, réutilisé d'un groupe de projet à l'autre.

12. **Le rôle de service (OE-1)** : une machine exposée jour et nuit, pilotée par un modèle qui lit des textes de tiers, tient une feuille du groupe. Les règles la traitent en membre pour les commits (`policy.rs`), lui interdisent tout changement de rôle (`check_roles`) et, par `may_sign`, toute signature d'un réglage, d'un coupe-circuit, d'un plugin privé, d'une proposition, d'une réponse à une invitation ; ces dernières sont vérifiées par chaque flux d'`escouade-team` à la lecture (par l'id du membre signataire, `Team::is_machine`, qui garde aussi les machines vues puis retirées). À revoir : qu'aucun flux nouveau n'oublie l'appel ; qu'une machine reste une feuille comme une autre pour la confidentialité (elle lit tout l'historique des projets qu'on lui ouvre).

## Ce que TS-2 a fait, ce qui reste (TS-3, TS-5, TS-18, TS-20)

Fait par TS-2 (ESC-458), ici et dans `escouade-team` :
- la persistance (stockage SQLite d'openmls), les clés d'appareil au trousseau, le point de contrôle hors de l'état ;
- les commits concurrents (le commit en deux temps, `Change`, `is_done`), rejoués depuis le flux du serveur ;
- les liens d'historique manquants écrits par n'importe quel membre (`history_link`, `learn_history`) ;
- les balises de tête (`beacon.rs`) et les six mots de la tête (`head_words`) ;
- le fuzzing de `policy::check_commit` et du décodage (les cibles de `crates/escouade-team/fuzz`).

Fait par TS-5 (ESC-466) : `hlc.rs`, `ops.rs` et `project.rs` (ci-dessus), `TeamDevice::seal_for` et `record_for`, et 3 tests (l'opération liée à son projet, à son époque et à son auteur ; le groupe d'un projet, son retrait au départ d'un membre et son historique ; un commit de projet hors des règles refusé).

Ce qui reste :
- l'ajout d'un appareil par QR code et par code court (PAKE) : même mécanisme que `Purpose::OwnDevice` (TS-3) ;
- l'audit externe (TS-20).

Fait par TS-18 (ESC-519) : le fuzzing de ce que la crate lit sans l'avoir vérifié, dans `fuzz/` à la racine (les corps dans `escouade-team`, `src/fuzzing/`). Ses vecteurs (`vectors/team-crypto-v1.json`) en sont des graines, sous la forme où le serveur les sert : l'objet scellé, le lien d'historique, l'enveloppe de secours, l'enregistrement d'époque, l'identité, le lien d'invitation. Ce que le fuzzing vérifie de cette crate, pour l'audit :
- `check_head`, `check_server_state`, `check_restored`, `check_beacon` (cible `journal`) : une tête forgée n'est jamais prise, une époque plus ancienne que le point de contrôle est toujours un retour en arrière, une balise changée n'est prise que telle que signée ;
- `SealedObject::open`, `HistoryLink::open`, `Sealed::open_operation` et `open_snapshot`, `EscrowKey::open`, `RecoveryWrap::open` (cibles `journal`, `operation`, `snapshot`, `envelope`) : changés octet par octet, ils s'ouvrent tels que scellés ou pas du tout ; une opération signée d'une clé qui n'est la feuille de personne n'est jamais ouverte ;
- `InviteLink::parse`, `verify_join_mac`, `verify_welcome_confirmation` (cible `invite`) ;
- `policy::check_commit` à travers `process_commit` (cible `commit`).

Ce qu'il ne couvre pas : MLS lui-même (openmls a sa suite), la dérivation des clés (les vecteurs la fixent), le temps constant des comparaisons (`verify_slice` de `hmac`, `verify_strict` d'`ed25519-dalek` : à lire, pas à fuzzer). L'horloge hybride (`hlc.rs`) a ses propriétés par proptest dans `escouade-team` (`tests/properties.rs`) : elle ne recule jamais, passe ce qu'elle a vu dans la dérive permise, et ses octets se trient comme ses horodatages.
