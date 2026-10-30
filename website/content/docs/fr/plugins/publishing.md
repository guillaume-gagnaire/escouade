# Publier sur le store

Un plugin se publie sur store.escouade.dev depuis son dépôt GitHub public et une étiquette git. Cette page dit comment soumettre une version, ce que le store contrôle, et comment corriger un refus.

## Un plugin par dépôt

Le store lit **un plugin à la racine d’un dépôt** : `escouade-plugin.json`, `README.md` et `LICENSE` (ou `COPYING`) au premier niveau, et `repository` du manifeste qui nomme ce dépôt. Un dépôt qui range plusieurs plugins dans des sous-dossiers, comme `plugin-examples/` du dépôt d’Escouade, ne se soumet pas tel quel : donne à chaque plugin son dépôt. Le dossier d’un plugin se copie tel quel à la racine du nouveau dépôt ; il lui faut alors son README, sa licence et son `repository`.

## Soumettre une version

1. Connecte-toi sur [store.escouade.dev](https://store.escouade.dev/submit) avec GitHub, et installe l’App « Escouade Store » sur le dépôt du plugin. Elle lit le code et les étiquettes, sans jamais écrire. Un dépôt privé est refusé : le code d’un plugin du store se lit par tous.
2. Choisis le dépôt, puis l’étiquette de la version (`v1.2.0` ou `1.2.0`). Le store la résout en commit une fois, à la soumission, et ne la relit plus pour cette version.
3. Les dix contrôles tournent sur la page, une ligne chacun. Le store télécharge l’archive du commit une fois, la garde telle quelle et en calcule l’empreinte SHA-256 ; rien du plugin ne s’exécute sur le serveur.
4. Si tout passe, publie : une première version demande une à trois catégories et des mots-clés. La version paraît **non vérifiée** ; la case « Demander la revue » (cochée) la met dans la file de la revue « Vérifié par Escouade », qui vaut pour cette version seulement.

Chaque version publiée a son fichier `version.json` : l’identifiant, le numéro, le dépôt, le commit, l’empreinte de l’archive, du manifeste et de chaque binaire. Son empreinte est celle de la version, que l’app vérifie et que la revue signe. Une version publiée ne change plus, et son numéro ne se réutilise pas.

## Les dix contrôles

| Contrôle | Ce qu’il demande |
| --- | --- |
| Manifeste | `escouade-plugin.json` à la racine, valide au schéma : les mêmes règles que dans l’app ([le manifeste](plugins/manifest)). |
| Version | `version` égale à l’étiquette (le `v` est facultatif), plus grande que la dernière publiée. |
| Identifiant | Libre ou déjà à toi, publié depuis ce dépôt ; `dev.escouade.*` est réservé. Un nom trop proche de celui d’un autre plugin passe d’abord par une revue manuelle. |
| Dépôt | `repository` égal au dépôt soumis, `https://github.com/<propriétaire>/<dépôt>`. |
| Licence | Une licence libre (reconnue par l’OSI ou la FSF) en identifiant SPDX, la même que celle que GitHub reconnaît dans le fichier `LICENSE` ou `COPYING`. |
| README | `README.md` (ou `README.fr.md` et `README.en.md`) à la racine, non vide : c’est la page du plugin. |
| Captures | Celles de `store.screenshots` : 6 au plus, en PNG, JPEG ou WebP, 1600 px de large et 1 Mo chacune au plus. Le store les réencode, sans leurs métadonnées. |
| Archive | 50 Mo au plus, sans lien symbolique, sans chemin absolu ni `..`, sans sous-module ni fichier Git LFS non résolu. |
| Permissions | Cohérentes avec le plugin : sans programme ni webhook, il ne demande ni réseau, ni événements, ni secrets ; un secret demandé est déclaré dans `contributes.secrets` ; un hôte s’écrit seul (`hooks.slack.com`). |
| Binaires | Le programme de `run` est dans le dépôt, ou dans la release de l’étiquette pour chaque système (voir plus bas). |

## La page du plugin

Une fois publiée, la version a sa page sur le store : son nom, son auteur, « Non vérifié » ou « En revue » jusqu’à sa signature, ce qu’elle pourra faire en mots (la permission la plus forte d’abord), son dépôt, son commit et son empreinte, ses versions, et « Ouvrir dans Escouade », qui ouvre sa page dans l’app.

Quand la revue a relu la version, tu reçois sa réponse par e-mail et dans ton compte. Vérifiée, elle porte le sceau « Vérifié par Escouade » et le résumé de la revue (les permissions relues, les hôtes joints, ce qui a été vérifié), sur sa page et dans l’app, qui contrôle la signature avec ses propres clés. Si la revue demande des changements ou refuse, la version reste publiée, non vérifiée, et le message dit pourquoi : une nouvelle version qui en tient compte repasse en revue.

Le README y est rendu une fois, à la soumission, dans la langue du visiteur (`README.fr.md` ou `README.en.md`, sinon `README.md`) ; les pages markdown du dossier `store.docs` font l’onglet « Documentation ». Le store en garde le markdown et une courte liste de balises HTML (titres, listes, liens, code, tableaux, `details`) ; il retire les scripts, les styles, les attributs `style` et `class`, les cadres et les formulaires. Les images du dépôt (PNG, JPEG ou WebP, 5 Mo au plus, en chemin relatif ou par leur adresse sur GitHub) sont copiées sur le store et réencodées ; les images d’ailleurs, comme les badges, sont retirées : un visiteur ne charge rien chez un tiers. Un lien vers un fichier du dépôt mène à GitHub, au commit publié.

Tout le monde peut signaler un plugin depuis sa page, sans compte, avec une adresse e-mail : une personne lit chaque signalement. Tu vois qu’un signalement est ouvert, jamais qui l’a fait.

## Les binaires

Un plugin qui lance un programme compilé attache à la release de l’étiquette une archive par système, dont le nom dit le système et l’architecture : `…-windows-x64.zip`, `…-macos-arm64.tar.gz`, `…-linux-x64.tar.gz` (50 Mo au plus chacune). Chaque système qui lance un programme absent du dépôt doit avoir la sienne. Le store les télécharge une fois, les garde et en calcule l’empreinte.

Construis-les dans un workflow GitHub Actions public avec `actions/attest-build-provenance` : l’attestation de provenance lie chaque fichier au commit de l’étiquette. Un binaire sans attestation est publié non vérifié et ne pourra pas être vérifié ; un binaire attesté depuis un autre commit est refusé.

## Préparer la revue

La revue « Vérifié par Escouade » est faite à la main, version par version, sur le poste de l’équipe d’Escouade : elle télécharge l’archive gardée par le store, en vérifie les empreintes, lit le code au regard des permissions déclarées et fait passer le [banc de conformité](plugins/testing) dans un conteneur sans réseau. Elle signe ensuite `version.json` avec une clé qui ne quitte jamais ce poste : ni le store, ni la CI ne l’ont. Ce qui la rend rapide :

- **des permissions exactes** : chaque hôte joint dans `permissions.network`, chaque secret dans `contributes.secrets`, rien de plus ;
- **du code lisible** : un fichier compilé (`dist/`) a sa source dans le dépôt et s’en reconstruit ; pas de code minifié sans source, pas de texte encodé en base64 ;
- **peu de dépendances**, à des versions fixes, sans script `postinstall` ni dépendance tirée d’un dépôt git ;
- **des binaires attestés** (voir plus haut) : sans attestation, une version ne peut pas être vérifiée ;
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
