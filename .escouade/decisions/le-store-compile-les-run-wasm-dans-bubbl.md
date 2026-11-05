---
number: null
id: "le-store-compile-les-run-wasm-dans-bubbl"
title: "Le store compile les run.wasm dans bubblewrap, sans repli, et refuse run.command à tous"
date: "2026-11-05"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-336"
pages: ["store/publier-et-securite","plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-336"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/publier-et-securite","added":11,"removed":0},{"page":"plugins/tous-les-plugins-en-wasm","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Compiler du Rust exécute en natif les build.rs et les macros des dépendances, sous l'utilisateur du store, qui pourrait lire ses secrets dans /proc. La décision 43 suffisait pour run.js, dont le code ne sort pas du bac à sable WASM : plus pour run.wasm. Le Docker par défaut refuse les espaces de noms utilisateur.

## Décision

1. Le onzième contrôle compile un run.wasm, une fois tous les autres contrôles passés : les fichiers de l'archive figée dans un dossier neuf, escouade-plugin fetch avec réseau, escouade-plugin build sans réseau, deux fois ; deux modules différents refusent la version.
2. Le téléchargement est une commande d'escouade-plugin, fetch, pas un cargo lancé par le store : ses refus passent avant cargo, sur le même CARGO_HOME. Il est refait à chaque compilation, pour qu'elles n'aient rien de commun. Il n'est pas filtré par hôte, écart avec le §8.5.3 : ce sont les refus qui gardent cargo sur crates.io.
3. La compilation tourne dans bubblewrap, sur une racine à part (debian:bookworm-slim, gcc, la chaîne d'outils, escouade-plugin), sans Node ni le store, avec un /etc vide, sans HOME ni variable du store. Bubblewrap lui-même démarre avec PATH seul, car son pid 1 garde son environnement de départ, lisible dans /proc/1/environ. Il vient en plus de la liste fermée d'A5.
4. Sans repli : là où bubblewrap ne tourne pas, une version run.wasm attend en error, la première main comprise. STORE_COMPILE_ISOLATION=none n'est entendu que hors production.
5. Un run.wasm a 5 minutes, 2 minutes pour fetch, 2 Gio de mémoire et 4 Gio de disque. Le délai de la file store.check passe à 30 minutes.
6. run.command, run.windows, run.macos et run.linux sont refusés à tout plugin, la première main comprise, et le store ne lit plus de release (décision 72). Les versions déjà publiées gardent leurs binaires. Le reste du code des attestations part à Z3.

## Conséquences

L'image du store passe de 338 Mo à 1,32 Go ; son build à froid de 139 s à 146 s. Sous le Docker par défaut de Coolify, aucun run.wasm ne compile : il faut --security-opt seccomp=unconfined (essayé sur Docker 27) ou un profil plus étroit (pas essayé), plus kernel.apparmor_restrict_unprivileged_userns=0 sur un hôte Ubuntu 24.04. Ce réglage reste à décider par l'utilisateur. Les dépendances se téléchargent deux fois par version. Une version run.wasm ne se signe qu'après A7.
