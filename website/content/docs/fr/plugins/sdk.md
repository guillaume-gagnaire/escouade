# Le SDK

`@escouade-dev/plugin` sert à écrire un plugin d’Escouade en TypeScript : les types du protocole, un client qui le parle sur l’entrée et la sortie standard, un faux Escouade pour les tests du plugin, et la commande `escouade-plugin`. Il est sous licence MIT, comme le schéma du protocole, et ne dépend de rien.

## Un premier plugin

```sh
npm create escouade-plugin@latest mon-plugin
cd mon-plugin
npm install
npm test            # ses fonctions, contre le faux Escouade, en mémoire
npm run validate    # le manifeste, champ par champ
npm run bench       # le banc de conformité, sur le vrai programme
```

La commande demande le genre de plugin ; `--kind program` ou `--kind declarative` le choisit sans demander, `--id` et `--name` donnent son identifiant et son nom.

- **Un programme** (`program`) : un plugin en TypeScript, avec ses tests. `src/plugin.ts` dit quand un ticket est prêt à tester et refuse les push forcés ; `src/main.ts` le démarre ; `npm run build` réunit les deux dans `dist/plugin.mjs`, que lancent `plugin` (macOS, Linux) et `plugin.cmd` (Windows), comme le dit `run` dans le manifeste.
- **Un plugin déclaratif** (`declarative`) : un thème, décrit dans le manifeste. Rien ne s’exécute.

Pour l’essayer dans Escouade, charge son dossier dans Réglages › Plugins, en mode développeur. Escouade lance un plugin tel que son dépôt le contient, sans `npm install` : commite `dist/` avant d’étiqueter une version.

## Le client

```ts
import { createPlugin } from '@escouade-dev/plugin';

const plugin = createPlugin();

// Un fait de permissions.events.
plugin.onEvent('ticket.column', async (fact, p) => {
  if (fact.ticket?.column !== 'test') return;
  const webhook = await p.secret('webhook'); // l'une de permissions.secrets, ou null
  if (!webhook) return p.log.warn('no webhook kept: nothing sent');
  await fetch(webhook, { method: 'POST', body: JSON.stringify({ text: `${fact.ticket.key} is ready to test` }) });
});

// Une autorisation qu'un agent demande (permissions.decide) : en moins de 500 ms.
plugin.decidePermission(({ request }) =>
  request.kind === 'command' && String(request.input?.command).startsWith('rm -rf')
    ? { decision: 'deny', message: 'Not here.' }
    : { decision: 'ask' },
);

void plugin.start();
```

| Aide | Ce qu’elle fait |
| --- | --- |
| `onEvent(nom, fonction)` | Appelle `fonction` à chaque fait `nom`, ou d’un motif qui finit par `*` (`ticket.*`), ou d’une liste. Les faits doivent être dans `permissions.events`. Une fonction qui échoue va au journal du plugin ; les autres sont appelées quand même. |
| `decidePermission(fonction)` | Répond à `permissions/decide` : `allow`, `deny` avec un `message`, ou `ask`, qui laisse la main à l’utilisateur. Sans fonction, la réponse est `ask`. |
| `secret(clé)` | Demande à Escouade l’un de ses secrets (`secrets/get`) ; `null` si aucun n’est gardé. Une autre clé échoue par l’erreur `-32001`. |
| `log.debug/info/warn/error(message)` | Une ligne de son journal (`log`). |
| `handle(méthode, fonction)` | Sert une autre méthode (un point d’extension à venir). Lance `new RpcError(code, message)` pour répondre une erreur. |
| `onInitialize(fonction)`, `onShutdown(fonction)` | Appelée à `initialize`, ou avant la réponse à `shutdown` (800 ms au plus : Escouade attend une seconde). |
| `call(méthode, params)`, `notify(méthode, params)` | Un appel ou une notification à Escouade, typés pour ses méthodes. |
| `start()` | Parle le protocole sur les flux standard du process, et le termine après `shutdown`. |

Le client répond à `initialize` par la version 1 du protocole, quelle que soit celle qu’on lui offre (Escouade arrête un plugin qui ne parle pas la sienne), sert les lots, répond `-32601` à une méthode qu’il ne connaît pas, et n’écrit rien d’autre que le protocole sur sa sortie standard : écris tes traces sur la sortie d’erreur, qui va au journal.

## Les types

Les types du protocole viennent d’un seul schéma, [`plugin-protocol-1.json`](/schemas/plugin-protocol-1.json) : ses méthodes, les paramètres et la réponse de chacune, leurs délais, les faits et les codes d’erreur. Le TypeScript du SDK en est généré, les types du manifeste de [`plugin-1.json`](/schemas/plugin-1.json), et les tests d’Escouade tiennent le cœur au même schéma : un changement d’un côté sans l’autre fait échouer les tests.

| Type | Ce que c’est |
| --- | --- |
| `Fact`, `FactName`, `FACT_NAMES` | Un fait (`events/fact`) : son nom, sa version, quand, et le projet, le ticket et l’agent par leurs ids et leurs noms. |
| `DecideParams`, `PermissionRequest`, `Decision` | Une autorisation à décider, et la réponse. |
| `InitializeParams`, `InitializeResult` | La poignée de main. |
| `ToPlugin`, `ToEscouade` | Chaque méthode, ses paramètres et sa réponse, par sens. |
| `TIMEOUTS_MS`, `DEFAULT_TIMEOUT_MS`, `ERRORS` | Les délais des appels d’Escouade, et les codes d’erreur. |
| `Manifest` | `escouade-plugin.json`. |

`@escouade-dev/plugin/manifest-check` donne les contrôles du manifeste que font le store et Escouade : `validate(manifeste)` et `message(erreur, 'fr')`.

## Les tests

`@escouade-dev/plugin/testing` donne un faux Escouade, en mémoire : il envoie au plugin les mêmes messages que l’app et le banc, sans lancer de process.

```ts
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connect } from '@escouade-dev/plugin/testing';
import { plugin } from '../src/plugin.ts';

test('a ticket moved to test is said in the journal', async () => {
  const escouade = connect(plugin, { secrets: { webhook: null } });
  await escouade.initialize();
  await escouade.fact('ticket.column', { ticket: { key: 'APP-7', column: 'test' } });
  assert.deepEqual(escouade.logs, [{ level: 'warn', message: 'no webhook kept: nothing sent' }]);
});
```

`fact(nom, champs)` envoie un fait et attend ses fonctions ; `decide(champs)` demande une décision ; `call` et `notify` envoient le reste. Les champs donnés remplacent ceux du banc (le projet « Banc », le ticket `BEN-1`, une commande `npm test`). Le faux garde ce que le plugin a écrit au journal (`logs`), les secrets qu’il a demandés (`secretsAsked`) et ses traces ; `secrets` donne leurs valeurs, et une clé hors d’elles est refusée, comme dans Escouade.

## La commande

Le SDK installe `escouade-plugin` dans les `node_modules` du plugin, pour ses scripts npm :

| Commande | Ce qu’elle fait |
| --- | --- |
| `escouade-plugin validate [dossier]` | Lit le manifeste et dit ce qui ne va pas, champ par champ, et vérifie que les programmes de `run` sont dans le dossier. |
| `escouade-plugin test [dossier]` | [Le banc de conformité](plugins/testing) : celui d’Escouade. |
| `escouade-plugin dev [dossier]` | [Le faux Escouade](plugins/testing#le-faux-escouade) : des faits, des décisions et des appels tapés, à un programme qui tourne. |
| `escouade-plugin pack [dossier]` | L’archive du plugin et son empreinte SHA-256. |

`test` et `dev` lancent l’`escouade-plugin` livré avec Escouade, pour qu’un plugin soit jugé par le banc de l’app : celui de `ESCOUADE_PLUGIN_BIN`, sinon celui du `PATH`, sinon celui de l’app (`%LOCALAPPDATA%\Escouade` sous Windows, `Escouade.app` sous macOS, `/usr/bin` avec le `.deb`). Sans lui, la commande dit où le trouver.

### L’archive

`escouade-plugin pack` écrit `packs/<id>-<version>.tar.gz` et son `.sha256`, avec tous les fichiers du dossier sauf `.git`, `node_modules` et `packs`. L’archive est la même, à l’octet près, pour les mêmes fichiers, sur n’importe quel système : les dates sont mises à zéro, et les programmes de `run` et les scripts qui commencent par `#!` sont exécutables. Un lien symbolique, ou une archive de plus de 50 Mo, est refusé, comme par le store.

`--target windows-x64` (ou `macos-arm64`, `linux-x64`…) nomme l’archive d’un système, `<id>-<version>-windows-x64.tar.gz` : le nom que [le store](plugins/publishing) attend des fichiers de la release d’un plugin compilé.

## Un exécutable unique

Un plugin en TypeScript a besoin de Node, que tout le monde n’a pas. Compile-le en un exécutable par système, et nomme chacun dans `run` :

```sh
npm run build
deno compile --allow-env --allow-read --allow-net --output bin/plugin-windows-x64.exe --target x86_64-pc-windows-msvc dist/plugin.mjs
deno compile --allow-env --allow-read --allow-net --output bin/plugin-macos-arm64 --target aarch64-apple-darwin dist/plugin.mjs
deno compile --allow-env --allow-read --allow-net --output bin/plugin-linux-x64 --target x86_64-unknown-linux-gnu dist/plugin.mjs
```

```json
"run": { "command": "plugin", "windows": "bin/plugin-windows-x64.exe", "macos": "bin/plugin-macos-arm64", "linux": "bin/plugin-linux-x64" }
```

- Ne donne au programme que les droits dont il se sert : `--allow-net` pour un plugin qui appelle un service, rien pour un plugin qui ne fait que décider.
- `bun build --compile --target=bun-windows-x64 dist/plugin.mjs --outfile bin/plugin-windows-x64.exe` fait de même avec Bun.
- Un exécutable pèse environ 100 Mo, 40 Mo dans son archive. Attache une archive par système à la release (`escouade-plugin pack . --target windows-x64`), construite par un workflow public avec son attestation de provenance : le store ne vérifie que les binaires qu’il peut lier au commit.
- `escouade-plugin test .` joue le banc sur l’exécutable du système où il tourne : lance-le sur chacun.

## En CI

```yaml
jobs:
  plugin:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npm ci
      - run: npm run typecheck && npm test && npm run build
      - run: npx escouade-plugin validate .
      # Le banc vient avec Escouade : installe son .deb, ou donne le chemin de son escouade-plugin.
      - run: npx escouade-plugin test .
        env:
          ESCOUADE_PLUGIN_BIN: /usr/bin/escouade-plugin
```

La CI d’Escouade fait de même sur les deux gabarits de `npm create escouade-plugin`, contre le banc de chaque version construite.

## Plus tard

Une crate Rust des mêmes types, puis l’interface WIT du runtime WebAssembly, avec la 2.2.
