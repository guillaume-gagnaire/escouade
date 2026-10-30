# The SDK

`@escouade-dev/plugin` writes a plugin of Escouade in TypeScript: the protocol’s types, a client that speaks it on the standard input and output, a fake Escouade for the plugin’s tests, and the `escouade-plugin` command. It is under the MIT license, like the protocol’s schema, and depends on nothing.

## A first plugin

```sh
npm create escouade-plugin@latest my-plugin
cd my-plugin
npm install
npm test            # its handlers, against the fake Escouade, in memory
npm run validate    # the manifest, field by field
npm run bench       # the conformance bench, on the real program
```

The command asks for the kind of plugin; `--kind program` or `--kind declarative` chooses without asking, `--id` and `--name` give its id and its name.

- **A program** (`program`): a TypeScript plugin with its tests. `src/plugin.ts` says when a ticket is ready to test and refuses force pushes; `src/main.ts` starts it; `npm run build` bundles both into `dist/plugin.mjs`, which `plugin` (macOS, Linux) and `plugin.cmd` (Windows) start, as `run` in the manifest says.
- **A declarative plugin** (`declarative`): a theme, described in the manifest. Nothing runs.

To try it in Escouade, load its folder in Settings › Plugins, in developer mode. Escouade runs a plugin as its repository holds it, without `npm install`: commit `dist/` before you tag a version.

## The client

```ts
import { createPlugin } from '@escouade-dev/plugin';

const plugin = createPlugin();

// A fact of permissions.events.
plugin.onEvent('ticket.column', async (fact, p) => {
  if (fact.data.to !== 'review') return;
  const webhook = await p.secret('webhook'); // one of permissions.secrets, or null
  if (!webhook) return p.log.warn('no webhook kept: nothing sent');
  await fetch(webhook, { method: 'POST', body: JSON.stringify({ text: `${fact.ticket?.key ?? ''} is ready to test` }) });
});

// A permission an agent asks for (permissions.decide): within 500 ms.
plugin.decidePermission(({ request }) =>
  request.kind === 'command' && String(request.input?.command).startsWith('rm -rf')
    ? { decision: 'deny', message: 'Not here.' }
    : { decision: 'ask' },
);

void plugin.start();
```

| Helper | What it does |
| --- | --- |
| `onEvent(name, handler)` | Runs `handler` on each fact of `name`, or of a pattern ending in `*` (`ticket.*`), or of a list. The facts must be in `permissions.events`. A handler that fails goes to the plugin’s journal; the others still run. |
| `decidePermission(handler)` | Answers `permissions/decide`: `allow`, `deny` with a `message`, or `ask`, which leaves it to the user. Without a handler, the answer is `ask`. |
| `secret(key)` | Asks Escouade for one of its secrets (`secrets/get`); `null` when none is kept. Another key rejects with the error `-32001`. |
| `log.debug/info/warn/error(message)` | A line of its journal (`log`). |
| `handle(method, handler)` | Serves another method (a later extension point). Throw `new RpcError(code, message)` to answer an error. |
| `onInitialize(handler)`, `onShutdown(handler)` | Runs at `initialize`, or before the answer to `shutdown` (800 ms at most: Escouade waits a second). |
| `call(method, params)`, `notify(method, params)` | A call or a notification to Escouade, typed for its methods. |
| `start()` | Speaks the protocol on the process’s standard streams, and ends it after `shutdown`. |

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

```ts
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connect } from '@escouade-dev/plugin/testing';
import { plugin } from '../src/plugin.ts';

test('a ticket moved to test is said in the journal', async () => {
  const escouade = connect(plugin, { secrets: { webhook: null } });
  await escouade.initialize();
  await escouade.fact('ticket.column', { ticket: { key: 'APP-7' }, data: { from: 'doing', to: 'review' } });
  assert.deepEqual(escouade.logs, [{ level: 'warn', message: 'no webhook kept: nothing sent' }]);
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

`test` and `dev` run the `escouade-plugin` that ships with Escouade, so that a plugin is judged by the bench of the app: the one of `ESCOUADE_PLUGIN_BIN`, else the one on the `PATH`, else the app’s (`%LOCALAPPDATA%\Escouade` on Windows, `Escouade.app` on macOS, `/usr/bin` with the `.deb`). Without one, the command says where to get it.

### The archive

`escouade-plugin pack` writes `packs/<id>-<version>.tar.gz` and its `.sha256`, with every file of the folder but `.git`, `node_modules` and `packs`. The archive is the same, to the byte, from the same files, on any system: the dates are zeroed, and the programs of `run` and the scripts that start with `#!` are executable. A symbolic link, or an archive past 50 MB, is refused, as by the store.

`--target windows-x64` (or `macos-arm64`, `linux-x64`…) names the archive of one system, `<id>-<version>-windows-x64.tar.gz`: the name [the store](plugins/publishing) expects of the release’s files of a compiled plugin.

## One executable

A plugin in TypeScript needs Node, which not everyone has. Compile it into one executable per system, and name each in `run`:

```sh
npm run build
deno compile --allow-env --allow-read --allow-net --output bin/plugin-windows-x64.exe --target x86_64-pc-windows-msvc dist/plugin.mjs
deno compile --allow-env --allow-read --allow-net --output bin/plugin-macos-arm64 --target aarch64-apple-darwin dist/plugin.mjs
deno compile --allow-env --allow-read --allow-net --output bin/plugin-linux-x64 --target x86_64-unknown-linux-gnu dist/plugin.mjs
```

```json
"run": { "command": "plugin", "windows": "bin/plugin-windows-x64.exe", "macos": "bin/plugin-macos-arm64", "linux": "bin/plugin-linux-x64" }
```

- Give the program only the rights it uses: `--allow-net` for a plugin that calls a service, nothing for one that only decides.
- `bun build --compile --target=bun-windows-x64 dist/plugin.mjs --outfile bin/plugin-windows-x64.exe` does the same with Bun.
- An executable weighs about 100 MB, 40 MB in its archive. Attach one archive per system to the release (`escouade-plugin pack . --target windows-x64`), built by a public workflow with its provenance attestation: the store only verifies the binaries it can tie to the commit.
- `escouade-plugin test .` plays the bench on the executable of the system it runs on: run it on each.

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

Escouade’s CI does the same on both templates of `npm create escouade-plugin`, against the bench of each build.

## Later

A Rust crate of the same types, then the WIT interface of the WebAssembly runtime, with the 2.2.
