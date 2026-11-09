# The protocol

A plugin with a program talks to Escouade in JSON-RPC 2.0: inside the app, in WASM, if it is in JS, TypeScript or Rust; on its standard input and output if it is a process. This page describes the messages and the process.

A `run.js` or `run.wasm` plugin speaks the same protocol: [what changes in WASM](#a-program-in-wasm) comes below, and [the calls](plugins/calls) give an example in TS and in Rust for each method.

## When the program starts

- **At its first activation, never before.** Escouade starts the program of `run` when a fact of its `activation` list happens (`event:ticket.column`), or at the first call it makes to it. A plugin without `activation` does not run; a plugin without `run` is declarative: nothing runs.
- **One process per plugin.** It serves every project and every agent where the plugin is enabled: Escouade tells its calls apart by their `id`.
- **In the plugin’s folder**, with three variables: `ESCOUADE_PLUGIN_ID` (its id), `ESCOUADE_PLUGIN_API` (the protocol’s version, `1`) and `ESCOUADE_VERSION` (Escouade’s).
- **With everything it starts.** On Windows, the process is in a Job object; on macOS and Linux, it leads its process group. When Escouade stops it, what it started stops with it; on Windows, even if Escouade crashes.

## The messages

- JSON-RPC 2.0, **one message per line**: JSON in UTF-8, without a line break, ended by `\n`. The standard output is for the protocol only; write your traces on the standard error, which goes to the [journal](#the-journal).
- **Batches both ways**: a JSON array of messages, on one line. Answer a batch with a batch; the answers can come in any order, each with the `id` of its request.
- A notification has no `id` and expects no answer.

```json
{"jsonrpc":"2.0","id":7,"method":"echo","params":{"a":1}}
[{"jsonrpc":"2.0","id":8,"method":"echo","params":1},{"jsonrpc":"2.0","method":"log","params":{"level":"info","message":"ready"}}]
```

## The handshake

Escouade’s first message is `initialize`. It gives the version of the protocol it speaks and what it can do:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "escouade": { "version": "2.1.1" },
    "plugin": { "id": "dev.escouade.slack", "version": "1.1.0" },
    "lang": "en",
    "capabilities": { "batch": true, "secrets": true, "settings": true, "log": true, "attach": true }
  }
}
```

The plugin answers the version it speaks and its own capabilities:

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"capabilities":{"batch":true}}}
```

- `lang` is the language of the interface (`fr`, `en`), for what the plugin writes for the user.
- Another version, or an answer without a version, stops the plugin, with the reason: “it speaks version 2 of the protocol; this Escouade speaks version 1”.
- Without an answer within 10 seconds, the plugin is stopped too.
- Nothing else is sent to it before its answer: what comes meanwhile waits in a queue.

## The delays

Every call of Escouade has a delay. Past it, Escouade stops waiting: **its default answer applies** (a permission request goes to the user, as without a plugin), and a late answer is ignored.

| Method | Delay |
| --- | --- |
| `initialize` | 10 s |
| `shutdown` | 1 s |
| `permissions/…` (a decision an agent waits for) | 500 ms |
| `tickets/…`, `forge/…`, `workflow/…`, `engine/oneShot` (an import, a call to a service) | 60 s |
| The others | 5 s |

Every delay that runs out is noted in the journal: “test/do: no answer within 300 ms”.

## What Escouade serves the plugin

| Method | Parameters | Answer |
| --- | --- | --- |
| `secrets/get` | `{ "key": "webhook" }` | The secret, or `null` if none is kept under that key. |
| `settings/get` | `{}`, or `{ "project": "<id>" }` | Its settings, as the user chose them in the Settings tab of its page: an object by key of `contributes.settings`, without those left to their default. With a project (the `project.id` of a fact), those its [`.escouade.yml`](escouade-yml) gives it under `plugins.<id>` go first, if it is on in that project: only the keys it declares, with a value of their type, never a secret. |
| `log` (notification) | `{ "level": "info", "message": "…" }` | None: the line goes to the plugin’s journal. |
| `files/read` | `{ "path": "data/words.txt" }` | `"…"`, the text of a file of its folder, as UTF-8 text, 256 KB at most, on both transports. Refused (`-32003`, kept in the journal): an absolute path, with `..` or a drive, a link that leads out of the folder, a hidden name (`.env`, `.git`), the manifest. `-32005` for a missing file, `-32004` for a file too big or that is not text. |
| `net/fetch` | `{ "url": "https://…", "method": "POST", "headers": {}, "body": "…" }` | `{ "status": 200, "headers": {}, "body": "…" }`, for a plugin in WASM only: see [the network](#the-network-through-escouade). |
| `net/stream`, `ws/*`, `timer/*`, `fs/*`, `proc/*` | See [the calls](plugins/calls). | For a plugin in WASM only; a process gets `-32601`. |

Any other method answers the error `-32601` and notes it in the journal. Escouade’s own errors: `-32001` for a secret that is not the plugin’s, `-32002` for a keychain out of reach, `-32003` for what the plugin may not reach (a host, a file), `-32004` for a request that fails on the way, `-32005` for a missing file, `-32602` for parameters that do not hold.

## The secrets

A plugin’s secrets are in the system’s keychain, under a name of its own: `plugin:<id>:<key>`. It asks for them with `secrets/get`, when it needs them.

- **Only its own.** The key must be one of its `permissions.secrets`; a pattern ending with `*` takes in several (`account:*`). Any other key is refused with `-32001` and noted in the journal, even another plugin’s written in full: `plugin:dev.other:token` is not a key of its permissions.
- A plugin never gets the secret of an account, a harness or an integration of Escouade.
- A secret given to the plugin is masked (`••••`) everywhere in its journal, its standard error included.

## Crashes and restarts

- **A crash restarts the plugin**, three times within five minutes. At the fourth, it is stopped: “stopped after 4 crashes within 5 minutes”. It no longer starts by itself; the “Restart” button of its journal, in the view Plugins, starts it again, its crashes forgotten.
- **The calls under way** when it crashes fail: their default answer applies.
- **While it restarts**, what Escouade sends it waits in a queue of 256 messages, then goes in order once `initialize` is answered. A full queue loses its oldest message, and the journal says so.
- **What each agent had attached to it** (for a harness, `agent/attach`: its session, its mode, its model) is given again to the new process before any other message.

## The memory

Escouade reads the memory of the plugin and of everything it started every two seconds, as it does the agents’. Past 1 GB, the plugin is stopped, without a restart: “stopped: 1100 MB of memory, past the limit of 1024 MB”.

## When Escouade closes

When Escouade closes, it sends `shutdown` to each running plugin, waits for its answer a second at most for all of them, then stops its process and everything it started. Answer `shutdown`, then exit: what you have not written before is lost.

## The journal

Each plugin has its journal, in `~/.escouade/plugin-logs/<id>.log`; past 1 MB, it becomes `<id>.log.1` and starts over. Its lines are in English, like the app’s log, and note:

- its starts, its answer to `initialize`, its stops and their reason;
- its crashes, with their exit code and its last lines of standard error;
- each line of its standard error;
- its messages: its `log`s, the secrets it asked for (never their value);
- the calls left without an answer, what it asked for and was refused, the messages lost from a full queue.

The last 500 lines are read in the Journal tab of its page, in the view Plugins ([installing a plugin](plugins/installer#the-journal)).

Its methods, their parameters, their answers and their delays are described by a schema, [`plugin-protocol-1.json`](/schemas/plugin-protocol-1.json), where the types of the [TypeScript SDK](plugins/sdk) come from. To check that a plugin keeps to this protocol without starting the app, play [the conformance bench](plugins/testing): `escouade-plugin test <folder>`.

## A program in WASM

A plugin whose manifest says `"run": { "js": "dist/plugin.mjs" }` or `"run": { "wasm": "Cargo.toml" }` starts no process: Escouade runs its module **inside the app, in WASM**, with no Node on the machine. One instance per plugin, on a thread of its own, which keeps its state from one call to the next like a process. The protocol is the same: the same messages, carried by the module’s standard input and output, the same handshake, the same delays, the same journal and the same restarts; [the SDK](plugins/sdk) and [the Rust kit](plugins/rust) speak it for you.

The module is compiled once: by the store for a plugin of the store (the app downloads it and checks its fingerprint against `version.json`), by the app for a `run.js` plugin installed from a folder or a git repository. A `run.wasm` plugin outside the store comes with its `plugin.wasm`, beside the manifest. The app precompiles it at install, keeps it in its cache and precompiles it again after an update of Escouade. It needs Escouade 2.2.0 or later.

### The network, through Escouade

A plugin in WASM opens no connection: everything goes through Escouade, by `net/fetch`, `net/stream` and `ws/open`, which only a plugin in WASM gets (a process has its own network, and these methods answer it `-32601`). `fetch` in JS and the kit’s client in Rust lead there. Escouade sends the request:

- **over https only**, to a host of `permissions.network` or one of its subdomains (`slack.com` covers `api.slack.com`); never to an IP address written in the URL, which `permissions.network` does not name;
- **following no redirect**: the plugin gets the 3xx answer, which the kit’s client follows in Rust;
- **within 10 s, for an answer of 1 MiB at most**, unless `limits` asks for more;
- **never to an address of the machine or of the local network**, even for a declared host: loopback, private, link-local, unspecified, multicast or broadcast, in IPv4 as in IPv6. Escouade judges the addresses the connection itself uses, once the name is resolved, not a separate lookup the DNS could contradict;
- **through the proxy of Settings › Advanced › Network**, if there is one: only the name is judged then, and the journal says so.

`{account}`, the host of an account the user typed, and `local:shared`, a harness’s local server, open http and the local network for these hosts alone: see [the host’s capabilities](plugins/capabilities#the-network).

A refused call answers `-32003` and the journal keeps it (“slack.com resolves to 127.0.0.1, an address of this machine or of the local network”, “example.org is not in permissions.network of …”); a call that fails on the way (no answer in time, an answer too large, a host out of reach) answers `-32004`. The plugin sees a `fetch` that rejects.

### The files of its folder

- **The JSON files at its root** (`rules.json`, `channels.json`) are given to it at its start: 256 KB each, 1 MB in all, never its manifest. `readFileSync` of `node:fs` reads them, as under Node (`readFileSync(new URL('../rules.json', import.meta.url), 'utf8')`).
- **The others** are read by `files/read` (the SDK’s `p.file()`), as UTF-8 text, 256 KB at most, a subfolder included (`data/words.txt`).
- **The rest** goes through `fs/*`, under the roots of `permissions.files`: see [the files](plugins/capabilities#the-files-files).

### The limits

- **64 MB of memory, 2 s per turn of work**, 500 ms while it holds a decision.
- **An endless loop, a growing memory or an uncaught exception** end the instance, and the journal says why (“a turn ran past 2000 ms: Escouade stopped its instance”, “its instance crashed: …”); a new one is made, as after a process crashes, with the same three restarts in five minutes, then the plugin is stopped.
- **Timers**: `setTimeout` and `clearTimeout` exist in JS, `sleep` and `timeout` in the kit, 10 ms at least, 64 at once.
- **A bundle made with SDK 0.2** does not run in WASM: the plugin stops and its journal says to rebuild it with `@escouade-dev/plugin` 0.3 or later.

### What a plugin in WASM cannot do

Nothing it does not declare: no network, no file outside its folder, no program. It does not read the environment and loads no native or remote module. A bundle that imports anything other than `node:fs` and `node:readline`, or that has `export`s, is not compiled, and the plugin stops with the reason. Its secrets and settings are its own only, as for a process. That is why the install window says “Runs inside Escouade, isolated”, then each [capability](plugins/capabilities) it asks for, not “Runs a program on your computer, with your rights”.

## What the user changes

A plugin switched off, a permission taken back or given again, an update installed: its process is stopped, and the next one gets the manifest and the permissions of now. A secret’s permission taken back makes `secrets/get` answer `-32001` for that key, as for a secret that is not its own.
