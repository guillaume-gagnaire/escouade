---
number: null
id: "la-2-3-de-main-et-la-2-2-reunies-par-un"
title: "La 2.3 de main et la 2.2 réunies par un merge : la structure de main, les contrôles de la 2.2 gardés"
date: "2026-11-14"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-442"
pages: ["harness/comptes","plugins/harness-acp-decrits","fenetre/reseau","store/store-dans-l-app"]
tickets: ["ESC-442","ESC-428","ESC-435","ESC-436","ESC-438"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":2,"removed":1},{"page":"plugins/harness-acp-decrits","added":2,"removed":2},{"page":"fenetre/reseau","added":7,"removed":3},{"page":"store/store-dans-l-app","added":20,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le 2026-11-14, l'utilisateur publie sur la 2.2 tout ce qui attend (bêta et 2.3), en 2.2.0-beta.15. main porte la 2.3 (crates du découpage, xterm 6, terminal et comptes lancés par les plugins : ESC-359, ESC-441). release/2.2 porte les correctifs de l'audit, repris pour ses propres chemins (ESC-428). La décision les-corrections-de-l-audit-dans-la-2-2-v notait que l'attach d'agent/open et la relance étaient jugés en 2.2, pas encore sur main.

## Décision

1. release/2.2 entre dans main par un vrai merge (ESC-442) ; main avance dessus, et release/2.2 avec. 2. La structure de main gagne : la vue terminal de chaque harness est lancée par son plugin. Les copies de la 2.2 dans le cœur ne reviennent pas : arguments de la vue écrits par le cœur, refus d'une vue sans clôture, hooks par chemin entier. Leurs correctifs vivent dans les plugins et dans escouade-acp. 3. Les deux contrôles que seule la 2.2 avait sont gardés sur main. La relance d'un agent (le programme suivant de son plugin) perd les drapeaux Bypass sans « Autoriser le mode Bypass ». Les variables de l'attach que rend agent/open sont jugées comme celles d'un lancement. 4. ESC-435 (SEC-27, SEC-28) et ESC-436/438 (SEC-29) entrent sur main, dans le crate escouade-plugins. 5. L'amorce est celle de la 2.2, plus Claude Code 2.2.7.

## Conséquences

Une seule branche à partir de la bêta 15. Le ticket pour main que la décision ESC-428 annonçait n'a plus lieu d'être. Les sections publiées du CHANGELOG restent telles quelles, et [Non publié] réunit les deux côtés. Les tests de la 2.2 qui visaient les copies du cœur partent avec elles. Le test du mode Bypass de la vue Claude Code reste, adapté au lancement par le plugin.
