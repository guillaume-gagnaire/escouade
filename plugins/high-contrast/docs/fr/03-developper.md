# Développer Contraste élevé

`dev.escouade.high-contrast` est un plugin de la main d’Escouade, dans `plugins/high-contrast/` de son dépôt. Il est déclaratif : tout est dans son manifeste.

## Son dossier

| Fichier | Ce que c’est |
| --- | --- |
| `escouade-plugin.json` | le manifeste : le thème (`contributes.themes`), sombre et clair, et sa palette de projets |
| `test/theme.test.mjs` | les ratios de contraste, vérifiés sur le manifeste |
| `README.fr.md`, `README.en.md` | sa page, sur le store et dans l’app |
| `docs/fr/`, `docs/en/` | cette documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | ses versions, sa licence (MIT) |

Ses tests vérifient, en sombre et en clair, sur `--frame`, `--raise`, `--raise-2` et `--well` : le texte (`--text`, `--soft`, `--muted`, `--dim`) à 7:1 au moins, l’accent et les états (`--p-ink`, `--ring`, `--run`, `--wait`, `--done`, `--add`, `--del`) à 4,5:1, les bordures (`--line`, `--line-2`, `--line-3`) à 3:1 ; et que les deux modes donnent les mêmes jetons, avec six couleurs de projet.

## Le vérifier

Dans son dossier :

```sh
node --test                                 # les ratios de contraste, sans dépendance
escouade-plugin validate . --first-party    # le manifeste
escouade-plugin test . --first-party        # le banc de conformité d’Escouade
```

Pour le voir, charge son dossier en développement (vue Plugins › **En développement** › **Charger un dossier…**), choisis-le dans Réglages › Général › Couleurs, et passe de Sombre à Clair. Escouade le relit à chaque changement d’un fichier.

`node plugins/ci.mjs`, depuis la racine du dépôt (avec `ESCOUADE_PLUGIN_BIN` qui nomme le `escouade-plugin` de l’app), fait tout cela pour chaque plugin du dossier, avec sa LICENSE et son CHANGELOG.

`--first-party` : son id `dev.escouade.*` est réservé aux plugins d’Escouade. Le store le prend de ce dossier du dépôt d’Escouade seulement ; ses étiquettes sont `plugins/high-contrast/v1.2.3`.
