---
number: 173
id: "entrer-dans-une-equipe-le-jeton-du-serve"
title: "Entrer dans une équipe : le jeton du serveur dérivé du secret du lien, la demande de jeton d'un membre dans le flux, le code court en SPAKE2, l'alerte qui arrête l'équipe"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-461"
pages: []
tickets: ["ESC-461"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/entree","added":49,"removed":0},{"page":"serveur-equipe/client","added":7,"removed":6},{"page":"serveur-equipe/serveur","added":14,"removed":7}]
accepted_at: "2026-11-15T09:47:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte



## Décision

- Le jeton d'inscription de l'invité est dérivé du secret du lien (HKDF) : l'administrateur en enregistre le SHA-256 auprès du serveur (POST /v1/devices/enrollments {token_hash}), le lien `https://<serveur>/join/<id>#s=…&t=…` suffit à l'invité, et le serveur ne voit jamais le secret. Un jeton sert une fois ; son second usage est refusé en 409 token_used (« Cette invitation a déjà servi ») et noté pour les administrateurs (GET /v1/devices/enrollments/alerts), dont l'app montre l'alerte sur l'invitation.
- Un membre qui n'est pas administrateur du serveur demande le jeton de son propre appareil dans le flux d'équipe : un objet scellé et signé par sa feuille, qui ne porte que l'empreinte du jeton ; un administrateur du serveur en ligne vérifie l'auteur et son droit, enregistre le jeton et le dit.
- Le code court : 8 caractères en base 32 de Crockford, 3 pour le créneau d'un relais en mémoire du serveur, 5 pour le mot de passe d'un SPAKE2 (crate spake2 de RustCrypto) ; un seul essai par créneau, confirmation de clé, réponse scellée sous la clé du PAKE.
- Les rôles du serveur suivent le registre : tout appareil administrateur du serveur corrige, à chaque nouvelle époque, le rôle des appareils dont le rôle au registre a changé (POST /v1/devices/{id}/role) ; le dernier administrateur du serveur reste.
- La récupération passe par un jeton que l'hébergeur donne (`escouade-team-server recovery-token --team <id>`) et les 24 mots : la feuille de secours revient, fait entrer l'appareil de la machine comme administrateur, qui remplace aussitôt la clé de secours.
- La clé de séquestre peut être faite dans l'app (sa graine montrée une fois au créateur) ; elle est montrée à chaque membre.
- Une fourche, un retour en arrière ou une entrée réécrite arrêtent l'équipe sur l'appareil : plus de lecture ni d'envoi (invitation, entrée d'un invité, nouvel appareil, clé de secours) jusqu'à « Réessayer » qui passe, ou « Retirer l'équipe de cet appareil ».
- Les clés vont au trousseau de l'app sous un préfixe propre au dossier de données ; http n'est admis que vers cette machine.
- Avant TS-4, ces écrans s'ouvrent par Réglages › Avancé › « Serveur d'équipe (aperçu) », par « Équipes… » du menu « + » de la barre des projets quand il montre un menu (des projets fermés : sinon « + » ouvre l'assistant, choix 47 inchangé), et par un lien escouade://team/join.

## Conséquences

- Le serveur garde les jetons servis (marqués, puis oubliés une semaine après leur terme) : de quoi dire « déjà utilisée » à l'invité légitime. Le schéma gagne deux colonnes (used_at, reused_at), ajoutées seules aux bases existantes.
- Une invitation n'est honorée que par l'appareil qui l'a faite : un autre administrateur en ligne ne la connaît pas (il faudrait la sceller pour les seuls administrateurs, plus tard).
- Le code court ne vaut que 25 bits : sa sûreté tient à l'essai unique par créneau ; c'est un point pour l'audit externe (TS-20), avec la demande de jeton dans le flux et la graine de séquestre montrée en clair.
- Les invitations n'ont pas encore de projets : il n'y a pas de projets d'équipe avant TS-5.
- Deux nouvelles dépendances : spake2 0.4 (MIT/Apache-2.0) et qrcode 0.14 sans image (le QR code en SVG). Brancher openmls dans l'app fait entrer deux avis RustSec dans cargo deny (crypto-bigint 0.5, par les courbes P-256/P-384/K-256 que la suite 0x0003 n'emploie pas ; proc-macro-error2, une macro de compilation) : ignorés dans deny.toml avec leur raison, faute de version compatible.
- Le groupe « Serveur d'équipe (aperçu) » de Réglages › Avancé partira quand l'équipe aura sa place (TS-4).
