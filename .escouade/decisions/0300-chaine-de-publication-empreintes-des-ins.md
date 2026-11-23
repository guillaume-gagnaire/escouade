---
number: 300
id: "chaine-de-publication-empreintes-des-ins"
title: "Chaîne de publication : empreintes des installateurs, contrôle de `testing`, secrets Apple à l'étape qui signe, précompilés vérifiés par leur empreinte"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-594"
pages: ["introduction/livraison","plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-594"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/livraison","added":6,"removed":0},{"page":"plugins/programmes-js-ts-en-wasm","added":3,"removed":0}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de sécurité de la 2.2.0 relève SEC-04 (avis npm du store et du site non bloquants, sans délai écrit ; raison des avis zip à récrire), SEC-16 (empreintes des installateurs non publiées) et SEC-18 (fonction `testing` non contrôlée, précompilés des plugins chargés sur la foi du fichier, secrets Apple dans l'environnement du job, trois variables lues par une version publiée). L'utilisateur a accepté les recommandations telles quelles le 2026-11-21 (a-7).

## Décision

Chaque release publie `SHA256SUMS.txt` (SHA-256 de chaque installateur et paquet), dit dans ses notes et sur la page Installation du site. La CI et la release échouent si une crate d'un build normal a `testing`. Les secrets Apple ne sont donnés qu'à l'étape qui construit et signe l'app macOS. Un module précompilé n'est chargé que si son SHA-256, écrit à côté de lui par l'app, concorde ; sinon il est refait. `npm audit` du store et du site échoue dès un avis élevé ; délai cible de 7 jours pour un avis élevé ou critique (docs/SECURITY-DEPENDENCIES.md). `ESCOUADE_TERM_TRACE` n'est lue qu'en développement, `ESCOUADE_ORT_DYLIB` qu'avec `ort-dynamic` ; `ESCOUADE_JAVY` reste lue (image du store) et est documentée.

## Conséquences

Un fichier de plus dans chaque release ; le job de publication télécharge les fichiers du brouillon pour les sommer (quelques minutes). L'étape de construction a deux formes (Developer ID ou non) à garder pareilles, ce qu'un test vérifie. Un précompilé d'avant la 2.2.6 est refait une fois (il l'est déjà au changement de version). Un avis élevé du store ou du site rend la CI des avis rouge jusqu'à sa correction.
