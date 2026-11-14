# escouade-team-crypto : la maquette de la cryptographie du serveur d'équipe (TS-0a)

Ticket ESC-453, ligne TS-0a de `docs/TEAM-SERVER.md` §16. Le choix C9-B (MLS dès le départ) a été tranché par l'utilisateur le 2026-11-14 (§15.2). Cette crate n'est encore branchée nulle part : TS-2 en fera la crate `escouade-team` du client.

## En bref

- **L'équipe est un groupe MLS** (RFC 9420), par `openmls`. Le journal signé des membres du §3.7 devient le groupe et ses commits.
- **Le registre de l'équipe** est une extension du GroupContext (type 0xff00, plage privée) : la genèse, les rôles, la clé de secours, la clé de séquestre. Tous les membres d'une époque ont le même, et seul un commit le change. Les commits partent en messages privés MLS : le serveur ne le lit pas.
- **Les règles des commits** (`policy.rs`) sont notre service d'authentification. Chaque appareil les vérifie avant de fusionner un commit. Un administrateur ajoute et retire ; un membre n'ajoute ou ne retire que ses propres appareils ; un commit externe n'est admis que pour la clé de secours.
- **Les clés de projet par époque** sont dérivées du groupe : un export MLS par époque, puis HKDF par projet, puis par objet. Le contenu est chiffré en XChaCha20-Poly1305 et signé par l'auteur (Ed25519).
- **L'historique** est une chaîne de liens : le lien de l'époque `e` contient l'époque `e − 1`. Un nouvel appareil remonte ainsi jusqu'à l'époque 0.
- **L'invitation** (C6-A) : le lien ne porte que le secret, après le `#`. Les clés arrivent par un Welcome MLS d'un administrateur en ligne.
- **La récupération** (C7-B) : une clé de secours de 24 mots, et une clé de séquestre en option, visible de tous.
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
| Le groupe, ses époques, TreeKEM : confidentialité persistante et sécurité après compromission à chaque commit (chaque commit de la crate force un chemin) | **Le registre et ses règles** (`roster.rs`, `policy.rs`) : qui est administrateur, qui peut ajouter qui. Le service d'authentification est hors du standard |
| KeyPackages, Add, Welcome : l'invité reçoit les secrets de l'époque où il entre | **L'invitation** (`invite.rs`) : le MAC `HMAC(s, id ‖ KeyPackage)` empêche le serveur de substituer sa clé. La confirmation `HMAC(s, groupe ‖ époque ‖ epoch_authenticator)`, avec l'empreinte du lien, empêche de montrer une fausse équipe à l'invité |
| Remove : le retiré n'a pas les secrets de l'époque suivante | **Les clés de projet** (`keys.rs`) : `export("escouade/content/v1")`, puis HKDF par projet et par objet. Le retiré ne dérive plus rien de nouveau |
| L'export de secrets (RFC 9420 §8.5), l'epoch_authenticator (§8.7) | **L'historique** (`history.rs`) : MLS oublie les époques passées, la chaîne des liens les rend à un nouvel appareil |
| Le commit externe (§12.4.3.2) | **La clé de secours** (`recovery.rs`) : 24 mots BIP-39, qui donnent par HKDF une clé Ed25519 et une clé HPKE. À chaque époque, l'enregistrement et le GroupInfo lui sont scellés (HPKE). Elle revient par un commit externe |
| — | **Le séquestre** (`recovery.rs`) : l'enregistrement de chaque époque est scellé (HPKE) à la clé de l'organisation. Il est inscrit au registre, donc montré à chaque membre. Il lit tout, il n'écrit rien |
| Un commit rejoué d'une autre époque est refusé | **La fourche et le retour en arrière** (`device.rs`) : chaque objet cite `head = SHA-256(groupe ‖ époque ‖ epoch_authenticator)`, et un appareil qui connaît une autre tête pour cette époque dit « fourche ». Le point de contrôle (la plus haute époque vue et sa tête) refuse un état plus ancien, que ce soit celui du serveur ou une sauvegarde remise |

`relay.rs` joue le serveur aveugle : il ordonne les commits (le premier pour une époque gagne), garde les liens, les enveloppes, les demandes et les Welcome, et ne lit rien. Ses champs sont publics pour que les tests jouent un serveur qui ment.

## Les tests et les vecteurs

`cargo nextest run -p escouade-team-crypto` lance 18 tests, en 1 s environ.

- **Groupe** : la suite, le registre, la genèse, les six mots ; ajout et retrait d'un membre ; rotation sans rechiffrer l'ancien contenu (C8-A) ; clés différentes par projet et par époque.
- **Un membre retiré ne lit plus rien de nouveau**, ni par l'API, ni avec aucune des clés qu'il a eues : il n'ouvre ni les objets, ni les liens d'historique, ni les commits suivants.
- **Un nouvel appareil lit tout l'historique**, y compris celui d'avant l'arrivée du membre, et les auteurs sont vérifiés époque par époque.
- **Les règles** : un membre qui ajoute un étranger ou se fait administrateur par MLS brut est refusé ; le serveur qui glisse son KeyPackage ou tente un commit externe est refusé ; un faux groupe ou une confirmation changée sont refusés.
- **Une invitation qui fuit** : le serveur ne voit pas le secret, le lien ne porte aucune clé, et rien de ce que garde le serveur ne s'ouvre avec le secret. Une seconde utilisation et un lien expiré sont refusés.
- **La fourche** est détectée par le premier objet qui passe d'une branche à l'autre, ou par une tête montrée. **Le retour en arrière** est refusé : rejeu d'un commit, état ancien montré, sauvegarde ancienne, lien d'historique déplacé.
- **La clé de secours récupère l'équipe** : tous les administrateurs perdus, les 24 mots (retapés en majuscules) rejoignent, font entrer un nouvel appareil qui lit tout, puis une nouvelle clé de secours remplace l'ancienne. Les anciens mots n'ouvrent plus rien, et leur commit externe est refusé.
- **Le séquestre** lit toutes les époques, se voit chez chaque membre, et ne reçoit plus rien une fois retiré.
- **Les mots** suivent BIP-39 (vecteurs officiels de 256 bits).

`vectors/team-crypto-v1.json` donne des entrées fixes et les sorties attendues : mots et clés de secours, empreinte et ses six mots, tête, clés de projet et d'objet, objet scellé et signé, lien d'historique, lien d'invitation, MAC et confirmation, enveloppe de secours (aléatoire, le test l'ouvre). `ESCOUADE_WRITE_VECTORS=1` les réécrit. Les vecteurs de MLS lui-même sont ceux de la suite d'openmls.

## Les points à soumettre à l'audit externe (TS-20)

1. **Les règles des commits** (`policy.rs`) : sont-elles complètes ? Elles couvrent toutes les propositions, les commits externes, la conservation de l'identité, l'état d'après. Ce sont elles qui empêchent de glisser une clé : à fuzzer (TS-18).
2. **L'historique contre la confidentialité persistante** : qui tient une époque lit toutes les précédentes. C'est voulu (un Kanban dure), mais un appareil compromis livre tout l'historique, comme les clés persistantes du premier schéma.
3. **Les labels d'export et de HKDF**, leur séparation de domaine et le format des données associées (`ObjectHeader::aad`, `link_aad`, `wrap_aad`).
4. **L'invitation** : la force du MAC face à un serveur actif, le fait qu'un faux Welcome **consomme le KeyPackage** de l'invité (déni de service, l'invité redemande ; vu par les tests), et le partage de l'invitation entre appareils administrateurs (ici gardée par l'appareil qui invite, en équipe un objet chiffré).
5. **La clé de secours** : dériver Ed25519 et HPKE d'une entropie BIP-39 par HKDF, sans phrase de passe ; le GroupInfo scellé à chaque époque (qui le tient peut tenter un commit externe, refusé par les règles) ; la clé de secours à changer après chaque usage.
6. **Le séquestre** : sa visibilité dépend de l'interface, et rien n'oblige un committer malveillant à sceller pour le séquestre (le séquestre ne lit alors plus, il ne lit jamais plus que voulu).
7. **La détection de fourche** n'a lieu que si quelque chose passe d'une branche à l'autre. Un serveur qui partitionne l'équipe pour toujours reste invisible sans comparaison hors bande : il faut prévoir des balises de tête régulières, signées, et les six mots de la tête dans l'interface.
8. **Le point de contrôle** doit être gardé hors de l'état restaurable (au trousseau), sinon une sauvegarde ancienne le ramène avec elle.
9. **La persistance d'openmls** (ici `MemoryStorage`) : son format, son chiffrement au repos, l'effacement des clés passées (avis GHSA-qr9h-x63w-vqfm).
10. **Les métadonnées** : le serveur voit la taille et l'heure des commits, des liens et des enveloppes, et le nombre d'époques.

## Ce qui reste à faire (TS-2, TS-3)

- La persistance (stockage SQLite d'openmls), les clés d'appareil au trousseau, le point de contrôle hors de l'état.
- Les commits concurrents : sur un conflit, rejouer depuis le journal du serveur. Les liens d'historique manquants écrits par n'importe quel membre. Les balises de tête.
- Les projets limités à quelques membres : un groupe MLS par projet, avec le même code.
- L'ajout d'un appareil par QR code et par code court (PAKE) : même mécanisme que `Purpose::OwnDevice`.
- Le fuzzing de `policy::check_commit`, du décodage des messages et du registre.
