---
title: "Le serveur d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team-server/README.md","crates/escouade-team-crypto/README.md","crates/escouade-team/README.md","crates/escouade-team-server/Dockerfile","crates/escouade-team-server/deploy/coolify.yml",".github/workflows/release.yml","website/content/docs/fr/team/install.md"]
tickets: ["ESC-456","ESC-458","ESC-461","ESC-466","ESC-586","ESC-637","ESC-633"]
decisions: [167,168,173,178,277,288,286]
compared: null
---

# Le serveur d'équipe

Un binaire à part, `escouade-team-server`, que l'équipe héberge elle-même (Docker avec Caddy, Coolify, ou systemd). Il est distinct du mode serveur, qui lance des agents et ne s'expose pas. Son client est la crate `escouade-team` (voir « Le client de l'équipe »), branchée dans l'app : créer une équipe, y inviter et y entrer, voir « Entrer dans une équipe » ; la synchronisation des projets, voir « La synchronisation des projets ». Publié avec chaque version d'Escouade depuis la 2.3 (voir *La publication*).

## Aveugle

- Il ne garde que des octets chiffrés de bout en bout (seul mode), et les métadonnées que voici : des ids opaques, les clés publiques des appareils, leurs rôles, les tailles, les heures, l'appareil qui a écrit, l'empreinte (SHA-256) des jetons d'inscription.
- Il n'exécute rien (aucun agent, commande, plugin ni harness) et n'a aucune clé de contenu. Il ne voit jamais le secret d'une invitation ni un code court : le lien d'une invitation (`escouade://team/join/…`) ne passe par aucune de ses pages.
- D'une opération d'un projet, il ne voit que son époque, sa taille, son heure et l'appareil qui l'envoie.
- Son journal et ses métriques Prometheus ne portent aucun id d'équipe, de projet, d'appareil ni de fichier, aucune adresse, aucun jeton : seulement les modèles des routes, les statuts et des compteurs.

## Les appareils et l'entrée

- Un appareil : un id, une clé publique Ed25519, une équipe, un rôle (administrateur ou membre), révoqué ou non. L'id d'un appareil sur le serveur est aussi celui de son identité dans le groupe MLS de l'équipe, mais sa clé pour le serveur n'est pas sa clé MLS.
- `escouade-team-server init` crée la base et affiche un jeton à usage unique : le premier appareil qui s'inscrit avec crée l'équipe et en devient administrateur.
- Un administrateur demande un jeton d'inscription par nouvel appareil (24 h par défaut, 7 jours au plus), ou enregistre l'empreinte d'un jeton qu'il a fait lui-même : celui d'une invitation, dérivé du secret après le `#`.
- **Un jeton sert une fois** : un second usage est refusé en disant « déjà utilisé » (et non « inconnu »), et noté ; les administrateurs de l'équipe lisent ces seconds usages. Un jeton expiré est inconnu.
- `escouade-team-server recovery-token --team <id>` affiche un jeton qui inscrit un appareil dans une équipe existante comme administrateur du serveur : pour le détenteur de la clé de secours, le jour où tous les administrateurs ont perdu leurs appareils. Il ne donne aucune clé.
- Un administrateur révoque un appareil de l'équipe, un appareil se révoque lui-même : ses sessions et ses connexions en direct se ferment aussitôt.
- Un administrateur change le rôle d'un appareil sur le serveur : c'est ainsi que les rôles du serveur suivent le registre de l'équipe. Le dernier administrateur non révoqué ne devient jamais membre.

## L'appairage par code court

- Un relais en mémoire pour « Ajouter un appareil » par un code de 8 caractères : l'appareil dans l'équipe ouvre un créneau (3 caractères, la tête du code), le nouvel appareil, pas encore inscrit, y fait **un seul essai**, puis prend la réponse scellée. Un créneau dure 10 minutes ; quatre au plus par équipe.
- Il ne porte que les messages d'un PAKE (SPAKE2) et une réponse scellée : rien qui permette au serveur d'essayer des codes.

## La page d'un lien d'invitation de l'ancienne forme

`/join/<id>`, ouvert dans un navigateur (un lien `https://<serveur>/join/<id>#…` d'avant la 2.3) : un **texte fixe**, le même pour tout id, qui dit que ce lien ne sert plus et qu'il faut en demander un neuf, à cliquer ou coller dans Escouade. **Aucun script** (sa politique de sécurité les interdit tous), aucune ressource d'ailleurs, aucun lien, rien du lien lu. Une page du serveur ne touche jamais le secret d'une invitation : le script d'une page qu'il sert pourrait lire ce qui suit le `#` dans le navigateur, et le serveur piraté de la menace n°1 entrerait à la place de l'invité.

## L'authentification

- Par défi signé : le serveur donne un défi (60 s, une seule fois), l'appareil le signe avec sa clé, le serveur rend un jeton de session de 15 minutes. Aucun mot de passe.
- Une mauvaise signature, un appareil révoqué ou une session expirée sont refusés.
- Toutes les routes demandent une session, sauf `/healthz`, la page `/join/<id>`, l'inscription par jeton, le défi, et les étapes du nouvel appareil dans le relais d'appairage.

## Les flux et les fichiers

- Un flux par équipe (les messages MLS) et un par projet (les opérations chiffrées, et les commits du groupe d'un projet gardé à quelques membres). Le serveur numérote chaque entrée, la garde et la diffuse ; un appareil relit depuis le numéro n pour reprendre.
- Les commits MLS, de l'équipe comme du groupe d'un projet : le premier pour une époque gagne. Un second pour la même époque est refusé, avec les entrées depuis le commit gagnant, pour que l'appareil rattrape.
- **Les instantanés d'un projet** (SR2-01) : un appareil du projet envoie l'état chiffré du projet jusqu'à un numéro. Le serveur le garde à côté des précédents, et ses octets comptent dans le quota.
  - **Rien n'est oublié à sa pose.** Les opérations qu'il couvre restent **30 jours**, puis le serveur les oublie (pas les commits) et rend leurs octets au quota, avec les instantanés d'avant. Il ne le fait jamais pour un instantané qu'un appareil a signalé comme illisible.
  - Un instantané qui n'est pas après le dernier, ou qui va au-delà du flux, est refusé.
  - **Aucun instantané sur les flux de l'équipe** : la liste des projets, les réglages imposés, les propositions, les coûts, le store privé, le coupe-circuit. Le serveur le refuse (`403`), à un administrateur comme à un membre.
  - La lecture d'un projet dit où est le dernier instantané : quand il est après le numéro demandé (l'appareil en part), et toujours (l'appareil déjà au-delà le vérifie une fois). Un appareil peut demander l'instantané d'avant un numéro.
  - Un appareil du projet signale un instantané illisible. Les administrateurs de l'équipe lisent la liste de ces instantanés : le projet, le numéro, l'appareil qui l'a posé et quand, le premier appareil qui l'a signalé et quand. Les membres ne la lisent pas.
  - **Un administrateur retire un instantané** : l'instantané d'avant et le journal reprennent sa place, ses octets reviennent au quota. Le serveur refuse de retirer celui sur lequel le flux a déjà été élagué, puisque ses opérations sont parties.
- Les fichiers chiffrés (documents, images) : sur le disque ou dans un stockage compatible S3 (MinIO, Garage, AWS), écrits une fois, avec une taille maximale par fichier (25 Mio par défaut) et un quota par équipe (2 Gio par défaut) qui compte aussi les flux et les instantanés.
- La diffusion en direct par WebSocket : un appareil abonné reçoit les nouvelles entrées de tous les flux de son équipe, et rien des autres équipes.

## Le protocole et les défenses

- Messages en CBOR, objets chiffrés en binaire.
- Poignée de main des versions : le serveur accepte la version du client et la précédente ; une plus vieille est refusée avec « Mets Escouade à jour », une plus récente dit que le serveur est en retard.
- Une limite de débit par adresse et par appareil (429 et `Retry-After`).
- Des tailles bornées partout ; une longueur annoncée trop grande est refusée avant d'être lue.
- TLS par rustls avec un certificat donné, ou derrière un proxy inverse (Caddy dans l'exemple, Traefik sous Coolify).
- La base : SQLite en WAL par défaut, PostgreSQL en option. Une base d'avant la rétention garde ses instantanés : le flux y est déjà élagué jusqu'à eux.

## La publication

- Chaque version d'Escouade publie le serveur d'équipe à son numéro (le même que l'app), par le Release comme `escouade-server` (voir *Livraison et mises à jour*, *CI/CD*) : pour Linux x64 et arm64, une archive `escouade-team-server-<version>-linux-<arch>.tar.gz` (le programme, sa licence, les exemples de `deploy/` : systemd, compose avec Caddy, Coolify), son SHA-256 et sa signature par la clé des mises à jour ; et l'image `ghcr.io/guillaume-gagnaire/escouade/escouade-team-server`, étiquetée de la version, de `latest` pour une version stable seulement et de `beta` pour une bêta. Archives et image portent une attestation de provenance.
- Le binaire est essayé dans un conteneur Debian nu avant tout envoi : sa version, `init` (son jeton jamais au journal), `/healthz` sur 127.0.0.1, un client sans la poignée de main refusé en 426, l'arrêt propre à SIGTERM.
- L'image (Debian 12 slim) tourne sous son propre utilisateur (`escouade-team`), écrit dans `/data` seul (`ESCOUADE_TEAM_DATA`), écoute sur `0.0.0.0:8443` de son conteneur, et se construit depuis les sources ou depuis l'archive d'une version (`PROGRAM=archive`, celle du Release).
- Les exemples tirent l'image publiée à l'étiquette de `ESCOUADE_TEAM_TAG` (`latest` par défaut ; `beta` ou une version pendant une bêta) : le compose derrière Caddy, qui tient TLS ; le compose de Coolify, derrière son Traefik (domaine de `SERVICE_FQDN_TEAM_8443`, `ESCOUADE_TEAM_TRUST_PROXY=1`, lecture seule, aucune capacité, aucun port publié). Leur mode d'emploi est dans la doc du site (Installer le serveur d'équipe), en français et en anglais.
