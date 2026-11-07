---
number: 91
id: "le-conteneur-du-store-a-deux-profils-etr"
title: "Le conteneur du store a deux profils étroits, pas unconfined"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-356"
pages: ["store/publier-et-securite"]
tickets: ["ESC-356"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/publier-et-securite","added":8,"removed":0}]
accepted_at: "2026-11-05T15:44:37+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le store compile les run.wasm dans bubblewrap, sans repli (ESC-336). Le Docker par défaut le refuse deux fois : seccomp empêche les espaces de noms, AppArmor empêche les montages. L'utilisateur ne veut pas d'un conteneur entier sans filtre (2026-11-05 : « Profil seccomp étroit »).

## Décision

Le store fournit store/deploy/seccomp-bubblewrap.json (moby 29.9.3 plus clone à deux jeux exacts d'espaces de noms, unshare de CLONE_NEWUSER seul, mount, umount2, pivot_root) et store/deploy/apparmor-bubblewrap (docker-default moins deny mount, plus les montages de bubblewrap nommés un à un, tmpfs et devpts seuls). image-test.mjs et la CI les prennent. La sysctl kernel.apparmor_restrict_unprivileged_userns reste telle quelle, un profil nommé n'en a pas besoin. setns, clone3 et unshare -rn restent refusés. Le geste sur l'hôte et dans Coolify reste à l'utilisateur.

## Conséquences

Tant que l'hôte n'est pas réglé, chaque version run.wasm attend en error, comme aujourd'hui. Il faut Coolify 4.5.0 ou plus, sans serveur de build, car compose lit le fichier seccomp dans le conteneur d'aide de Coolify. Un changement de store/deploy/ se recopie à la main sur l'hôte. Tout process du conteneur peut créer un espace de noms utilisateur à ces jeux de drapeaux : une surface du noyau que Docker ferme par défaut. Un run.js garde le réseau du conteneur.
