---
title: "Le serveur d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team-server/README.md","crates/escouade-team-crypto/README.md","crates/escouade-team/README.md","crates/escouade-team-server/Dockerfile","crates/escouade-team-server/deploy/coolify.yml",".github/workflows/release.yml","website/content/docs/fr/team/install.md","Revue de sécurité de la 2.3 (ESC-625), SR2-05, SR2-06, SR2-07 : le serveur d'équipe borné","Revue de sécurité de la 2.3 (ESC-625), SR2-04, SR2-10 à SR2-13 : quotas, journaux, jeton d'init, révoqués, dernier administrateur","docs/TEAM-AUDIT.md §3.1, §4.4 (AU-2, ESC-609 : les options du serveur et sa clé d'audit)"]
tickets: ["ESC-456","ESC-458","ESC-461","ESC-466","ESC-586","ESC-637","ESC-633","ESC-636","ESC-635","ESC-639","ESC-638","ESC-642","ESC-609"]
decisions: [167,168,173,178,368,370,286,314,330,332,340,344,360]
compared: null
---

# Le serveur d'équipe

Un binaire à part, `escouade-team-server`, que l'équipe héberge elle-même (Docker avec Caddy, Coolify, ou systemd). Il est distinct du mode serveur, qui lance des agents et ne s'expose pas. Son client est la crate `escouade-team` (voir « Le client de l'équipe »), branchée dans l'app : créer une équipe, y inviter et y entrer, voir « Entrer dans une équipe » ; la synchronisation des projets, voir « La synchronisation des projets ». Publié avec chaque version d'Escouade depuis la 2.3 (voir *La publication*).

## Aveugle

- Il ne garde que des octets chiffrés de bout en bout (seul mode), et les métadonnées que voici : des ids opaques, les clés publiques des appareils, leurs rôles, les tailles, les heures, l'appareil qui a écrit, le projet auquel un fichier est rattaché, l'empreinte (SHA-256) des jetons d'inscription.
- Il n'exécute rien (aucun agent, commande, plugin ni harness) et n'a aucune clé de contenu. Il ne voit jamais le secret d'une invitation ni un code court : le lien d'une invitation (`escouade://team/join/…`) ne passe par aucune de ses pages.
- D'une opération d'un projet, il ne voit que son époque, sa taille, son heure et l'appareil qui l'envoie.
- Son journal et ses métriques Prometheus ne portent aucun id d'équipe, de projet, d'appareil ni de fichier, aucune adresse, aucun jeton : seulement les modèles des routes, les statuts et des compteurs. **Ses erreurs non plus** (SR2-10) : une erreur de la base n'y dit que sa sorte (son code), jamais son texte, qui peut citer une valeur ; une erreur du stockage S3, que sa sorte (délai, connexion, envoi, corps) ou le statut rendu, jamais l'adresse de la requête, qui porte l'équipe et le fichier.

## Les appareils et l'entrée

- Un appareil : un id, une clé publique Ed25519, une équipe, un rôle (administrateur ou membre), révoqué ou non. L'id d'un appareil sur le serveur est aussi celui de son identité dans le groupe MLS de l'équipe, mais sa clé pour le serveur n'est pas sa clé MLS.
- `escouade-team-server init` crée la base et affiche un jeton à usage unique : le premier appareil qui s'inscrit avec crée l'équipe et en devient administrateur. **Le jeton dure 24 heures** (SR2-11), ce que `init` dit en l'affichant : resté dans un terminal ou un historique et jamais servi, il ne crée plus d'équipe ensuite. Une base d'avant donne aussi 24 heures, depuis sa création, à un jeton d'`init` non servi.
- Un administrateur demande un jeton d'inscription par nouvel appareil (24 h par défaut, 7 jours au plus), ou enregistre l'empreinte d'un jeton qu'il a fait lui-même : celui d'une invitation, dérivé du secret après le `#`.
- **Un jeton sert une fois** : un second usage est refusé en disant « déjà utilisé » (et non « inconnu »), et noté ; les administrateurs de l'équipe lisent ces seconds usages. Un jeton expiré est inconnu.
- `escouade-team-server recovery-token --team <id>` affiche un jeton qui inscrit un appareil dans une équipe existante comme administrateur du serveur : pour le détenteur de la clé de secours, le jour où tous les administrateurs ont perdu leurs appareils. Il ne donne aucune clé.
- Un administrateur révoque un appareil de l'équipe, un appareil se révoque lui-même : ses sessions et ses connexions en direct se ferment aussitôt. **L'appareil révoqué reste listé**, pour l'histoire de l'équipe, mais ne compte plus dans ses 500 appareils (SR2-12) : une équipe qui renouvelle ses postes en inscrit toujours.
- Un administrateur change le rôle d'un appareil sur le serveur : c'est ainsi que les rôles du serveur suivent le registre de l'équipe. **Le dernier administrateur non révoqué ne devient jamais membre et n'est jamais révoqué**, ni par lui-même ni par un autre administrateur (SR2-13) : le serveur refuse (`409 last_admin`) en disant qu'une équipe garde un administrateur sur le serveur ; on en nomme un autre d'abord. Sans cela, l'équipe n'aurait plus personne pour inviter ou restreindre un projet jusqu'à un jeton de récupération de l'opérateur.

## L'appairage par code court

- Un relais en mémoire pour « Ajouter un appareil » par un code de 10 caractères : l'appareil dans l'équipe ouvre un créneau (5 caractères, la tête du code, un parmi 33 millions), le nouvel appareil, pas encore inscrit, y fait **un seul essai**, qui lui rend un jeton de 128 bits, puis prend la réponse scellée avec ce jeton. Un créneau dure 10 minutes ; quatre au plus par équipe.
- Il ne porte que les messages d'un PAKE (SPAKE2) et une réponse scellée : rien qui permette au serveur d'essayer des codes.
- **Difficile à trouver** (SR2-05) : lire le message d'un créneau et y faire son essai, sans session, ont leur propre limite de débit par adresse (10 d'un coup, puis une toutes les 5 secondes ; une adresse IPv6 par son /64), et la réponse ne se prend qu'avec le jeton de l'essai (403 sous un autre). Un inconnu du réseau ne parcourt pas les créneaux pour brûler l'unique essai d'un autre.

## La page d'un lien d'invitation de l'ancienne forme

`/join/<id>`, ouvert dans un navigateur (un lien `https://<serveur>/join/<id>#…` d'avant la 2.3) : un **texte fixe**, le même pour tout id, qui dit que ce lien ne sert plus et qu'il faut en demander un neuf, à cliquer ou coller dans Escouade. **Aucun script** (sa politique de sécurité les interdit tous), aucune ressource d'ailleurs, aucun lien, rien du lien lu. Une page du serveur ne touche jamais le secret d'une invitation : le script d'une page qu'il sert pourrait lire ce qui suit le `#` dans le navigateur, et le serveur piraté de la menace n°1 entrerait à la place de l'invité.

## L'authentification

- Par défi signé : le serveur donne un défi (60 s, une seule fois), l'appareil le signe avec sa clé, le serveur rend un jeton de session de 15 minutes. Aucun mot de passe.
- Une mauvaise signature, un appareil révoqué ou une session expirée sont refusés.
- **Bornée par appareil et par équipe** (SR2-06) : deux défis en attente au plus par appareil (un nouveau prend la place du plus ancien), 1 024 par équipe, 100 000 sur le serveur ; huit sessions ouvertes par appareil (une nouvelle ferme la plus ancienne). Un appareil, ou une équipe, qui demande des défis sans fin ne remplit que ses propres places : les autres équipes se connectent comme avant.
- Toutes les routes demandent une session, sauf `/healthz`, la page `/join/<id>`, l'inscription par jeton, le défi, et les étapes du nouvel appareil dans le relais d'appairage.

## Les flux et les fichiers

- Un flux par équipe (les messages MLS) et un par projet (les opérations chiffrées, et les commits du groupe d'un projet gardé à quelques membres). Le serveur numérote chaque entrée, la garde et la diffuse ; un appareil relit depuis le numéro n pour reprendre.
- Les commits MLS, de l'équipe comme du groupe d'un projet : le premier pour une époque gagne. Un second pour la même époque est refusé, avec les entrées depuis le commit gagnant, pour que l'appareil rattrape.
- **Les commits sont aux administrateurs** (SR2-02) : le serveur refuse (`403`) le commit d'un membre qui ne dit pas, en clair, ne changer que ses propres appareils. Les appareils vérifient ce que le commit signe (voir « Le client de l'équipe », Le commit nul).
- **L'annulation d'un commit gagnant** (SR2-02) : un appareil administrateur, et lui seul, déclare nul le dernier commit du flux (l'équipe ou un projet), une fois. Le serveur ne la lit pas : il la garde dans le flux après le commit, qui reste servi tel quel, et l'époque redevient libre, **réservée aux administrateurs jusqu'à leur prochain commit** (un membre reçoit `409 epoch_reserved`). Une annulation qui ne vise pas l'époque du dernier commit, ou un commit déjà déclaré nul, est refusée (`409 void_refused`).
- **Les instantanés d'un projet** (SR2-01) : un appareil du projet envoie l'état chiffré du projet jusqu'à un numéro. Le serveur le garde à côté des précédents, et ses octets comptent dans le quota.
  - **Rien n'est oublié à sa pose.** Les opérations qu'il couvre restent **30 jours**, puis le serveur les oublie (pas les commits) et rend leurs octets au quota, avec les instantanés d'avant. Il ne le fait jamais pour un instantané qu'un appareil a signalé comme illisible.
  - Un instantané qui n'est pas après le dernier, ou qui va au-delà du flux, est refusé.
  - **Aucun instantané sur les flux de l'équipe** : la liste des projets, les réglages imposés, les propositions, les coûts, le store privé, le coupe-circuit. Le serveur le refuse (`403`), à un administrateur comme à un membre.
  - La lecture d'un projet dit où est le dernier instantané : quand il est après le numéro demandé (l'appareil en part), et toujours (l'appareil déjà au-delà le vérifie une fois). Un appareil peut demander l'instantané d'avant un numéro.
  - Un appareil du projet signale un instantané illisible. Les administrateurs de l'équipe lisent la liste de ces instantanés : le projet, le numéro, l'appareil qui l'a posé et quand, le premier appareil qui l'a signalé et quand. Les membres ne la lisent pas.
  - **Un administrateur retire un instantané** : l'instantané d'avant et le journal reprennent sa place, ses octets reviennent au quota. Le serveur refuse de retirer celui sur lequel le flux a déjà été élagué, puisque ses opérations sont parties.
- Les fichiers chiffrés (documents, images, conversations passées, dépôts d'une passation, archives du store privé) : sur le disque ou dans un stockage compatible S3 (MinIO, Garage, AWS), écrits une fois, avec une taille maximale par fichier (25 Mio par défaut) et un quota par équipe (2 Gio par défaut) qui compte aussi les flux et les instantanés.
- **Un fichier suit les droits de son projet** (SR2-03) :
  - **Rattaché à sa pose** à un projet, ou à l'équipe par l'un de ses propres flux (le store privé, ouvert à tous ses appareils). Un appareil hors d'un projet n'y pose rien.
  - **Lu selon les droits de ce projet**, comme son flux : un appareil hors du projet est refusé ; sous un autre projet, le fichier n'est pas trouvé.
  - **Effacé par son auteur ou un administrateur seulement** : l'effacement est définitif. Un autre membre est refusé. Le dépôt d'une passation est donc effacé par qui passe la main, quand l'offre est finie (prise ou abandonnée).
  - **Les fichiers posés avant** (et ceux de l'ancienne route, gardée une version) ne sont rattachés à rien : seuls leur auteur et les administrateurs les lisent et les effacent, et leur lecture sous un projet les y rattache. La migration n'ouvre rien de plus qu'avant : un membre qui les lisait les retrouve quand leur auteur ou un administrateur les a rouverts.
- La diffusion en direct par WebSocket : un appareil abonné reçoit les nouvelles entrées de tous les flux de son équipe, et rien des autres équipes.

## Le protocole et les défenses

- Messages en CBOR, objets chiffrés en binaire.
- Poignée de main des versions : le serveur accepte la version du client et la précédente ; une plus vieille est refusée avec « Mets Escouade à jour », une plus récente dit que le serveur est en retard.
- Une limite de débit par adresse et par appareil (429 et `Retry-After`). Une adresse IPv6 compte par son préfixe /64 : changer d'adresse dans le sien ne change rien.
- Des tailles bornées partout ; une longueur annoncée trop grande est refusée avant d'être lue.
- **Chaque ligne qu'un appareil fait compte** (SR2-04) : un projet est connu du serveur dès sa première écriture, quelle qu'elle soit (une entrée de son flux, son accès, une prise de ticket, un numéro de ticket), et compte dès lors dans les 10 000 projets de l'équipe ; au-delà, un projet inconnu est refusé (`403 too_many`), quelle que soit l'écriture. Une prise compte 128 octets dans le quota de l'équipe, une liste d'accès les octets de ses appareils, et chaque appareil n'y figure qu'une fois (`400 duplicate_device`) ; le compteur des numéros et l'accès d'un projet sont une ligne par projet. Ainsi le quota borne tout ce qu'un membre (ou un agent détourné) fait grossir, et il ne remplit pas le disque des autres équipes du serveur.
- **Des délais** (SR2-07) : l'en-tête d'une requête est lu en 15 secondes ; son corps arrive en entier en 30 secondes, plus le temps de sa longueur à 64 Kio par seconde (environ 7 minutes pour un fichier de 25 Mio), sinon `408` et la connexion se ferme ; une connexion où rien ne passe, dans un sens comme dans l'autre, pendant 60 secondes se ferme.
- **Un nombre de connexions plafonné** (SR2-07) : 8 192 ouvertes à la fois, et 512 d'une même adresse (une adresse IPv6 par son /64). Au-delà, une nouvelle connexion est fermée dès qu'elle est acceptée, et comptée dans les métriques. Un WebSocket compte jusqu'à sa fermeture. Derrière un proxy de confiance (`--trust-proxy`), seul le plafond global tient, puisque toutes les connexions viennent du proxy.
- TLS par rustls avec un certificat donné, ou derrière un proxy inverse (Caddy dans l'exemple, Traefik sous Coolify). **Joignable depuis Internet, le serveur reste derrière le proxy** : la doc d'installation le dit, le proxy encaisse d'abord les connexions lentes ou trop nombreuses ; TLS par le serveur lui-même est pour un réseau privé.
- La base : SQLite en WAL par défaut, PostgreSQL en option. Une base d'avant la rétention garde ses instantanés : le flux y est déjà élagué jusqu'à eux. Une base d'avant SR2-03 garde ses fichiers, rattachés à rien.

## La publication

- Chaque version d'Escouade publie le serveur d'équipe à son numéro (le même que l'app), par le Release comme `escouade-server` (voir *Livraison et mises à jour*, *CI/CD*) : pour Linux x64 et arm64, une archive `escouade-team-server-<version>-linux-<arch>.tar.gz` (le programme, sa licence, les exemples de `deploy/` : systemd, compose avec Caddy, Coolify), son SHA-256 et sa signature par la clé des mises à jour ; et l'image `ghcr.io/guillaume-gagnaire/escouade/escouade-team-server`, étiquetée de la version, de `latest` pour une version stable seulement et de `beta` pour une bêta. Archives et image portent une attestation de provenance.
- Le binaire est essayé dans un conteneur Debian nu avant tout envoi : sa version, `init` (son jeton jamais au journal), `/healthz` sur 127.0.0.1, un client sans la poignée de main refusé en 426, l'arrêt propre à SIGTERM.
- L'image (Debian 12 slim) tourne sous son propre utilisateur (`escouade-team`), écrit dans `/data` seul (`ESCOUADE_TEAM_DATA`), écoute sur `0.0.0.0:8443` de son conteneur, et se construit depuis les sources ou depuis l'archive d'une version (`PROGRAM=archive`, celle du Release).
- Les exemples tirent l'image publiée à l'étiquette de `ESCOUADE_TEAM_TAG` (`latest` par défaut ; `beta` ou une version pendant une bêta) : le compose derrière Caddy, qui tient TLS ; le compose de Coolify, derrière son Traefik (domaine de `SERVICE_FQDN_TEAM_8443`, `ESCOUADE_TEAM_TRUST_PROXY=1`, lecture seule, aucune capacité, aucun port publié). Leur mode d'emploi est dans la doc du site (Installer le serveur d'équipe), en français et en anglais.

## Dégeler une équipe ou un projet (SR2-02)

Le serveur garde le premier commit d'une époque sans pouvoir le lire, puis refuse tout autre commit de cette époque. Un commit que les appareils ne lisent pas (des octets quelconques, ou un commit contraire aux règles de l'équipe) ne gèle plus l'équipe : les appareils le tiennent pour nul et un administrateur l'annule (voir *Les flux et les fichiers*). **L'hébergeur le retire** encore par une commande, sans toucher à la base à la main, quand aucun administrateur ne peut l'annuler (le groupe d'un projet sans administrateur, une équipe dont aucun administrateur n'est en ligne).

- `escouade-team-server drop-entry --team <id> --stream team --seq <n>` retire une entrée du flux de l'équipe ; `--project <id>` celle du flux d'un projet ; `--stream <nom>` celle d'un flux propre à l'équipe (`escouade/setting`…). En SQLite comme en PostgreSQL, dans une seule transaction.
- **Confirmée** : la commande montre l'entrée (genre, époque, appareil qui l'a écrite, date, taille, empreinte) et attend `yes` ; `--yes` s'en passe. **Elle refuse sans l'id de l'équipe** (elle ne devine jamais l'équipe), sans flux ou avec deux, sans numéro, pour une entrée ou une équipe que la base n'a pas.
- **Un commit doit être le dernier de son flux** : son époque redevient libre, et le commit suivant (celui de l'administrateur) la prend. Un commit que d'autres suivent est refusé : on retire les plus récents d'abord. **Une annulation, et le commit qu'elle a déclaré nul, ne se retirent pas** : les appareils les ont déjà dépassés.
- **Le flux garde sa numérotation** : le trou se voit ; les octets de l'entrée reviennent au quota.
- **Journalisée sans contenu** : une ligne d'audit dans la base, dans la même transaction : l'équipe, le flux, le numéro, le genre et l'époque, l'appareil qui l'avait écrite et quand, sa taille, son empreinte (SHA-256 de l'entrée telle que servie), qui la retire (`--by`, ou le nom du compte) et quand. Jamais ses octets. Le journal du serveur n'en garde qu'une ligne sans identifiant, comme le reste. `drop-entry --team <id> --list` relit l'audit.
- **Les appareils voient le trou.** Le serveur liste à chaque appareil de l'équipe les entrées retirées (`GET /v1/dropped`) : le flux (celles d'un projet aux seuls appareils qui y ont accès), le numéro, le genre et l'époque, l'appareil qui l'avait écrite et quand, l'empreinte, l'heure du retrait ; jamais qui l'a retirée. La dernière entrée qu'un appareil a lue, retirée, n'est pas un retour en arrière si son empreinte est celle qu'il en a gardée : il lit la suite. Une entrée cachée sans être listée reste un retour en arrière. Un appareil qui avait noté le commit retiré sans l'avoir pris (avant d'entrer) l'oublie, pour que le commit qui reprend l'époque ne soit pas une fourche.
- **Les administrateurs sont prévenus** : leur appareil lit la liste au plus toutes les cinq minutes, aussitôt après un trou vu ; la carte de l'équipe montre « Entrées retirées par l'opérateur du serveur » pendant 30 jours (le flux, le numéro, commit ou message, qui l'avait écrite, quand elle a été retirée). Les membres ne le voient pas.
- **Ce qu'elle ne permet pas** : lire une entrée ou la rendre, ni en retirer une en silence. Retirer autre chose que l'entrée qui bloque casse ce qui la suit : un commit lu par des appareils, remplacé, est une fourche pour eux ; une opération valide d'un projet fait refuser les suivantes de son appareil ; une entrée d'un administrateur (liste des projets, réglages imposés, store privé) disparaît pour tous.

## Son audit vers un SIEM (AU-2)

Le serveur émet seul son journal d'audit (la source « serveur » de « L'audit vers un SIEM »). **Éteint par défaut** ; l'équipe qui l'héberge l'allume par une option de `escouade-team-server serve` :

- `--audit-file <chemin>` : JSON lines dans un fichier tourné à 100 Mio et chaque jour ; `--audit-stdout` : sur la sortie standard (son journal passe alors sur la sortie d'erreur) ; `--audit-syslog tls://hôte:port` : syslog RFC 5424 sur TLS (ou `tcp://`, jamais UDP), avec `--audit-syslog-ca` pour les certificats auxquels se fier et `--audit-cef` pour un message en CEF. Les sorties se cumulent ; chacune a sa variable d'environnement.
- `--audit-ip=full|truncated|off` : l'adresse IP en entier (par défaut), coupée à son /24 ou son /48, ou retirée.
- **Sa clé d'audit** : Ed25519, tirée à `init` (et au premier démarrage d'un serveur initialisé avant), gardée dans son dossier de données (`--data`, même avec PostgreSQL et S3), qui ne sert qu'à signer l'audit. `init` affiche sa clé publique ; `escouade-team-server audit-key` affiche l'id de l'émetteur (`server-` et 16 chiffres hexadécimaux), sa clé publique et le fichier de clés que lit `escouade-audit verify`. Un administrateur l'inscrit au journal des membres (AU-5).
- **Un tampon par sortie** dans le dossier de données : une sortie qui ne répond pas est réessayée, ses événements attendent 7 jours ou 1 Gio, puis `audit.dropped`. À l'arrêt, ce qui attend part ; au redémarrage, la chaîne reprend sans trou ni doublon. Une requête n'attend jamais l'audit.
- Ce qu'il émet, et comment, est dans « L'audit vers un SIEM », *Ce qu'émet le serveur seul*.
