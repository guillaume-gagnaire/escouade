# Développer Slack

`dev.escouade.slack` est un plugin de la main d’Escouade, dans `plugins/slack/` de son dépôt : un programme en TypeScript sur `@escouade-dev/plugin`, le SDK d’Escouade. Escouade fait tourner `dist/plugin.mjs` en WASM, tel que le dépôt le contient ; Node 22.18 ou plus récent ne sert qu’à le construire et le tester.

## Son dossier

| Fichier | Ce que c’est |
| --- | --- |
| `escouade-plugin.json` | le manifeste |
| `src/plugin.ts` | ses gestionnaires : le salon d’un projet, le texte d’un fait, l’envoi |
| `src/main.ts` | son démarrage |
| `dist/plugin.mjs` | son build, commité : c’est lui qu’Escouade fait tourner et que le store compile |
| `channels.json` | le salon de certains projets, par leur nom |
| `test/plugin.test.ts` | ses tests, contre le faux Escouade du SDK et un faux Slack, sans réseau |
| `README.fr.md`, `README.en.md` | sa page, sur le store et dans l’app |
| `docs/fr/`, `docs/en/` | cette documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | ses versions, sa licence (MIT) |

Son `package-lock.json` n’est pas commité (`.gitignore`) : ses dépendances ne servent qu’à le construire et le tester, et esbuild est fixé dans `package.json`.

## Le vérifier

Dans son dossier :

```sh
npm install
npm run typecheck && npm test        # ses tests
npm run build                        # dist/plugin.mjs à nouveau, puis commite-le
npm run validate && npm run bench    # le manifeste, puis le banc de conformité d’Escouade
```

`npm run dev` le lance contre le faux Escouade du banc, pour lui envoyer des faits à la main.

`node plugins/ci.mjs`, depuis la racine du dépôt (avec `ESCOUADE_PLUGIN_BIN` qui nomme le `escouade-plugin` de l’app), fait tout cela pour chaque plugin du dossier, et vérifie que `dist/plugin.mjs` est bien le build de `src/` : un `dist/` oublié fait échouer la CI.

Ses scripts passent `--first-party` : son id `dev.escouade.*` est réservé aux plugins d’Escouade. Le store le prend de ce dossier du dépôt d’Escouade seulement ; ses étiquettes sont `plugins/slack/v1.2.3`.

