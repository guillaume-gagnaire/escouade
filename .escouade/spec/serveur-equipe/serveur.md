---
title: "Le serveur d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team-server/README.md","crates/escouade-team-crypto/README.md","crates/escouade-team/README.md"]
tickets: ["ESC-456","ESC-458","ESC-461"]
decisions: [167,168,173]
compared: null
---

# Le serveur d'équipe

Un binaire à part, `escouade-team-server`, que l'équipe héberge elle-même (Docker avec Caddy, ou systemd). Il est distinct du mode serveur, qui lance des agents et ne s'expose pas. Son client est la crate `escouade-team` (voir « Le client de l'équipe »), branchée dans l'app : créer une équipe, y inviter et y entrer, voir « Entrer dans une équipe ». Rien n'est publié avant la 2.3.

## Aveugle

- Il ne garde que des octets chiffrés de bout en bout (seul mode), et les métadonnées que voici : des ids opaques, les clés publiques des appareils, leurs rôles, les tailles, les heures, l'appareil qui a écrit, l'empreinte (SHA-256) des jetons d'inscription.
- Il n'exécute rien (aucun agent, commande, plugin ni harness) et n'a aucune clé de contenu. Il ne voit jamais le secret d'une invitation ni un code court.
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

## La page d'un lien d'invitation

`/join/<id>`, ouvert dans un navigateur : une page sans ressource d'ailleurs, dont le script (par son nonce seul) lit le secret après le `#` dans le navigateur et passe le lien entier à Escouade (`escouade://team/join/…`). Le secret n'atteint jamais le serveur.

## L'authentification

- Par défi signé : le serveur donne un défi (60 s, une seule fois), l'appareil le signe avec sa clé, le serveur rend un jeton de session de 15 minutes. Aucun mot de passe.
- Une mauvaise signature, un appareil révoqué ou une session expirée sont refusés.
- Toutes les routes demandent une session, sauf `/healthz`, la page `/join/<id>`, l'inscription par jeton, le défi, et les étapes du nouvel appareil dans le relais d'appairage.

## Les flux et les fichiers

- Un flux par équipe (les messages MLS) et un par projet (les opérations chiffrées). Le serveur numérote chaque entrée, la garde et la diffuse ; un appareil relit depuis le numéro n pour reprendre.
- Les commits MLS : le premier pour une époque gagne. Un second pour la même époque est refusé, avec les entrées depuis le commit gagnant, pour que l'appareil rattrape.
- Les fichiers chiffrés (documents, images) : sur le disque ou dans un stockage compatible S3 (MinIO, Garage, AWS), écrits une fois, avec une taille maximale par fichier (25 Mio par défaut) et un quota par équipe (2 Gio par défaut) qui compte aussi les flux.
- La diffusion en direct par WebSocket : un appareil abonné reçoit les nouvelles entrées de tous les flux de son équipe, et rien des autres équipes.

## Le protocole et les défenses

- Messages en CBOR, objets chiffrés en binaire.
- Poignée de main des versions : le serveur accepte la version du client et la précédente ; une plus vieille est refusée avec « Mets Escouade à jour », une plus récente dit que le serveur est en retard.
- Une limite de débit par adresse et par appareil (429 et `Retry-After`).
- Des tailles bornées partout ; une longueur annoncée trop grande est refusée avant d'être lue.
- TLS par rustls avec un certificat donné, ou derrière un proxy inverse (Caddy dans l'exemple).
- La base : SQLite en WAL par défaut, PostgreSQL en option.
