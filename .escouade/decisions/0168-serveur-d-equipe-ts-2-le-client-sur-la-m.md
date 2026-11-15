---
number: 168
id: "serveur-d-equipe-ts-2-le-client-sur-la-m"
title: "Serveur d'équipe (TS-2) : le client sur la maquette, son état en SQLite chiffré, le commit appliqué après le serveur"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-458"
pages: ["serveur-equipe/client","serveur-equipe/serveur"]
tickets: ["ESC-458"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/client","added":38,"removed":0},{"page":"serveur-equipe/serveur","added":3,"removed":2}]
accepted_at: "2026-11-15T01:04:36+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-2 (ESC-458) fait le client du serveur d'équipe, sur MLS (C9-B). Il fallait choisir : reprendre la maquette (escouade-team-crypto, TS-0a) dans la nouvelle crate ou en faire sa couche cryptographique ; où garder l'état d'openmls (openmls_sqlite_storage demande rusqlite 0.37, alors que le verrou tient libsqlite3-sys 0.39.1, une seule version par workspace) ; comment ne jamais tenir deux époques divergentes quand le serveur refuse un commit (409 epoch_taken) ; comment éviter les fausses alertes de fourche.

## Décision

- escouade-team-crypto reste une crate à part, pure et synchrone, et devient la couche cryptographique du client escouade-team : TeamDevice est générique sur son fournisseur openmls ; les vecteurs et les tests de la maquette restent les siens.
- L'état d'openmls et du client est notre propre StorageProvider sur rusqlite 0.41 (une table clé-valeur, appel pour appel comme MemoryStorage), un fichier par équipe, chaque valeur chiffrée sous une clé d'état du trousseau ; secure_delete, max_past_epochs 0 et le journal WAL tronqué à chaque époque effacent les clés passées.
- Les clés d'appareil (une clé pour le serveur, une autre pour la feuille MLS, la clé d'état) et le point de contrôle vont au trousseau par un trait Keychain que le cœur fournira avec le SecretStore ; sans trousseau, un fichier lisible du seul propriétaire, que l'interface signale.
- Un commit est d'abord préparé, puis appliqué seulement une fois pris par le serveur ; refusé, il est abandonné, le gagnant appliqué, puis le changement refait s'il reste à faire.
- Une balise de tête ne compte que signée par un membre de l'époque courante, et un objet doit être signé avant qu'on compare sa tête.

## Conséquences

La crate n'ajoute aucune version au verrou et cargo deny l'accepte. Le stockage suit MemoryStorage d'openmls : il faudra le relire à chaque mise à jour d'openmls (avant la 1.0). Côté serveur, il reste pour TS-3 : laisser un membre couper son propre autre appareil, et aligner le rôle serveur d'un appareil sur le registre ; d'ici là, ces cas s'en remettent au commit seul. Une fourche détectée arrête la lecture du flux jusqu'à ce qu'un humain s'en occupe (TS-3 montrera l'alerte).
