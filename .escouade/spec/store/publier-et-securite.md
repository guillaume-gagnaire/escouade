---
title: "Publier sur le store, la sécurité des plugins et l'essai réel"
subject: "store"
sources: ["docs/STORE.md","website/content/docs/fr/plugins/publishing.md","website/content/docs/fr/plugins/security.md","store/server/utils/checks.ts","store/server/utils/folder.ts","store/shared/folder.ts","store/server/utils/submissions.ts","store/server/utils/builtin.ts","store/tests/unit/trial.test.ts","store/deploy/seccomp-bubblewrap.json","store/deploy/apparmor-bubblewrap","store/server/utils/readme.ts","store/app/components/StorePlugin.vue"]
tickets: ["ESC-237","ESC-313","ESC-314","ESC-323","ESC-184","ESC-336","ESC-356","ESC-366"]
decisions: [21,25,43,72,86,91]
compared: "2026-10-31T19:50:09+01:00"
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

## Les plugins en JS/TS : la compilation au store (ESC-323)
- Une onzième ligne, **« Compilation »**, rejoint les contrôles. Un plugin `run.js` est compilé deux fois en WASM par le store, depuis le bundle de l'archive figée, chaque fois dans un dossier neuf. Deux empreintes différentes refusent la version. Son module entre dans `version.json` (`wasm`), que la signature couvre. Le contrôle des binaires refuse à un tiers un script ou un interpréteur (`node`…). Le détail est dans `plugins/programmes-js-ts-en-wasm`, et le bac à sable dans la décision `la-compilation-des-plugins-run-js-au-sto`.
- La documentation des auteurs du site (« Publier sur le store ») dit encore « les dix contrôles » : WASM-6 (ESC-325) la met à jour.

## L'essai réel sur store.escouade.dev (ESC-184)
- Fait le 2026-11-04 sous Windows sur les quatre plugins de première main publiés et signés, avec une app de debug qui vérifie les sceaux contre les vraies clés : `tests/e2e/real-store.spec.ts`, sauté sans `ESCOUADE_E2E_REAL_STORE`. Sceau, permissions, activation, désactivation, désinstallation ; Discord et Slack arrêtés à leur configuration, sans webhook ni jeton réels. Résultats : `docs/PLUGINS.md` §7.
- Reste à faire à la release : macOS et Linux, la révocation (aucune liste publiée), une mise à jour réelle (aucune version installable plus récente).

## Les plugins en Rust : la compilation au store (ESC-336)

Cette partie l'emporte sur ce qui précède pour le contrôle des binaires : la phrase sur le script ou l'interpréteur refusés à un tiers, et les « binaires attestés » de la documentation des auteurs, sont d'avant la décision 72. La documentation du site suit à ESC-227.

- **Aucun plugin ne fournit plus de programme.** Le contrôle « Binaires » passe pour un `run.js`, un `run.wasm` ou un plugin sans programme. `run.command`, `run.windows`, `run.macos` et `run.linux` sont refusés à tout plugin, la première main comprise, chacun à son champ, avec la raison et le correctif. Le store ne lit plus les releases. Les versions publiées avant gardent leurs binaires, servis tels quels.
- **Le contrôle « Compilation » compile un `run.wasm`**, une fois tous les autres contrôles passés. Il prend tous les fichiers du dossier dans l'archive figée, sans lien ni fichier spécial, et il compile deux fois, chaque fois dans un dossier neuf. Deux empreintes différentes refusent la version. Le module est gardé sous son SHA-256, noté dans `version.json` (`wasm`, avec `compiler` : `rust`, `target`, `canon`, `lock`), couvert par la signature, et servi comme celui d'un `run.js`.
- **Les dépendances viennent de crates.io, dans une étape à part.** `escouade-plugin fetch` juge d'abord le dossier sans rien lancer, puis fait `cargo fetch --locked` avec réseau ; rien du plugin ne s'exécute dans cette étape. La compilation tourne ensuite sans réseau. Le réseau de cette étape n'est pas filtré par hôte : ce sont ces refus qui gardent cargo sur crates.io.
- **Refusée, avec la raison** : tout ce qu'`escouade-plugin` refuse, dont un `build.rs` ou une macro hors de la liste relue, un `Cargo.lock` qui ne correspond pas, une erreur de compilation, ou le dépassement d'une limite.
- **En attente en `error`, à relancer** : un réseau en panne, un compilateur absent, ou pas de bac à sable.
- **Le bac à sable est bubblewrap**, en plus de la liste relue, qui reste la première défense. Une racine à part, sans Node ni le store ; nouveaux espaces de noms, nouvelle session, aucune variable du store, pas de `HOME`, un `/etc` vide, le seul dossier de la compilation. Un `include_bytes!` avec un chemin absolu ne trouve rien. La compilation n'a pas de réseau.
- **Sans repli, et sans exception pour la première main.** Là où bubblewrap ne peut pas tourner, une version `run.wasm` attend, et le journal de démarrage le dit. `STORE_COMPILE_ISOLATION=none` n'est entendu que hors production.
- **Les limites** d'un `run.wasm` : 5 minutes pour compiler, 2 pour télécharger, 2 Gio de mémoire, 4 Gio de disque. Celles d'un `run.js` ne changent pas.
- **Il reste à l'hébergement** : le Docker par défaut refuse bubblewrap. Sans `--security-opt seccomp=unconfined` ou un profil plus étroit sur Coolify, aucun `run.wasm` ne compile en production. La revue ne recompile pas encore un `run.wasm` (A7) : `sign` le refuse d'ici là.

- **Le bac à sable n'a aucun `/proc`** : ni un neuf, que le noyau peut refuser dans un conteneur, ni celui du conteneur, qui montrerait ses process. La sortie de `bwrap` est dite au démarrage du store et dans l'erreur d'une version qui attend.
- **Sur un Docker d'Ubuntu 24.04**, il faut attendre au moins `seccomp=unconfined` et `apparmor=unconfined`, plus `kernel.apparmor_restrict_unprivileged_userns=0` sur l'hôte. La CI du store les confirme ; ESC-356 en tire un profil étroit.

## Les profils du conteneur du store (ESC-356)

Cette partie remplace, dans la section précédente, « Il reste à l'hébergement » pour ce qui est du réglage, et « Sur un Docker d'Ubuntu 24.04 ».

- **Deux profils, pas `unconfined`.** `store/deploy/seccomp-bubblewrap.json` est le profil de moby 29.9.3 plus quatre règles : `clone` à deux jeux exacts d'espaces de noms (ceux de `--unshare-all`, avec et sans le réseau), `unshare` de `CLONE_NEWUSER` seul, `mount`, `umount2`, `pivot_root`. `store/deploy/apparmor-bubblewrap` est `docker-default`, moins `deny mount`, plus les montages de bubblewrap nommés un à un : `tmpfs` et `devpts` seuls, et des sources tirées de l'arbre du bac à sable, jamais `/app` ni `/data`.
- **Il n'y a rien à changer au noyau** : avec un profil AppArmor nommé, `kernel.apparmor_restrict_unprivileged_userns=1` n'empêche rien. C'est `apparmor=unconfined` qui l'exigerait à `0`.
- **`setns`, `clone3` et `unshare -rn` restent refusés.** Un `run.js` garde donc le réseau du conteneur.
- **Le réglage reste un geste de l'utilisateur sur l'hôte**, décrit dans `docs/STORE.md` §3.3 : Coolify 4.5.0 ou plus sans serveur de build, les deux fichiers posés, le profil AppArmor chargé, la ligne « Custom Docker Options ». Tant qu'il n'est pas fait, le journal du store le dit et chaque version `run.wasm` attend.
- **Ce que cela ouvre** : tout process du conteneur, pas bubblewrap seul, peut créer un espace de noms utilisateur à ces jeux de drapeaux et y monter ce qu'AppArmor laisse. C'est une surface du noyau que Docker ferme par défaut.
- **La preuve** : `store/scripts/image-test.mjs` essaie les deux profils, chacun seul et sans eux, et vérifie ce qui reste refusé. Joué sur une VM Ubuntu 24.04, Docker 29.9.3, AppArmor en `enforce`. Une vraie instance de Coolify n'a pas été essayée.

## La documentation d'un plugin, par langue (ESC-366)

- **Des pages par langue** : sous le dossier de `store.docs`, les pages de `fr/` et de `en/` ont leur langue (`docs/fr/01-installer.md`, `docs/en/01-install.md`). La page du plugin montre celles de la langue du visiteur, sinon celles de l'autre. Une page demandée par son adresse (`?tab=docs&page=…`) se montre avec celles de sa langue.
- **Un dossier sans `fr/` ni `en/`** garde le comportement d'avant : toutes ses pages markdown, dans les deux langues, sans langue. Avec l'un des deux, les pages hors de `fr/` et `en/` ne sont pas lues. Un README n'est jamais une page.
- **L'ordre et les titres** : l'ordre des noms de fichier, 20 pages au plus par langue (`DOC_PAGES_MAX`). Le titre d'un onglet est le premier titre de la page (pas un `#` d'un bloc de code), sinon le nom du fichier sans son préfixe d'ordre (`01-`).
- **Les liens** : un lien vers une autre page de la documentation ou vers le README reste sur le store, dans son onglet, sans recharger la page ; vers un autre fichier du dépôt, il mène à GitHub au commit figé, comme avant. Les images en chemin relatif (`../images/x.png`) sont copiées une fois pour les deux langues.
- **Le contrôle** : un `store.docs` qui ne mène à aucune page markdown (dossier absent, sans `.md`) est un avertissement du contrôle README, qui passe quand même. Le contrôle du manifeste refuse déjà un chemin qui sort du plugin. La page de soumission montre l'avertissement aux couleurs d'une attente.
- **Dans l'app**, la vue Plugins ne montre ni le README ni les pages : elle renvoie à la page du store (« Lire sa documentation sur le store »). Montrer la documentation dans l'app reste un chantier à part.
- La documentation des auteurs (`plugins/publishing`, `plugins/manifest`, fr et en) donne la forme avec un exemple d'arborescence.
