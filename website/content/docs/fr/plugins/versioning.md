# Les versions

Comment un plugin dit pour quelle Escouade il est écrit, ce qui se passe quand l’une ou l’autre change, et comment le store choisit la version à proposer.

## La version du plugin

`version`, dans le manifeste, est en trois nombres (`1.2.0`, `0.3.0-beta.1`), la même que l’étiquette git de la version (`v1.2.0`, ou `plugins/<dossier>/v1.2.0` dans un dépôt de plusieurs plugins). Le store refuse une version qui n’est pas plus grande que la dernière publiée ; une version publiée est figée, son numéro ne revient pas.

## Pour quelle Escouade

`escouade` dit trois choses :

- `api` : la version majeure du [protocole](plugins/api) que parle le plugin, `"1"`. Escouade sert aujourd’hui la version 1.
- `min` : la plus ancienne Escouade avec laquelle il a été essayé (`2.1.0`, ou `2.1`).
- `max`, facultatif : la plus récente. `2.4` comprend toutes les 2.4.x.

Hors de cette plage, ou d’une autre API, le plugin reste installé mais désactivé, et Escouade dit pourquoi : « Ce plugin demande Escouade 2.3.0 ou plus récent ; tu as la 2.1.0. Mets Escouade à jour. »

## Ce qui ne casse pas

- **Un champ inconnu du manifeste est ignoré** : un manifeste écrit pour une Escouade plus récente se charge quand même, et les champs ignorés sont listés.
- **Une méthode inconnue** répond l’erreur `-32601`, des deux côtés : un plugin qui ne sert pas un appel ne plante pas, et Escouade applique sa réponse par défaut.
- **Un fait nouveau** n’est envoyé qu’au plugin qui le demande dans `permissions.events`.

## Ce qui casse

- **Une autre version du protocole** : à `initialize`, un plugin qui ne parle pas celle d’Escouade est arrêté, avec la raison (« it speaks version 2 of the protocol; this Escouade speaks version 1 »).
- **Un manifeste invalide** pour cette Escouade : il ne se charge pas, et la raison est dite champ par champ.

## Les mises à jour

- **Escouade** propose la **dernière version qu’elle peut faire tourner** : une version écrite pour une Escouade plus récente est sautée, une version retirée par son auteur aussi. Une mise à jour est proposée, jamais installée seule (voir [installer un plugin](plugins/installer#les-mises-a-jour)).
- **Une version qui demande plus** de permissions redemande l’accord de l’utilisateur.
- **Les plugins livrés avec Escouade** ont sa version et changent avec elle.

## Retirer une version

- **Son auteur** la retire du store depuis son compte : elle n’est plus proposée, et ceux qui l’ont la gardent, marquée « retirée par l’auteur » (voir [publier sur le store](plugins/publishing)).
- **Escouade** peut retirer une version dangereuse par sa liste de révocation : elle est alors arrêtée et désactivée chez chacun, avec la raison (voir [le retrait par Escouade](plugins/installer#le-retrait-par-escouade)).
