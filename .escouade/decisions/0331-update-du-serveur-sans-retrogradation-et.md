---
number: 331
id: "update-du-serveur-sans-retrogradation-et"
title: "Update du serveur sans rétrogradation et image épinglée"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-640"
pages: ["a-distance/mode-serveur","introduction/livraison"]
tickets: ["ESC-640","ESC-624"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/mode-serveur","added":18,"removed":13},{"page":"introduction/livraison","added":19,"removed":3}]
accepted_at: "2026-11-26T08:27:11+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-624, SR1-07, 12, 13, 14) a trouvé que `escouade-server update` installait une version plus ancienne (de `latest.json`, `beta.json` ou `--to`) et que sa signature minisign ne liait pas l'archive à sa version (le commentaire de confiance `file:…` n'était jamais lu) : un miroir ou un `latest.json` changé pouvait servir une ancienne version signée et vulnérable, ou l'archive signée d'une version sous le nom d'une autre. L'image Docker partait de bases non épinglées, les Compose tiraient `:latest`, `ESCOUADE_SERVER_LISTEN=0.0.0.0` mettait l'API en clair sur le réseau du conteneur sans le dire ; un serveur lancé à la main laissait son journal en 0644 ; heapless 0.8 « unsound » venait de smoltcp 0.12 du cœur.

## Décision

1. `escouade-server update` ne vise jamais une version plus ancienne que la sienne (une bêta compte avant sa version), sauf nommée par `--to X.Y.Z --downgrade` ; `--downgrade` sans `--to` est refusé. Avec `--beta`, la plus haute de `latest.json` et `beta.json`. 2. Le commentaire de confiance de la signature doit nommer l'archive attendue (`file:escouade-server-<v>-linux-<arch>.tar.gz`), sinon refus. 3. L'app de bureau garde la même faiblesse de principe (la signature de Tauri ne couvre pas la version de `latest.json`) : documentée dans la spec avec ce qui la borne (HTTPS vers github.com, installeurs signés seulement, retour en arrière vu au démarrage par `update.json`, rien qui tienne), sans correction dans ce ticket. 4. Les bases de l'image épinglées par `@sha256:` (Dependabot docker les monte), les images stables étiquetées aussi de leur mineure (`2.3`), les Compose sur `:2.3` ; le Dockerfile, les Compose et la doc disent que `0.0.0.0` met l'API en clair sur le réseau du conteneur et conseillent le proxy TLS ou le tunnel. 5. Le journal du serveur en 0600 (Unix), le dossier de données en 0700 (déjà fait par `DataDir::ensure`, désormais testé côté serveur). 6. Le cœur sur smoltcp 0.14 ; `cargo audit` a ses ignores justifiés dans `src-tauri/.cargo/audit.toml`, les mêmes raisons que `deny.toml`.

## Conséquences

Un serveur ne peut plus être ramené en arrière par un miroir ou un `latest.json` changé ; revenir en arrière demande `--to X.Y.Z --downgrade` en clair. Une image `:2.3` est publiée à chaque version stable de la 2.3 ; passer à la 2.4 avec Compose demande de changer la ligne `image:`. Les empreintes des bases doivent être montées (Dependabot, chaque mois). La correction de l'app de bureau (lire `Update.signature` avant l'installation) reste à faire dans un ticket à part.
