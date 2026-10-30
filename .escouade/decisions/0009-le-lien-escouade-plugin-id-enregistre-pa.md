---
number: 9
title: "Le lien escouade://plugin/<id> : enregistré par les installeurs, il montre un plugin et n'installe jamais rien"
date: "2026-10-30"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-287"
pages: ["store/store-dans-l-app"]
tickets: ["ESC-287"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":6,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

STORE-4 et STORE-9 citaient « Ouvrir dans Escouade » (escouade://plugin/<id>) sans que l'app le sache recevoir : seul single-instance était branché. STORE-11 (ESC-287) l'enregistre ; il restait à trancher ce qu'un lien a le droit de faire, comment il arrive à l'instance ouverte, et ce qui est dit quand il est mauvais.

## Décision

Le schéma escouade est déclaré dans tauri.conf.json (plugin deep-link) : NSIS/MSI l'écrivent au registre de l'utilisateur, le .app dans CFBundleURLTypes, .deb/.rpm dans le .desktop (MimeType, Exec %u ajouté au modèle) ; une AppImage s'enregistre au démarrage. Le lien arrive par argv (premier lancement, et relayé par single-instance sous Windows et Linux) ou par un événement Apple (macOS), passe par une boîte que la fenêtre relève (rien ne se perd avant qu'elle écoute). Seul escouade://plugin/<id> est lu (id au format du manifeste, / final et %-codage tolérés, 300 caractères) : il ouvre la page du plugin dans la vue Plugins, celle d'installé s'il l'est, et n'installe ni ne télécharge rien. Tout autre lien du schéma est dit dans une alerte et rien d'autre n'est fait ; un id valide absent du catalogue est dit sur la page.

## Conséquences

Un nouveau type de lien (un projet, un ticket) devra s'ajouter à deeplink::parse, qui refuse tout le reste. Le relais à l'instance ouverte ne se teste pas en e2e (single-instance est coupé dans les lancements isolés) : il passe par la même lecture d'argv, testée en Rust. Escouade ne livre que NSIS sous Windows et deb/AppImage sous Linux ; MSI et rpm l'enregistreraient aussi s'ils étaient construits.
