---
number: 21
title: "Store : plusieurs plugins par dépôt, leurs étiquettes et le changement de source"
date: "2026-10-31"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-313"
pages: ["store/publier-et-securite","store/store-dans-l-app"]
tickets: ["ESC-313","ESC-210"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/publier-et-securite","added":16,"removed":9},{"page":"store/store-dans-l-app","added":4,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le store ne lisait qu'un plugin à la racine d'un dépôt (§9 de docs/STORE.md, « plus tard »). L'utilisateur veut soumettre un plugin depuis un dossier d'un dépôt (2026-10-31), d'abord pour le dépôt de l'app, qui portera une vingtaine de plugins de première main sous plugins/<id>/ (ESC-210, décision 8 de TODO-2.2) et dont les étiquettes v* sont les releases de l'app (release.yml). Il faut une convention d'étiquettes lisible par l'humain et par le webhook, une règle pour le dossier déplacé, et dire comment la première main passe au store.

## Décision

1. Un plugin est (dépôt, dossier), la racine valant ''. Le dossier se choisit à la soumission (listé ou tapé, la racine par défaut) ; un dossier absolu, avec .., par un lien symbolique ou sans manifeste est refusé. Il est lu par l'arbre du commit et ses seuls fichiers (jamais l'archive du dépôt entier, sauf arbre coupé par GitHub), et le store garde l'archive du dossier seul.
2. Étiquettes : un plugin à la racine prend v1.2.3 / 1.2.3 (et, seul plugin du dépôt, toute étiquette comme avant). Un plugin en dossier prend <dossier>/v1.2.3 (façon Go, conseillée et seule écrite par create-escouade-plugin), <nom>@1.2.3 ou <nom>-v1.2.3 (le nom = dernier segment du dossier). Une étiquette nue v… n'est jamais celle d'un plugin en dossier : dans le dépôt de l'app, v2.1.1 reste la release de l'app et ne déclenche aucune vérification ; plugins/<id>/v1.2.3 ne commence pas par v et ne déclenche pas release.yml (seul workflow sur étiquette, tags: ['v*']). Le webhook relie l'étiquette au plugin dont elle nomme le dossier ; une étiquette qui en nomme plusieurs n'est vérifiée pour aucun, une version refusée le dit à chaque auteur.
3. README et LICENSE : ceux du dossier gagnent ; ceux de la racine les remplacent quand le dossier n'en a pas. Une LICENSE de dossier compte si c'est le texte de la racine ou par sa ligne SPDX-License-Identifier. Les plugins de première main ont donc les leurs dans leur dossier (ceux de la racine sont ceux de l'app).
4. repository du manifeste : l'URL du dépôt, ou …/tree/<ref>/<dossier> du même dossier (ref quelconque).
5. version.json gagne path (signé) ; l'app et la revue refusent un path mal formé ; sans path = racine.
6. Dossier déplacé / changement de source : permis à l'auteur du plugin, vers un dépôt à lui où l'App est installée ou un autre dossier, en y soumettant la version suivante ; le contrôle Identifiant le dit, la publication note la source d'avant (journal, page publique, file de revue « à relire en entier ») et envoie la version à la revue même case décochée. Les versions d'avant gardent leur version.json et leur source. Un autre compte reste refusé.
7. Première main (proposé pour ESC-210) : elle passe par la même soumission en sous-dossier que les tiers (le dépôt de l'app, plugins/<id>/, étiquettes plugins/<id>/v1.2.3) ; builtin.ts ne garde que la réserve des ids dev.escouade.* (permis au seul dépôt de l'app, à câbler par ESC-210) et l'origine builtin. Un seul chemin à revoir, le lien vers le dossier vient de path.

## Conséquences

Migration 0009 (path sur plugins et versions, source_changed_at et source_from sur plugins). version.json change de forme (path) : les apps d'avant l'ignorent et installent l'archive du dossier, qui a son manifeste à sa racine. Le store appelle l'API Git Trees/Blobs pour un plugin en dossier (un appel d'arbre, un par fichier du dossier, huit à la fois). Le changement de source remplace l'ancien refus « vient d'un autre dépôt » pour un même compte. ESC-210 change : plus de champ source dans builtin.ts, les plugins de première main sont soumis comme des tiers depuis plugins/<id>/ du dépôt de l'app.
