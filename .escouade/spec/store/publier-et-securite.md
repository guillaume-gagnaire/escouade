---
title: "Publier sur le store, la sécurité des plugins et l'essai réel"
subject: "store"
sources: ["docs/STORE.md","website/content/docs/fr/plugins/publishing.md","website/content/docs/fr/plugins/security.md","store/server/utils/checks.ts","store/server/utils/folder.ts","store/shared/folder.ts","store/server/utils/submissions.ts","store/server/utils/builtin.ts","store/tests/unit/trial.test.ts"]
tickets: ["ESC-237","ESC-313","ESC-314"]
decisions: [21,25]
compared: "2026-10-31T15:57:56+01:00"
---

# Publier sur le store, la sécurité des plugins et l'essai réel

Détail : `docs/STORE.md` §3 (§3.6 pour les dossiers et la première main), §4, §7, §9.1 et §9.2 (il fait foi).

## Un ou plusieurs plugins par dépôt (ESC-313)
- Un plugin est **(dépôt public, dossier)** : la racine (`''`), ou un dossier qui porte son `escouade-plugin.json`. Un dépôt peut en porter plusieurs ; chacun a son id. La cible principale : le dépôt de l'app et ses plugins de première main sous `plugins/<id>/` (ESC-210).
- **Le choix du dossier** (deuxième étape de `/submit`, avant la version) : les dossiers à manifeste, lus dans l'arbre du commit de l'étiquette la plus récente (un appel, 50 listés au plus), la racine d'abord, et un chemin tapé. La racine par défaut ; une racine sans manifeste invite à choisir un dossier. Refusé avec la raison en fr et en : chemin absolu, `..`, antislash ou caractère de contrôle, plus de 200 caractères, un lien symbolique sur le chemin, pas de manifeste.
- **La lecture** d'un dossier ne télécharge jamais le dépôt entier : l'arbre du commit, puis les seuls fichiers du dossier (blobs) et le README / la LICENSE de la racine ; 2 000 fichiers, 200 Mo et 50 Mo d'archive au plus ; un arbre coupé par GitHub fait lire l'archive bornée à 50 Mo. Un plugin à la racine garde l'archive de GitHub.
- **L'archive gardée** d'un plugin en dossier est celle du dossier seul, faite par le store, les mêmes octets pour le même dossier au même commit, l'en-tête nommant le commit.
- **Les étiquettes** : à la racine `v1.2.3` ou `1.2.3` (seul plugin du dépôt : toute étiquette, refusée par les contrôles si elle ne nomme pas la version). En dossier : `<dossier>/v1.2.3` (conseillé), `<nom>@1.2.3`, `<nom>-v1.2.3`, le nom étant le dernier segment du dossier. Une étiquette nue `v…` n'est jamais celle d'un plugin en dossier (les releases de l'app restent les siennes). La page ne montre que les étiquettes du dossier choisi.
- **Le webhook** relie une étiquette au plugin dont elle nomme le dossier ; une étiquette qui en nomme plusieurs n'est vérifiée pour aucun : une version refusée, sans plugin, le dit à chaque auteur (page et e-mail) avec l'étiquette à pousser.
- **Les contrôles** lisent le dossier (manifeste, icône, captures, programme, permissions, archive). Le README et la LICENSE du dossier gagnent ; sans eux, ceux de la racine. Une LICENSE de dossier compte si c'est le texte de la racine ou par sa ligne `SPDX-License-Identifier:`. `repository` : l'URL du dépôt, ou `…/tree/<ref>/<dossier>` du même dossier. Les messages disent « dans le dossier `x` ».
- **`version.json`** porte `path`, signé avec le reste ; l'app montre le dossier et ouvre `<dépôt>/tree/<étiquette>/<dossier>`, refuse un `path` qui n'est pas un chemin relatif simple ; un `version.json` sans `path` est la racine. La revue refuse aussi un `path` mal formé ; rapport et résumés publics nomment le dossier. Pages publiques et file de revue nomment le dossier.
- **Changer de source** : un plugin publié peut passer, par son auteur, à un autre de ses dépôts où l'App est installée ou à un autre dossier, en y soumettant sa version suivante : le contrôle « Identifiant » le dit, la publication note la source d'avant (`source_from`, journal `plugin.source_changed`), la version va à la revue même case décochée, la file et la page publique le montrent ; les versions d'avant gardent leur `version.json` et leur source. Un autre compte reste refusé.
- `npm create escouade-plugin` dans un dossier d'un dépôt git écrit le plugin là, `repository` rempli, et dit l'étiquette `<dossier>/v0.1.0`.

## La première main et le préfixe réservé (ESC-314)
- Les plugins d'Escouade non livrés avec l'app vivent dans `plugins/<id-court>/` du dépôt de l'app et passent par **la même soumission en dossier que les tiers**, à leurs étiquettes `plugins/<id-court>/v1.2.3` : d'abord `dev.escouade.high-contrast`, `dev.escouade.discord`, `dev.escouade.slack` et `dev.escouade.team-rules` (1.1.0).
- **Le préfixe `dev.escouade.*`** n'est accepté que d'une soumission dont le dépôt est celui de l'app. Ce dépôt est donné par la variable du service `STORE_FIRST_PARTY_REPO` (défaut `guillaume-gagnaire/escouade`), résolue en son **id GitHub** au contrôle et comparée à l'id du dépôt soumis, jamais à son seul nom : un fork, une copie ou un dépôt qui prendrait l'ancien nom de l'app après un renommage reçoivent le refus de toujours (« `dev.escouade.x` est réservé », avec l'invitation à prendre `io.github.<login>.<plugin>`), au contrôle « Identifiant ». Un nom que GitHub ne connaît pas n'est le dépôt de personne ; GitHub qui ne répond pas laisse la version à recontrôler. Le contrôle « Manifeste » accepte l'id pour le même dépôt seulement.
- **Les plugins livrés avec l'app** (harness, sources, hébergeurs : l'origine `builtin` d'INT-4) gardent leur page « Livré avec Escouade » tirée de `builtin.ts`, qui ne porte que la réserve et cette liste, **sans champ `source`** : un `dev.escouade.*` publié depuis `plugins/<id>/` est un plugin du store comme un autre, listé au catalogue avec son dépôt et son dossier (son lien vers le dossier vient de `path`) ; seul un id de la liste `BUILTINS` est caché du catalogue au profit de sa page livrée.
- La revue relit leur dossier comme celui d'un tiers ; ni l'adresse du dépôt écrite dans le manifeste, ni l'id `dev.escouade.*` ne sont une piste (hôte, dossier d'Escouade).

## La documentation des auteurs (site, fr et en)
- « Publier sur le store » (`plugins/publishing`) : un ou plusieurs plugins par dépôt, soumettre une version, les dix contrôles (dont `dev.escouade.*` réservé aux plugins d'Escouade, soumis depuis son dépôt), la page du plugin, les binaires attestés, **préparer la revue** (permissions exactes, code lisible avec sa source, dépendances épinglées sans `postinstall`, binaires attestés, mises à jour courtes), corriger un refus, les nouvelles étiquettes, gérer ses plugins.
- « La sécurité des plugins » (`plugins/security`) : ce que l'app vérifie (empreintes, signature avec ses clés embarquées, clé secrète jamais sur le serveur ni dans la CI, le dossier de `version.json`), ce que le sceau couvre (une version) et ne promet pas, **ce que la revue refuse** :
  - refusé d'office : une empreinte qui ne tient pas, une version qui n'est plus celle que le store a figée ;
  - pas de sceau tant que ce n'est pas corrigé : hôte non déclaré, secrets ou dossiers personnels lus, fichiers hors du dossier, programme non annoncé, code dynamique, code illisible, dépendances non épinglées ou `postinstall`, consignes qui détournent les agents, binaire sans attestation, étiquette déplacée, entrée d'archive refusée, banc qui échoue ;
  - retiré : un comportement malveillant ;
  - puis la liste de révocation (retirer le sceau, retrait d'urgence), les signalements, un plugin hors du store « non vérifié ».

## L'essai réel
- Joué sans l'utilisateur contre les faux stores, avec des clés de test : soumission des quatre plugins de première main depuis leur dossier du dépôt de l'app (chacun passe les dix contrôles, l'id refusé à un fork), revue, signature, installation dans l'app sous Windows, révocation ; un dépôt à deux plugins (deux dossiers, étiquettes préfixées) et un dossier piégé.
- L'essai réel avec l'utilisateur (ses clés minisign, sa GitHub App, store.escouade.dev, Windows, macOS et Linux) suit la liste pas à pas de `docs/STORE.md` §9.1 ; la mise en ligne de la première main, ses gestes du chef (étiquettes, App sur le dépôt de l'app, soumission, revue, signature, retrait et révocation des `dev.exemple.*` qui servent d'étape 8, archivage des dépôts `escouade-dev/plugin-*`), suit §9.2.
