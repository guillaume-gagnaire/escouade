# Publier sur le store

Un plugin se publie sur store.escouade.dev depuis son dépôt GitHub public et une étiquette git. Cette page dit comment soumettre une version, ce que le store contrôle, et comment corriger un refus.

## Un ou plusieurs plugins par dépôt

Un plugin est **un dossier d’un dépôt** : la racine, ou un sous-dossier qui porte son `escouade-plugin.json`. Un dépôt peut en ranger plusieurs, un par dossier (`plugins/slack/`, `plugins/ntfy/`) ; chacun est un plugin à part, avec son identifiant.

- **Le dossier** se choisit à la soumission, avant la version : le store liste les dossiers du dépôt qui portent un manifeste, et accepte un chemin tapé, relatif à la racine. Par défaut, la racine. Un chemin absolu, avec `..`, qui passe par un lien symbolique ou sans manifeste est refusé.
- **Les étiquettes** d’un plugin en dossier nomment ce dossier : `plugins/slack/v1.2.3` (conseillé), ou `slack@1.2.3` et `slack-v1.2.3`, le nom étant le dernier segment du dossier. Une étiquette nue `v1.2.3` n’est celle que d’un plugin à la racine : dans un dépôt dont les releases partent sur `v*`, elle reste la sienne. Une étiquette qui pourrait être celle de deux dossiers (même dernier segment) n’est vérifiée pour aucun, et le store te dit laquelle pousser.
- **Le README et la licence** du dossier comptent d’abord ; sans eux, ceux de la racine du dépôt. Le store ne reconnaît une `LICENSE` de dossier que si c’est le texte de celle de la racine, ou par sa ligne `SPDX-License-Identifier: MIT`.
- **`repository`** du manifeste nomme le dépôt (`https://github.com/toi/plugins`), ou le dossier (`https://github.com/toi/plugins/tree/main/plugins/slack`).
- **L’archive** que le store garde et que l’app installe est celle du dossier seul ; `version.json` dit le dossier (`path`), et la page du plugin y mène.
- **Changer de source** : un plugin peut passer d’un dépôt à toi à un autre (ou d’un dossier à l’autre), en soumettant sa version suivante depuis la nouvelle source. Le contrôle « Identifiant » le dit ; à la publication, le changement est noté sur la page du plugin et la version part à la revue. Les versions d’avant gardent leur source.

`npm create escouade-plugin` lancé dans un dossier d’un dépôt git déjà là y écrit le plugin, `repository` rempli, et son README dit l’étiquette à pousser (`plugins/<dossier>/v0.1.0`).

## Soumettre une version

1. Connecte-toi sur [store.escouade.dev](https://store.escouade.dev/submit) avec GitHub, et installe l’App « Escouade Store » sur le dépôt du plugin. Elle lit le code et les étiquettes, sans jamais écrire. Un dépôt privé est refusé : le code d’un plugin du store se lit par tous.
2. Choisis le dépôt, le dossier du plugin (la racine par défaut), puis l’étiquette de la version (`v1.2.0` ou `1.2.0` à la racine, `plugins/slack/v1.2.0` dans un dossier). Le store la résout en commit une fois, à la soumission, et ne la relit plus pour cette version.
3. Les onze contrôles tournent sur la page, une ligne chacun. Le store télécharge l’archive du commit une fois, la garde telle quelle et en calcule l’empreinte SHA-256 ; rien du plugin ne s’exécute sur le serveur.
4. Si tout passe, publie : une première version demande une à trois catégories et des mots-clés. La version paraît **non vérifiée** ; la case « Demander la revue » (cochée) la met dans la file de la revue « Vérifié par Escouade », qui vaut pour cette version seulement.

Chaque version publiée a son fichier `version.json` : l’identifiant, le numéro, le dépôt, le commit, l’empreinte de l’archive, du manifeste et du module. Son empreinte est celle de la version, que l’app vérifie et que la revue signe. Une version publiée ne change plus, et son numéro ne se réutilise pas.

## Les onze contrôles

| Contrôle | Ce qu’il demande |
| --- | --- |
| Manifeste | `escouade-plugin.json` à la racine du plugin (celle du dépôt, ou son dossier), valide au schéma : les mêmes règles que dans l’app ([le manifeste](plugins/manifest)). |
| Version | `version` égale à l’étiquette (le `v` est facultatif ; dans un dossier, `<dossier>/v1.2.3`), plus grande que la dernière publiée. |
| Identifiant | Libre ou déjà à toi, publié depuis ce dépôt et ce dossier, ou depuis une nouvelle source à toi (notée, la version revue) ; `dev.escouade.*` est réservé aux plugins d’Escouade, soumis depuis son propre dépôt. Un nom trop proche de celui d’un autre plugin passe d’abord par une revue manuelle. |
| Dépôt | `repository` égal au dépôt soumis, `https://github.com/<propriétaire>/<dépôt>`, ou à son dossier, `…/tree/<ref>/<dossier>`. |
| Licence | Une licence libre (reconnue par l’OSI ou la FSF) en identifiant SPDX, la même que celle que GitHub reconnaît dans le fichier `LICENSE` ou `COPYING`. |
| README | `README.md` (ou `README.fr.md` et `README.en.md`) à la racine, non vide : c’est la page du plugin. Dans un dossier, celui de la racine du dépôt le remplace s’il n’en a pas ; de même pour la licence. |
| Captures | Celles de `store.screenshots` : 6 au plus, en PNG, JPEG ou WebP, 1600 px de large et 1 Mo chacune au plus. Le store les réencode, sans leurs métadonnées. |
| Archive | 50 Mo au plus, sans lien symbolique, sans chemin absolu ni `..`, sans sous-module ni fichier Git LFS non résolu. |
| Permissions | Cohérentes avec le plugin : sans programme ni webhook, il ne demande ni réseau, ni événements, ni secrets ; un secret demandé est déclaré dans `contributes.secrets` ; un hôte s’écrit seul (`hooks.slack.com`). |
| Binaires | Aucun : `run.command`, `run.windows`, `run.macos` et `run.linux` sont refusés à tout plugin, chacun à son champ : chaque plugin du store tourne en WASM. Un programme de l’utilisateur se déclare par `permissions.commands`. Un plugin `run.js` ou `run.wasm` n’a rien à fournir. |
| Compilation | Pour un plugin `run.js` : son bundle, pris dans l’archive (5 Mo au plus, jamais de `npm install`), compilé en WASM deux fois par `escouade-plugin build`, isolé, chaque fois dans un dossier neuf. Pour un plugin `run.wasm` : son crate, une fois les autres contrôles passés, de la même façon (voir [plus bas](#un-plugin-en-rust)). Deux modules différents refusent la version ; un `import` hors de `node:fs` et `node:readline` aussi. Le module gardé est celui que l’app télécharge et vérifie. |

## Un plugin en JS ou en TypeScript

Un plugin écrit en JS ou en TypeScript se publie en `run.js` : son manifeste nomme son bundle, `"run": { "js": "dist/plugin.mjs" }`, un fichier de son dossier commité avec le reste (sans `node_modules`). Tu ne publies pas de `.wasm` : le store le fait.

- **Le store compile le bundle tel que l’archive le contient** : jamais de `npm install`, jamais ton build. Il le compile deux fois, chaque fois dans un dossier neuf, sans cache, sans réseau quand le noyau du serveur le permet, par la Javy épinglée et le moteur d’Escouade, en mode déterministe ; le compilateur est arrêté au bout de 2 minutes ou au-delà de 1 Go de mémoire, et la version est refusée.
- **Reproductible** : le même bundle donne le même module à l’octet, sous Windows, macOS ou Linux. Deux modules différents refusent la version (« la compilation n’est pas reproductible »), avec leurs deux empreintes. `escouade-plugin build` fait le même module chez toi : `packs/<id>-<version>.wasm` et son `.sha256`, avec `--json` pour le compilateur (la version de Javy, l’empreinte du moteur, celle du prélude).
- **Ce que garde le store** : le module, sous son SHA-256, et ce qui l’a fait, dans `version.json` (`wasm: { sha256, size, compiler }`) ; l’empreinte de la version, donc sa signature, le couvre. L’app le télécharge, compare sa taille et son empreinte à `version.json` et n’installe rien s’ils diffèrent ; Javy ne tourne pas sur la machine de l’utilisateur.
- **La revue « Vérifié par Escouade »** recompile le bundle relu et compare l’empreinte à celle du module publié : ce qui a été lu est ce qui tourne. Une autre empreinte refuse la version.
- **La page du plugin** dit « Tourne dans Escouade, isolé · compilé en WASM par le store », et sa ligne de liste « Compilé en WASM par le store ».

Un plugin du store ne lance ni Node ni aucun programme à lui : le contrôle « Binaires » refuse `run.command`, avec la raison (« `run.command` lancerait un programme natif : le store n’en publie plus, chaque plugin tourne en WASM. »). Installé depuis un dossier ou un dépôt git, hors du store, un plugin lancé par une commande reste permis, non vérifié.

## Un plugin en Rust

Un plugin écrit en Rust se publie en `run.wasm` : son manifeste nomme son `Cargo.toml`, `"run": { "wasm": "Cargo.toml" }`, commité avec ses sources et son `Cargo.lock`. Tu ne publies pas de `.wasm` : le store le fait, et il écrit le sien à la place de tout `plugin.wasm` que l’archive tiendrait.

- **Une fois les autres contrôles passés**, le store compile le crate tel que l’archive le contient, seul dans son dossier : ce qui entoure le plugin dans ton dépôt, un workspace ou un `[patch]`, ne compte pas.
- **Deux étapes, deux fois**, chaque fois dans un dossier neuf : `escouade-plugin fetch` télécharge les dépendances de crates.io sans rien lancer du plugin, puis `escouade-plugin build` compile sans réseau, dans un bac à sable (bubblewrap) sans Node, sans les variables du store, sans son `/etc`. 5 minutes et 2 Gio au plus.
- **Reproductible** : rustc 1.98.0, `wasm32-wasip1`, un profil imposé, et `rustc-canon` qui retire ce qui tient à la machine. Pour les mêmes sources et le même `Cargo.lock`, le module est le même à l’octet sous Windows et sous Linux. Deux modules différents refusent la version, avec leurs deux empreintes.
- **Ce qui est refusé**, avant que rien ne tourne : pas de `Cargo.lock`, un `build.rs`, une macro à toi, un crate hors de la liste fermée, un chemin hors du dossier. La liste est dans [le kit Rust](plugins/rust#ce-que-la-compilation-refuse), avec ce qu’il faut faire. Un `Cargo.lock` qui ne correspond pas à `Cargo.toml` ou à crates.io est refusé aussi.
- **Essaie avant de pousser** : `escouade-plugin validate .` dit ces refus sans rien lancer, et `escouade-plugin build . --json` donne le module et l’empreinte que le store trouvera.
- **Ce que garde le store** : le module, sous son SHA-256, et ce qui l’a fait, dans `version.json` (`wasm: { sha256, size, compiler: { rust, target, canon, lock } }`). L’empreinte de la version, donc sa signature, le couvre.
- **La revue** recompile dans un conteneur sans réseau et compare l’empreinte. Elle lit en plus `Cargo.lock`, dépendance par dépendance, et son diff avec la version d’avant : ce que le lock ajoute est du code qui tourne. Le code `unsafe`, les `extern` et les `#[link]` se justifient.

Une panne du store, un réseau coupé par exemple, ne refuse pas la version : elle attend, à relancer.

## La page du plugin

Une fois publiée, la version a sa page sur le store : son nom, son auteur, « Non vérifié » ou « En revue » jusqu’à sa signature, ce qu’elle pourra faire en mots (la permission la plus forte d’abord), son dépôt, son commit et son empreinte, ses versions, et « Ouvrir dans Escouade », qui ouvre sa page dans l’app.

Quand la revue a relu la version, tu reçois sa réponse par e-mail et dans ton compte. Vérifiée, elle porte le sceau « Vérifié par Escouade » et le résumé de la revue (les permissions relues, les hôtes joints, ce qui a été vérifié), sur sa page et dans l’app, qui contrôle la signature avec ses propres clés. Si la revue demande des changements ou refuse, la version reste publiée, non vérifiée, et le message dit pourquoi : une nouvelle version qui en tient compte repasse en revue.

Le README y est rendu une fois, à la soumission, dans la langue du visiteur (`README.fr.md` ou `README.en.md`, sinon `README.md`) ; les pages markdown du dossier `store.docs` font l’onglet « Documentation ». Le store en garde le markdown et une courte liste de balises HTML (titres, listes, liens, code, tableaux, `details`) ; il retire les scripts, les styles, les attributs `style` et `class`, les cadres et les formulaires. Les images du dépôt (PNG, JPEG ou WebP, 5 Mo au plus, en chemin relatif ou par leur adresse sur GitHub) sont copiées sur le store et réencodées ; les images d’ailleurs, comme les badges, sont retirées : un visiteur ne charge rien chez un tiers. Un lien vers un fichier du dépôt mène à GitHub, au commit publié ; un lien vers une page de la documentation ou vers le README reste sur le store, dans son onglet.

Tout le monde peut signaler un plugin depuis sa page, sans compte, avec une adresse e-mail : une personne lit chaque signalement. Tu vois qu’un signalement est ouvert, jamais qui l’a fait.

### La documentation, par langue

Le dossier que nomme `store.docs` donne ses pages en deux langues dans les sous-dossiers `fr/` et `en/` : le visiteur voit celles de sa langue, sinon celles de l’autre. Un dossier sans `fr/` ni `en/` montre toutes ses pages dans les deux langues. Les pages viennent dans l’ordre de leurs noms (préfixe-les : `01-installer.md`, `02-regler.md`), 20 au plus par langue ; le titre d’un onglet est le premier titre (`#`) de la page, sinon son nom sans le préfixe. Les images se rangent à côté, dans un dossier que les deux langues partagent.

```text
mon-plugin/
├── escouade-plugin.json      "store": { "docs": "docs/" }
├── README.fr.md
├── README.en.md
└── docs/
    ├── fr/
    │   ├── 01-installer.md   # Installer   …   [la suite](02-regler.md)
    │   └── 02-regler.md      # Régler le sujet
    ├── en/
    │   ├── 01-install.md     # Install     …   ![Le sujet](../images/sujet.png)
    │   └── 02-set-up.md      # Set it up
    └── images/
        └── sujet.png
```

Les liens entre pages s’écrivent en chemins relatifs (`02-regler.md`, `../en/01-install.md`) et les images aussi (`../images/sujet.png`). Si `store.docs` ne mène à aucune page markdown, le contrôle du README le dit en avertissement : la version passe, et sa page montre le README seul.

## La signature et le sceau

Une version vérifiée reçoit le sceau « Vérifié par Escouade » : la revue signe son `version.json`, donc ses empreintes, celle du module comprise, avec une clé qui ne quitte pas son poste. L’app contrôle cette signature elle-même, avec ses propres clés. Le sceau vaut pour cette version seulement : la suivante est « Non vérifié » jusqu’à sa propre revue. Ce qu’il couvre est dans [la sécurité des plugins](plugins/security#ce-que-le-sceau-couvre).

## Les mises à jour

Publier une version ne change rien chez ceux qui ont le plugin : Escouade la **propose, et ne l’installe jamais seule**, avec la même fenêtre que l’installation. Si elle demande plus, la fenêtre le dit, marque « nouveau » chaque permission en plus et redemande l’accord. Voir [installer un plugin](plugins/installer#les-mises-a-jour) et [les versions](plugins/versioning).

## Préparer la revue

La revue « Vérifié par Escouade » est faite à la main, version par version, sur le poste de l’équipe d’Escouade : elle télécharge l’archive gardée par le store, en vérifie les empreintes, lit le code au regard des permissions déclarées et fait passer le [banc de conformité](plugins/testing) dans un conteneur sans réseau. Elle signe ensuite `version.json` avec une clé qui ne quitte jamais ce poste : ni le store, ni la CI ne l’ont. Ce qui la rend rapide :

- **des permissions exactes** : chaque hôte joint dans `permissions.network`, chaque secret dans `contributes.secrets`, rien de plus ;
- **du code lisible** : un fichier compilé (`dist/`) a sa source dans le dépôt et s’en reconstruit ; pas de code minifié sans source, pas de texte encodé en base64 ;
- **peu de dépendances**, à des versions fixes, sans script `postinstall` ni dépendance tirée d’un dépôt git ;
- **un `Cargo.lock` court** pour un plugin en Rust : chaque dépendance se relit ;
- **des mises à jour courtes** : une nouvelle version se relit sur son diff avec la dernière vérifiée, les permissions qui grandissent en tête.

Ce que la revue refuse, et ce que le sceau couvre : [la sécurité des plugins](plugins/security).

## Corriger un refus

Chaque contrôle qui échoue dit ce qui ne va pas, où (le champ ou le fichier, comme `permissions.network[0]`), et comment le corriger. Si tu as donné une adresse e-mail dans ton compte, le même texte y part.

Pour corriger : commite, puis pousse une **nouvelle étiquette**. Le store la voit par l’App et relance les contrôles seul. Si la panne venait du store ou de GitHub, la page propose « Relancer les contrôles ».

## Les nouvelles étiquettes

Une fois le plugin publié, chaque nouvelle étiquette du dépôt devient une version contrôlée seule, qui attend ton clic pour paraître. La case « Publier seul les nouvelles étiquettes » la publie sans attendre, si tous les contrôles passent.

Une étiquette déplacée après la publication ne change rien à la version publiée : le store garde le commit et l’archive qu’il a figés, la marque « étiquette déplacée sur le dépôt » et te prévient. Pour publier ce nouveau code, pousse une nouvelle étiquette.

## Gérer ses plugins

Ton compte, sur [store.escouade.dev/account](https://store.escouade.dev/account), montre tes plugins, leur état, leur dernière version et leurs installations (celles des seuls utilisateurs qui ont accepté les statistiques d’Escouade). Une version trouvée sur une nouvelle étiquette y attend en tête, avec les permissions qu’elle demande en plus : « Publier » ou « Ignorer ». Le menu « … » d’un plugin ouvre ses versions, ses installations semaine par semaine et ce que tu peux en faire :

- **Retirer une version** : elle n’est plus proposée à l’installation ni comme mise à jour ; ceux qui l’ont la gardent, marquée « retirée par l’auteur ». Son numéro ne revient pas.
- **Retirer de la liste** : le plugin quitte la recherche et le catalogue de l’app ; sa page reste lisible, les installations gardent leurs versions, sans mises à jour. Il se remet dans la liste d’un clic.
- **Transférer** : à un autre compte du store, qui l’accepte depuis le sien. Si le dépôt change de propriétaire sur GitHub, le nouveau propriétaire, une fois l’App installée chez lui, peut reprendre le plugin. Dans les deux cas, ses publications sont gelées 7 jours, le temps d’un signalement, et sa page dit le transfert.
- **Supprimer**, en tapant son identifiant : il quitte le catalogue tout de suite ; sa page et ses archives partent après 30 jours, pendant lesquels tu peux annuler. Son identifiant reste à ton compte, et ses numéros de version ne servent plus.

Chaque action passe au journal et, si tu as donné une adresse, part par e-mail : une action que tu n’as pas faite se voit. Un compte qui a encore des plugins ne se supprime pas : transfère-les ou supprime-les d’abord.
