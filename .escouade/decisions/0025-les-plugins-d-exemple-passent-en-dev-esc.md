---
number: 25
title: "Les plugins d'exemple passent en dev.escouade.* dans plugins/<id>/, et le préfixe réservé est accordé au seul dépôt de l'app"
date: "2026-10-31"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-314"
pages: ["plugins/plugins-d-exemple","store/publier-et-securite","store/store-dans-l-app"]
tickets: ["ESC-314","ESC-210","ESC-313"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/plugins-d-exemple","added":17,"removed":14},{"page":"store/publier-et-securite","added":9,"removed":4},{"page":"store/store-dans-l-app","added":5,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur veut que les plugins d'exemple (Contraste élevé, Discord, Slack, Règles de l'équipe), publiés au store sous dev.exemple.* depuis quatre dépôts escouade-dev/plugin-*, prennent les identifiants dev.escouade.* (2026-10-31). La décision 21 (point 7) dit que la première main passe par la même soumission en dossier que les tiers et renvoyait le câblage du préfixe réservé à ESC-210 ; jusqu'ici le store refusait dev.escouade.* à toute soumission, et le validateur du manifeste (Rust, JS, SDK) ne l'acceptait que pour un plugin livré par l'app (builtin), qui prend aussi les programmes {escouade}/….

## Décision

1. Les quatre plugins sont dev.escouade.{high-contrast,discord,slack,team-rules} 1.1.0, dans plugins/<id-court>/ du dépôt de l'app (discord-webhook devient discord), chacun avec son manifeste (repository = …/tree/main/plugins/<id-court>), son README, sa LICENSE MIT avec la ligne SPDX-License-Identifier (la racine est GPL-3.0), son CHANGELOG et ses tests (node --test pour les deux déclaratifs). plugin-examples/ disparaît ; sa CI devient plugins/ci.mjs. Étiquettes plugins/<id-court>/v1.2.3.
2. Store : dev.escouade.* n'est accepté que d'une soumission du dépôt de l'app, donné par STORE_FIRST_PARTY_REPO (défaut guillaume-gagnaire/escouade) et comparé par son id GitHub (isFirstParty), jamais par son nom ; tout autre dépôt reçoit le refus d'avant. builtin.ts garde la réserve et la liste des plugins livrés, sans champ source ; isBuiltinId ne vaut plus que pour cette liste, si bien qu'un dev.escouade.* publié depuis plugins/<id>/ est listé au catalogue comme un plugin du store.
3. Validateur : un troisième niveau, la première main (Hand::First en Rust, firstParty en JS), qui prend les ids dev.escouade.* sans les programmes de l'app ; le banc et escouade-plugin le prennent avec --first-party.
4. App : un dev.escouade.* s'installe depuis le store signé seulement (refusé non signé, et refusé d'un dossier copié ou d'un dépôt cloné ; permis en développement). Il garde l'origine store, rangé dans « Ajoutés » ; un plugin livré a l'origine builtin (INT-4). La vue les trie par l'origine, jamais par l'id.
5. Les tests et fixtures qui lisent ces dossiers ou parlent de ces plugins passent aux nouveaux ids ; ceux qui jouent un plugin de n'importe qui (copies des faux stores, liens profonds, clés, recommandations) gardent dev.exemple.*. Les faux stores gagnent un vrai cas de première main (dev.escouade.discord signé, dev.escouade.slack non signé).

## Conséquences

Les gestes de mise en ligne sont au chef (docs/STORE.md §9.2) : étiquettes plugins/<id-court>/v1.1.0, App du store installée sur le dépôt de l'app, soumission, revue et signature des quatre dev.escouade.*, révocation d'essai puis retrait des dev.exemple.* (étape 8 de §9.1), archivage des dépôts escouade-dev/plugin-*. Un utilisateur qui avait un dev.exemple.* le remplace depuis la vue Plugins. Le store fait un appel GitHub de plus par soumission d'un id réservé. La revue ne compte plus le repository du manifeste comme hôte ni dev.escouade. comme dossier d'Escouade. ESC-210 n'a plus à câbler la réserve ; l'origine builtin reste à INT-4 (le type de la fenêtre la porte déjà).
