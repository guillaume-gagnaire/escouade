---
number: 86
id: "le-store-compile-les-run-wasm-dans-bubbl"
title: "Le store compile les run.wasm dans bubblewrap, sans repli, et refuse run.command à tous"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-336"
pages: ["store/publier-et-securite","plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-336"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/publier-et-securite","added":13,"removed":0},{"page":"plugins/tous-les-plugins-en-wasm","added":6,"removed":0}]
accepted_at: "2026-11-05T13:21:39+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Compiler du Rust exécute en natif les build.rs et les macros des dépendances, sous l'utilisateur du store, qui pourrait lire ses secrets dans /proc. La décision 43 suffisait pour run.js, dont le code ne sort pas du bac à sable WASM : plus pour run.wasm. Le Docker par défaut refuse les espaces de noms utilisateur, et sur le runner de la CI bubblewrap a échoué avec un --proc /proc, même seccomp et AppArmor levés.

## Décision

1. Le onzième contrôle compile un run.wasm, une fois tous les autres contrôles passés : les fichiers de l'archive figée dans un dossier neuf, escouade-plugin fetch avec réseau, escouade-plugin build sans réseau, deux fois ; deux modules différents refusent la version.
2. Le téléchargement est une commande d'escouade-plugin, fetch, pas un cargo lancé par le store : ses refus passent avant cargo, sur le même CARGO_HOME. Il est refait à chaque compilation, pour qu'elles n'aient rien de commun. Il n'est pas filtré par hôte, écart avec le §8.5.3 : ce sont les refus qui gardent cargo sur crates.io.
3. La compilation tourne dans bubblewrap, sur une racine à part (debian:bookworm-slim, gcc, la chaîne d'outils, escouade-plugin), sans Node ni le store, avec un /etc vide, sans HOME ni variable du store. Bubblewrap lui-même démarre avec PATH seul, car son pid 1 garde son environnement de départ. Il vient en plus de la liste fermée d'A5.
4. Le bac à sable n'a aucun /proc. Un --ro-bind /proc ne laisse pas lire /proc/<pid>/environ, mais montre les process du conteneur, leurs lignes de commande et son réseau : ne rien monter est plus strict et ne demande aucune option de plus. escouade-plugin prend donc son chemin dans argv[0], rustc ses bibliothèques par LD_LIBRARY_PATH, et le ld.lld de la chaîne d'outils, qui lie build.rs et macros, est un script dans l'image. L'empreinte des modules ne change pas.
5. Sans repli : là où bubblewrap ne tourne pas, une version run.wasm attend en error, la première main comprise, et le démarrage du store comme l'erreur de la version disent la sortie de bwrap. STORE_COMPILE_ISOLATION=none n'est entendu que hors production.
6. Un run.wasm a 5 minutes, 2 minutes pour fetch, 2 Gio de mémoire et 4 Gio de disque. Le délai de la file store.check passe à 30 minutes.
7. run.command, run.windows, run.macos et run.linux sont refusés à tout plugin, la première main comprise, et le store ne lit plus de release (décision 72). Les versions déjà publiées gardent leurs binaires. Le reste du code des attestations part à Z3.

## Conséquences

L'image du store passe de 338 Mo à 1,32 Go ; son build à froid de 139 s à 146 s. Sous le Docker par défaut de Coolify, aucun run.wasm ne compile : options attendues sur Ubuntu 24.04, seccomp=unconfined et apparmor=unconfined, plus kernel.apparmor_restrict_unprivileged_userns=0 sur l'hôte, à confirmer par le run de CI ; ESC-356 en tire un profil étroit. Ce réglage reste à décider par l'utilisateur. Une dépendance native qui exigerait /proc serait refusée par erreur de compilation : à voir sur les vrais plugins au portage. Les dépendances se téléchargent deux fois par version. Une version run.wasm ne se signe qu'après A7.
