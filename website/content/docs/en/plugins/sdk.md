# The SDK

`@escouade-dev/plugin` writes a plugin of Escouade in TypeScript: the protocol’s types, a client that speaks it, inside Escouade in WASM or under Node, a fake Escouade for the plugin’s tests, and the `escouade-plugin` command. It is under the MIT license, like the protocol’s schema, and depends on nothing.

## A first plugin

```sh
npm create escouade-plugin@latest my-plugin
cd my-plugin
npm install
npm test            # its handlers, against the fake Escouade, in memory
npm run validate    # the manifest, field by field
npm run bench       # the conformance bench, on the real program
```

The command asks for the kind of plugin; `--kind program`, `--kind rust` or `--kind declarative` chooses without asking, `--id` and `--name` give its id and its name.

- **A program** (`program`): a TypeScript plugin with its tests. `src/plugin.ts` says when a ticket is ready to test and refuses force pushes; `src/main.ts` starts it; `npm run build` bundles both into `dist/plugin.mjs`, which `run.js` in the manifest names: Escouade runs it inside the app, in WASM, without Node.
- **A module in Rust** (`rust`): the same plugin, on [the Rust kit](plugins/rust), with its tests and its `Cargo.lock`. No `package.json`, no `node_modules`.
- **A declarative plugin** (`declarative`): a theme, described in the manifest. Nothing runs.

To try it in Escouade, load its folder in Settings › Plugins, in developer mode. Escouade runs a plugin as its repository holds it, without `npm install`: commit `dist/` before you tag a version.

## The client

The `program` template, as `npm create escouade-plugin` writes it: what the plugin does, apart so that its tests drive it without a process,

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

and the program, which `run.js` names once bundled into `dist/plugin.mjs`:

`sdk/create-escouade-plugin/templates/program/src/main.ts`

```ts
// The program (run.js in escouade-plugin.json, once bundled): speaks the protocol to Escouade, which runs it inside the
// app in WASM; under Node too (the bench of escouade-plugin, a test), on stdin and stdout. start() finds which.
import { plugin } from './plugin.ts';

void plugin.start();
```

| Helper | What it does |
| --- | --- |
| `onEvent(name, handler)` | Runs `handler` on each fact of `name`, or of a pattern ending in `*` (`ticket.*`), or of a list. The facts must be in `permissions.events`. A handler that fails goes to the plugin’s journal; the others still run. |
| `decidePermission(handler)` | Answers `permissions/decide`: `allow`, `deny` with a `message`, or `ask`, which leaves it to the user. Without a handler, the answer is `ask`. |
| `secret(key)` | Asks Escouade for one of its secrets (`secrets/get`); `null` when none is kept. Another key rejects with the error `-32001`. |
| `settings(project?)` | Asks for its settings (`settings/get`); with a project’s id, those its `.escouade.yml` gives it (`plugins.<id>`) go first. |
| `log.debug/info/warn/error(message)` | A line of its journal (`log`). |
| `file(path)` | Reads a file of its folder as text (`files/read`), a subfolder included (`data/words.txt`), 256 KB at most, under Node as in WASM: never its manifest, a hidden file or a path that leaves it (`RpcError` `ERRORS.refused`); a missing file fails with `ERRORS.notFound`. |
| `fs.read/text/write/stat/list/canon(file)` | The folders of `permissions.files`, in WASM ([the files](plugins/calls#the-files)). |
| `runProgram(params, onOut?)` | Runs a line of `permissions.commands`, in WASM: `exited`, `write`, `close`, `kill` ([the programs](plugins/calls#the-programs-proc)). |
| `openStream(p, url, init?)`, `openSocket(p, url, headers?)` | Imported apart: a response read as it comes and a WebSocket, in WASM ([the network](plugins/calls#the-network)). |
| `handle(method, handler)` | Serves another method (a later extension point). Throw `new RpcError(code, message)` to answer an error. |
| `onInitialize(handler)`, `onShutdown(handler)` | Runs at `initialize`, or before the answer to `shutdown` (800 ms at most: Escouade waits a second). |
| `call(method, params)`, `notify(method, params)` | A call or a notification to Escouade, typed for its methods. |
| `start()` | Speaks the protocol to Escouade: to its WASM host when the app runs the plugin, on the standard input and output under Node (the bench, a test); the same code on both. Ends the plugin after `shutdown`. |

The client answers `initialize` with version 1 of the protocol, whatever is offered (Escouade stops a plugin that does not speak its own), serves batches, answers a method it does not know with `-32601`, and never writes anything but the protocol on its standard output: write your traces on the standard error, which goes to the journal.

## The types

The protocol’s types come from one schema, [`plugin-protocol-1.json`](/schemas/plugin-protocol-1.json): its methods, the parameters and answers of each, their delays, the facts and the error codes. The SDK’s TypeScript is generated from it, the manifest’s types from [`plugin-1.json`](/schemas/plugin-1.json), and Escouade’s tests hold the core to the same schema: a change on one side without the other fails the tests.

| Type | What it is |
| --- | --- |
| `Fact`, `FactName`, `FACT_NAMES` | A fact (`events/fact`): its name, its version, when, and the project, the ticket and the agent by their ids and names. |
| `DecideParams`, `PermissionRequest`, `Decision` | A permission to decide, and the answer. |
| `InitializeParams`, `InitializeResult` | The handshake. |
| `ToPlugin`, `ToEscouade` | Each method, its parameters and its answer, by direction. |
| `TIMEOUTS_MS`, `DEFAULT_TIMEOUT_MS`, `ERRORS` | The delays of Escouade’s calls, and the error codes. |
| `Manifest` | `escouade-plugin.json`. |

`@escouade-dev/plugin/manifest-check` gives the checks of the manifest that the store and Escouade make: `validate(manifest)` and `message(error, 'en')`.

## The tests

`@escouade-dev/plugin/testing` gives a fake Escouade, in memory: it sends the plugin the same messages as the app and the bench, without starting a process.

The template’s tests:

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

`fact(name, fields)` sends a fact and waits for its handlers; `decide(fields)` asks for a decision; `call` and `notify` send anything else. The fields you give are merged over those of the bench (the project “Banc”, the ticket `BEN-1`, a `npm test` command). The fake keeps what the plugin logged (`logs`), the secrets it asked for (`secretsAsked`) and its traces; `secrets` gives their values, and a key outside them is refused, as in Escouade.

## The command

The SDK installs `escouade-plugin` in the plugin’s `node_modules`, for its npm scripts:

| Command | What it does |
| --- | --- |
| `escouade-plugin validate [folder]` | Reads the manifest and says what is wrong, field by field, and checks that the programs of `run` are in the folder. |
| `escouade-plugin test [folder]` | [The conformance bench](plugins/testing): Escouade’s own. |
| `escouade-plugin dev [folder]` | [The fake Escouade](plugins/testing#the-fake-escouade): facts, decisions and calls you type, to a running program. |
| `escouade-plugin pack [folder]` | The plugin’s archive and its SHA-256. |
| `escouade-plugin build [folder]` | Compiles `run.js` or `run.wasm` to WASM, as the store does: `packs/<id>-<version>.wasm` and its `.sha256`. |
| `escouade-plugin fetch [folder]` | Downloads the dependencies of a `run.wasm` from crates.io, running nothing of the plugin. |

`test` and `dev` run the `escouade-plugin` that ships with Escouade, so that a plugin is judged by the bench of the app: the one of `ESCOUADE_PLUGIN_BIN`, else the one on the `PATH`, else the app’s (`%LOCALAPPDATA%\Escouade` on Windows, `Escouade.app` on macOS, `/usr/bin` with the `.deb`). Without one, the command says where to get it.

### The archive

`escouade-plugin pack` writes `packs/<id>-<version>.tar.gz` and its `.sha256`, with every file of the folder but `.git`, `node_modules` and `packs`. The archive is the same, to the byte, from the same files, on any system: the dates are zeroed, and the programs of `run` and the scripts that start with `#!` are executable. A symbolic link, or an archive past 50 MB, is refused, as by the store.

`--target windows-x64` (or `macos-arm64`, `linux-x64`…) names the archive of one system, `<id>-<version>-windows-x64.tar.gz`.

## Without Node

A plugin in JS or TypeScript ships as its bundle, named by `run.js`:

`sdk/create-escouade-plugin/templates/program/escouade-plugin.json` (excerpt)

```json
  "run": { "js": "dist/plugin.mjs" },
```

Escouade runs it **inside the app, in WASM**, with no Node on the user’s machine: one isolated instance per plugin, which keeps its state from one call to the next like a process. The store compiles the bundle when it is submitted, and the app downloads that module and checks it against the version’s `version.json`; a plugin installed from a folder or a git repository is compiled by the app. The install window says “Runs inside Escouade, isolated”, and `permissions.network` is enforced there: `fetch` goes through Escouade, over https, to those hosts only (see [the protocol](plugins/api#a-program-in-wasm)).

- **The same code on both transports**: `start()` speaks to Escouade’s WASM host when the app runs the plugin, on standard input and output under Node (its tests, a plugin started by a command). A plugin written for the SDK runs unchanged on both sides.
- **Its files**: `readFileSync(new URL('../rules.json', import.meta.url), 'utf8')` still reads a JSON file at the root of its folder; `p.file()` reads the others. The standard `fetch` serves the network, through Escouade in WASM.
- **What the bundle may import**: `node:fs` and `node:readline`, which Escouade serves, and nothing else; it is one file, without `export`. Any other `import` fails its compilation, and `escouade-plugin validate` says so in your language (“the bundle imports node:child_process, which a plugin in WASM does not have (only node:fs, node:readline)”). `setTimeout` and `clearTimeout` exist there.
- **SDK 0.3 or later**: a bundle made with SDK 0.2 does not run in WASM; rebuild it (`npm run build`).
- **The tools, without Node**: `escouade-plugin build` makes the module as the store does, to check that it compiles; `test` and `dev` play the bundle in WASM, in the app’s host. The Javy that compiles it (9.1.0) is downloaded the first time it is needed into `~/.escouade/tools` and checked by its hash; the app and `escouade-plugin` share it. Node only serves the author, for `npm run build` and the plugin’s tests; whoever installs it never needs it. No more need to make it an executable (`deno compile`, `bun build --compile`) for people without Node.
- **What the plugin can ask for on top**: `p.fs` for the folders of `permissions.files`, `p.runProgram` for the programs of `permissions.commands`, `openStream` and `openSocket` for the network (see [the calls](plugins/calls)). The store no longer publishes a `run.command`.

## In CI

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
      # The bench comes with Escouade: install its .deb, or give the path of its escouade-plugin.
      - run: npx escouade-plugin test .
        env:
          ESCOUADE_PLUGIN_BIN: /usr/bin/escouade-plugin
```

Escouade’s CI does the same on the templates of `npm create escouade-plugin`, against the bench of each build.

