---
number: 243
id: "un-temporaire-propre-a-chaque-ecriture-e"
title: "Un temporaire propre à chaque écriture, et les sauvegardes de l'état sérialisées"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-535"
pages: ["introduction/donnees-et-coupures"]
tickets: ["ESC-535","ESC-469","ESC-440"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/donnees-et-coupures","added":14,"removed":3}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

À la sortie pour une mise à jour, app.log du poste de l'utilisateur dit « cannot save state: Le fichier spécifié est introuvable. (os error 2) » (2026-11-17 21:13, 2026-11-18 08:10). paths::write_atomic écrivait par un temporaire au nom fixe <nom>.tmp : l'arrêt (Core::shutdown) et la sauvegarde périodique de 400 ms écrivaient l'état en même temps, l'une renommait le temporaire de l'autre. Reproduit par test : 105 à 155 sauvegardes ratées sur 240, et un tour sur deux à trois où le disque n'avait pas le dernier instantané pris. Les réglages (cinq écrivains), la liste des plugins, l'index de la mémoire et les vérifications des pages de la spec prenaient aussi leur instantané hors du verrou de leur écriture.

## Décision

1) write_atomic, write_private et write_cache écrivent par un temporaire propre à chaque écriture, <fichier>.<process>-<n>.tmp, ouvert en create_new ; un temporaire d'un autre process vieux d'une minute est effacé à la première écriture dans son dossier. Préféré à un verrou par chemin dans le process, parce que l'app et escouade-plugin écrivent tous deux le cache des modules WASM. 2) L'état prend son instantané et l'écrit sous un seul verrou du cœur (Core::saving) ; de même les réglages (settings_writing), plugins.json, l'index de la mémoire et les vérifications des pages de la spec. 3) La dernière sauvegarde à la sortie est réessayée une fois, chaque échec dit au journal.

## Conséquences

Aucune écriture concurrente d'un même fichier n'échoue plus, et le dernier instantané pris est le dernier écrit. Le forçage sur le disque d'ESC-469 ne change pas. Un .tmp au nom fixe laissé par une version d'avant n'est plus nettoyé. Les tests qui faisaient échouer une sauvegarde en posant un dossier à la place de state.tmp en posent un à la place de state.json.
