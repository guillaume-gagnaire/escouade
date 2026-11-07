# Développer Discord

`dev.escouade.discord` est un plugin de la main d’Escouade, dans `plugins/discord/` de son dépôt. Il est déclaratif : tout est dans son manifeste.

## Son dossier

| Fichier | Ce que c’est |
| --- | --- |
| `escouade-plugin.json` | le manifeste : ses deux webhooks (`contributes.webhooks`), son secret et ses permissions |
| `test/webhooks.test.mjs` | quels faits postent quoi, et ce qu’il ne peut pas faire |
| `README.fr.md`, `README.en.md` | sa page, sur le store et dans l’app |
| `docs/fr/`, `docs/en/` | cette documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | ses versions, sa licence (MIT) |

Un webhook a une adresse (`url`, ici `{{secrets.webhook}}`), les faits qui le déclenchent (`events`), un filtre sur leurs valeurs (`when`, ici `data.to` à `review` ou `data.status` à `waiting`) et un corps (`body`), dont les `{{…}}` sont remplis avec le fait. La page « Les événements » de la documentation d’Escouade dit tout ce qu’un webhook peut déclarer.

## Le vérifier

Dans son dossier :

```sh
node --test                                 # les webhooks, sur le manifeste, sans dépendance
escouade-plugin validate . --first-party    # le manifeste
escouade-plugin test . --first-party        # le banc de conformité d’Escouade
```

Ses tests remplissent les modèles comme Escouade et vérifient qu’un ticket arrivé dans `review` et un agent `waiting` postent le bon texte, que rien d’autre ne poste, que les mentions sont coupées, et qu’il ne demande que ce qu’il utilise.

`node plugins/ci.mjs`, depuis la racine du dépôt (avec `ESCOUADE_PLUGIN_BIN` qui nomme le `escouade-plugin` de l’app), fait tout cela pour chaque plugin du dossier, avec sa LICENSE et son CHANGELOG.

`--first-party` : son id `dev.escouade.*` est réservé aux plugins d’Escouade. Le store le prend de ce dossier du dépôt d’Escouade seulement ; ses étiquettes sont `plugins/discord/v1.2.3`.
