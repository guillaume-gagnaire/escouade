---
number: 167
id: "serveur-d-equipe-ts-1-rusqlite-et-tokio"
title: "Serveur d'équipe (TS-1) : rusqlite et tokio-postgres au lieu de sqlx"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-456"
pages: ["serveur-equipe/serveur"]
tickets: ["ESC-456"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/serveur","added":27,"removed":0}]
accepted_at: "2026-11-14T22:48:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

docs/TEAM-SERVER.md §14 proposait sqlx pour SQLite et PostgreSQL. Mais libsqlite3-sys déclare links = "sqlite3" : un workspace n'en tient qu'une version. Le verrou de l'app tient la 0.39.1 (rusqlite 0.41) ; sqlx 0.8.6 demande la 0.30.1 et sqlx 0.9.0 une version >=0.30.1, <0.38.

## Décision

Le binaire escouade-team-server (ESC-456) prend rusqlite 0.41 pour SQLite (en WAL), à la version du verrou, et tokio-postgres pour PostgreSQL, tous deux derrière le trait db::Store. Il porte son propre transport (CBOR, versions, bornes, débit, TLS) dans un module sans dépendance au reste de la crate, à extraire en crate commune avec le mode serveur (ESC-451). Le serveur a ses propres rôles (administrateur, membre) en défense en profondeur ; un administrateur émet les jetons d'inscription des appareils suivants, point d'entrée de TS-3.

## Conséquences

Aucune version existante du verrou ne change (seules des crates nouvelles entrent : axum, ciborium, tokio-postgres…), cargo deny reste vert. Le SQL est écrit deux fois (SQLite et PostgreSQL), testé deux fois (variantes postgres:: quand ESCOUADE_TEST_POSTGRES_URL est posée, service postgres dans le job linux-rust de la CI). Cette version tient en un seul processus : sessions, limites et diffusion en mémoire.
