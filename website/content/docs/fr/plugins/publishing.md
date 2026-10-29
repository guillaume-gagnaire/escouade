# Publier sur le store

Un plugin se publie sur store.escouade.dev depuis son dépôt GitHub public et une étiquette git. Cette page dit comment soumettre une version, ce que le store contrôle, et comment corriger un refus.

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

## Les binaires

Un plugin qui lance un programme compilé attache à la release de l’étiquette une archive par système, dont le nom dit le système et l’architecture : `…-windows-x64.zip`, `…-macos-arm64.tar.gz`, `…-linux-x64.tar.gz` (50 Mo au plus chacune). Chaque système qui lance un programme absent du dépôt doit avoir la sienne. Le store les télécharge une fois, les garde et en calcule l’empreinte.

Construis-les dans un workflow GitHub Actions public avec `actions/attest-build-provenance` : l’attestation de provenance lie chaque fichier au commit de l’étiquette. Un binaire sans attestation est publié non vérifié et ne pourra pas être vérifié ; un binaire attesté depuis un autre commit est refusé.

## Corriger un refus

Chaque contrôle qui échoue dit ce qui ne va pas, où (le champ ou le fichier, comme `permissions.network[0]`), et comment le corriger. Si tu as donné une adresse e-mail dans ton compte, le même texte y part.

Pour corriger : commite, puis pousse une **nouvelle étiquette**. Le store la voit par l’App et relance les contrôles seul. Si la panne venait du store ou de GitHub, la page propose « Relancer les contrôles ».

## Les nouvelles étiquettes

Une fois le plugin publié, chaque nouvelle étiquette du dépôt devient une version contrôlée seule, qui attend ton clic pour paraître. La case « Publier seul les nouvelles étiquettes » la publie sans attendre, si tous les contrôles passent.

Une étiquette déplacée après la publication ne change rien à la version publiée : le store garde le commit et l’archive qu’il a figés, la marque « étiquette déplacée sur le dépôt » et te prévient. Pour publier ce nouveau code, pousse une nouvelle étiquette.
