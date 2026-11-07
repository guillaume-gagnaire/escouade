---
number: 89
id: "la-revue-locale-recompile-les-run-wasm-s"
title: "La revue locale recompile les run.wasm : sur le poste pour la première main, dans des conteneurs pour un tiers"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-337"
pages: ["store/revue-et-signature","plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-337"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":10,"removed":0},{"page":"plugins/tous-les-plugins-en-wasm","added":2,"removed":0}]
accepted_at: "2026-11-05T15:26:14+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue ne savait recompiler qu'un run.js : pour un run.wasm, rien ne prouvait que les sources relues soient ce qui tourne, et sign refusait donc tout module Rust. Or compiler du Rust exécute en natif les build.rs et les macros, et un include_bytes! d'un chemin absolu ferait entrer un fichier du poste dans le module. Le poste qui relit porte la clé de signature.

## Décision

1. Un run.wasm est recompilé par escouade-plugin fetch puis build, les étapes du store. Même empreinte : ce qui a été lu est ce qui tourne. Autre empreinte ou autre lock : « Refuser », code 1. Autre rust, target ou canon, ou recompilation impossible : la revue continue, et sign refuse.
2. La première main (l'identifiant réservé et le dépôt de l'app, firstPartyBench) compile sur le poste. Un tiers ne compile jamais sur le poste nu : sans conteneur, il n'est pas recompilé.
3. Un tiers compile dans quatre conteneurs neufs, et non un seul comme le disait le §8.5.5 : préparer les volumes (root avec la seule capacité CHOWN, sans le plugin ni réseau), fetch avec réseau (rien du plugin ne s'exécute), build en --network=none avec les outils en lecture seule, lire le module par sa sortie standard. Un seul conteneur ne peut pas avoir le réseau pour fetch et s'en passer pour build.
4. Le conteneur ne voit ni HOME, ni ~/.escouade-signing, ni le dépôt : seulement les fichiers réguliers de l'archive et l'escouade-plugin Linux, en lecture seule. Aucun dossier du poste ne lui est ouvert en écriture.
5. Pas de bubblewrap dans le conteneur : il n'y tournerait pas sous un vrai Docker sans lever seccomp et AppArmor (ESC-356), et le conteneur est déjà le bac à sable.
6. L'image est rust:1.98.0-trixie (ESCOUADE_REVIEW_RUST_IMAGE), pour cc et une glibc récente ; son rustc ne sert pas, la chaîne d'outils reste celle épinglée par escouade-plugin.
7. Les pistes de Rust s'ajoutent : build.rs et macros, unsafe, inclusions hors du dossier, imports du module et méthodes nommées comparés aux capacités, Cargo.lock et son diff, code illisible.

## Conséquences

Relire un tiers en run.wasm demande un moteur de conteneur, un escouade-plugin Linux qui a la commande fetch (ESCOUADE_PLUGIN_LINUX) et 3 Gio pour la machine du moteur. Un volume garde la chaîne d'outils et les crates d'une revue à l'autre ; seul fetch y écrit. Les conteneurs n'ont pas de limite de disque. Les imports réels d'un module sont toujours les mêmes : la piste compare aussi les noms de méthodes écrits dans le module, et un nom calculé lui échappe, l'hôte restant celui qui refuse. Le banc d'un tiers suit la règle d'ESC-343 : joué comme l'outil du poste, un plugin qui demande 2.2.0 échoue au manifeste tant que l'outil dit 2.1.2.
