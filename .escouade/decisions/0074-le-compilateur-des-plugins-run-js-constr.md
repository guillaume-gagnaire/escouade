---
number: 74
id: "le-compilateur-des-plugins-run-js-constr"
title: "Le compilateur des plugins run.js construit dans l'image du store, et un escouade-js reproductible sous Linux"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-328"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-328"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-04T23:50:09+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'image du store n'avait ni escouade-plugin ni Javy : une version run.js attendait en `error`, et slack et team-rules 2.0.0 ne pouvaient pas être publiés. Construire l'escouade-plugin de l'app dans le build Docker aurait compilé toute l'app, GTK et WebKit compris. L'utilisateur a demandé le 2026-11-04 que la compilation soit en place avant la 2.2.0, donc sans attendre un asset de release. En le construisant, on a vu qu'escouade-js changeait à chaque construction : `javy init-plugin` n'était pas déterministe, et les chemins de la machine entraient dans le module. Le store et la revue locale n'auraient jamais comparé le même compilateur.

## Décision

1. `crates/escouade-plugin-cli`, sans Tauri, GTK, WebKit ni wasmtime, tient le lecteur du manifeste, la compilation et `validate`, `build`, `--version` ; l'app en dépend. `test` et `dev` restent dans l'app, car ils ont besoin de son hôte wasmtime. 2. `store/Dockerfile` construit ce crate et escouade-js depuis le dépôt, dans une étape Rust épinglée, et sa dernière étape passe sur glibc sans y lancer de commande. 3. `scripts/escouade-js.sh` donne les mêmes octets sous Linux où qu'on le construise (`init-plugin --deterministic`, les chemins retirés par un wrapper de rustc) ; la CI compare l'escouade-js de l'image au sien. 4. La référence est l'escouade-js de Linux : un poste Windows qui relit un plugin construit son escouade-plugin avec lui (`ESCOUADE_JS_WASM`).

## Conséquences

L'image passe de 222 à 337 Mo et son build à froid de 52 à 116 s sur le poste de 32 cœurs ; un changement du store seul ne rejoue aucune étape Rust. Les empreintes de modules changent une fois avec le nouvel escouade-js, sans effet sur ce qui est publié. La version du crate suit celle de l'app (`scripts/version.mjs`). Les chemins surveillés de Coolify ne couvrent pas le crate : à compléter à la main. Sous Docker avec son seccomp par défaut, `unshare -rn` est refusé : la compilation se fait avec le réseau du conteneur, le code du plugin restant dans le bac à sable WASM ; obtenir l'espace de noms demande une option de conteneur dans Coolify, à décider par l'utilisateur. Pour ESC-330, l'étape Rust a déjà `wasm32-wasip1` et le crate reçoit une commande de plus dans son `match`.
