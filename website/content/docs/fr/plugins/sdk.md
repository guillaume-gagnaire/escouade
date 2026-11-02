# Le SDK

`@escouade-dev/plugin` sert à écrire un plugin d’Escouade en TypeScript : les types du protocole, un client qui le parle, dans Escouade en WASM ou sous Node, un faux Escouade pour les tests du plugin, et la commande `escouade-plugin`. Il est sous licence MIT, comme le schéma du protocole, et ne dépend de rien.

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

- **Un programme** (`program`) : un plugin en TypeScript, avec ses tests. `src/plugin.ts` dit quand un ticket est prêt à tester et refuse les push forcés ; `src/main.ts` le démarre ; `npm run build` réunit les deux dans `dist/plugin.mjs`, que nomme `run.js` dans le manifeste : Escouade le fait tourner dans l’app, en WASM, sans Node.
- **Un plugin déclaratif** (`declarative`) : un thème, décrit dans le manifeste. Rien ne s’exécute.

Pour l’essayer dans Escouade, charge son dossier dans Réglages › Plugins, en mode développeur. Escouade lance un plugin tel que son dépôt le contient, sans `npm install` : commite `dist/` avant d’étiqueter une version.

## Le client

Le gabarit `program`, tel que `npm create escouade-plugin` l’écrit : ce que fait le plugin, à part pour que ses tests le pilotent sans process,

`sdk/create-escouade-plugin/templates/program/src/plugin.ts`

```ts
// What the plugin does: its handlers, kept apart from main.ts so that the tests drive them without a process.
import { createPlugin } from '@escouade-dev/plugin';

export const plugin = createPlugin();

// A fact of permissions.events: a ticket changed columns (its data says from and to).
plugin.onEvent('ticket.column', (fact, p) => {
  // A ticket's fact says its ticket; the fact's type, one for every fact, leaves it optional.
  if (fact.data.to === 'review') p.log.info(`${fact.ticket?.key ?? ''} is ready to test (${fact.project?.name ?? ''})`);
});

// A permission an agent asks for, before the user is asked (permissions.decide). Answer within 500 ms.
plugin.decidePermission(({ request }) => {
  const command = typeof request.input?.command === 'string' ? request.input.command : '';
  if (request.kind === 'command' && /\bgit\s+push\b.*(--force\b|\s-f\b)/.test(command)) {
    return { decision: 'deny', message: 'No force push in this team: push a new commit instead.' };
  }
  // Leaves it to the user, as without the plugin.
  return { decision: 'ask' };
});
```

et le programme, que `run.js` nomme une fois réuni dans `dist/plugin.mjs` :

`sdk/create-escouade-plugin/templates/program/src/main.ts`

```ts
// The program (run.js in escouade-plugin.json, once bundled): speaks the protocol to Escouade, which runs it inside the
// app in WASM; under Node too (the bench of escouade-plugin, a test), on stdin and stdout. start() finds which.
import { plugin } from './plugin.ts';

void plugin.start();
```

| Aide | Ce qu’elle fait |
| --- | --- |
| `onEvent(nom, fonction)` | Appelle `fonction` à chaque fait `nom`, ou d’un motif qui finit par `*` (`ticket.*`), ou d’une liste. Les faits doivent être dans `permissions.events`. Une fonction qui échoue va au journal du plugin ; les autres sont appelées quand même. |
| `decidePermission(fonction)` | Répond à `permissions/decide` : `allow`, `deny` avec un `message`, ou `ask`, qui laisse la main à l’utilisateur. Sans fonction, la réponse est `ask`. |
| `secret(clé)` | Demande à Escouade l’un de ses secrets (`secrets/get`) ; `null` si aucun n’est gardé. Une autre clé échoue par l’erreur `-32001`. |
| `settings(projet?)` | Demande ses réglages (`settings/get`) ; avec l’id d’un projet, ceux que son `.escouade.yml` lui donne (`plugins.<id>`) passent devant. |
| `log.debug/info/warn/error(message)` | Une ligne de son journal (`log`). |
| `file(chemin)` | Lit un fichier de son dossier (`files/read`), 256 Ko au plus : jamais son manifeste, un fichier caché ou un chemin qui en sort. |
| `handle(méthode, fonction)` | Sert une autre méthode (un point d’extension à venir). Lance `new RpcError(code, message)` pour répondre une erreur. |
| `onInitialize(fonction)`, `onShutdown(fonction)` | Appelée à `initialize`, ou avant la réponse à `shutdown` (800 ms au plus : Escouade attend une seconde). |
| `call(méthode, params)`, `notify(méthode, params)` | Un appel ou une notification à Escouade, typés pour ses méthodes. |
| `start()` | Parle le protocole à Escouade : à son hôte WASM quand l’app fait tourner le plugin, sur l’entrée et la sortie standard sous Node (le banc, un test) ; le même code des deux côtés. Termine le plugin après `shutdown`. |

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

Les tests du gabarit :

`sdk/create-escouade-plugin/templates/program/test/plugin.test.ts`

```ts
// The plugin's handlers, driven by the SDK's fake Escouade, in memory. `npm run bench` plays the real program.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connect } from '@escouade-dev/plugin/testing';
import { plugin } from '../src/plugin.ts';

test('it speaks version 1 of the protocol', async () => {
  const escouade = connect(plugin);
  const hello = await escouade.initialize();
  assert.equal(hello.protocolVersion, 1);
});

test('a ticket moved to review is said in the journal', async () => {
  const escouade = connect(plugin);
  await escouade.initialize();
  await escouade.fact('ticket.column', { ticket: { key: 'APP-7' }, data: { from: 'doing', to: 'review' } });
  assert.deepEqual(escouade.logs, [{ level: 'info', message: 'APP-7 is ready to test (Banc)' }]);
});

test('a force push is refused, the rest left to the user', async () => {
  const escouade = connect(plugin);
  await escouade.initialize();
  const push = await escouade.decide({ request: { input: { command: 'git push --force origin main' } } });
  assert.equal(push.decision, 'deny');
  assert.equal((await escouade.decide()).decision, 'ask');
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
| `escouade-plugin build [dossier]` | Compile `run.js` en WASM, comme le store : `packs/<id>-<version>.wasm` et son `.sha256`. |

`test` et `dev` lancent l’`escouade-plugin` livré avec Escouade, pour qu’un plugin soit jugé par le banc de l’app : celui de `ESCOUADE_PLUGIN_BIN`, sinon celui du `PATH`, sinon celui de l’app (`%LOCALAPPDATA%\Escouade` sous Windows, `Escouade.app` sous macOS, `/usr/bin` avec le `.deb`). Sans lui, la commande dit où le trouver.

### L’archive

`escouade-plugin pack` écrit `packs/<id>-<version>.tar.gz` et son `.sha256`, avec tous les fichiers du dossier sauf `.git`, `node_modules` et `packs`. L’archive est la même, à l’octet près, pour les mêmes fichiers, sur n’importe quel système : les dates sont mises à zéro, et les programmes de `run` et les scripts qui commencent par `#!` sont exécutables. Un lien symbolique, ou une archive de plus de 50 Mo, est refusé, comme par le store.

`--target windows-x64` (ou `macos-arm64`, `linux-x64`…) nomme l’archive d’un système, `<id>-<version>-windows-x64.tar.gz` : le nom que [le store](plugins/publishing) attend des fichiers de la release d’un plugin compilé.

## Sans Node

Un plugin en JS ou en TypeScript se publie par son bundle, nommé par `run.js` :

`sdk/create-escouade-plugin/templates/program/escouade-plugin.json` (extrait)

```json
  "run": { "js": "dist/plugin.mjs" },
```

Escouade le fait tourner **dans l’app, en WASM**, sans Node chez l’utilisateur : une instance isolée par plugin, qui garde son état d’un appel à l’autre comme un process. Le store compile le bundle à la soumission, et l’app télécharge ce module et le vérifie contre le `version.json` de la version ; un plugin installé depuis un dossier ou un dépôt git est compilé par l’app. La fenêtre d’installation dit « Tourne dans Escouade, isolé », et `permissions.network` y est appliquée : `fetch` passe par Escouade, en https, vers ces hôtes seulement (voir [le protocole](plugins/api#un-programme-en-js-ou-en-typescript)).

- Le bundle n’importe que `node:fs` et `node:readline`, que sert Escouade ; un autre `import` refuse sa compilation, et `escouade-plugin validate` le dit.
- `escouade-plugin build` fait le module comme le store, pour vérifier qu’il compile ; `test` et `dev` jouent le bundle en WASM, sans Node. La Javy qui compile est téléchargée au premier besoin et vérifiée par son empreinte.
- Un plugin qui doit lancer un programme de l’ordinateur (un harness, une commande) garde un `run.command` : un exécutable par système, dans son dossier ou attaché à sa release avec son attestation de provenance (voir [publier](plugins/publishing)). Sa fenêtre d’installation dit « Lance un programme sur ton ordinateur, avec tes droits ». Le store refuse un `run.command` qui est un script ou qui nomme un interpréteur (`node`…) : un programme en JS ou en TypeScript se publie en `run.js`.

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

