---
title: "Le serveur d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team-server/README.md","crates/escouade-team-crypto/README.md","crates/escouade-team/README.md"]
tickets: ["ESC-456","ESC-458"]
decisions: [167,168]
compared: null
---

# Le serveur d'équipe

Un binaire à part, `escouade-team-server`, que l'équipe héberge elle-même (Docker avec Caddy, ou systemd). Il est distinct du mode serveur, qui lance des agents et ne s'expose pas. Son client est la crate `escouade-team` (TS-2, voir « Le client de l'équipe »), pas encore branchée dans l'app : l'interface viendra avec TS-3. Rien n'est publié avant la 2.3.

## Aveugle

- Il ne garde que des octets chiffrés de bout en bout (seul mode), et les métadonnées que voici : des ids opaques, les clés publiques des appareils, leurs rôles, les tailles, les heures, l'appareil qui a écrit.
- Il n'exécute rien (aucun agent, commande, plugin ni harness) et n'a aucune clé de contenu.
- Son journal et ses métriques Prometheus ne portent aucun id d'équipe, de projet, d'appareil ni de fichier, aucune adresse, aucun jeton : seulement les modèles des routes, les statuts et des compteurs.

## Les appareils et l'entrée

- Un appareil : un id, une clé publique Ed25519, une équipe, un rôle (administrateur ou membre), révoqué ou non. L'id d'un appareil sur le serveur est aussi celui de son identité dans le groupe MLS de l'équipe, mais sa clé pour le serveur n'est pas sa clé MLS.
- `escouade-team-server init` crée la base et affiche un jeton à usage unique : le premier appareil qui s'inscrit avec crée l'équipe et en devient administrateur. Une seconde utilisation est refusée.
- Un administrateur demande un jeton d'inscription par nouvel appareil (24 h par défaut, 7 jours au plus) : c'est le point d'entrée des invitations de TS-3.
- Un administrateur révoque un appareil de l'équipe, un appareil se révoque lui-même : ses sessions et ses connexions en direct se ferment aussitôt.
- Le rôle sur le serveur est celui de l'inscription : un membre nommé administrateur dans le registre de l'équipe ne l'est pas encore sur le serveur (TS-3).

## L'authentification

- Par défi signé : le serveur donne un défi (60 s, une seule fois), l'appareil le signe avec sa clé, le serveur rend un jeton de session de 15 minutes. Aucun mot de passe.
- Une mauvaise signature, un appareil révoqué ou une session expirée sont refusés.
- Toutes les routes demandent une session, sauf `/healthz`, l'inscription par jeton et le défi.

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
